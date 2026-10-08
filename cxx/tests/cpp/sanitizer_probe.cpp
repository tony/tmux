#include <charconv>
#include <string_view>

namespace {
/** @brief Lose a test allocation so LeakSanitizer must reject first-party leaks. */
[[gnu::noinline]] void abandon_allocation(int count) {
    auto *bytes = new volatile char[static_cast<std::size_t>(count)];
    bytes[0] = 'x';
}
} // namespace

/** @brief Trigger one deliberate memory, arithmetic, or leak failure selected by a test argument.
 */
int main(int argc, char **argv) {
    if (argc != 3) {
        return 2;
    }
    const std::string_view argument(argv[2]);
    int number = 0;
    const auto parsed = std::from_chars(argument.data(), argument.data() + argument.size(), number);
    if (parsed.ec != std::errc{} || parsed.ptr != argument.data() + argument.size() || number < 1) {
        return 2;
    }
    if (std::string_view(argv[1]) == "address") {
        auto *values = new int[1]{42};
        const int value = values[number];
        delete[] values;
        return value;
    }
    if (std::string_view(argv[1]) == "undefined") {
        return number + 1;
    }
    if (std::string_view(argv[1]) == "leak") {
        abandon_allocation(number);
        return 0;
    }
    return 2;
}
