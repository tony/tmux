//! tmux-vm: Headless terminal multiplexer for CI.
//!
//! Runs a terminal multiplexer without a real terminal, useful for
//! automated testing and CI pipelines.

use clap::Parser;

/// tmux-vm: Headless multiplexer for CI.
#[derive(Parser, Debug)]
#[command(name = "tmux-vm", version, about)]
struct Cli {
    /// Command to execute in the VM.
    #[arg(short, long)]
    command: Option<String>,

    /// Session name.
    #[arg(short, long, default_value = "vm")]
    session: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    println!(
        "tmux-vm: session={}, command={:?}",
        cli.session, cli.command
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults() {
        let cli = Cli::parse_from(["tmux-vm"]);
        assert_eq!(cli.session, "vm");
        assert!(cli.command.is_none());
    }

    #[test]
    fn cli_with_command() {
        let cli = Cli::parse_from(["tmux-vm", "--command", "echo hello"]);
        assert_eq!(cli.command.as_deref(), Some("echo hello"));
    }

    #[test]
    fn cli_with_session() {
        let cli = Cli::parse_from(["tmux-vm", "--session", "ci-test"]);
        assert_eq!(cli.session, "ci-test");
    }
}
