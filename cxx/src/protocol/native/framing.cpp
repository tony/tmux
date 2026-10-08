#include "protocol/native/framing.hpp"
#include "protocol/native/handshake.hpp"
#include "text/utf8.hpp"

#include <algorithm>
#include <stdexcept>
#include <utility>

namespace tmux_cxx::protocol::native {
namespace {
constexpr std::size_t compatibility_list_limit = 128; ///< Bound each report list on native input.
constexpr std::size_t compatibility_name_limit = 128; ///< Bound commands and quirk identities.
constexpr std::size_t compatibility_note_limit = 512; ///< Bound evidence and limitation entries.

/** @brief Encode one bounded UTF-8 report list without admitting embedded NUL bytes. */
void write_compatibility_text(NativePayloadWriter &writer, const std::vector<std::string> &texts,
                              std::size_t text_byte_limit) {
    if (texts.size() > compatibility_list_limit) {
        throw std::length_error("native compatibility list exceeds capacity");
    }
    writer.u32(static_cast<std::uint32_t>(texts.size()));
    for (const auto &text_entry : texts) {
        if (text_entry.size() > text_byte_limit || !text::valid_utf8(text_entry) ||
            text_entry.find('\0') != std::string::npos) {
            throw std::invalid_argument("invalid native compatibility text");
        }
        writer.string(text_entry);
    }
}

/** @brief Decode one bounded UTF-8 report list without retaining payload references. */
std::vector<std::string> read_compatibility_text(NativePayloadReader &reader,
                                                 std::size_t text_byte_limit) {
    const auto text_count = reader.u32();
    if (text_count > compatibility_list_limit) {
        throw std::invalid_argument("native compatibility list exceeds capacity");
    }
    std::vector<std::string> texts;
    texts.reserve(text_count);
    for (std::uint32_t index = 0; index < text_count; ++index) {
        auto text_entry = reader.string();
        if (text_entry.size() > text_byte_limit || !text::valid_utf8(text_entry) ||
            text_entry.find('\0') != std::string::npos) {
            throw std::invalid_argument("invalid native compatibility text");
        }
        texts.push_back(std::move(text_entry));
    }
    return texts;
}

/** @brief Decode one closed compatibility enum or reject values outside its declaration. */
template <class Enum> Enum read_compatibility_enum(NativePayloadReader &reader, Enum last) {
    const auto value = reader.u16();
    if (value > std::to_underlying(last)) {
        throw std::invalid_argument("invalid native compatibility classification");
    }
    return static_cast<Enum>(value);
}
} // namespace

void NativePayloadWriter::u16(std::uint16_t value) {
    for (unsigned i = 0; i < 2; ++i) {
        bytes.push_back(static_cast<std::uint8_t>(value >> (i * 8)));
    }
}
void NativePayloadWriter::u32(std::uint32_t value) {
    for (unsigned i = 0; i < 4; ++i) {
        bytes.push_back(static_cast<std::uint8_t>(value >> (i * 8)));
    }
}
void NativePayloadWriter::u64(std::uint64_t value) {
    for (unsigned i = 0; i < 8; ++i) {
        bytes.push_back(static_cast<std::uint8_t>(value >> (i * 8)));
    }
}
void NativePayloadWriter::string(std::string_view value) {
    if (value.size() > max_payload) {
        throw std::length_error("native string exceeds frame capacity");
    }
    u32(static_cast<std::uint32_t>(value.size()));
    bytes.insert(bytes.end(), value.begin(), value.end());
}
void NativePayloadWriter::session(const SessionSnapshot &snapshot) {
    u64(snapshot.id.value);
    u64(snapshot.window.value);
    u64(snapshot.link.value);
    u64(snapshot.pane.value);
    string(snapshot.name);
    u64(snapshot.revision);
}
void NativePayloadWriter::window_link(const WindowLinkSnapshot &snapshot) {
    u64(snapshot.id.value);
    u64(snapshot.session.value);
    u64(snapshot.window.value);
    u32(snapshot.index);
    u64(snapshot.revision);
}
void NativePayloadWriter::window(const WindowSnapshot &snapshot) {
    u64(snapshot.id.value);
    u64(snapshot.pane.value);
    string(snapshot.name);
    u32(snapshot.rows);
    u32(snapshot.columns);
    u64(snapshot.revision);
}
void NativePayloadWriter::pane(const PaneSnapshot &snapshot) {
    u64(snapshot.id.value);
    u64(snapshot.window.value);
    u32(snapshot.top);
    u32(snapshot.left);
    u32(snapshot.rows);
    u32(snapshot.columns);
    u16(snapshot.active ? 1 : 0);
    u64(snapshot.revision);
}
void NativePayloadWriter::client(const ClientSnapshot &snapshot) {
    u64(snapshot.id.value);
    u64(snapshot.session ? snapshot.session->value : 0);
    string(snapshot.detached_from);
    u16(static_cast<std::uint16_t>((snapshot.identified ? 1U : 0U) |
                                   (snapshot.has_terminal ? 2U : 0U)));
    u32(snapshot.rows);
    u32(snapshot.columns);
    u64(snapshot.revision);
}
void NativePayloadWriter::stock_compatibility(
    const protocol::tmux::StockConnectionCompatibility &compatibility) {
    string(compatibility.profile);
    string(compatibility.release);
    u16(compatibility.release_uncertain ? 1 : 0);
    u16(std::to_underlying(compatibility.direction));
    u16(std::to_underlying(compatibility.mode));
    u16(std::to_underlying(compatibility.tier));
    u16(std::to_underlying(compatibility.support));
    u16(std::to_underlying(compatibility.verification));
    write_compatibility_text(*this, compatibility.commands, compatibility_name_limit);
    write_compatibility_text(*this, compatibility.quirks, compatibility_name_limit);
    write_compatibility_text(*this, compatibility.evidence, compatibility_note_limit);
    write_compatibility_text(*this, compatibility.limitations, compatibility_note_limit);
}
std::uint64_t NativePayloadReader::integer(std::size_t width) {
    if (width > bytes_.size() - offset_) {
        throw std::invalid_argument("truncated native value");
    }
    std::uint64_t value = 0;
    for (std::size_t i = 0; i < width; ++i) {
        value |= static_cast<std::uint64_t>(bytes_[offset_++]) << (i * 8);
    }
    return value;
}
std::uint16_t NativePayloadReader::u16() { return static_cast<std::uint16_t>(integer(2)); }
std::uint32_t NativePayloadReader::u32() { return static_cast<std::uint32_t>(integer(4)); }
std::uint64_t NativePayloadReader::u64() { return integer(8); }
std::string NativePayloadReader::string() {
    auto size = u32();
    if (size > bytes_.size() - offset_) {
        throw std::invalid_argument("truncated native string");
    }
    std::string value(reinterpret_cast<const char *>(bytes_.data() + offset_), size);
    offset_ += size;
    return value;
}
SessionSnapshot NativePayloadReader::session() {
    SessionSnapshot snapshot{};
    snapshot.id = SessionId{u64()};
    snapshot.window = WindowId{u64()};
    snapshot.link = WindowLinkId{u64()};
    snapshot.pane = PaneId{u64()};
    snapshot.name = string();
    snapshot.revision = u64();
    if (!text::valid_utf8(snapshot.name)) {
        throw std::invalid_argument("invalid UTF-8 session name");
    }
    return snapshot;
}
WindowLinkSnapshot NativePayloadReader::window_link() {
    return {WindowLinkId{u64()}, SessionId{u64()}, WindowId{u64()}, u32(), u64()};
}
WindowSnapshot NativePayloadReader::window() {
    WindowSnapshot snapshot{WindowId{u64()}, PaneId{u64()}, string(), u32(), u32(), u64()};
    if (!text::valid_utf8(snapshot.name) || snapshot.rows == 0 || snapshot.columns == 0 ||
        snapshot.rows > 512 || snapshot.columns > 512 ||
        static_cast<std::uint64_t>(snapshot.rows) * snapshot.columns > 16384) {
        throw std::invalid_argument("invalid window name or terminal dimensions");
    }
    return snapshot;
}
PaneSnapshot NativePayloadReader::pane() {
    PaneSnapshot snapshot{PaneId{u64()}, WindowId{u64()}, u32(), u32(), u32(), u32(), false, 0};
    const auto active = u16();
    snapshot.revision = u64();
    snapshot.active = active != 0;
    if (snapshot.id.value == 0 || snapshot.window.value == 0 || snapshot.rows == 0 ||
        snapshot.columns == 0 || snapshot.rows > 512 || snapshot.columns > 512 ||
        snapshot.top > 511 || snapshot.left > 511 || snapshot.top + snapshot.rows > 512 ||
        snapshot.left + snapshot.columns > 512 ||
        static_cast<std::uint64_t>(snapshot.rows) * snapshot.columns > 16384 || active > 1 ||
        snapshot.revision == 0) {
        throw std::invalid_argument("invalid native pane geometry");
    }
    return snapshot;
}
ClientSnapshot NativePayloadReader::client() {
    ClientSnapshot snapshot{};
    snapshot.id = ClientId{u64()};
    const auto session = u64();
    if (session != 0) {
        snapshot.session = SessionId{session};
    }
    snapshot.detached_from = string();
    const auto flags = u16();
    snapshot.identified = (flags & 1U) != 0;
    snapshot.has_terminal = (flags & 2U) != 0;
    snapshot.rows = u32();
    snapshot.columns = u32();
    snapshot.revision = u64();
    const auto terminal_area = static_cast<std::uint64_t>(snapshot.rows) * snapshot.columns;
    if (snapshot.id.value == 0 || snapshot.revision == 0 || flags > 3 ||
        snapshot.detached_from.size() > 128 || !text::valid_utf8(snapshot.detached_from) ||
        snapshot.detached_from.find('\0') != std::string::npos ||
        (snapshot.session && !snapshot.detached_from.empty()) ||
        (!snapshot.identified && (snapshot.has_terminal || snapshot.session)) ||
        (snapshot.has_terminal &&
         (snapshot.rows == 0 || snapshot.columns == 0 || snapshot.rows > 512 ||
          snapshot.columns > 512 || terminal_area > 16384)) ||
        (!snapshot.has_terminal && (snapshot.rows != 0 || snapshot.columns != 0))) {
        throw std::invalid_argument("invalid client lifecycle or terminal dimensions");
    }
    return snapshot;
}

protocol::tmux::StockConnectionCompatibility NativePayloadReader::stock_compatibility() {
    using namespace protocol::tmux;
    StockConnectionCompatibility compatibility{};
    compatibility.profile = string();
    compatibility.release = string();
    const auto uncertain = u16();
    if (uncertain > 1) {
        throw std::invalid_argument("invalid native release uncertainty");
    }
    compatibility.release_uncertain = uncertain != 0;
    compatibility.direction =
        read_compatibility_enum(*this, ProtocolDirection::cxx_client_to_stock_server);
    compatibility.mode = read_compatibility_enum(*this, ConnectionMode::control);
    compatibility.tier = read_compatibility_enum(*this, SupportTier::baseline);
    compatibility.support = read_compatibility_enum(*this, CapabilitySupport::unknown);
    compatibility.verification =
        read_compatibility_enum(*this, CompatibilityVerification::passed_baseline);
    compatibility.commands = read_compatibility_text(*this, compatibility_name_limit);
    compatibility.quirks = read_compatibility_text(*this, compatibility_name_limit);
    compatibility.evidence = read_compatibility_text(*this, compatibility_note_limit);
    compatibility.limitations = read_compatibility_text(*this, compatibility_note_limit);
    if (compatibility.profile.empty() || compatibility.profile.size() > compatibility_name_limit ||
        !text::valid_utf8(compatibility.profile) ||
        compatibility.profile.find('\0') != std::string::npos ||
        compatibility.release.size() > compatibility_name_limit ||
        !text::valid_utf8(compatibility.release) ||
        compatibility.release.find('\0') != std::string::npos ||
        (compatibility.release.empty() && !compatibility.release_uncertain)) {
        throw std::invalid_argument("invalid native compatibility identity or release evidence");
    }
    return compatibility;
}

void NativePayloadReader::require_end() const {
    if (!finished()) {
        throw std::invalid_argument("excess native arguments");
    }
}

Bytes frame(OperationId operation, std::span<const std::uint8_t> payload) {
    if (payload.size() > max_payload) {
        throw std::length_error("native frame exceeds capacity");
    }
    NativePayloadWriter writer;
    writer.bytes.insert(writer.bytes.end(), {'T', 'M', 'X', 'D'});
    writer.u16(protocol_version);
    writer.u16(operation.value);
    writer.u32(static_cast<std::uint32_t>(payload.size()));
    writer.bytes.insert(writer.bytes.end(), payload.begin(), payload.end());
    return writer.bytes;
}

Result<std::size_t> frame_size(std::span<const std::uint8_t> frame_prefix) {
    if (frame_prefix.size() < header_size || frame_prefix[0] != 'T' || frame_prefix[1] != 'M' ||
        frame_prefix[2] != 'X' || frame_prefix[3] != 'D') {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, "invalid native frame header"});
    }
    NativePayloadReader reader(frame_prefix.subspan(4));
    if (reader.u16() != protocol_version) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "unsupported native protocol version"});
    }
    reader.u16();
    auto payload_bytes = reader.u32();
    if (payload_bytes > max_payload) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, "native frame exceeds capacity"});
    }
    return header_size + payload_bytes;
}

Result<Frame> decode(std::span<const std::uint8_t> encoded_frame) {
    auto complete_frame_size = frame_size(encoded_frame);
    if (!complete_frame_size) {
        return std::unexpected(complete_frame_size.error());
    }
    if (*complete_frame_size != encoded_frame.size()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "incomplete or excess native frame bytes"});
    }
    NativePayloadReader operation_field(encoded_frame.subspan(6, 2));
    return Frame{OperationId{operation_field.u16()},
                 Bytes(encoded_frame.begin() + header_size, encoded_frame.end())};
}
} // namespace tmux_cxx::protocol::native
