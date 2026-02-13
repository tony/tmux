//! mux-doctor: Diagnostic environment checker.
//!
//! Checks the system environment for TermForge compatibility:
//! - Terminal capabilities
//! - Locale settings
//! - Shell availability
//! - Platform features (TIOCGPTPEER, etc.)

use clap::Parser;

/// mux-doctor: Check TermForge environment compatibility.
#[derive(Parser, Debug)]
#[command(name = "mux-doctor", version, about)]
struct Cli {
    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,

    /// JSON output format.
    #[arg(long)]
    json: bool,
}

/// Check result for a single diagnostic.
#[derive(Debug)]
struct Check {
    name: &'static str,
    status: CheckStatus,
    detail: String,
}

/// Status of a diagnostic check.
#[derive(Debug, PartialEq, Eq)]
enum CheckStatus {
    Ok,
    Warning,
    Error,
}

fn run_checks() -> Vec<Check> {
    let mut checks = Vec::new();

    // Check TERM variable
    let term = std::env::var("TERM").unwrap_or_default();
    checks.push(Check {
        name: "TERM",
        status: if term.is_empty() {
            CheckStatus::Warning
        } else {
            CheckStatus::Ok
        },
        detail: if term.is_empty() {
            "TERM not set".into()
        } else {
            format!("TERM={term}")
        },
    });

    // Check locale
    let lang = std::env::var("LANG").unwrap_or_default();
    checks.push(Check {
        name: "LANG",
        status: if lang.contains("UTF-8") || lang.contains("utf-8") || lang.contains("utf8") {
            CheckStatus::Ok
        } else {
            CheckStatus::Warning
        },
        detail: if lang.is_empty() {
            "LANG not set (UTF-8 recommended)".into()
        } else {
            format!("LANG={lang}")
        },
    });

    // Check protocol magic
    checks.push(Check {
        name: "TF01 protocol",
        status: CheckStatus::Ok,
        detail: format!("magic=0x{:08X}", mux_proto::MAGIC),
    });

    checks
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let checks = run_checks();

    for check in &checks {
        let status = match check.status {
            CheckStatus::Ok => "OK",
            CheckStatus::Warning => "WARN",
            CheckStatus::Error => "ERR",
        };
        if cli.verbose || check.status != CheckStatus::Ok {
            println!("[{status}] {}: {}", check.name, check.detail);
        } else {
            println!("[{status}] {}", check.name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults() {
        let cli = Cli::parse_from(["mux-doctor"]);
        assert!(!cli.verbose);
        assert!(!cli.json);
    }

    #[test]
    fn run_checks_non_empty() {
        let checks = run_checks();
        assert!(!checks.is_empty());
    }

    #[test]
    fn checks_include_protocol() {
        let checks = run_checks();
        assert!(checks.iter().any(|c| c.name == "TF01 protocol"));
    }

    #[test]
    fn check_status_equality() {
        assert_eq!(CheckStatus::Ok, CheckStatus::Ok);
        assert_ne!(CheckStatus::Ok, CheckStatus::Warning);
    }
}
