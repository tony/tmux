//! tmux-sniff: Wire protocol inspector.
//!
//! Reads data from a file or stdin and decodes TF01 frames or tmux imsg.

use clap::Parser;

/// tmux-sniff: Inspect TF01/tmux wire protocol.
#[derive(Parser, Debug)]
#[command(name = "tmux-sniff", version, about)]
struct Cli {
    /// Input file (default: stdin).
    #[arg(short, long)]
    input: Option<String>,

    /// Hex dump mode.
    #[arg(long)]
    hex: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    println!(
        "tmux-sniff: input={:?}, hex={}",
        cli.input, cli.hex
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults() {
        let cli = Cli::parse_from(["tmux-sniff"]);
        assert!(cli.input.is_none());
        assert!(!cli.hex);
    }

    #[test]
    fn cli_with_file() {
        let cli = Cli::parse_from(["tmux-sniff", "--input", "dump.bin"]);
        assert_eq!(cli.input.as_deref(), Some("dump.bin"));
    }

    #[test]
    fn cli_hex_mode() {
        let cli = Cli::parse_from(["tmux-sniff", "--hex"]);
        assert!(cli.hex);
    }
}
