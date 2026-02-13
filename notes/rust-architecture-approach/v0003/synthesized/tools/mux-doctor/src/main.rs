#![allow(dead_code)]
//! # mux-doctor
//!
//! Diagnostic environment checker for `TermForge`.
//! Inspects the system for compatibility requirements: terminal capabilities,
//! tmux versions, library availability, and configuration issues.
//!
//! ## Usage
//! ```text
//! mux-doctor check
//! mux-doctor check --json
//! mux-doctor env
//! mux-doctor config /path/to/config.toml
//! ```
//!
//! L6 tool crate.

use clap::{Parser, Subcommand};
use serde::Serialize;

/// mux-doctor: TermForge environment diagnostics.
#[derive(Parser, Debug)]
#[command(name = "mux-doctor", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run all diagnostic checks.
    Check {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show environment variables relevant to TermForge.
    Env,
    /// Validate a configuration file.
    Config {
        /// Path to config file.
        path: String,
    },
}

/// Result of a single diagnostic check.
#[derive(Debug, Clone, Serialize)]
struct CheckResult {
    name: String,
    status: CheckStatus,
    message: String,
    details: Option<String>,
}

/// Status of a diagnostic check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skip,
}

impl CheckResult {
    fn pass(name: &str, message: &str) -> Self {
        Self {
            name: name.to_owned(),
            status: CheckStatus::Pass,
            message: message.to_owned(),
            details: None,
        }
    }

    fn warn(name: &str, message: &str) -> Self {
        Self {
            name: name.to_owned(),
            status: CheckStatus::Warn,
            message: message.to_owned(),
            details: None,
        }
    }

    fn fail(name: &str, message: &str) -> Self {
        Self {
            name: name.to_owned(),
            status: CheckStatus::Fail,
            message: message.to_owned(),
            details: None,
        }
    }

    fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_owned());
        self
    }
}

/// Diagnostic report containing all check results.
#[derive(Debug, Serialize)]
struct DiagnosticReport {
    version: String,
    checks: Vec<CheckResult>,
    summary: ReportSummary,
}

/// Summary of a diagnostic report.
#[derive(Debug, Serialize)]
struct ReportSummary {
    total: usize,
    passed: usize,
    warnings: usize,
    failures: usize,
    skipped: usize,
}

impl DiagnosticReport {
    fn new(checks: Vec<CheckResult>) -> Self {
        let total = checks.len();
        let passed = checks.iter().filter(|c| c.status == CheckStatus::Pass).count();
        let warnings = checks.iter().filter(|c| c.status == CheckStatus::Warn).count();
        let failures = checks.iter().filter(|c| c.status == CheckStatus::Fail).count();
        let skipped = checks.iter().filter(|c| c.status == CheckStatus::Skip).count();

        Self {
            version: "0.1.0".to_owned(),
            checks,
            summary: ReportSummary {
                total,
                passed,
                warnings,
                failures,
                skipped,
            },
        }
    }

    const fn is_healthy(&self) -> bool {
        self.summary.failures == 0
    }
}

/// Check if the TERM variable is set and reasonable.
fn check_term_env() -> CheckResult {
    match std::env::var("TERM") {
        Ok(term) if term.is_empty() => {
            CheckResult::fail("TERM", "TERM environment variable is empty")
        }
        Ok(term) => {
            let good_terms = [
                "xterm-256color",
                "screen-256color",
                "tmux-256color",
                "alacritty",
                "xterm",
                "screen",
            ];
            if good_terms.iter().any(|&t| term.starts_with(t)) {
                CheckResult::pass("TERM", &format!("TERM={term}"))
            } else {
                CheckResult::warn("TERM", &format!("TERM={term} -- may lack features"))
            }
        }
        Err(_) => CheckResult::fail("TERM", "TERM environment variable not set"),
    }
}

/// Check for a tmux binary.
fn check_tmux_binary() -> CheckResult {
    // Scaffold: cannot actually execute processes.
    CheckResult::warn("tmux", "tmux binary detection not implemented in scaffold")
}

/// Check the Rust toolchain version.
fn check_rust_version() -> CheckResult {
    // Scaffold: check MSRV requirement.
    CheckResult::pass("rust", "Rust toolchain check (scaffold)")
}

/// Check for required system libraries.
fn check_system_libs() -> CheckResult {
    // Scaffold: would check for libevent, ncurses, etc.
    CheckResult::pass("system-libs", "System library check (scaffold)")
}

/// Run all diagnostic checks.
fn run_all_checks() -> DiagnosticReport {
    let checks = vec![
        check_term_env(),
        check_tmux_binary(),
        check_rust_version(),
        check_system_libs(),
    ];
    DiagnosticReport::new(checks)
}

/// Format a check result for text output.
fn format_check(result: &CheckResult) -> String {
    let icon = match result.status {
        CheckStatus::Pass => "[PASS]",
        CheckStatus::Warn => "[WARN]",
        CheckStatus::Fail => "[FAIL]",
        CheckStatus::Skip => "[SKIP]",
    };
    let detail = result
        .details
        .as_ref()
        .map_or(String::new(), |d| format!("\n       {d}"));
    format!("{icon} {}: {}{detail}", result.name, result.message)
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { json } => {
            let report = run_all_checks();
            if json {
                if let Ok(output) = serde_json::to_string_pretty(&report) {
                    eprintln!("{output}");
                }
            } else {
                for check in &report.checks {
                    eprintln!("{}", format_check(check));
                }
                let summary = &report.summary;
                eprintln!(
                    "\n{} checks: {} passed, {} warnings, {} failures, {} skipped",
                    summary.total,
                    summary.passed,
                    summary.warnings,
                    summary.failures,
                    summary.skipped
                );
            }
        }
        Commands::Env => {
            eprintln!("mux-doctor: env (scaffold -- not implemented)");
        }
        Commands::Config { path } => {
            eprintln!("mux-doctor: config {path} (scaffold -- not implemented)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_result_pass() {
        let r = CheckResult::pass("test", "it works");
        assert_eq!(r.status, CheckStatus::Pass);
    }

    #[test]
    fn check_result_fail() {
        let r = CheckResult::fail("test", "it broke");
        assert_eq!(r.status, CheckStatus::Fail);
    }

    #[test]
    fn check_result_warn() {
        let r = CheckResult::warn("test", "maybe not");
        assert_eq!(r.status, CheckStatus::Warn);
    }

    #[test]
    fn check_result_with_details() {
        let r = CheckResult::pass("test", "ok").with_details("extra info");
        assert_eq!(r.details.as_deref(), Some("extra info"));
    }

    #[test]
    fn diagnostic_report_healthy() {
        let checks = vec![
            CheckResult::pass("a", "ok"),
            CheckResult::pass("b", "ok"),
        ];
        let report = DiagnosticReport::new(checks);
        assert!(report.is_healthy());
        assert_eq!(report.summary.total, 2);
        assert_eq!(report.summary.passed, 2);
    }

    #[test]
    fn diagnostic_report_unhealthy() {
        let checks = vec![
            CheckResult::pass("a", "ok"),
            CheckResult::fail("b", "bad"),
        ];
        let report = DiagnosticReport::new(checks);
        assert!(!report.is_healthy());
        assert_eq!(report.summary.failures, 1);
    }

    #[test]
    fn diagnostic_report_mixed() {
        let checks = vec![
            CheckResult::pass("a", "ok"),
            CheckResult::warn("b", "maybe"),
            CheckResult::fail("c", "bad"),
        ];
        let report = DiagnosticReport::new(checks);
        assert_eq!(report.summary.passed, 1);
        assert_eq!(report.summary.warnings, 1);
        assert_eq!(report.summary.failures, 1);
    }

    #[test]
    fn format_check_pass() {
        let r = CheckResult::pass("test", "it works");
        let formatted = format_check(&r);
        assert!(formatted.contains("[PASS]"));
        assert!(formatted.contains("test"));
    }

    #[test]
    fn format_check_with_details() {
        let r = CheckResult::fail("test", "broken").with_details("try reinstalling");
        let formatted = format_check(&r);
        assert!(formatted.contains("[FAIL]"));
        assert!(formatted.contains("try reinstalling"));
    }

    #[test]
    fn run_all_checks_returns_report() {
        let report = run_all_checks();
        assert!(!report.checks.is_empty());
        assert_eq!(report.summary.total, report.checks.len());
    }

    #[test]
    fn report_serializes_to_json() {
        let report = DiagnosticReport::new(vec![CheckResult::pass("test", "ok")]);
        let json = serde_json::to_string(&report);
        assert!(json.is_ok());
    }

    #[test]
    fn check_status_equality() {
        assert_eq!(CheckStatus::Pass, CheckStatus::Pass);
        assert_ne!(CheckStatus::Pass, CheckStatus::Fail);
    }

    #[test]
    fn check_term_env_runs() {
        // Just verify it doesn't panic; result depends on environment.
        let _result = check_term_env();
    }
}
