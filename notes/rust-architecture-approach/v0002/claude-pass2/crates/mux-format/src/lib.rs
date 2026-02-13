//! # mux-format
//!
//! tmux format string parser and evaluator.
//! Handles `#{...}` variable expansion, conditional formats, and aliases.

#![forbid(unsafe_code)]

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatExpr {
    Literal(String),
    Variable(String),
    Conditional {
        condition: Box<FormatExpr>,
        if_true: Box<FormatExpr>,
        if_false: Box<FormatExpr>,
    },
    Equals(Box<FormatExpr>, Box<FormatExpr>),
    Concat(Vec<FormatExpr>),
}

pub trait FormatProvider {
    fn get(&self, name: &str) -> Option<String>;
}

impl FormatProvider for HashMap<String, String> {
    fn get(&self, name: &str) -> Option<String> {
        HashMap::get(self, name).cloned()
    }
}

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
                    chars.next();
                    let var = collect_until_brace(&mut chars)?;
                    parts.push(FormatExpr::Variable(var));
                }
                Some(&c) => {
                    if !literal.is_empty() {
                        parts.push(FormatExpr::Literal(std::mem::take(&mut literal)));
                    }
                    chars.next();
                    parts.push(FormatExpr::Variable(format_alias(c)));
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

pub fn eval_format(expr: &FormatExpr, provider: &dyn FormatProvider) -> String {
    match expr {
        FormatExpr::Literal(s) => s.clone(),
        FormatExpr::Variable(name) => provider.get(name).unwrap_or_default(),
        FormatExpr::Conditional { condition, if_true, if_false } => {
            let c = eval_format(condition, provider);
            if !c.is_empty() && c != "0" {
                eval_format(if_true, provider)
            } else {
                eval_format(if_false, provider)
            }
        }
        FormatExpr::Equals(a, b) => {
            if eval_format(a, provider) == eval_format(b, provider) {
                "1".into()
            } else {
                "0".into()
            }
        }
        FormatExpr::Concat(parts) => parts.iter().map(|p| eval_format(p, provider)).collect(),
    }
}

fn collect_until_brace(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<String, FormatError> {
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
        'S' => "session_name",
        'W' => "window_name",
        'T' => "pane_title",
        'I' => "window_index",
        'P' => "pane_index",
        'D' => "pane_id",
        'F' => "window_flags",
        'H' => "host",
        _ => return format!("#{ch}"),
    }
    .into()
}

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
        assert!(matches!(
            parse_format("hello"),
            Ok(FormatExpr::Literal(s)) if s == "hello"
        ));
    }

    #[test]
    fn parse_variable() {
        assert!(matches!(
            parse_format("#{session_name}"),
            Ok(FormatExpr::Variable(s)) if s == "session_name"
        ));
    }

    #[test]
    fn eval_variable() {
        let mut vars: HashMap<String, String> = HashMap::new();
        vars.insert("session_name".into(), "main".into());
        let expr = parse_format("Session: #{session_name}")
            .unwrap_or(FormatExpr::Literal(String::new()));
        assert_eq!(eval_format(&expr, &vars), "Session: main");
    }

    #[test]
    fn format_alias_s() {
        let mut vars: HashMap<String, String> = HashMap::new();
        vars.insert("session_name".into(), "test".into());
        let expr = parse_format("#S").unwrap_or(FormatExpr::Literal(String::new()));
        assert_eq!(eval_format(&expr, &vars), "test");
    }

    #[test]
    fn unmatched_brace() {
        assert!(parse_format("#{unclosed").is_err());
    }
}
