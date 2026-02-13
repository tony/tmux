//! # tmux-vm
//!
//! tmux version manager -- downloads, builds, and manages multiple tmux
//! versions for compatibility testing.
//!
//! ## Usage
//! ```text
//! tmux-vm install 3.5a
//! tmux-vm list
//! tmux-vm use 3.4
//! tmux-vm run 3.3a -- list-sessions
//! tmux-vm remove 3.2
//! ```
//!
//! L6 tool crate.

#![allow(dead_code)]

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

/// tmux-vm: tmux version manager for compatibility testing.
#[derive(Parser, Debug)]
#[command(name = "tmux-vm", version, about)]
struct Cli {
    /// Base directory for tmux installations.
    #[arg(long, default_value = "~/.tmux-vm")]
    base_dir: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install a tmux version.
    Install {
        /// Version to install (e.g., "3.5a").
        version: String,
    },
    /// List installed tmux versions.
    List,
    /// Set the default tmux version.
    Use {
        /// Version to use.
        version: String,
    },
    /// Run a command with a specific tmux version.
    Run {
        /// tmux version to use.
        version: String,
        /// Arguments to pass to tmux.
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Remove a tmux version.
    Remove {
        /// Version to remove.
        version: String,
    },
    /// Show the path to a tmux binary.
    Which {
        /// Version to look up.
        version: String,
    },
}

/// Represents an installed tmux version.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TmuxInstallation {
    version: String,
    path: PathBuf,
    is_default: bool,
}

/// Resolve the base directory, expanding ~.
fn resolve_base_dir(base: &str) -> PathBuf {
    if base.starts_with('~') {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(&base[2..]);
        }
    }
    PathBuf::from(base)
}

/// Get the binary path for a specific version.
fn binary_path(base_dir: &Path, version: &str) -> PathBuf {
    base_dir
        .join("versions")
        .join(version)
        .join("bin")
        .join("tmux")
}

/// Validate a version string.
fn validate_version(version: &str) -> Result<(), String> {
    if version.is_empty() {
        return Err("version cannot be empty".to_owned());
    }

    // Must start with a digit.
    let first = version
        .chars()
        .next()
        .ok_or_else(|| "version cannot be empty".to_owned())?;
    if !first.is_ascii_digit() {
        return Err(format!("version must start with a digit, got: {first}"));
    }

    // Must contain a dot.
    if !version.contains('.') {
        return Err("version must contain a dot (e.g., 3.5a)".to_owned());
    }

    Ok(())
}

/// Get the download URL for a tmux release tarball.
fn download_url(version: &str) -> String {
    format!("https://github.com/tmux/tmux/releases/download/{version}/tmux-{version}.tar.gz")
}

fn main() {
    let cli = Cli::parse();
    let base_dir = resolve_base_dir(&cli.base_dir);

    match cli.command {
        Commands::Install { version } => {
            eprintln!("tmux-vm: install {version} to {} (scaffold -- not implemented)", base_dir.display());
        }
        Commands::List => {
            eprintln!("tmux-vm: list installed versions (scaffold -- not implemented)");
        }
        Commands::Use { version } => {
            eprintln!("tmux-vm: use {version} (scaffold -- not implemented)");
        }
        Commands::Run { version, args } => {
            eprintln!("tmux-vm: run {version} with args {args:?} (scaffold -- not implemented)");
        }
        Commands::Remove { version } => {
            eprintln!("tmux-vm: remove {version} (scaffold -- not implemented)");
        }
        Commands::Which { version } => {
            let path = binary_path(&base_dir, &version);
            eprintln!("tmux-vm: {version} -> {}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_version_valid() {
        assert!(validate_version("3.5a").is_ok());
        assert!(validate_version("3.4").is_ok());
        assert!(validate_version("2.9a").is_ok());
    }

    #[test]
    fn validate_version_empty() {
        assert!(validate_version("").is_err());
    }

    #[test]
    fn validate_version_no_dot() {
        assert!(validate_version("35a").is_err());
    }

    #[test]
    fn validate_version_starts_with_letter() {
        assert!(validate_version("v3.5").is_err());
    }

    #[test]
    fn binary_path_format() {
        let base = PathBuf::from("/home/user/.tmux-vm");
        let path = binary_path(&base, "3.5a");
        assert_eq!(
            path,
            PathBuf::from("/home/user/.tmux-vm/versions/3.5a/bin/tmux")
        );
    }

    #[test]
    fn download_url_format() {
        let url = download_url("3.5a");
        assert_eq!(
            url,
            "https://github.com/tmux/tmux/releases/download/3.5a/tmux-3.5a.tar.gz"
        );
    }

    #[test]
    fn resolve_base_dir_absolute() {
        let resolved = resolve_base_dir("/opt/tmux-vm");
        assert_eq!(resolved, PathBuf::from("/opt/tmux-vm"));
    }

    #[test]
    fn resolve_base_dir_tilde() {
        // This test depends on HOME being set; if not, returns raw path.
        let resolved = resolve_base_dir("~/.tmux-vm");
        if std::env::var_os("HOME").is_some() {
            assert!(!resolved.to_string_lossy().contains('~'));
        }
    }

    #[test]
    fn tmux_installation_equality() {
        let a = TmuxInstallation {
            version: "3.5a".to_owned(),
            path: PathBuf::from("/a"),
            is_default: false,
        };
        let b = TmuxInstallation {
            version: "3.5a".to_owned(),
            path: PathBuf::from("/a"),
            is_default: false,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn tmux_installation_different_versions() {
        let a = TmuxInstallation {
            version: "3.5a".to_owned(),
            path: PathBuf::from("/a"),
            is_default: false,
        };
        let b = TmuxInstallation {
            version: "3.4".to_owned(),
            path: PathBuf::from("/a"),
            is_default: false,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn validate_version_minor_only() {
        assert!(validate_version("3.").is_ok()); // technically has a dot
    }

    #[test]
    fn download_url_no_suffix() {
        let url = download_url("3.4");
        assert!(url.ends_with("tmux-3.4.tar.gz"));
    }
}
