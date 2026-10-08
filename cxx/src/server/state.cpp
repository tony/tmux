#include <tmux_cxx/server/server.hpp>

#include "server/state.hpp"

#include <algorithm>
#include <array>
#include <cerrno>
#include <iomanip>
#include <poll.h>
#include <sstream>
#include <sys/random.h>
#include <unistd.h>

namespace tmux_cxx {

Server::Server() : state_(std::make_unique<State>()) {
    std::array<unsigned char, 16> identity_bytes{};
    if (::getrandom(identity_bytes.data(), identity_bytes.size(), 0) !=
        static_cast<ssize_t>(identity_bytes.size())) {
        throw std::runtime_error("cannot establish server instance identity");
    }
    std::ostringstream identity;
    for (auto byte : identity_bytes) {
        identity << std::hex << std::setfill('0') << std::setw(2) << static_cast<unsigned>(byte);
    }
    state_->server_instance = identity.str();
}
Server::~Server() {
    if (!state_->shutdown_attempted) {
        (void)shutdown();
    }
}
const std::string &Server::server_instance() const { return state_->server_instance; }

SessionSnapshot Server::State::session_snapshot(const Session &session) const {
    const auto &link = window_links.at(session.current);
    const auto &window = windows.at(link.window);
    return {session.id,    window.id,    link.id,
            window.active, session.name, std::max(session.revision, window.revision)};
}

WindowLinkSnapshot Server::State::window_link_snapshot(const WindowLink &link) const {
    return {link.id, link.session, link.window, link.index, link.revision};
}

ClientSnapshot Server::State::client_snapshot(const Client &client) const {
    const auto size = client.terminal ? client.terminal->size() : terminal::CellSize{};
    return {client.id,
            client.session,
            client.detached_from,
            client.stock_identification_complete,
            client.terminal != nullptr,
            size.rows,
            size.columns,
            client.revision};
}

PaneSnapshot Server::State::pane_snapshot(const Window &window,
                                          const PaneGeometry &geometry) const {
    return {geometry.pane,
            window.id,
            geometry.top,
            geometry.left,
            geometry.size.rows,
            geometry.size.columns,
            geometry.pane == window.active,
            window.revision};
}

std::vector<PasteBuffer>::iterator Server::State::find_paste_buffer(std::string_view name) {
    return std::ranges::find(paste_buffers, name, &PasteBuffer::name);
}

std::vector<PasteBuffer>::const_iterator
Server::State::find_paste_buffer(std::string_view name) const {
    return std::ranges::find(paste_buffers, name, &PasteBuffer::name);
}

void Server::State::remove_window_link(WindowLinkId window_link_id) {
    const auto window_link = window_links.at(window_link_id);
    auto &session = sessions.at(window_link.session);
    auto &window = windows.at(window_link.window);
    session.windows.erase(window_link.index);
    if (session.current == window_link_id) {
        session.current =
            session.windows.empty() ? WindowLinkId{} : session.windows.begin()->second;
    }
    session.revision = ++revision;
    window.links.erase(window_link_id);
    window_links.erase(window_link_id);
    if (window.links.empty()) {
        for (const auto &geometry : window.layout.panes()) {
            retire_pane(geometry.pane);
        }
        windows.erase(window_link.window);
    }
}
void Server::State::retire_pane(PaneId pane_id) {
    auto pane_program = pane_programs.find(pane_id);
    if (pane_program != pane_programs.end()) {
        pane_program->second.stop();
        if (pane_program->second.reaped()) {
            pane_programs.erase(pane_program);
        }
    }
    panes.erase(pane_id);
}

void Server::reap_pane_programs() {
    const auto now = PaneProgram::Clock::now();
    for (auto pane_program = state_->pane_programs.begin();
         pane_program != state_->pane_programs.end();) {
        pane_program->second.advance_reaping(now);
        if (pane_program->second.reaped() && !state_->panes.contains(pane_program->first)) {
            pane_program = state_->pane_programs.erase(pane_program);
        } else {
            ++pane_program;
        }
    }
}
std::vector<int> Server::pane_program_readiness_descriptors() const {
    std::vector<int> readiness_descriptors;
    for (const auto &[pane_id, program] : state_->pane_programs) {
        (void)pane_id;
        if (program.readiness_descriptor() >= 0) {
            readiness_descriptors.push_back(program.readiness_descriptor());
        }
    }
    return readiness_descriptors;
}
std::optional<PaneProgram::Clock::time_point> Server::next_pane_program_deadline() const {
    std::optional<PaneProgram::Clock::time_point> deadline;
    for (const auto &[pane_id, program] : state_->pane_programs) {
        (void)pane_id;
        if (program.termination_deadline() &&
            (!deadline || *program.termination_deadline() < *deadline)) {
            deadline = program.termination_deadline();
        }
    }
    return deadline;
}
Result<void> Server::shutdown() {
    state_->shutdown_attempted = true;
    const auto shutdown_deadline = PaneProgram::Clock::now() + std::chrono::milliseconds(700);
    for (auto &[pane_id, program] : state_->pane_programs) {
        (void)pane_id;
        program.stop();
    }
    state_->clients.clear();
    state_->paste_buffers.clear();
    state_->panes.clear();
    state_->window_links.clear();
    state_->windows.clear();
    state_->sessions.clear();
    for (;;) {
        reap_pane_programs();
        if (state_->pane_programs.empty()) {
            return {};
        }
        const auto now = PaneProgram::Clock::now();
        if (now >= shutdown_deadline) {
            return std::unexpected(TmuxError{TmuxErrorCode::timeout,
                                             "owned child outlived the bounded server shutdown"});
        }
        const auto next_wakeup =
            std::min(next_pane_program_deadline().value_or(shutdown_deadline), shutdown_deadline);
        const auto timeout_ms = std::max<std::int64_t>(
            0, std::chrono::ceil<std::chrono::milliseconds>(next_wakeup - now).count());
        std::vector<pollfd> readiness;
        for (auto descriptor : pane_program_readiness_descriptors()) {
            readiness.push_back({descriptor, POLLIN, 0});
        }
        if (::poll(readiness.data(), readiness.size(), static_cast<int>(timeout_ms)) < 0 &&
            errno != EINTR) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::io, "owned child readiness failed during shutdown"});
        }
    }
}
} // namespace tmux_cxx
