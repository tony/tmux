use mux_parser::{CommandAst, ParseError, Parser};
use mux_target::{parse_target, Target};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    pub name: String,
    pub args: Vec<String>,
    pub target: Option<Target>,
}

#[derive(Debug, Error)]
pub enum CommandParseError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Target(#[from] mux_target::TargetError),
}

#[derive(Debug)]
pub struct CommandParser<P> {
    parser: P,
}

impl<P> CommandParser<P>
where
    P: Parser,
{
    pub fn new(parser: P) -> Self {
        Self { parser }
    }

    pub fn parse(&self, input: &str) -> Result<ParsedCommand, CommandParseError> {
        let CommandAst { name, args } = self.parser.parse_command(input)?;
        let mut target = None;
        let mut filtered = Vec::new();

        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            if arg == "-t" {
                if let Some(raw) = iter.next() {
                    target = Some(parse_target(&raw)?);
                }
            } else {
                filtered.push(arg);
            }
        }

        Ok(ParsedCommand {
            name,
            args: filtered,
            target,
        })
    }
}
