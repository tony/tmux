//! # mux-cmd-parse
//!
//! Command and config file parser (RULE-S33-87: hand-written, not yacc port).

#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command { pub name: String, pub args: Vec<Argument> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument { Flag(char), FlagValue(char, String), LongFlag(String), Positional(String), Target(String) }

pub fn parse_command(input: &str) -> Result<Command, ParseError> {
    let input = input.trim();
    if input.is_empty() { return Err(ParseError::EmptyCommand); }
    let tokens = tokenize(input)?;
    if tokens.is_empty() { return Err(ParseError::EmptyCommand); }
    let name = tokens[0].clone();
    let mut args = Vec::new();
    let mut i = 1;
    while i < tokens.len() {
        let token = &tokens[i];
        if let Some(stripped) = token.strip_prefix('-') {
            if stripped.is_empty() { args.push(Argument::Positional("-".into())); }
            else if stripped.starts_with('-') { args.push(Argument::LongFlag(stripped[1..].to_owned())); }
            else {
                let flag = stripped.chars().next().unwrap_or('-');
                if is_value_flag(flag, &name) && i + 1 < tokens.len() { i += 1; args.push(Argument::FlagValue(flag, tokens[i].clone())); }
                else { for ch in stripped.chars() { args.push(Argument::Flag(ch)); } }
            }
        } else { args.push(Argument::Positional(token.clone())); }
        i += 1;
    }
    Ok(Command { name, args })
}

fn is_value_flag(flag: char, _command: &str) -> bool {
    matches!(flag, 's' | 't' | 'n' | 'c' | 'e' | 'f' | 'x' | 'y' | 'F' | 'l' | 'p')
}

fn tokenize(input: &str) -> Result<Vec<String>, ParseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_sq = false;
    let mut in_dq = false;
    while let Some(&ch) = chars.peek() {
        match ch {
            '\'' if !in_dq => { chars.next(); in_sq = !in_sq; }
            '"' if !in_sq => { chars.next(); in_dq = !in_dq; }
            '\\' if !in_sq => { chars.next(); if let Some(&n) = chars.peek() { chars.next(); current.push(n); } }
            ' ' | '\t' if !in_sq && !in_dq => { chars.next(); if !current.is_empty() { tokens.push(std::mem::take(&mut current)); } }
            ';' if !in_sq && !in_dq => { if !current.is_empty() { tokens.push(std::mem::take(&mut current)); } break; }
            _ => { chars.next(); current.push(ch); }
        }
    }
    if in_sq || in_dq { return Err(ParseError::UnterminatedQuote); }
    if !current.is_empty() { tokens.push(current); }
    Ok(tokens)
}

pub fn parse_config(input: &str) -> Result<ConfigFile, ParseError> {
    let mut commands = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        match parse_command(line.trim_end_matches('\\')) { Ok(cmd) => commands.push(cmd), Err(ParseError::EmptyCommand) => {}, Err(e) => return Err(e) }
    }
    Ok(ConfigFile { commands })
}

#[derive(Debug, Clone)]
pub struct ConfigFile { pub commands: Vec<Command> }

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("empty command")] EmptyCommand,
    #[error("unterminated quote")] UnterminatedQuote,
    #[error("invalid escape sequence")] InvalidEscape,
    #[error("syntax error: {0}")] SyntaxError(String),
    #[error("unknown command: {0}")] UnknownCommand(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let cmd = parse_command("new-session -d -s test");
        assert!(cmd.is_ok());
        assert_eq!(cmd.unwrap_or_else(|_| Command { name: String::new(), args: vec![] }).name, "new-session");
    }

    #[test]
    fn parse_quoted() {
        let cmd = parse_command("new-session -s 'my session'").unwrap_or_else(|_| Command { name: String::new(), args: vec![] });
        assert!(cmd.args.iter().any(|a| matches!(a, Argument::FlagValue('s', v) if v == "my session")));
    }

    #[test]
    fn parse_config_skips_comments() {
        let config = parse_config("# comment\nset -g base-index 1\n").unwrap_or_else(|_| ConfigFile { commands: vec![] });
        assert_eq!(config.commands.len(), 1);
    }

    #[test]
    fn empty_command_error() { assert!(parse_command("").is_err()); }

    #[test]
    fn unterminated_quote_error() { assert!(parse_command("echo 'hello").is_err()); }
}
