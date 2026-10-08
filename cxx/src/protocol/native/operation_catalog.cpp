#include "protocol/native/operation_catalog.hpp"

#include <algorithm>
#include <stdexcept>
#include <utility>

namespace tmux_cxx::protocol::native {
OperationCatalog::OperationCatalog(std::vector<Entry> operations)
    : operations_(std::move(operations)) {}

std::vector<OperationDescription> OperationCatalog::descriptions() const {
    std::vector<OperationDescription> descriptions;
    descriptions.reserve(operations_.size());
    for (const auto &operation : operations_) {
        descriptions.push_back(operation.description);
    }
    return descriptions;
}

bool OperationCatalog::contains(OperationId id) const {
    return std::ranges::binary_search(
        operations_, id.value, {},
        /** @brief Compare catalog entries by their stable wire identity. */
        [](const Entry &operation) { return operation.description.id.value; });
}

Result<OperationReply> OperationCatalog::invoke(OperationId id, Server &server,
                                                std::span<const std::uint8_t> arguments) const {
    auto operation = std::ranges::lower_bound(
        operations_, id.value, {},
        /** @brief Locate the requested wire identity in the immutable catalog. */
        [](const Entry &candidate) { return candidate.description.id.value; });
    if (operation == operations_.end() || operation->description.id != id) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, "unknown native operation"});
    }
    try {
        NativePayloadReader reader(arguments);
        auto operation_reply = operation->adapter(server, reader);
        if (!operation_reply) {
            operation_reply.error().operation = operation->description.name;
        }
        return operation_reply;
    } catch (const std::invalid_argument &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what(),
                                         std::string(operation->description.name)});
    }
}
} // namespace tmux_cxx::protocol::native
