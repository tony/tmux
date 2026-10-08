#include "cli/action_catalog.hpp"

#include <tmux_cxx/connection/server_connection.hpp>
#include <tmux_cxx/server/server.hpp>

#include <charconv>
#include <iostream>
#include <stdexcept>
#include <string_view>
#include <vector>

namespace {
/** @brief Parse one nonnegative readiness descriptor without accepting a trailing suffix. */
int parse_readiness_descriptor(std::string_view text) {
    int descriptor = -1;
    auto [end, error] = std::from_chars(text.data(), text.data() + text.size(), descriptor);
    if (error != std::errc{} || end != text.data() + text.size() || descriptor < 0) {
        throw std::invalid_argument("invalid readiness descriptor");
    }
    return descriptor;
}
} // namespace
/** @brief Select daemon startup or invoke one registered command-line adapter. */
int main(int argc, char **argv) {
    try {
        if (argc >= 4 && std::string_view(argv[1]) == "serve" &&
            std::string_view(argv[2]) == "--socket") {
            int readiness_descriptor = -1;
            if (argc == 6 && std::string_view(argv[4]) == "--ready-fd") {
                readiness_descriptor = parse_readiness_descriptor(argv[5]);
            } else if (argc != 4) {
                throw std::invalid_argument("invalid serve arguments");
            }
            return tmux_cxx::run_server(argv[3], readiness_descriptor);
        }
        auto action_catalog = tmux_cxx::cli::build_action_catalog();
        if (!action_catalog) {
            throw std::runtime_error(action_catalog.error().message);
        }
        if (argc >= 4 && std::string_view(argv[1]) == "--socket") {
            tmux_cxx::ServerConnection connection(argv[2]);
            std::vector<std::string_view> action_arguments;
            action_arguments.reserve(static_cast<std::size_t>(argc - 3));
            for (int index = 3; index < argc; ++index) {
                action_arguments.emplace_back(argv[index]);
            }
            tmux_cxx::cli::ActionInvocation invocation{connection, std::cout};
            if (auto action_result = action_catalog->invoke(invocation, action_arguments);
                !action_result) {
                throw std::runtime_error(action_result.error().message);
            }
            return 0;
        }
        std::cerr << "usage: tmux-cxx serve --socket PATH [--ready-fd FD]\n";
        if (auto usage_result =
                action_catalog->write_usage(std::cerr, "       tmux-cxx --socket PATH ");
            !usage_result) {
            throw std::runtime_error(usage_result.error().message);
        }
        return 2;
    } catch (const std::exception &error) {
        std::cerr << "tmux-cxx: " << error.what() << '\n';
        return 1;
    }
}
