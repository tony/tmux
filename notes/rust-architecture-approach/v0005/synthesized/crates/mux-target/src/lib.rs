//! tmux target syntax parser.
//!
//! Parses target specifiers like `session:window.pane`, `=exact`, `$id`, etc.
//! Used by command dispatch to resolve which entity a command operates on.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Errors from target parsing.
#[derive(Debug, Error)]
pub enum TargetError {
    /// Invalid target syntax.
    #[error("invalid target: {0}")]
    Invalid(String),
}

/// A parsed target specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    /// Session component (name, index, or id).
    pub session: Option<TargetComponent>,
    /// Window component.
    pub window: Option<TargetComponent>,
    /// Pane component.
    pub pane: Option<TargetComponent>,
}

/// A single component of a target specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetComponent {
    /// Numeric index.
    Index(i32),
    /// Name prefix match.
    Name(String),
    /// Exact name match (prefixed with =).
    Exact(String),
    /// Dollar-prefixed unique ID.
    Id(u64),
    /// Relative offset (+N or -N).
    Relative(i32),
    /// Current/active (empty component).
    Current,
    /// Last active (!).
    Last,
    /// Next (+) without number.
    Next,
    /// Previous (-) without number.
    Previous,
}

impl TargetSpec {
    /// Parse a target string.
    ///
    /// Formats: `session:window.pane`, `:window.pane`, `.pane`, etc.
    pub fn parse(input: &str) -> Result<Self, TargetError> {
        if input.is_empty() {
            return Ok(Self {
                session: Some(TargetComponent::Current),
                window: Some(TargetComponent::Current),
                pane: Some(TargetComponent::Current),
            });
        }

        let mut spec = Self {
            session: None,
            window: None,
            pane: None,
        };

        // Split on ':' for session:rest
        let (session_part, rest) = if let Some(idx) = input.find(':') {
            (Some(&input[..idx]), &input[idx + 1..])
        } else if let Some(_idx) = input.find('.') {
            (None, input)
        } else {
            // Could be session-only or window-only
            (Some(input), "")
        };

        if let Some(sp) = session_part {
            spec.session = Some(parse_component(sp)?);
        }

        if !rest.is_empty() {
            if let Some(idx) = rest.find('.') {
                let window_part = &rest[..idx];
                let pane_part = &rest[idx + 1..];
                if !window_part.is_empty() {
                    spec.window = Some(parse_component(window_part)?);
                }
                if !pane_part.is_empty() {
                    spec.pane = Some(parse_component(pane_part)?);
                }
            } else {
                spec.window = Some(parse_component(rest)?);
            }
        }

        Ok(spec)
    }

    /// Format back to a target string.
    pub fn format(&self) -> String {
        let mut result = String::new();
        if let Some(ref s) = self.session {
            result.push_str(&format_component(s));
        }
        if self.window.is_some() || self.pane.is_some() {
            result.push(':');
            if let Some(ref w) = self.window {
                result.push_str(&format_component(w));
            }
        }
        if let Some(ref p) = self.pane {
            result.push('.');
            result.push_str(&format_component(p));
        }
        result
    }
}

fn parse_component(s: &str) -> Result<TargetComponent, TargetError> {
    if s.is_empty() {
        return Ok(TargetComponent::Current);
    }
    if s == "!" {
        return Ok(TargetComponent::Last);
    }
    if s == "+" {
        return Ok(TargetComponent::Next);
    }
    if s == "-" {
        return Ok(TargetComponent::Previous);
    }
    if let Some(rest) = s.strip_prefix('=') {
        return Ok(TargetComponent::Exact(rest.to_owned()));
    }
    if let Some(rest) = s.strip_prefix('$') {
        return rest
            .parse::<u64>()
            .map(TargetComponent::Id)
            .map_err(|_| TargetError::Invalid(format!("invalid id: {s}")));
    }
    if let Some(rest) = s.strip_prefix('+') {
        if let Ok(n) = rest.parse::<i32>() {
            return Ok(TargetComponent::Relative(n));
        }
    }
    if let Some(rest) = s.strip_prefix('-') {
        if let Ok(n) = rest.parse::<i32>() {
            return Ok(TargetComponent::Relative(-n));
        }
    }
    if let Ok(n) = s.parse::<i32>() {
        return Ok(TargetComponent::Index(n));
    }
    Ok(TargetComponent::Name(s.to_owned()))
}

fn format_component(c: &TargetComponent) -> String {
    match c {
        TargetComponent::Index(n) => n.to_string(),
        TargetComponent::Name(s) => s.clone(),
        TargetComponent::Exact(s) => format!("={s}"),
        TargetComponent::Id(id) => format!("${id}"),
        TargetComponent::Relative(n) if *n >= 0 => format!("+{n}"),
        TargetComponent::Relative(n) => n.to_string(),
        TargetComponent::Current => String::new(),
        TargetComponent::Last => "!".to_owned(),
        TargetComponent::Next => "+".to_owned(),
        TargetComponent::Previous => "-".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let spec = TargetSpec::parse("").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.session, Some(TargetComponent::Current));
    }

    #[test]
    fn parse_session_only() {
        let spec = TargetSpec::parse("dev").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.session, Some(TargetComponent::Name("dev".into())));
    }

    #[test]
    fn parse_session_window() {
        let spec = TargetSpec::parse("dev:1").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.session, Some(TargetComponent::Name("dev".into())));
        assert_eq!(spec.window, Some(TargetComponent::Index(1)));
    }

    #[test]
    fn parse_window_pane() {
        let spec = TargetSpec::parse(":1.2").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.window, Some(TargetComponent::Index(1)));
        assert_eq!(spec.pane, Some(TargetComponent::Index(2)));
    }

    #[test]
    fn parse_exact_match() {
        let spec = TargetSpec::parse("=dev").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.session, Some(TargetComponent::Exact("dev".into())));
    }

    #[test]
    fn parse_id() {
        let spec = TargetSpec::parse("$5").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.session, Some(TargetComponent::Id(5)));
    }

    #[test]
    fn parse_last() {
        let spec = TargetSpec::parse("!").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.session, Some(TargetComponent::Last));
    }

    #[test]
    fn parse_relative() {
        let spec = TargetSpec::parse("+2").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.session, Some(TargetComponent::Relative(2)));
    }

    #[test]
    fn parse_next_previous() {
        let next = TargetSpec::parse("+").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(next.session, Some(TargetComponent::Next));

        let prev = TargetSpec::parse("-").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(prev.session, Some(TargetComponent::Previous));
    }

    #[test]
    fn format_roundtrip_name() {
        let spec = TargetSpec {
            session: Some(TargetComponent::Name("dev".into())),
            window: Some(TargetComponent::Index(1)),
            pane: Some(TargetComponent::Index(0)),
        };
        let formatted = spec.format();
        assert!(formatted.contains("dev"));
        assert!(formatted.contains("1"));
    }

    #[test]
    fn parse_full_target() {
        let spec = TargetSpec::parse("main:2.3").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.session, Some(TargetComponent::Name("main".into())));
        assert_eq!(spec.window, Some(TargetComponent::Index(2)));
        assert_eq!(spec.pane, Some(TargetComponent::Index(3)));
    }

    #[test]
    fn parse_pane_only() {
        let spec = TargetSpec::parse(":.1").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.pane, Some(TargetComponent::Index(1)));
    }

    #[test]
    fn parse_negative_relative() {
        let spec = TargetSpec::parse("-3").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(spec.session, Some(TargetComponent::Relative(-3)));
    }

    #[test]
    fn error_display() {
        let e = TargetError::Invalid("bad".into());
        assert!(e.to_string().contains("invalid target"));
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn parse_never_panics(input in ".*") {
                let _ = TargetSpec::parse(&input);
            }

            #[test]
            fn index_roundtrip(n in 0i32..100) {
                let c = TargetComponent::Index(n);
                let s = format_component(&c);
                let parsed = parse_component(&s).unwrap_or(TargetComponent::Current);
                prop_assert_eq!(parsed, c);
            }
        }
    }
}
