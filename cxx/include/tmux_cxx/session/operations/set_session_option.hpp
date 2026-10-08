#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

#include <string>
#include <variant>

namespace tmux_cxx {
/** @brief Select the server-owned defaults inherited by every session option table. */
struct GlobalSessionOptions {};

/** @brief Select either global session defaults or one live session's local overrides. */
using SessionOptionTarget = std::variant<GlobalSessionOptions, SessionId>;

/** @brief Describe the typed session.set_option operation and its native identity. */
struct SetSessionOption {
    /** @brief Carry one resolved option-table target, canonical name, and tmux text value. */
    struct Request {
        SessionOptionTarget target; ///< Global defaults or one resolved live session.
        std::string name;           ///< Canonical session option name.
        std::string value;          ///< Tmux text parsed before mutation.
    };
    /** @brief Successful mutation carries no result payload. */
    using Response = void;
    /** @brief Declare session-option mutation and its required capability. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{28}, "session.set_option",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::session_options)};
    /** @brief Decode a global assignment or an assignment ending with one session identity. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @brief Assign one parsed value to the request's resolved session-option table. */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode successful option mutation without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
