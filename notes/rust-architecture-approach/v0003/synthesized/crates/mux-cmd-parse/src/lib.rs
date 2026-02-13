//! # mux-cmd-parse
//!
//! tmux command parser. Parses command strings like "new-session -s main"
//! into structured command objects.
//!
//! L2 data crate.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Command parse errors.
#[derive(Debug, Error)]
pub enum CmdParseError {
    /// Unknown command.
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    /// Missing required argument.
    #[error("missing argument for {0}")]
    MissingArgument(String),
    /// Invalid flag.
    #[error("invalid flag: {0}")]
    InvalidFlag(String),
}

/// A parsed tmux command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    /// Command name (e.g., "new-session", "kill-server").
    pub name: String,
    /// Positional arguments.
    pub args: Vec<String>,
    /// Flag arguments (e.g., "-s" -> "main").
    pub flags: Vec<(String, Option<String>)>,
}

impl ParsedCommand {
    /// Get a flag value by flag name.
    #[must_use]
    pub fn flag_value(&self, flag: &str) -> Option<&str> {
        self.flags
            .iter()
            .find(|(f, _)| f == flag)
            .and_then(|(_, v)| v.as_deref())
    }

    /// Whether a boolean flag is present.
    #[must_use]
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|(f, _)| f == flag)
    }
}

/// Parse a tmux command string.
pub fn parse_command(input: &str) -> Result<ParsedCommand, CmdParseError> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(CmdParseError::UnknownCommand(String::new()));
    }

    let name = tokens[0].to_owned();
    let mut args = Vec::new();
    let mut flags = Vec::new();

    let mut i = 1;
    while i < tokens.len() {
        let token = tokens[i];
        if let Some(flag) = token.strip_prefix('-') {
            if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                flags.push((flag.to_owned(), Some(tokens[i + 1].to_owned())));
                i += 2;
            } else {
                flags.push((flag.to_owned(), None));
                i += 1;
            }
        } else {
            args.push(token.to_owned());
            i += 1;
        }
    }

    Ok(ParsedCommand { name, args, flags })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_command() {
        let cmd = parse_command("kill-server").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
            flags: Vec::new(),
        });
        assert_eq!(cmd.name, "kill-server");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn parse_command_with_flags() {
        let cmd = parse_command("new-session -s main").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
            flags: Vec::new(),
        });
        assert_eq!(cmd.name, "new-session");
        assert_eq!(cmd.flag_value("s"), Some("main"));
    }

    #[test]
    fn parse_command_with_boolean_flag() {
        let cmd = parse_command("list-sessions -F").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
            flags: Vec::new(),
        });
        assert!(cmd.has_flag("F"));
    }

    #[test]
    fn parse_empty_fails() {
        assert!(parse_command("").is_err());
    }

    #[test]
    fn parse_command_with_args() {
        let cmd = parse_command("send-keys -t main hello").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
            flags: Vec::new(),
        });
        assert_eq!(cmd.name, "send-keys");
        assert_eq!(cmd.flag_value("t"), Some("main"));
        assert_eq!(cmd.args.first().map(String::as_str), Some("hello"));
    }

    #[test]
    fn flag_not_found() {
        let cmd = parse_command("kill-server").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
            flags: Vec::new(),
        });
        assert!(cmd.flag_value("s").is_none());
        assert!(!cmd.has_flag("s"));
    }

    #[test]
    fn parse_multiple_flags() {
        let cmd = parse_command("new-window -t main -n editor").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
            flags: Vec::new(),
        });
        assert_eq!(cmd.flag_value("t"), Some("main"));
        assert_eq!(cmd.flag_value("n"), Some("editor"));
    }

    #[test]
    fn parse_whitespace_only() {
        assert!(parse_command("   ").is_err());
    }

    #[test]
    fn parse_command_equality() {
        let a = parse_command("kill-server").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
            flags: Vec::new(),
        });
        let b = parse_command("kill-server").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
            flags: Vec::new(),
        });
        assert_eq!(a, b);
    }

    #[test]
    fn parse_mixed_flags_and_args() {
        let cmd = parse_command("send-keys -t dev ls -la").unwrap_or_else(|_| ParsedCommand {
            name: String::new(),
            args: Vec::new(),
            flags: Vec::new(),
        });
        assert_eq!(cmd.name, "send-keys");
        assert_eq!(cmd.flag_value("t"), Some("dev"));
    }

    #[test]
    fn parse_empty_string_fails() {
        let result = parse_command("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_spaces_only_fails() {
        let result = parse_command("   ");
        assert!(result.is_err());
    }
}
