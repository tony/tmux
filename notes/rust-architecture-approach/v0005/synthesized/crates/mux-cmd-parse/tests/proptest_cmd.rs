//! Property-based tests for mux-cmd-parse.

use proptest::prelude::*;
use mux_cmd_parse::{tokenize, parse_command, parse_config, Token};

proptest! {
    #[test]
    fn tokenize_simple_command(cmd in "[a-z-]{1,20}") {
        let tokens = tokenize(&cmd).unwrap_or_default();
        prop_assert!(!tokens.is_empty());
        if let Some(Token::Word(w)) = tokens.first() {
            prop_assert_eq!(w, &cmd);
        }
    }

    #[test]
    fn tokenize_with_args(
        cmd in "[a-z-]{1,10}",
        args in proptest::collection::vec("[a-zA-Z0-9]{1,10}", 0..5),
    ) {
        let input = std::iter::once(cmd)
            .chain(args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        let tokens = tokenize(&input).unwrap_or_default();
        prop_assert_eq!(tokens.len(), 1 + args.len());
    }

    #[test]
    fn tokenize_quoted_string(content in "[a-zA-Z0-9 ]{1,20}") {
        let input = format!("\"{content}\"");
        let tokens = tokenize(&input).unwrap_or_default();
        prop_assert!(!tokens.is_empty());
    }

    #[test]
    fn parse_set_command(key in "[a-z-]{1,15}", value in "[a-zA-Z0-9]{1,15}") {
        let input = format!("set -g {key} {value}");
        let tokens = tokenize(&input).unwrap_or_default();
        let result = parse_command(&tokens);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn parse_bind_command(key_char in proptest::char::range('a', 'z'), cmd in "[a-z-]{1,10}") {
        let input = format!("bind {key_char} {cmd}");
        let tokens = tokenize(&input).unwrap_or_default();
        let result = parse_command(&tokens);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn tokenize_preserves_order(words in proptest::collection::vec("[a-z]{1,5}", 1..8)) {
        let input = words.join(" ");
        let tokens = tokenize(&input).unwrap_or_default();
        for (i, token) in tokens.iter().enumerate() {
            if let Token::Word(w) = token {
                prop_assert_eq!(w, &words[i]);
            }
        }
    }
}

#[test]
fn parse_empty_input() {
    let tokens = tokenize("").unwrap_or_default();
    // Empty input yields no tokens; parse_command would fail
    assert!(tokens.is_empty());
}

#[test]
fn parse_comment_line() {
    let tokens = tokenize("# this is a comment").unwrap_or_default();
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_semicolons() {
    let tokens = tokenize("set -g status on ; set -g prefix C-a").unwrap_or_default();
    // Should contain multiple words and a separator
    assert!(tokens.len() >= 7);
}

#[test]
fn parse_unbind_command() {
    let tokens = tokenize("unbind C-b").unwrap_or_default();
    let result = parse_command(&tokens);
    assert!(result.is_ok());
}

#[test]
fn parse_source_file() {
    let tokens = tokenize("source-file ~/.tmux.conf").unwrap_or_default();
    let result = parse_command(&tokens);
    assert!(result.is_ok());
}

#[test]
fn tokenize_single_quotes() {
    let tokens = tokenize("set -g status-left 'hello world'").unwrap_or_default();
    assert!(tokens.len() >= 3);
}

#[test]
fn parse_set_option() {
    let tokens = tokenize("set-option -g history-limit 10000").unwrap_or_default();
    let result = parse_command(&tokens);
    assert!(result.is_ok());
}
