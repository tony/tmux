#pragma once

#include "protocol/native/framing.hpp"

namespace tmux_cxx::protocol::native {
constexpr std::uint16_t protocol_version =
    1; ///< Native protocol revision, separate from stock tmux.
constexpr OperationDescription handshake_description{
    OperationId{0}, "connection.handshake",
    OperationEffect::query}; ///< Reserved negotiation frame.
constexpr std::uint64_t known_capabilities =
    capability_mask(Capability::session_lifecycle) | capability_mask(Capability::session_names) |
    capability_mask(Capability::window_links) | capability_mask(Capability::pane_input) |
    capability_mask(Capability::pane_raw_output) | capability_mask(Capability::pane_output_wait) |
    capability_mask(Capability::pane_terminal_text) |
    capability_mask(Capability::client_attachment) |
    capability_mask(Capability::stock_client_compatibility) |
    capability_mask(Capability::pane_layout) | capability_mask(Capability::paste_buffers) |
    capability_mask(Capability::metadata_observation) |
    capability_mask(Capability::session_options);
///< Declared native capability bits in version one.

/** @brief Own a server-advertised operation without retaining received traffic. */
struct AdvertisedOperation {
    OperationId id;                      ///< Stable native wire identity.
    std::string name;                    ///< Entity and action identity advertised by the server.
    OperationEffect effect;              ///< Effects admitted by this operation.
    std::uint64_t required_capabilities; ///< Capabilities required before operation execution.
};
/** @brief Retain the selected native version, traffic bound, capabilities, and frozen operations.
 */
struct NativeHandshake {
    std::uint16_t version;      ///< Selected native protocol revision.
    std::uint32_t max_payload;  ///< Maximum bytes in an admitted native payload.
    std::uint64_t capabilities; ///< Native server behavior available to this connection.
    std::vector<AdvertisedOperation> operations; ///< Owned operation declarations in wire-ID order.
};
/** @brief Request one native version without admitting entity effects. */
Frame handshake_request(std::uint16_t requested_version);
/** @brief Encode the frozen native catalog and only the capabilities its operations require. */
Bytes encode_handshake(std::span<const OperationDescription> operation_descriptions);
/** @brief Reject malformed, conflicting, or unsupported native negotiation declarations. */
Result<NativeHandshake> decode_handshake(std::span<const std::uint8_t> payload);
} // namespace tmux_cxx::protocol::native
