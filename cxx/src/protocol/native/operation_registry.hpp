#pragma once

#include "operation_catalog.hpp"

#include <concepts>
#include <type_traits>

namespace tmux_cxx::protocol::native {
/** @brief Require an operation's typed decoder, execution result, and reply encoding contract. */
template <class Operation>
concept RegisteredOperation =
    requires(Server &server, NativePayloadReader &reader, typename Operation::Request request) {
        typename Operation::Response;
        { Operation::description } -> std::same_as<const OperationDescription &>;
        { Operation::read(reader) } -> std::same_as<typename Operation::Request>;
        {
            Operation::execute(server, std::move(request))
        } -> std::same_as<Result<typename Operation::Response>>;
    } && ((std::is_void_v<typename Operation::Response> && requires {
              { Operation::write() } -> std::same_as<OperationReply>;
          }) || requires(typename Operation::Response response) {
        { Operation::write(std::move(response)) } -> std::same_as<OperationReply>;
    });

/** @brief Validate and freeze native operation registrations before accepting traffic. */
class OperationRegistry {
  public:
    /** @brief Register a typed operation; its immutable description must outlive the catalog. */
    template <RegisteredOperation Operation> Result<void> add() {
        /** @brief Decode completely before executing the operation and materializing its reply. */
        auto operation_adapter =
            +[](Server &server, NativePayloadReader &reader) -> Result<OperationReply> {
            auto request = Operation::read(reader);
            reader.require_end();
            auto response = Operation::execute(server, std::move(request));
            if (!response) {
                return std::unexpected(response.error());
            }
            if constexpr (std::is_void_v<typename Operation::Response>) {
                return Operation::write();
            } else {
                return Operation::write(std::move(*response));
            }
        };
        return add_registration(Operation::description, operation_adapter);
    }
    /** @brief Validate all registrations and consume them into an immutable operation catalog. */
    Result<OperationCatalog> freeze(std::span<const OperationDescription> required = {}) &&;

  private:
    /** @brief Reject invalid descriptions, duplicate identities or names, and late registrations.
     */
    Result<void> add_registration(OperationDescription description,
                                  OperationCatalog::OperationAdapter adapter);
    std::vector<OperationCatalog::Entry> registrations_; ///< Mutable startup registrations.
    bool registration_closed_ = false; ///< Prevent reuse after catalog construction.
};
} // namespace tmux_cxx::protocol::native
