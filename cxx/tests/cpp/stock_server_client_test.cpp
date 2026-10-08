#include <tmux_cxx/connection/stock_server_connection.hpp>

#include <iostream>
#include <string>
#include <string_view>
#include <vector>

/** @brief Verify the public C++ stock-server adapter against one owned fixture endpoint. */
int main(int argc, char **argv) {
    if (argc != 5) {
        std::cerr << "usage: stock_server_client_test PROFILE SOCKET RELEASE SESSION\n";
        return 2;
    }
    const tmux_cxx::protocol::tmux::TmuxProtocolProfile *profile = nullptr;
    if (std::string_view(argv[1]) == "legacy") {
        profile = &tmux_cxx::protocol::tmux::legacy_profile;
    } else if (std::string_view(argv[1]) == "modern") {
        profile = &tmux_cxx::protocol::tmux::modern_profile;
    } else {
        std::cerr << "unknown stock protocol profile\n";
        return 2;
    }
    tmux_cxx::StockServerConnection connection(argv[2], *profile);
    const auto compatibility = connection.connection_compatibility();
    if (!compatibility || compatibility->release != argv[3] ||
        compatibility->support != tmux_cxx::protocol::tmux::CapabilitySupport::limited ||
        compatibility->verification !=
            tmux_cxx::protocol::tmux::CompatibilityVerification::tested_subset ||
        compatibility->commands != std::vector<std::string>{"list-sessions"}) {
        std::cerr << (compatibility ? "unexpected compatibility report"
                                    : compatibility.error().message)
                  << '\n';
        return 1;
    }
    auto names = connection.session_names();
    if (!names) {
        std::cerr << names.error().message << '\n';
        return 1;
    }
    if (*names != std::vector<std::string>{argv[4]}) {
        std::cerr << "unexpected stock session names\n";
        return 1;
    }
    connection.close();
    return 0;
}
