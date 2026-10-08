#include <tmux_cxx/pane/pane_output_cursor.hpp>
#include <tmux_cxx/protocol/tmux/compatibility_policy.hpp>
#include <tmux_cxx/protocol/tmux/protocol_quirk.hpp>
#include <tmux_cxx/session/session_event.hpp>

#include "protocol/tmux/control/limits.hpp"

#include <string>

namespace tmux_cxx::protocol::tmux {
StockConnectionCompatibility TmuxCompatibilityPolicy::assess(const TmuxProtocolProfile &profile,
                                                             ProtocolDirection direction,
                                                             ConnectionMode mode) const {
    StockConnectionCompatibility report;
    report.profile = profile.name;
    report.direction = direction;
    report.mode = mode;
    report.limitations.push_back("wire evidence does not identify the remote tmux release");
    if (direction == ProtocolDirection::cxx_client_to_stock_server) {
        report.evidence.push_back("the caller selected an explicit stock-server profile");
        report.limitations.push_back("server support requires a #{version} response");
        return report;
    }
    report.evidence.push_back("descriptor-bearing identification selected the framing profile");
    report.verification = CompatibilityVerification::tested_subset;
    report.evidence.push_back(profile.layout == legacy_profile.layout
                                  ? "tmux 3.4 client fixture passed the declared subset"
                                  : "tmux 3.7 client fixture passed the declared subset");
    report.limitations.push_back(
        "paste-buffer names require UTF-8, at most 128 bytes, and no control characters");
    report.limitations.push_back(
        "at most 50 explicitly named paste buffers of 256 KiB each are retained");
    report.limitations.push_back("automatic paste buffers and buffer listing are unsupported");
    report.limitations.push_back(
        "paste-buffer requires -b name and -t %pane; delete-after, custom separators, raw "
        "paste, and bracketed paste are unsupported");
    report.limitations.push_back(
        "set-option supports only -g and exact -t session assignments for prefix and prefix2; "
        "unset and implicit current-session assignment are unsupported");
    report.limitations.push_back(
        "prefix and prefix2 accept None, one ASCII byte, or C-a through C-z; broader tmux key "
        "syntax is unsupported");
    if (mode == ConnectionMode::command) {
        report.tier = SupportTier::general;
        report.support = CapabilitySupport::limited;
        report.limitations.push_back("only registered command spellings and syntax are available");
        report.limitations.push_back("session names require UTF-8 and at most 128 bytes");
    } else if (mode == ConnectionMode::control) {
        report.tier = SupportTier::general;
        report.support = CapabilitySupport::limited;
        report.evidence.push_back(profile.layout == legacy_profile.layout
                                      ? "tmux 3.4 control client passed the declared subset"
                                      : "tmux 3.7 control client passed the declared subset");
        (void)activate_protocol_quirk(report, ProtocolQuirk::unattached_control_channel_stays_open);
        report.limitations.push_back(
            "control input supports one registered command per line with quotes and escapes");
        report.limitations.push_back(
            "command groups, configuration grammar, and asynchronous commands are unsupported");
        report.evidence.push_back(
            "attached control clients receive session selection, committed session catalog and "
            "rename notifications, and new linked-pane output");
        report.limitations.push_back(
            "only explicit attach-session -t target attachment and switching are supported");
        report.limitations.push_back("echo-disabled -CC control transport is unsupported");
        report.limitations.push_back(
            "no-output, pause-after, and wait-exit control flags are rejected before effects");
        report.limitations.push_back(
            "control lines use a " + std::to_string(control::maximum_line_bytes / 1024U) +
            " KiB bound and at most " + std::to_string(control::maximum_arguments) + " arguments");
        report.limitations.push_back(
            "each readiness turn admits at most " +
            std::to_string(control::maximum_commands_per_turn) +
            " commands and output backlog is bounded to " +
            std::to_string(control::maximum_output_bytes / (1024U * 1024U)) + " MiB");
        report.limitations.push_back("pane output retains " +
                                     std::to_string(pane_output_retention_bytes / 1024U) +
                                     " KiB per pane and queues at most " +
                                     std::to_string(control::pane_output_bytes_per_turn / 1024U) +
                                     " KiB of raw bytes per control-client update turn");
        report.limitations.push_back("pane output preserves per-pane byte order without "
                                     "reconstructing cross-pane chronology");
        report.limitations.push_back(
            "other control notifications, pause/continue, and subscriptions are unsupported");
        report.limitations.push_back("control metadata retains at most " +
                                     std::to_string(session_event_retention_records) +
                                     " committed session events");
        report.limitations.push_back(
            "a metadata observation gap closes the control client instead of resynchronizing it");
    } else {
        report.tier = SupportTier::general;
        report.support = CapabilitySupport::limited;
        report.limitations.push_back("ordinary-client borders use basic ASCII without "
                                     "capability-sensitive glyphs or styles");
        report.limitations.push_back(
            "the prefix table contains only C-b send-prefix, quote and percent split, d detach, "
            "and o next-pane; configurable key tables and bindings are unsupported");
        report.limitations.push_back(
            "the latest attached client controls shared window dimensions");
    }
    return report;
}

StockConnectionCompatibility
TmuxCompatibilityPolicy::assess_stock_server(const TmuxProtocolProfile &profile,
                                             std::string_view release, ConnectionMode mode) const {
    StockConnectionCompatibility report;
    report.profile = profile.name;
    report.release = release;
    report.release_uncertain = false;
    report.direction = ProtocolDirection::cxx_client_to_stock_server;
    report.mode = mode;
    report.evidence.push_back("stock server returned #{version} over the selected profile");
    const bool verified_pair = (profile.layout == legacy_profile.layout && release == "3.4") ||
                               (profile.layout == modern_profile.layout && release == "3.7");
    if (!verified_pair) {
        report.limitations.push_back(
            "the server release and selected profile are not a tested pair");
        return report;
    }
    if (mode != ConnectionMode::command) {
        report.limitations.push_back("the C++ stock-server adapter supports command mode only");
        return report;
    }
    report.tier = SupportTier::general;
    report.support = CapabilitySupport::limited;
    report.verification = CompatibilityVerification::tested_subset;
    report.evidence.push_back(release == "3.4" ? "tmux 3.4 server fixture passed command mode"
                                               : "tmux 3.7 server fixture passed command mode");
    report.limitations.push_back("only stdout and stderr command streams are captured");
    report.limitations.push_back(
        "session-name listing can fail if sessions change between its live queries");
    report.limitations.push_back(
        "interactive, control, read-file, and arbitrary file streams are unsupported");
    return report;
}
} // namespace tmux_cxx::protocol::tmux
