//! Command and config file parser for tmux command syntax.
//!
//! Handles tokenization, command parsing, .tmux.conf files, and command chaining.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Command parse errors.
#[derive(Debug, Error)]
pub enum CmdParseError {
    /// Unterminated quoted string.
    #[error("unterminated quote at position {0}")]
    UnterminatedQuote(usize),
    /// Unknown command.
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    /// Wrong number of arguments.
    #[error("wrong argument count for '{command}': expected {min}-{max}, got {actual}")]
    ArgCount {
        command: String,
        min: usize,
        max: usize,
        actual: usize,
    },
    /// Invalid flag.
    #[error("invalid flag '{flag}' for command '{command}'")]
    InvalidFlag { command: String, flag: String },
}

/// A parsed token from a command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A word (possibly quoted).
    Word(String),
    /// Command separator (semicolon).
    Separator,
}

/// Tokenize a command string respecting quotes and escapes.
pub fn tokenize(input: &str) -> Result<Vec<Token>, CmdParseError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut pos = 0usize;

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' => {
                chars.next();
                pos += 1;
            }
            '#' => {
                // Comment: skip to end of line
                break;
            }
            ';' => {
                tokens.push(Token::Separator);
                chars.next();
                pos += 1;
            }
            '\'' => {
                chars.next();
                pos += 1;
                let start = pos;
                let mut word = String::new();
                loop {
                    match chars.next() {
                        Some('\'') => {
                            pos += 1;
                            break;
                        }
                        Some(c) => {
                            word.push(c);
                            pos += c.len_utf8();
                        }
                        None => return Err(CmdParseError::UnterminatedQuote(start)),
                    }
                }
                tokens.push(Token::Word(word));
            }
            '"' => {
                chars.next();
                pos += 1;
                let start = pos;
                let mut word = String::new();
                loop {
                    match chars.next() {
                        Some('"') => {
                            pos += 1;
                            break;
                        }
                        Some('\\') => {
                            pos += 1;
                            if let Some(escaped) = chars.next() {
                                pos += escaped.len_utf8();
                                match escaped {
                                    'n' => word.push('\n'),
                                    't' => word.push('\t'),
                                    '\\' => word.push('\\'),
                                    '"' => word.push('"'),
                                    other => {
                                        word.push('\\');
                                        word.push(other);
                                    }
                                }
                            }
                        }
                        Some(c) => {
                            word.push(c);
                            pos += c.len_utf8();
                        }
                        None => return Err(CmdParseError::UnterminatedQuote(start)),
                    }
                }
                tokens.push(Token::Word(word));
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ' ' || c == '\t' || c == ';' || c == '#' {
                        break;
                    }
                    word.push(c);
                    chars.next();
                    pos += c.len_utf8();
                }
                tokens.push(Token::Word(word));
            }
        }
    }

    Ok(tokens)
}

/// A parsed command with name, flags, and arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    /// Command name.
    pub name: String,
    /// Single-character flags (e.g., -d, -p).
    pub flags: Vec<char>,
    /// Positional arguments.
    pub args: Vec<String>,
    /// Flag arguments (flags that take a value).
    pub flag_args: Vec<(char, String)>,
}

/// Parse a sequence of tokens into a command.
pub fn parse_command(tokens: &[Token]) -> Result<ParsedCommand, CmdParseError> {
    let words: Vec<&str> = tokens
        .iter()
        .filter_map(|t| match t {
            Token::Word(w) => Some(w.as_str()),
            Token::Separator => None,
        })
        .collect();

    if words.is_empty() {
        return Err(CmdParseError::UnknownCommand(String::new()));
    }

    let name = words[0].to_owned();
    let mut flags = Vec::new();
    let mut args = Vec::new();
    let mut flag_args = Vec::new();
    let mut i = 1;

    while i < words.len() {
        let w = words[i];
        if let Some(flag_chars) = w.strip_prefix('-') {
            if flag_chars.is_empty() {
                // Bare '-' means end of flags
                i += 1;
                break;
            }
            for c in flag_chars.chars() {
                // Check if this flag takes an argument (simplified: t, s, F flags take args)
                if matches!(c, 't' | 's' | 'F' | 'f' | 'c') && i + 1 < words.len() {
                    i += 1;
                    flag_args.push((c, words[i].to_owned()));
                } else {
                    flags.push(c);
                }
            }
        } else {
            args.push(w.to_owned());
        }
        i += 1;
    }

    // Remaining words are args
    while i < words.len() {
        args.push(words[i].to_owned());
        i += 1;
    }

    Ok(ParsedCommand {
        name,
        flags,
        args,
        flag_args,
    })
}

/// Split a command string into individual commands (separated by `;`).
pub fn split_commands(input: &str) -> Result<Vec<Vec<Token>>, CmdParseError> {
    let tokens = tokenize(input)?;
    let mut commands: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();

    for token in tokens {
        if token == Token::Separator {
            if !current.is_empty() {
                commands.push(current);
                current = Vec::new();
            }
        } else {
            current.push(token);
        }
    }

    if !current.is_empty() {
        commands.push(current);
    }

    Ok(commands)
}

/// Parse a .tmux.conf file into a list of commands.
pub fn parse_config(content: &str) -> Result<Vec<ParsedCommand>, CmdParseError> {
    let mut commands = Vec::new();
    let mut continued = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(prefix) = trimmed.strip_suffix('\\') {
            continued.push_str(prefix);
            continued.push(' ');
            continue;
        }

        let full_line = if continued.is_empty() {
            trimmed.to_owned()
        } else {
            continued.push_str(trimmed);
            let result = continued.clone();
            continued.clear();
            result
        };

        let cmd_groups = split_commands(&full_line)?;
        for tokens in cmd_groups {
            commands.push(parse_command(&tokens)?);
        }
    }

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple() {
        let tokens = tokenize("set -g mouse on").unwrap_or_default();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], Token::Word("set".into()));
    }

    #[test]
    fn tokenize_single_quotes() {
        let tokens = tokenize("bind 'x' kill-pane").unwrap_or_default();
        assert_eq!(tokens[1], Token::Word("x".into()));
    }

    #[test]
    fn tokenize_double_quotes() {
        let tokens = tokenize(r#"set -g status-left "hello world""#).unwrap_or_default();
        assert_eq!(tokens[3], Token::Word("hello world".into()));
    }

    #[test]
    fn tokenize_escape_in_double_quotes() {
        let tokens = tokenize(r#""hello\"world""#).unwrap_or_default();
        assert_eq!(tokens[0], Token::Word("hello\"world".into()));
    }

    #[test]
    fn tokenize_semicolon_separator() {
        let tokens = tokenize("cmd1 ; cmd2").unwrap_or_default();
        assert!(tokens.contains(&Token::Separator));
    }

    #[test]
    fn tokenize_comment() {
        let tokens = tokenize("cmd arg # comment").unwrap_or_default();
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn unterminated_single_quote() {
        assert!(tokenize("'unterminated").is_err());
    }

    #[test]
    fn unterminated_double_quote() {
        assert!(tokenize("\"unterminated").is_err());
    }

    #[test]
    fn parse_simple_command() {
        let tokens = tokenize("new-session -d -s main").unwrap_or_default();
        let cmd = parse_command(&tokens).unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            flags: vec![],
            args: vec![],
            flag_args: vec![],
        });
        assert_eq!(cmd.name, "new-session");
        assert!(cmd.flags.contains(&'d'));
    }

    #[test]
    fn parse_command_with_flag_arg() {
        let tokens = tokenize("new-session -s mysession").unwrap_or_default();
        let cmd = parse_command(&tokens).unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            flags: vec![],
            args: vec![],
            flag_args: vec![],
        });
        assert_eq!(cmd.flag_args.len(), 1);
        assert_eq!(cmd.flag_args[0], ('s', "mysession".into()));
    }

    #[test]
    fn split_commands_multiple() {
        let cmds = split_commands("cmd1 ; cmd2 ; cmd3").unwrap_or_default();
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn parse_config_simple() {
        let content = "set -g mouse on\nbind r source-file ~/.tmux.conf\n";
        let cmds = parse_config(content).unwrap_or_default();
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn parse_config_comments() {
        let content = "# This is a comment\nset -g status on\n";
        let cmds = parse_config(content).unwrap_or_default();
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn parse_config_empty_lines() {
        let content = "\n\nset -g mouse off\n\n";
        let cmds = parse_config(content).unwrap_or_default();
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn parse_config_continuation() {
        let content = "set -g status-left \\\n  'hello'\n";
        let cmds = parse_config(content).unwrap_or_default();
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn empty_input() {
        let tokens = tokenize("").unwrap_or_default();
        assert!(tokens.is_empty());
    }

    #[test]
    fn parse_command_no_args() {
        let tokens = tokenize("list-sessions").unwrap_or_default();
        let cmd = parse_command(&tokens).unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            flags: vec![],
            args: vec![],
            flag_args: vec![],
        });
        assert_eq!(cmd.name, "list-sessions");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn command_with_positional_args() {
        let tokens = tokenize("send-keys ls Enter").unwrap_or_default();
        let cmd = parse_command(&tokens).unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            flags: vec![],
            args: vec![],
            flag_args: vec![],
        });
        assert_eq!(cmd.args, vec!["ls", "Enter"]);
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn tokenize_never_panics(input in "[ -~]{0,100}") {
                let _ = tokenize(&input);
            }

            #[test]
            fn command_name_preserved(name in "[a-z-]{1,20}") {
                let tokens = vec![Token::Word(name.clone())];
                let cmd = parse_command(&tokens);
                if let Ok(c) = cmd {
                    prop_assert_eq!(c.name, name);
                }
            }
        }
    }
}
