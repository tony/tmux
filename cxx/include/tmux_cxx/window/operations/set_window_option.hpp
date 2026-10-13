#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

#include <string>
#include <variant>

namespace tmux_cxx {
/** @brief Select server-wide defaults inherited by shared windows without local overrides. */
struct GlobalWindowOptions {};

/** @brief Select global window defaults or one live shared window's local overrides. */
using WindowOptionTarget = std::variant<GlobalWindowOptions, WindowId>;

/** @brief Describe assignment of one implemented window option through the shared server. */
struct SetWindowOption {
    /** @brief Carry a resolved option owner and its canonical tmux name and value. */
    struct Request {
        WindowOptionTarget target; ///< Global defaults or one resolved shared window.
        std::string name;          ///< Exact implemented window option name.
        std::string value;         ///< Canonical tmux text validated before mutation.
    };
    using Response = void; ///< Successful assignment carries no result payload.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{39}, "window.set_option",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::window_options)};
    ///< Stable window-option mutation identity and required capability.
    /** @brief Decode an explicit global-or-window target before the option name and value. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::set_window_option */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode successful option assignment without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
