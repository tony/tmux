//! Command dispatch table.
//!
//! Maps command names to handler functions. Initial set covers the
//! ~30 most-used tmux commands.

/// A command handler result.
pub type CommandResult = Result<String, String>;

/// Built-in command names (initial 30).
pub const BUILTIN_COMMANDS: &[&str] = &[
    "attach-session", "bind-key", "break-pane", "choose-tree",
    "clock-mode", "command-prompt", "confirm-before", "copy-mode",
    "detach-client", "display-message", "display-panes", "find-window",
    "has-session", "join-pane", "kill-pane", "kill-server",
    "kill-session", "kill-window", "last-pane", "last-window",
    "link-window", "list-keys", "list-panes", "list-sessions",
    "list-windows", "move-pane", "move-window", "new-session",
    "new-window", "next-layout", "next-window", "paste-buffer",
    "pipe-pane", "previous-layout", "previous-window", "refresh-client",
    "rename-session", "rename-window", "resize-pane", "resize-window",
    "respawn-pane", "respawn-window", "rotate-window", "run-shell",
    "select-layout", "select-pane", "select-window", "send-keys",
    "send-prefix", "set-option", "show-options", "source-file",
    "split-window", "swap-pane", "swap-window", "switch-client",
    "unbind-key", "unlink-window", "wait-for",
];

/// Check if a command name is a known built-in.
#[must_use]
pub fn is_builtin(name: &str) -> bool {
    BUILTIN_COMMANDS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_commands() {
        assert!(is_builtin("new-session"));
        assert!(is_builtin("split-window"));
        assert!(is_builtin("kill-pane"));
    }

    #[test]
    fn unknown_commands() {
        assert!(!is_builtin("frobnicate"));
        assert!(!is_builtin(""));
    }

    #[test]
    fn command_count() {
        assert!(BUILTIN_COMMANDS.len() >= 30);
    }
}
