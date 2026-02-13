//! tmux format string `#{}` evaluator.
//!
//! Supports variable expansion, conditionals, comparisons, and substitutions.
//! Invalid format tokens pass through unchanged (matching tmux behavior).

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// Context for format string evaluation.
#[derive(Debug, Clone, Default)]
pub struct FormatContext {
    vars: HashMap<String, String>,
}

impl FormatContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a variable value.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(key.into(), value.into());
    }

    /// Get a variable value.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }
}

/// Expand a format string using the given context.
///
/// Format syntax:
/// - `#{variable}` - variable substitution
/// - `#{?test,true_val,false_val}` - conditional
/// - `#{==:left,right}` - equality comparison
/// - `##{` - literal `#{`
pub fn expand(format: &str, ctx: &FormatContext) -> String {
    let mut result = String::with_capacity(format.len());
    let bytes = format.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'#' && bytes[i + 1] == b'#' {
            // Escaped hash
            result.push('#');
            i += 2;
            continue;
        }

        if i + 2 < bytes.len() && bytes[i] == b'#' && bytes[i + 1] == b'{' {
            // Find matching closing brace
            if let Some(end) = find_matching_brace(format, i + 2) {
                let expr = &format[i + 2..end];
                result.push_str(&eval_expr(expr, ctx));
                i = end + 1;
                continue;
            }
        }

        result.push(bytes[i] as char);
        i += 1;
    }

    result
}

fn find_matching_brace(s: &str, start: usize) -> Option<usize> {
    let mut depth = 1;
    let bytes = s.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn eval_expr(expr: &str, ctx: &FormatContext) -> String {
    // Conditional: ?test,true_val,false_val
    if let Some(rest) = expr.strip_prefix('?') {
        if let Some(comma1) = rest.find(',') {
            let test = &rest[..comma1];
            let remaining = &rest[comma1 + 1..];
            if let Some(comma2) = remaining.find(',') {
                let true_val = &remaining[..comma2];
                let false_val = &remaining[comma2 + 1..];
                let test_result = eval_expr(test, ctx);
                return if !test_result.is_empty() && test_result != "0" {
                    expand(true_val, ctx)
                } else {
                    expand(false_val, ctx)
                };
            }
        }
    }

    // Equality: ==:left,right
    if let Some(rest) = expr.strip_prefix("==:") {
        if let Some(comma) = rest.find(',') {
            let left = expand(&rest[..comma], ctx);
            let right = expand(&rest[comma + 1..], ctx);
            return if left == right {
                "1".to_owned()
            } else {
                "0".to_owned()
            };
        }
    }

    // Variable lookup
    ctx.get(expr).unwrap_or("").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        let ctx = FormatContext::new();
        assert_eq!(expand("hello world", &ctx), "hello world");
    }

    #[test]
    fn variable_substitution() {
        let mut ctx = FormatContext::new();
        ctx.set("session_name", "dev");
        assert_eq!(expand("#{session_name}", &ctx), "dev");
    }

    #[test]
    fn missing_variable() {
        let ctx = FormatContext::new();
        assert_eq!(expand("#{unknown}", &ctx), "");
    }

    #[test]
    fn escaped_hash() {
        let ctx = FormatContext::new();
        assert_eq!(expand("##{literal}", &ctx), "#{literal}");
    }

    #[test]
    fn conditional_true() {
        let mut ctx = FormatContext::new();
        ctx.set("flag", "1");
        assert_eq!(expand("#{?flag,yes,no}", &ctx), "yes");
    }

    #[test]
    fn conditional_false() {
        let mut ctx = FormatContext::new();
        ctx.set("flag", "0");
        assert_eq!(expand("#{?flag,yes,no}", &ctx), "no");
    }

    #[test]
    fn conditional_empty_is_false() {
        let ctx = FormatContext::new();
        assert_eq!(expand("#{?flag,yes,no}", &ctx), "no");
    }

    #[test]
    fn equality_true() {
        let mut ctx = FormatContext::new();
        ctx.set("x", "abc");
        assert_eq!(expand("#{==:#{x},abc}", &ctx), "1");
    }

    #[test]
    fn equality_false() {
        let mut ctx = FormatContext::new();
        ctx.set("x", "abc");
        assert_eq!(expand("#{==:#{x},xyz}", &ctx), "0");
    }

    #[test]
    fn multiple_variables() {
        let mut ctx = FormatContext::new();
        ctx.set("a", "hello");
        ctx.set("b", "world");
        assert_eq!(expand("#{a} #{b}", &ctx), "hello world");
    }

    #[test]
    fn format_context_new() {
        let ctx = FormatContext::new();
        assert!(ctx.get("anything").is_none());
    }

    #[test]
    fn unmatched_brace_passes_through() {
        let ctx = FormatContext::new();
        assert_eq!(expand("#{unclosed", &ctx), "#{unclosed");
    }

    #[test]
    fn mixed_text_and_vars() {
        let mut ctx = FormatContext::new();
        ctx.set("name", "test");
        ctx.set("idx", "3");
        assert_eq!(
            expand("[#{name}:#{idx}]", &ctx),
            "[test:3]"
        );
    }

    #[test]
    fn context_overwrite() {
        let mut ctx = FormatContext::new();
        ctx.set("x", "old");
        ctx.set("x", "new");
        assert_eq!(ctx.get("x"), Some("new"));
    }

    #[test]
    fn nested_variable_in_conditional() {
        let mut ctx = FormatContext::new();
        ctx.set("active", "1");
        ctx.set("name", "dev");
        assert_eq!(expand("#{?active,#{name},none}", &ctx), "dev");
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn expand_never_panics(s in ".*") {
                let ctx = FormatContext::new();
                let _ = expand(&s, &ctx);
            }

            #[test]
            fn plain_text_passthrough(s in "[a-zA-Z0-9 ]{0,50}") {
                let ctx = FormatContext::new();
                prop_assert_eq!(expand(&s, &ctx), s);
            }

            #[test]
            fn variable_expansion_idempotent(key in "[a-z]{1,10}", val in "[a-z]{1,10}") {
                let mut ctx = FormatContext::new();
                ctx.set(key.clone(), val.clone());
                let fmt = format!("#{{{key}}}");
                let result = expand(&fmt, &ctx);
                prop_assert_eq!(result, val);
            }
        }
    }
}
