#include "terminal/terminal_key_decoder.hpp"

#include <algorithm>
#include <charconv>

// Independent subset of tmux 3.7 tty-keys.c at
// 81f88f8517c9fc5371b56cf117530c6b477c96ac: CSI/SS3, UTF-8, Meta, and partial-key timing.
// The modern 10 ms default replaces tmux 3.4's 500 ms default; terminfo and extended keys remain
// open.

namespace tmux_cxx {
namespace {
using key_table::KeyCode;
using key_table::KeyModifier;
constexpr DecodedTerminalKey pending_key{TerminalKeyStatus::pending, std::nullopt,
                                         0}; ///< Incomplete input retains all original bytes.
constexpr std::chrono::milliseconds key_timeout{10}; ///< Modern tmux partial-key deadline.

/** @brief Decode one Unicode scalar, rejecting invalid or expired partial encodings. */
DecodedTerminalKey decode_utf8_key(std::string_view bytes, bool expired) {
    const auto lead = static_cast<unsigned char>(bytes.front());
    if (lead < 0x80) {
        return {TerminalKeyStatus::decoded, KeyCode{lead}, 1};
    }
    std::size_t width = 0;
    std::uint32_t scalar = 0;
    if (lead >= 0xc2 && lead <= 0xdf) {
        width = 2;
        scalar = lead & 0x1fU;
    } else if (lead >= 0xe0 && lead <= 0xef) {
        width = 3;
        scalar = lead & 0x0fU;
    } else if (lead >= 0xf0 && lead <= 0xf4) {
        width = 4;
        scalar = lead & 0x07U;
    } else {
        return {TerminalKeyStatus::unsupported, std::nullopt, 1};
    }
    for (std::size_t index = 1; index < width; ++index) {
        if (index == bytes.size()) {
            return expired ? DecodedTerminalKey{TerminalKeyStatus::unsupported, std::nullopt, index}
                           : pending_key;
        }
        const auto continuation = static_cast<unsigned char>(bytes[index]);
        if ((continuation & 0xc0U) != 0x80U) {
            return {TerminalKeyStatus::unsupported, std::nullopt, index};
        }
        scalar = (scalar << 6U) | (continuation & 0x3fU);
    }
    const std::uint32_t minimum = width == 2 ? 0x80U : width == 3 ? 0x800U : 0x10000U;
    if (scalar < minimum || scalar > 0x10ffffU || (scalar >= 0xd800U && scalar <= 0xdfffU)) {
        return {TerminalKeyStatus::unsupported, std::nullopt, width};
    }
    return {TerminalKeyStatus::decoded, KeyCode{scalar}, width};
}

/** @brief Normalize the supported xterm cursor and paging sequence parameters. */
std::optional<KeyCode> xterm_sequence_key(char final, std::string_view parameters) {
    std::uint32_t primary = 1;
    KeyModifier modifiers = KeyModifier::none;
    if (!parameters.empty()) {
        const auto separator = parameters.find(';');
        const auto first = parameters.substr(0, separator);
        const auto parsed = std::from_chars(first.data(), first.data() + first.size(), primary);
        if (parsed.ec != std::errc{} || parsed.ptr != first.data() + first.size()) {
            return std::nullopt;
        }
        if (separator != std::string_view::npos) {
            const auto modifier = parameters.substr(separator + 1);
            if (modifier.size() != 1 || modifier.front() < '1' || modifier.front() > '9') {
                return std::nullopt;
            }
            const auto bits =
                modifier.front() == '9' ? 2U : static_cast<unsigned>(modifier.front() - '1');
            modifiers = static_cast<KeyModifier>(bits);
        }
    }
    std::optional<KeyCode> key;
    if (final == '~') {
        if (parameters.empty()) {
            return std::nullopt;
        }
        switch (primary) {
        case 1:
        case 7:
            key = key_table::home_key;
            break;
        case 4:
        case 8:
            key = key_table::end_key;
            break;
        case 5:
            key = key_table::page_up_key;
            break;
        case 6:
            key = key_table::page_down_key;
            break;
        default:
            return std::nullopt;
        }
    } else if (primary == 1) {
        switch (final) {
        case 'A':
            key = key_table::up_key;
            break;
        case 'B':
            key = key_table::down_key;
            break;
        case 'C':
            key = key_table::right_key;
            break;
        case 'D':
            key = key_table::left_key;
            break;
        case 'H':
            key = key_table::home_key;
            break;
        case 'F':
            key = key_table::end_key;
            break;
        default:
            return std::nullopt;
        }
    }
    if (key) {
        key->modifiers = modifiers;
    }
    return key;
}

/** @brief Inspect one bounded CSI or SS3 sequence without swallowing following control bytes. */
DecodedTerminalKey decode_escape_sequence(std::string_view bytes, bool expired,
                                          std::size_t maximum_bytes) {
    const bool ss3 = bytes[1] == 'O';
    const auto inspected_bytes = std::min(bytes.size(), maximum_bytes);
    for (std::size_t index = 2; index < inspected_bytes; ++index) {
        const auto character = static_cast<unsigned char>(bytes[index]);
        if (character < 0x20 || character >= 0x7f) {
            return {TerminalKeyStatus::unsupported, std::nullopt, index};
        }
        if (character >= 0x40 && character <= 0x7e) {
            const auto parameters = bytes.substr(2, index - 2);
            const auto key = ss3 && !parameters.empty()
                                 ? std::nullopt
                                 : xterm_sequence_key(static_cast<char>(character), parameters);
            return {key ? TerminalKeyStatus::decoded : TerminalKeyStatus::unsupported, key,
                    index + 1};
        }
        if ((character < '0' || character > '9') && character != ';') {
            return {TerminalKeyStatus::unsupported, std::nullopt, index + 1};
        }
    }
    if (inspected_bytes == maximum_bytes) {
        return {TerminalKeyStatus::unsupported, std::nullopt, maximum_bytes};
    }
    return expired ? DecodedTerminalKey{TerminalKeyStatus::decoded, key_table::escape_key, 1}
                   : pending_key;
}

/** @brief Normalize one terminal key while preserving literal Escape and Meta boundaries. */
DecodedTerminalKey inspect_terminal_key(std::string_view bytes, bool expired) {
    if (bytes.front() != '\x1b') {
        return decode_utf8_key(bytes, expired);
    }
    if (bytes.size() == 1) {
        return expired ? DecodedTerminalKey{TerminalKeyStatus::decoded, key_table::escape_key, 1}
                       : pending_key;
    }
    if (bytes[1] == '[' || bytes[1] == 'O') {
        return decode_escape_sequence(bytes, expired, TerminalKeyDecoder::maximum_key_bytes);
    }
    if (bytes[1] == '\x1b') {
        if (bytes.size() == 2 && !expired) {
            return pending_key;
        }
        if (bytes.size() >= 3 && (bytes[2] == '[' || bytes[2] == 'O')) {
            auto inner = decode_escape_sequence(bytes.substr(1), expired,
                                                TerminalKeyDecoder::maximum_key_bytes - 1);
            if (inner.status == TerminalKeyStatus::pending) {
                return pending_key;
            }
            if (inner.key && inner.key != key_table::escape_key &&
                (static_cast<std::uint8_t>(inner.key->modifiers) &
                 static_cast<std::uint8_t>(KeyModifier::meta)) == 0) {
                inner.key->modifiers = inner.key->modifiers | KeyModifier::meta;
                ++inner.byte_count;
                return inner;
            }
        }
        return {TerminalKeyStatus::decoded, key_table::escape_key, 1};
    }
    auto key = decode_utf8_key(bytes.substr(1), expired);
    if (key.status != TerminalKeyStatus::pending) {
        if (key.key) {
            key.key->modifiers = key.key->modifiers | KeyModifier::meta;
        }
        ++key.byte_count;
    }
    return key;
}
} // namespace

DecodedTerminalKey TerminalKeyDecoder::peek(std::string_view bytes, Clock::time_point now) {
    if (completed_) {
        return *completed_;
    }
    if (bytes.empty()) {
        return pending_key;
    }
    auto inspected = inspect_terminal_key(bytes, deadline_ && now >= *deadline_);
    if (inspected.status == TerminalKeyStatus::pending) {
        if (!deadline_) {
            deadline_ = now + key_timeout;
        }
    } else {
        completed_ = inspected;
        deadline_.reset();
    }
    return inspected;
}

void TerminalKeyDecoder::consume() {
    completed_.reset();
    deadline_.reset();
}

std::optional<TerminalKeyDecoder::Clock::time_point> TerminalKeyDecoder::next_deadline() const {
    return deadline_;
}
} // namespace tmux_cxx
