//! # mux-cmd-parse
//!
//! Parser for tmux command strings and `.tmux.conf` files.
//!
//! Features:
//! - `#` comment lines
//! - Quoted strings (single and double)
//! - Backslash continuation
//! - Semicolon command chaining
//!
//! L2 data crate -- no internal dependencies.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Command parse errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CmdParseError {
    /// Unterminated quoted string.
    #[error("unterminated quote at line {line}")]
    UnterminatedQuote { line: usize },
    /// Empty command.
    #[error("empty command at line {line}")]
    EmptyCommand { line: usize },
}

/// A parsed command with its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    /// The command name (e.g., "new-session", "set-option").
    pub name: String,
    /// Arguments to the command.
    pub args: Vec<String>,
}

/// Parse a single command string into name + args.
///
/// # Errors
///
/// Returns `CmdParseError::UnterminatedQuote` for unmatched quotes.
pub fn parse_command(input: &str) -> Result<ParsedCommand, CmdParseError> {
    let tokens = tokenize(input, 1)?;
    if tokens.is_empty() {
        return Err(CmdParseError::EmptyCommand { line: 1 });
    }
    Ok(ParsedCommand {
        name: tokens[0].clone(),
        args: tokens[1..].to_vec(),
    })
}

/// Parse a multi-line config file into a list of commands.
///
/// # Errors
///
/// Returns parse errors for invalid syntax.
pub fn parse_config(input: &str) -> Result<Vec<ParsedCommand>, CmdParseError> {
    let mut commands = Vec::new();
    let mut continued_line = String::new();
    let mut line_num = 0usize;

    for raw_line in input.lines() {
        line_num += 1;
        let trimmed = raw_line.trim();

        // Skip empty lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if !continued_line.is_empty() {
                // Process the continued line before skipping
                let cmds = split_semicolons(&continued_line);
                for cmd_str in cmds {
                    if !cmd_str.is_empty() {
                        let tokens = tokenize(&cmd_str, line_num)?;
                        if !tokens.is_empty() {
                            commands.push(ParsedCommand {
                                name: tokens[0].clone(),
                                args: tokens[1..].to_vec(),
                            });
                        }
                    }
                }
                continued_line.clear();
            }
            continue;
        }

        // Handle backslash continuation.
        if let Some(stripped) = trimmed.strip_suffix('\\') {
            continued_line.push_str(stripped);
            continued_line.push(' ');
            continue;
        }

        continued_line.push_str(trimmed);

        let cmds = split_semicolons(&continued_line);
        for cmd_str in cmds {
            if !cmd_str.is_empty() {
                let tokens = tokenize(&cmd_str, line_num)?;
                if !tokens.is_empty() {
                    commands.push(ParsedCommand {
                        name: tokens[0].clone(),
                        args: tokens[1..].to_vec(),
                    });
                }
            }
        }
        continued_line.clear();
    }

    Ok(commands)
}

/// Split a line on semicolons (respecting quotes).
fn split_semicolons(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;

    for c in input.chars() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ';' if !in_single && !in_double => {
                parts.push(current.trim().to_owned());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    let trimmed = current.trim().to_owned();
    if !trimmed.is_empty() {
        parts.push(trimmed);
    }
    parts
}

/// Tokenize a command string respecting quoted arguments.
fn tokenize(input: &str, line: usize) -> Result<Vec<String>, CmdParseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\\' if in_double => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if in_single || in_double {
        return Err(CmdParseError::UnterminatedQuote { line });
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_command() {
        let cmd = parse_command("new-session -d -s main").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
        });
        assert_eq!(cmd.name, "new-session");
        assert_eq!(cmd.args, vec!["-d", "-s", "main"]);
    }

    #[test]
    fn parse_quoted_args() {
        let cmd = parse_command("set -g status-left '#S'").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
        });
        assert_eq!(cmd.name, "set");
        assert_eq!(cmd.args, vec!["-g", "status-left", "#S"]);
    }

    #[test]
    fn parse_double_quoted() {
        let cmd =
            parse_command(r#"send-keys "echo hello""#).unwrap_or_else(|_| ParsedCommand {
                name: String::new(),
                args: Vec::new(),
            });
        assert_eq!(cmd.name, "send-keys");
        assert_eq!(cmd.args, vec!["echo hello"]);
    }

    #[test]
    fn parse_empty_returns_error() {
        assert!(parse_command("").is_err());
    }

    #[test]
    fn parse_unterminated_quote() {
        assert!(parse_command("set 'unterminated").is_err());
    }

    #[test]
    fn parse_config_comments() {
        let config = "# This is a comment\nnew-session -d\n";
        let cmds = parse_config(config).unwrap_or_default();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "new-session");
    }

    #[test]
    fn parse_config_semicolons() {
        let config = "set -g mouse on; set -g status on\n";
        let cmds = parse_config(config).unwrap_or_default();
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn parse_config_backslash_continuation() {
        let config = "set -g \\\n  mouse on\n";
        let cmds = parse_config(config).unwrap_or_default();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "set");
    }

    #[test]
    fn split_semicolons_with_quotes() {
        let parts = split_semicolons("echo 'a;b'; echo c");
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn parse_config_empty_lines_skipped() {
        let config = "\n\nnew-session\n\n\n";
        let cmds = parse_config(config).unwrap_or_default();
        assert_eq!(cmds.len(), 1);
    }
}
