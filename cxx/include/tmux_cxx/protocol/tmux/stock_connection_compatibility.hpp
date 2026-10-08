#pragma once

#include <string>
#include <string_view>
#include <vector>

namespace tmux_cxx::protocol::tmux {
/** @brief Distinguish the two stock adapter roles without sharing their connection state. */
enum class ProtocolDirection { stock_client_to_cxx_server, cxx_client_to_stock_server };
/** @brief Identify command exchange, interactive attachment, or stock control grammar. */
enum class ConnectionMode { command, interactive, control };
/** @brief Separate capability fidelity from release support tiers and verification. */
enum class CapabilitySupport { supported, adapted, limited, unsupported, unknown };
/** @brief Classify declared release support independently of individual capabilities. */
enum class SupportTier { unsupported, general, baseline };
/** @brief Keep connection support declarations separate from actual compatibility evidence. */
enum class CompatibilityVerification { unverified, tested_subset, passed_baseline };
/** @brief Report selected framing, unknown release evidence, effective support, and active
 * adaptations. */
struct StockConnectionCompatibility {
    /** @brief Selected wire layout identity, separate from a tmux release. */
    std::string profile;
    /** @brief Remote tmux release from trusted metadata or a verified server response. */
    std::string release;
    /** @brief True until the remote tmux endpoint supplies trustworthy release evidence. */
    bool release_uncertain = true;
    /** @brief Adapter role whose behavior this report assesses. */
    ProtocolDirection direction = ProtocolDirection::stock_client_to_cxx_server;
    /** @brief Requested stock connection behavior. */
    ConnectionMode mode = ConnectionMode::command;
    /** @brief Effective release-support tier without implying complete older-version parity. */
    SupportTier tier = SupportTier::unsupported;
    /** @brief Effective capability fidelity for the requested mode. */
    CapabilitySupport support = CapabilitySupport::unsupported;
    /** @brief Evidence status; a profile match alone proves no behavioral baseline. */
    CompatibilityVerification verification = CompatibilityVerification::unverified;
    /** @brief Registered command spellings available under this connection policy. */
    std::vector<std::string> commands;
    /** @brief Active named adaptations; an empty list means none. */
    std::vector<std::string> quirks;
    /** @brief Conformance and detection facts supporting this report. */
    std::vector<std::string> evidence;
    /** @brief Observable restrictions and missing release evidence. */
    std::vector<std::string> limitations;
};

/** @brief Return the stable public spelling of a stock adapter direction. */
constexpr std::string_view protocol_direction_name(ProtocolDirection direction) {
    switch (direction) {
    case ProtocolDirection::stock_client_to_cxx_server:
        return "stock_client_to_cxx_server";
    case ProtocolDirection::cxx_client_to_stock_server:
        return "cxx_client_to_stock_server";
    }
    return "unknown";
}

/** @brief Return the stable public spelling of a stock connection mode. */
constexpr std::string_view connection_mode_name(ConnectionMode mode) {
    switch (mode) {
    case ConnectionMode::command:
        return "command";
    case ConnectionMode::interactive:
        return "interactive";
    case ConnectionMode::control:
        return "control";
    }
    return "unknown";
}

/** @brief Return the stable public spelling of effective capability fidelity. */
constexpr std::string_view capability_support_name(CapabilitySupport support) {
    switch (support) {
    case CapabilitySupport::supported:
        return "supported";
    case CapabilitySupport::adapted:
        return "adapted";
    case CapabilitySupport::limited:
        return "limited";
    case CapabilitySupport::unsupported:
        return "unsupported";
    case CapabilitySupport::unknown:
        return "unknown";
    }
    return "unknown";
}

/** @brief Return the stable public spelling of a release-support tier. */
constexpr std::string_view support_tier_name(SupportTier tier) {
    switch (tier) {
    case SupportTier::unsupported:
        return "unsupported";
    case SupportTier::general:
        return "general";
    case SupportTier::baseline:
        return "baseline";
    }
    return "unknown";
}

/** @brief Return the stable public spelling of compatibility evidence strength. */
constexpr std::string_view compatibility_verification_name(CompatibilityVerification verification) {
    switch (verification) {
    case CompatibilityVerification::unverified:
        return "unverified";
    case CompatibilityVerification::tested_subset:
        return "tested_subset";
    case CompatibilityVerification::passed_baseline:
        return "passed_baseline";
    }
    return "unknown";
}
} // namespace tmux_cxx::protocol::tmux

namespace tmux_cxx {
/** @brief Public value alias for stock-tmux compatibility reports exposed by server operations. */
using StockConnectionCompatibility = protocol::tmux::StockConnectionCompatibility;
} // namespace tmux_cxx
