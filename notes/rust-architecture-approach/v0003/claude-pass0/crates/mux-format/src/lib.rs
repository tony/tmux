//! # mux-format
//!
//! Format string parser and evaluator for tmux-style `#{}` expressions.
//!
//! Supports:
//! - `#S` -> session name
//! - `#W` -> window name
//! - `#I` -> window index
//! - `#P` -> pane index
//! - `#{pane_title}` -> variable lookup
//! - `#[fg=red]` -> style directives (passed through)
//! - Literal `##` -> `#`
//!
//! L2 data crate -- no internal dependencies.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use thiserror::Error;

/// Format evaluation errors.
#[derive(Debug, Error)]
pub enum FormatError {
    /// Unterminated format expression.
    #[error("unterminated format expression at position {0}")]
    Unterminated(usize),
}

/// A parsed format token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatToken {
    /// Literal text.
    Literal(String),
    /// A short-form variable (#S, #W, etc.).
    ShortVar(char),
    /// A long-form variable (#{name}).
    LongVar(String),
    /// A style directive (#[...]).
    Style(String),
}

/// Parse a tmux format string into tokens.
///
/// # Errors
///
/// Returns `FormatError::Unterminated` for unmatched `#{` or `#[`.
pub fn parse_format(input: &str) -> Result<Vec<FormatToken>, FormatError> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();
    let mut literal = String::new();

    while let Some((pos, c)) = chars.next() {
        if c == '#' {
            if let Some(&(_, next)) = chars.peek() {
                match next {
                    '#' => {
                        chars.next();
                        literal.push('#');
                    }
                    '{' => {
                        if !literal.is_empty() {
                            tokens.push(FormatToken::Literal(std::mem::take(&mut literal)));
                        }
                        chars.next(); // consume '{'
                        let mut var_name = String::new();
                        let mut found_close = false;
                        for (_, vc) in chars.by_ref() {
                            if vc == '}' {
                                found_close = true;
                                break;
                            }
                            var_name.push(vc);
                        }
                        if !found_close {
                            return Err(FormatError::Unterminated(pos));
                        }
                        tokens.push(FormatToken::LongVar(var_name));
                    }
                    '[' => {
                        if !literal.is_empty() {
                            tokens.push(FormatToken::Literal(std::mem::take(&mut literal)));
                        }
                        chars.next(); // consume '['
                        let mut style = String::new();
                        let mut found_close = false;
                        for (_, sc) in chars.by_ref() {
                            if sc == ']' {
                                found_close = true;
                                break;
                            }
                            style.push(sc);
                        }
                        if !found_close {
                            return Err(FormatError::Unterminated(pos));
                        }
                        tokens.push(FormatToken::Style(style));
                    }
                    _ => {
                        if !literal.is_empty() {
                            tokens.push(FormatToken::Literal(std::mem::take(&mut literal)));
                        }
                        chars.next();
                        tokens.push(FormatToken::ShortVar(next));
                    }
                }
            } else {
                literal.push('#');
            }
        } else {
            literal.push(c);
        }
    }

    if !literal.is_empty() {
        tokens.push(FormatToken::Literal(literal));
    }

    Ok(tokens)
}

/// Evaluate a parsed format against a variable map.
#[must_use]
pub fn evaluate_format(tokens: &[FormatToken], vars: &HashMap<String, String>) -> String {
    let mut output = String::new();
    for token in tokens {
        match token {
            FormatToken::Literal(s) => output.push_str(s),
            FormatToken::ShortVar(c) => {
                let key = match c {
                    'S' => "session_name",
                    'W' => "window_name",
                    'I' => "window_index",
                    'P' => "pane_index",
                    'T' => "pane_title",
                    'H' => "host",
                    _ => "",
                };
                if let Some(val) = vars.get(key) {
                    output.push_str(val);
                }
            }
            FormatToken::LongVar(name) => {
                if let Some(val) = vars.get(name.as_str()) {
                    output.push_str(val);
                }
            }
            FormatToken::Style(s) => {
                output.push_str("#[");
                output.push_str(s);
                output.push(']');
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_literal() {
        let tokens = parse_format("hello").unwrap_or_default();
        assert_eq!(tokens, vec![FormatToken::Literal("hello".into())]);
    }

    #[test]
    fn parse_short_var() {
        let tokens = parse_format("#S").unwrap_or_default();
        assert_eq!(tokens, vec![FormatToken::ShortVar('S')]);
    }

    #[test]
    fn parse_long_var() {
        let tokens = parse_format("#{pane_title}").unwrap_or_default();
        assert_eq!(tokens, vec![FormatToken::LongVar("pane_title".into())]);
    }

    #[test]
    fn parse_escaped_hash() {
        let tokens = parse_format("##").unwrap_or_default();
        assert_eq!(tokens, vec![FormatToken::Literal("#".into())]);
    }

    #[test]
    fn parse_style() {
        let tokens = parse_format("#[fg=red]").unwrap_or_default();
        assert_eq!(tokens, vec![FormatToken::Style("fg=red".into())]);
    }

    #[test]
    fn parse_mixed() {
        let tokens = parse_format("[#S] #I:#W").unwrap_or_default();
        // "[" ShortVar(S) "] " ShortVar(I) ":" ShortVar(W) = 6 tokens
        assert_eq!(tokens.len(), 6);
    }

    #[test]
    fn parse_unterminated_long_var() {
        assert!(parse_format("#{unclosed").is_err());
    }

    #[test]
    fn evaluate_short_vars() {
        let tokens = parse_format("#S:#I").unwrap_or_default();
        let mut vars = HashMap::new();
        vars.insert("session_name".into(), "main".into());
        vars.insert("window_index".into(), "1".into());
        assert_eq!(evaluate_format(&tokens, &vars), "main:1");
    }

    #[test]
    fn evaluate_long_var() {
        let tokens = parse_format("#{pane_title}").unwrap_or_default();
        let mut vars = HashMap::new();
        vars.insert("pane_title".into(), "vim".into());
        assert_eq!(evaluate_format(&tokens, &vars), "vim");
    }

    #[test]
    fn evaluate_missing_var() {
        let tokens = parse_format("#{nonexistent}").unwrap_or_default();
        let vars = HashMap::new();
        assert_eq!(evaluate_format(&tokens, &vars), "");
    }

    #[test]
    fn parse_trailing_hash() {
        let tokens = parse_format("end#").unwrap_or_default();
        assert_eq!(tokens, vec![FormatToken::Literal("end#".into())]);
    }
}
