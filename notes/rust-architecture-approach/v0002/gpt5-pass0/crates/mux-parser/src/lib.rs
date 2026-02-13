use mux_format::FormatExpr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAst {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("empty command")]
    Empty,
    #[error("invalid syntax: {0}")]
    Invalid(String),
}

pub trait Parser {
    fn parse_command(&self, input: &str) -> Result<CommandAst, ParseError>;
    fn parse_format(&self, input: &str) -> Result<FormatExpr, ParseError>;
}

#[derive(Debug, Default)]
pub struct DefaultParser;

impl Parser for DefaultParser {
    fn parse_command(&self, input: &str) -> Result<CommandAst, ParseError> {
        let mut parts = input.split_whitespace();
        let name = parts.next().ok_or(ParseError::Empty)?.to_owned();
        let args = parts.map(ToOwned::to_owned).collect();
        Ok(CommandAst { name, args })
    }

    fn parse_format(&self, input: &str) -> Result<FormatExpr, ParseError> {
        mux_format::parse_format(input).map_err(|error| ParseError::Invalid(error.to_string()))
    }
}
