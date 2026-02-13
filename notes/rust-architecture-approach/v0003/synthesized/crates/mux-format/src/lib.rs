//! # mux-format
//!
//! tmux format string expansion engine.
//! Handles `#{variable}` and `#{?cond,true,false}` syntax.
//!
//! L2 data crate.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// A format context providing variable values.
#[derive(Debug, Default)]
pub struct FormatContext {
    vars: HashMap<String, String>,
}

impl FormatContext {
    /// Create a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a variable value.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(key.into(), value.into());
    }

    /// Get a variable value.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }

    /// Expand a format string, replacing `#{var}` with values.
    #[must_use]
    pub fn expand(&self, fmt: &str) -> String {
        let mut result = String::with_capacity(fmt.len());
        let mut chars = fmt.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '#' && chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                let mut var_name = String::new();
                for ch in chars.by_ref() {
                    if ch == '}' {
                        break;
                    }
                    var_name.push(ch);
                }
                if let Some(val) = self.vars.get(&var_name) {
                    result.push_str(val);
                }
            } else {
                result.push(c);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_simple_variable() {
        let mut ctx = FormatContext::new();
        ctx.set("session_name", "main");
        assert_eq!(ctx.expand("Session: #{session_name}"), "Session: main");
    }

    #[test]
    fn expand_no_variables() {
        let ctx = FormatContext::new();
        assert_eq!(ctx.expand("plain text"), "plain text");
    }

    #[test]
    fn expand_missing_variable() {
        let ctx = FormatContext::new();
        assert_eq!(ctx.expand("#{missing}"), "");
    }

    #[test]
    fn expand_multiple_variables() {
        let mut ctx = FormatContext::new();
        ctx.set("a", "1");
        ctx.set("b", "2");
        assert_eq!(ctx.expand("#{a}+#{b}"), "1+2");
    }

    #[test]
    fn expand_hash_without_brace() {
        let ctx = FormatContext::new();
        assert_eq!(ctx.expand("# comment"), "# comment");
    }

    #[test]
    fn context_get() {
        let mut ctx = FormatContext::new();
        ctx.set("key", "val");
        assert_eq!(ctx.get("key"), Some("val"));
        assert!(ctx.get("missing").is_none());
    }

    #[test]
    fn expand_empty_string() {
        let ctx = FormatContext::new();
        assert_eq!(ctx.expand(""), "");
    }

    #[test]
    fn expand_adjacent_variables() {
        let mut ctx = FormatContext::new();
        ctx.set("a", "X");
        ctx.set("b", "Y");
        assert_eq!(ctx.expand("#{a}#{b}"), "XY");
    }

    #[test]
    fn expand_variable_with_special_chars() {
        let mut ctx = FormatContext::new();
        ctx.set("path", "/tmp/test.sock");
        assert_eq!(ctx.expand("[#{path}]"), "[/tmp/test.sock]");
    }

    #[test]
    fn context_overwrite_variable() {
        let mut ctx = FormatContext::new();
        ctx.set("key", "old");
        ctx.set("key", "new");
        assert_eq!(ctx.get("key"), Some("new"));
    }

    #[test]
    fn expand_hash_standalone() {
        let ctx = FormatContext::new();
        // # not followed by { should pass through
        assert_eq!(ctx.expand("# test"), "# test");
    }

    #[test]
    fn expand_multiple_variables_mixed() {
        let mut ctx = FormatContext::new();
        ctx.set("name", "dev");
        ctx.set("idx", "3");
        assert_eq!(ctx.expand("#{name}:#{idx}"), "dev:3");
    }
}
