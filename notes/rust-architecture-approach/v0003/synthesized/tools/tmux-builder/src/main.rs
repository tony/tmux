//! # tmux-builder
//!
//! YAML/TOML workspace layout tool for tmux sessions.
//! Reads a declarative workspace file and creates tmux sessions
//! with windows, panes, and commands.
//!
//! ## Usage
//! ```text
//! tmux-builder load workspace.toml
//! tmux-builder validate workspace.toml
//! tmux-builder freeze > current.toml
//! ```
//!
//! L6 tool crate.

#![allow(dead_code)]

use clap::{Parser, Subcommand};
use serde::Deserialize;

/// tmux-builder: Declarative tmux workspace manager.
#[derive(Parser, Debug)]
#[command(name = "tmux-builder", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Load a workspace definition.
    Load {
        /// Path to workspace file.
        path: String,
    },
    /// Validate a workspace definition without applying.
    Validate {
        /// Path to workspace file.
        path: String,
    },
    /// Freeze the current tmux session to a workspace file.
    Freeze {
        /// Output format.
        #[arg(long, default_value = "toml")]
        format: String,
    },
}

/// A workspace definition (from TOML).
#[derive(Debug, Deserialize, Clone)]
struct WorkspaceConfig {
    /// Workspace name.
    name: String,
    /// Sessions to create.
    #[serde(default)]
    sessions: Vec<SessionDef>,
}

/// A session definition.
#[derive(Debug, Deserialize, Clone)]
struct SessionDef {
    /// Session name.
    name: String,
    /// Windows in this session.
    #[serde(default)]
    windows: Vec<WindowDef>,
}

/// A window definition.
#[derive(Debug, Deserialize, Clone)]
struct WindowDef {
    /// Window name.
    name: String,
    /// Layout algorithm.
    #[serde(default = "default_layout")]
    layout: String,
    /// Panes in this window.
    #[serde(default)]
    panes: Vec<PaneDef>,
}

/// A pane definition.
#[derive(Debug, Deserialize, Clone)]
struct PaneDef {
    /// Command to run in this pane.
    command: Option<String>,
    /// Working directory.
    directory: Option<String>,
}

fn default_layout() -> String {
    "even-horizontal".to_owned()
}

/// Validate a workspace config.
fn validate_workspace(config: &WorkspaceConfig) -> Vec<String> {
    let mut errors = Vec::new();

    if config.name.is_empty() {
        errors.push("workspace name cannot be empty".to_owned());
    }

    for (si, session) in config.sessions.iter().enumerate() {
        if session.name.is_empty() {
            errors.push(format!("session[{si}] name cannot be empty"));
        }
        for (wi, window) in session.windows.iter().enumerate() {
            if window.name.is_empty() {
                errors.push(format!("session[{si}].window[{wi}] name cannot be empty"));
            }
            let valid_layouts = [
                "even-horizontal",
                "even-vertical",
                "main-horizontal",
                "main-vertical",
                "tiled",
            ];
            if !valid_layouts.contains(&window.layout.as_str()) {
                errors.push(format!(
                    "session[{si}].window[{wi}] invalid layout: {}",
                    window.layout
                ));
            }
        }
    }

    errors
}

/// Parse a workspace TOML file.
fn parse_workspace(content: &str) -> Result<WorkspaceConfig, String> {
    toml::from_str(content).map_err(|e| format!("TOML parse error: {e}"))
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Load { path } => {
            eprintln!("tmux-builder: load from {path} (scaffold -- not implemented)");
        }
        Commands::Validate { path } => {
            eprintln!("tmux-builder: validate {path} (scaffold -- not implemented)");
        }
        Commands::Freeze { format } => {
            eprintln!("tmux-builder: freeze to {format} (scaffold -- not implemented)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_toml() -> &'static str {
        r#"
name = "dev-workspace"

[[sessions]]
name = "main"

[[sessions.windows]]
name = "editor"
layout = "even-horizontal"

[[sessions.windows.panes]]
command = "vim"

[[sessions.windows.panes]]
command = "cargo watch"

[[sessions]]
name = "monitoring"

[[sessions.windows]]
name = "logs"
layout = "tiled"

[[sessions.windows.panes]]
command = "tail -f /var/log/syslog"
"#
    }

    #[test]
    fn parse_valid_workspace() {
        let result = parse_workspace(sample_toml());
        assert!(result.is_ok());
    }

    #[test]
    fn parse_workspace_name() {
        let config = parse_workspace(sample_toml()).unwrap_or_else(|_| WorkspaceConfig {
            name: String::new(),
            sessions: Vec::new(),
        });
        assert_eq!(config.name, "dev-workspace");
    }

    #[test]
    fn parse_workspace_sessions() {
        let config = parse_workspace(sample_toml()).unwrap_or_else(|_| WorkspaceConfig {
            name: String::new(),
            sessions: Vec::new(),
        });
        assert_eq!(config.sessions.len(), 2);
    }

    #[test]
    fn parse_workspace_windows() {
        let config = parse_workspace(sample_toml()).unwrap_or_else(|_| WorkspaceConfig {
            name: String::new(),
            sessions: Vec::new(),
        });
        let first_session = &config.sessions[0];
        assert_eq!(first_session.windows.len(), 1);
        assert_eq!(first_session.windows[0].name, "editor");
    }

    #[test]
    fn parse_workspace_panes() {
        let config = parse_workspace(sample_toml()).unwrap_or_else(|_| WorkspaceConfig {
            name: String::new(),
            sessions: Vec::new(),
        });
        let panes = &config.sessions[0].windows[0].panes;
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].command.as_deref(), Some("vim"));
    }

    #[test]
    fn parse_workspace_layout() {
        let config = parse_workspace(sample_toml()).unwrap_or_else(|_| WorkspaceConfig {
            name: String::new(),
            sessions: Vec::new(),
        });
        assert_eq!(config.sessions[0].windows[0].layout, "even-horizontal");
    }

    #[test]
    fn validate_valid_workspace() {
        let config = parse_workspace(sample_toml()).unwrap_or_else(|_| WorkspaceConfig {
            name: String::new(),
            sessions: Vec::new(),
        });
        let errors = validate_workspace(&config);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn validate_empty_name() {
        let config = WorkspaceConfig {
            name: String::new(),
            sessions: Vec::new(),
        };
        let errors = validate_workspace(&config);
        assert!(!errors.is_empty());
    }

    #[test]
    fn validate_invalid_layout() {
        let config = WorkspaceConfig {
            name: "test".to_owned(),
            sessions: vec![SessionDef {
                name: "s1".to_owned(),
                windows: vec![WindowDef {
                    name: "w1".to_owned(),
                    layout: "invalid-layout".to_owned(),
                    panes: Vec::new(),
                }],
            }],
        };
        let errors = validate_workspace(&config);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("invalid layout"));
    }

    #[test]
    fn validate_empty_session_name() {
        let config = WorkspaceConfig {
            name: "test".to_owned(),
            sessions: vec![SessionDef {
                name: String::new(),
                windows: Vec::new(),
            }],
        };
        let errors = validate_workspace(&config);
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_invalid_toml() {
        let result = parse_workspace("not valid [toml");
        assert!(result.is_err());
    }

    #[test]
    fn parse_minimal_workspace() {
        let toml = r#"name = "minimal""#;
        let result = parse_workspace(toml);
        assert!(result.is_ok());
        let config = result.unwrap_or_else(|_| WorkspaceConfig {
            name: String::new(),
            sessions: Vec::new(),
        });
        assert!(config.sessions.is_empty());
    }

    #[test]
    fn default_layout_value() {
        assert_eq!(default_layout(), "even-horizontal");
    }
}
