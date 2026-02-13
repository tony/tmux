use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Text(String),
    Variable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatExpr {
    pub segments: Vec<Segment>,
}

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("unclosed format variable")]
    UnclosedVariable,
    #[error("missing variable: {0}")]
    MissingVariable(String),
}

pub fn parse_format(input: &str) -> Result<FormatExpr, FormatError> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '#' && chars.peek() == Some(&'{') {
            let _ = chars.next();
            if !current.is_empty() {
                segments.push(Segment::Text(std::mem::take(&mut current)));
            }
            let mut variable = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some(c) => variable.push(c),
                    None => return Err(FormatError::UnclosedVariable),
                }
            }
            segments.push(Segment::Variable(variable));
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        segments.push(Segment::Text(current));
    }

    Ok(FormatExpr { segments })
}

pub fn render_format(expr: &FormatExpr, vars: &HashMap<String, String>) -> Result<String, FormatError> {
    let mut out = String::new();
    for segment in &expr.segments {
        match segment {
            Segment::Text(text) => out.push_str(text),
            Segment::Variable(name) => {
                let value = vars
                    .get(name)
                    .ok_or_else(|| FormatError::MissingVariable(name.clone()))?;
                out.push_str(value);
            }
        }
    }
    Ok(out)
}
