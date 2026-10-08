#pragma once

#include <unistd.h>
#include <utility>

namespace tmux_cxx::posix {
/** @brief Own one descriptor and transfer its close obligation through moves. */
class OwnedFd {
  public:
    /** @brief Start without owning a descriptor. */
    OwnedFd() = default;
    /** @brief Adopt a descriptor and its close obligation. */
    explicit OwnedFd(int value) : value_(value) {}
    /** @brief Close the owned descriptor once. */
    ~OwnedFd() { reset(); }
    /** @brief Prevent duplicating a descriptor's close obligation. */
    OwnedFd(const OwnedFd &) = delete;
    /** @brief Prevent assigning shared ownership to a descriptor. */
    OwnedFd &operator=(const OwnedFd &) = delete;
    /** @brief Transfer the descriptor and leave the source empty. */
    OwnedFd(OwnedFd &&other) noexcept : value_(std::exchange(other.value_, -1)) {}
    /** @brief Close the previous descriptor before adopting the source's ownership. */
    OwnedFd &operator=(OwnedFd &&other) noexcept {
        if (this != &other) {
            reset(std::exchange(other.value_, -1));
        }
        return *this;
    }
    /** @brief Borrow the descriptor until this owner resets or is destroyed. */
    int get() const { return value_; }
    /** @brief Close the current descriptor before adopting its replacement. */
    void reset(int value = -1) {
        if (value_ >= 0) {
            ::close(value_);
        }
        value_ = value;
    }

  private:
    int value_ = -1; ///< Owned descriptor, or minus one when empty.
};
} // namespace tmux_cxx::posix
