//! tmux-builder: YAML/TOML workspace layout tool.
//!
//! Reads a workspace definition and creates sessions/windows/panes accordingly.

use clap::Parser;

/// tmux-builder: Create tmux workspaces from layout files.
#[derive(Parser, Debug)]
#[command(name = "tmux-builder", version, about)]
struct Cli {
    /// Path to the workspace layout file.
    #[arg(short, long, default_value = "workspace.toml")]
    config: String,

    /// Dry-run mode (print commands without executing).
    #[arg(short, long)]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if cli.dry_run {
        println!("tmux-builder: dry run with config={}", cli.config);
    } else {
        println!("tmux-builder: building workspace from {}", cli.config);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parse_defaults() {
        let cli = Cli::parse_from(["tmux-builder"]);
        assert_eq!(cli.config, "workspace.toml");
        assert!(!cli.dry_run);
    }

    #[test]
    fn cli_parse_custom() {
        let cli = Cli::parse_from(["tmux-builder", "--config", "my.toml", "--dry-run"]);
        assert_eq!(cli.config, "my.toml");
        assert!(cli.dry_run);
    }

    #[test]
    fn cli_parse_short_flags() {
        let cli = Cli::parse_from(["tmux-builder", "-c", "w.toml", "-d"]);
        assert_eq!(cli.config, "w.toml");
        assert!(cli.dry_run);
    }
}
