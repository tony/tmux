//! # mux-cmd-parse
//!
//! Command and config file parser for TermForge.
//!
//! ## Design Decision (RULE-S33-87)
//! Hand-written recursive descent parser (not yacc port).
//! winnow may be used for subgrammars, but the top-level parser is
//! hand-written for full control and better error messages.
//!
//! ## Responsibilities
//! - Parse tmux command strings (`new-session -d -s foo`)
//! - Parse config files (`.tmux.conf` / `termforge.toml`)
//! - Handle quoting, escaping, variable expansion
//! - Produce structured `Command` values for kernel dispatch

#![forbid(unsafe_code)]

/// A parsed tmux-style command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// Command name (e.g., "new-session", "set-option").
    pub name: String,
    /// Positional and flag arguments.
    pub args: Vec<Argument>,
}

/// A command argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    /// A flag: `-d`, `-s`.
    Flag(char),
    /// A flag with a value: `-s foo`.
    FlagValue(char, String),
    /// A long flag: `--flag`.
    LongFlag(String),
    /// A positional argument.
    Positional(String),
    /// A target: `-t session:window.pane`.
    Target(String),
}

/// Parse a command string into a [`Command`].
///
/// Handles quoting (single and double), escaping, and flag parsing.
///
/// # Errors
/// Returns error if the command string is malformed.
pub fn parse_command(input: &str) -> Result<Command, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::EmptyCommand);
    }

    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(ParseError::EmptyCommand);
    }

    let name = tokens[0].clone();
    let mut args = Vec::new();
    let mut i = 1;

    while i < tokens.len() {
        let token = &tokens[i];
        if let Some(stripped) = token.strip_prefix('-') {
            if stripped.is_empty() {
                args.push(Argument::Positional("-".into()));
            } else if stripped.starts_with('-') {
                // Long flag
                args.push(Argument::LongFlag(stripped[1..].to_owned()));
            } else {
                let flag = stripped.chars().next().unwrap_or('-');
                if is_value_flag(flag, &name) && i + 1 < tokens.len() {
                    i += 1;
                    args.push(Argument::FlagValue(flag, tokens[i].clone()));
                } else {
                    // Single character flags may be combined: -dP
                    for ch in stripped.chars() {
                        args.push(Argument::Flag(ch));
                    }
                }
            }
        } else {
            args.push(Argument::Positional(token.clone()));
        }
        i += 1;
    }

    Ok(Command { name, args })
}

/// Check if a flag expects a value argument.
fn is_value_flag(flag: char, _command: &str) -> bool {
    // Common value flags across tmux commands
    matches!(flag, 's' | 't' | 'n' | 'c' | 'e' | 'f' | 'x' | 'y' | 'F' | 'l' | 'p')
}

/// Tokenize a command string, handling quoting and escaping.
fn tokenize(input: &str) -> Result<Vec<String>, ParseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(&ch) = chars.peek() {
        match ch {
            '\'' if !in_double_quote => {
                chars.next();
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                chars.next();
                in_double_quote = !in_double_quote;
            }
            '\\' if !in_single_quote => {
                chars.next();
                if let Some(&next) = chars.peek() {
                    chars.next();
                    current.push(next);
                }
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                chars.next();
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            ';' if !in_single_quote && !in_double_quote => {
                // Command separator
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                // For now, stop at semicolons (multi-command support later)
                break;
            }
            _ => {
                chars.next();
                current.push(ch);
            }
        }
    }

    if in_single_quote || in_double_quote {
        return Err(ParseError::UnterminatedQuote);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

/// A parsed config file.
#[derive(Debug, Clone)]
pub struct ConfigFile {
    /// Commands from the config file.
    pub commands: Vec<Command>,
}

/// Parse a config file (`.tmux.conf` format).
///
/// # Errors
/// Returns error if the config file has syntax errors.
pub fn parse_config(input: &str) -> Result<ConfigFile, ParseError> {
    let mut commands = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Handle line continuation
        let line = line.trim_end_matches('\\');
        match parse_command(line) {
            Ok(cmd) => commands.push(cmd),
            Err(ParseError::EmptyCommand) => {} // skip
            Err(e) => return Err(e),
        }
    }

    Ok(ConfigFile { commands })
}

/// Errors from command/config parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("empty command")]
    EmptyCommand,
    #[error("unterminated quote")]
    UnterminatedQuote,
    #[error("invalid escape sequence")]
    InvalidEscape,
    #[error("syntax error: {0}")]
    SyntaxError(String),
    #[error("unknown command: {0}")]
    UnknownCommand(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_command() {
        let cmd = parse_command("new-session -d -s test");
        assert!(cmd.is_ok());
        let cmd = cmd.unwrap_or_else(|_| Command { name: String::new(), args: vec![] });
        assert_eq!(cmd.name, "new-session");
    }

    #[test]
    fn parse_quoted_argument() {
        let cmd = parse_command("new-session -s 'my session'");
        assert!(cmd.is_ok());
        let cmd = cmd.unwrap_or_else(|_| Command { name: String::new(), args: vec![] });
        assert_eq!(cmd.name, "new-session");
        assert!(cmd.args.iter().any(|a| matches!(a, Argument::FlagValue('s', v) if v == "my session")));
    }

    #[test]
    fn parse_config_skips_comments() {
        let config = parse_config("# comment\nset -g base-index 1\n");
        assert!(config.is_ok());
        let config = config.unwrap_or_else(|_| ConfigFile { commands: vec![] });
        assert_eq!(config.commands.len(), 1);
    }

    #[test]
    fn empty_command_error() {
        assert!(parse_command("").is_err());
    }

    #[test]
    fn unterminated_quote_error() {
        assert!(parse_command("echo 'hello").is_err());
    }
}
