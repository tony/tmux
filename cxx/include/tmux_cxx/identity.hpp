#pragma once

#include <compare>
#include <cstdint>

/** @brief Independent terminal multiplexer and its native connection interface. */
namespace tmux_cxx {
/** @brief An entity identity scoped to one server instance; zero does not identify an entity. */
template <class Entity> struct Identity {
    std::uint64_t value{}; ///< Numeric identity within one server instance.
    /** @brief Compare identities of the same entity kind. */
    auto operator<=>(const Identity &) const = default;
};
using SessionId = Identity<struct SessionTag>;       ///< @copybrief Identity
using WindowId = Identity<struct WindowTag>;         ///< @copybrief Identity
using WindowLinkId = Identity<struct WindowLinkTag>; ///< @copybrief Identity
using PaneId = Identity<struct PaneTag>;             ///< @copybrief Identity
using ClientId = Identity<struct ClientTag>;         ///< @copybrief Identity
} // namespace tmux_cxx
