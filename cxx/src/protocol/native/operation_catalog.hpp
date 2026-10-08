#pragma once

#include "framing.hpp"

#include <span>
#include <vector>

namespace tmux_cxx::protocol::native {
class OperationRegistry;

/** @brief Provide immutable native-operation lookup after startup registration succeeds. */
class OperationCatalog {
  public:
    /** @brief Report whether this server exposes one native operation identity. */
    bool contains(OperationId id) const;
    /** @brief Copy operation descriptions while borrowing their immutable names. */
    std::vector<OperationDescription> descriptions() const;
    /** @brief Invoke one request or reject an unknown identity and malformed arguments. */
    Result<OperationReply> invoke(OperationId id, Server &server,
                                  std::span<const std::uint8_t> arguments) const;

  private:
    friend class OperationRegistry;
    /** @brief Erase only validated operation invocation at the immutable lookup boundary. */
    using OperationAdapter = Result<OperationReply> (*)(Server &, NativePayloadReader &);
    /** @brief Retain immutable metadata beside its typed invocation adapter. */
    struct Entry {
        OperationDescription description; ///< Borrowed operation identity and capability metadata.
        OperationAdapter adapter;         ///< Decoder and execution entry point.
    };
    /** @brief Adopt validated registrations already sorted by native wire identity. */
    explicit OperationCatalog(std::vector<Entry> operations);
    std::vector<Entry> operations_; ///< Frozen native operations sorted by wire identity.
};
} // namespace tmux_cxx::protocol::native
