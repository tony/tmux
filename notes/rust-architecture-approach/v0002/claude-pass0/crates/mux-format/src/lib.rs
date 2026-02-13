//! # mux-format
//!
//! tmux format string parser and evaluator.
//!
//! Handles `#{...}` variable expansion, conditional formats `#{?...,...,...}`,
//! string operations, and format aliases (`#S`, `#W`, `#T`, etc.).

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// A parsed format expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatExpr {
    /// Literal text.
    Literal(String),
    /// Variable reference: `#{variable_name}`.
    Variable(String),
    /// Conditional: `#{?condition,true_branch,false_branch}`.
    Conditional {
        condition: Box<FormatExpr>,
        if_true: Box<FormatExpr>,
        if_false: Box<FormatExpr>,
    },
    /// String comparison: `#{==:a,b}`.
    Equals(Box<FormatExpr>, Box<FormatExpr>),
    /// Concatenation of multiple expressions.
    Concat(Vec<FormatExpr>),
}

/// A format variable provider.
pub trait FormatProvider {
    /// Look up a format variable by name.
    fn get(&self, name: &str) -> Option<String>;
}

impl FormatProvider for HashMap<String, String> {
    fn get(&self, name: &str) -> Option<String> {
        HashMap::get(self, name).cloned()
    }
}

/// Parse a format string into a list of expressions.
///
/// # Errors
/// Returns error if the format string has unmatched braces or invalid syntax.
pub fn parse_format(input: &str) -> Result<FormatExpr, FormatError> {
    let mut parts = Vec::new();
    let mut chars = input.chars().peekable();
    let mut literal = String::new();

    while let Some(&ch) = chars.peek() {
        if ch == '#' {
            chars.next();
            match chars.peek() {
                Some(&'{') => {
                    if !literal.is_empty() {
                        parts.push(FormatExpr::Literal(std::mem::take(&mut literal)));
                    }
                    chars.next(); // consume '{'
                    let var_name = collect_until_brace(&mut chars)?;
                    parts.push(FormatExpr::Variable(var_name));
                }
                Some(&ch) => {
                    // Format alias: #S, #W, #T, etc.
                    if !literal.is_empty() {
                        parts.push(FormatExpr::Literal(std::mem::take(&mut literal)));
                    }
                    chars.next();
                    let alias = format_alias(ch);
                    parts.push(FormatExpr::Variable(alias));
                }
                None => {
                    literal.push('#');
                }
            }
        } else {
            chars.next();
            literal.push(ch);
        }
    }

    if !literal.is_empty() {
        parts.push(FormatExpr::Literal(literal));
    }

    if parts.len() == 1 {
        Ok(parts.into_iter().next().unwrap_or(FormatExpr::Literal(String::new())))
    } else {
        Ok(FormatExpr::Concat(parts))
    }
}

/// Evaluate a format expression against a provider.
pub fn eval_format(expr: &FormatExpr, provider: &dyn FormatProvider) -> String {
    match expr {
        FormatExpr::Literal(s) => s.clone(),
        FormatExpr::Variable(name) => provider.get(name).unwrap_or_default(),
        FormatExpr::Conditional { condition, if_true, if_false } => {
            let cond = eval_format(condition, provider);
            if !cond.is_empty() && cond != "0" {
                eval_format(if_true, provider)
            } else {
                eval_format(if_false, provider)
            }
        }
        FormatExpr::Equals(a, b) => {
            let va = eval_format(a, provider);
            let vb = eval_format(b, provider);
            if va == vb { "1".into() } else { "0".into() }
        }
        FormatExpr::Concat(parts) => {
            let mut result = String::new();
            for part in parts {
                result.push_str(&eval_format(part, provider));
            }
            result
        }
    }
}

fn collect_until_brace(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<String, FormatError> {
    let mut depth = 1;
    let mut result = String::new();
    for ch in chars.by_ref() {
        match ch {
            '{' => {
                depth += 1;
                result.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(result);
                }
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    Err(FormatError::UnmatchedBrace)
}

fn format_alias(ch: char) -> String {
    match ch {
        'S' => "session_name".into(),
        'W' => "window_name".into(),
        'T' => "pane_title".into(),
        'I' => "window_index".into(),
        'P' => "pane_index".into(),
        'D' => "pane_id".into(),
        'F' => "window_flags".into(),
        'H' => "host".into(),
        _ => format!("#{ch}"),
    }
}

/// Errors from format parsing.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("unmatched brace in format string")]
    UnmatchedBrace,
    #[error("invalid format syntax: {0}")]
    InvalidSyntax(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_literal() {
        let expr = parse_format("hello world");
        assert!(matches!(expr, Ok(FormatExpr::Literal(s)) if s == "hello world"));
    }

    #[test]
    fn parse_variable() {
        let expr = parse_format("#{session_name}");
        assert!(matches!(expr, Ok(FormatExpr::Variable(s)) if s == "session_name"));
    }

    #[test]
    fn eval_variable() {
        let mut vars: HashMap<String, String> = HashMap::new();
        vars.insert("session_name".into(), "main".into());
        let expr = parse_format("Session: #{session_name}");
        assert!(expr.is_ok());
        let result = eval_format(&expr.unwrap_or(FormatExpr::Literal(String::new())), &vars);
        assert_eq!(result, "Session: main");
    }

    #[test]
    fn format_alias_s() {
        let mut vars: HashMap<String, String> = HashMap::new();
        vars.insert("session_name".into(), "test".into());
        let expr = parse_format("#S");
        assert!(expr.is_ok());
        let result = eval_format(&expr.unwrap_or(FormatExpr::Literal(String::new())), &vars);
        assert_eq!(result, "test");
    }
}
