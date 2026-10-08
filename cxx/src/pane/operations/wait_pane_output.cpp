#include <tmux_cxx/pane/operations/wait_pane_output.hpp>

#include "protocol/native/framing.hpp"
#include <tmux_cxx/server/server.hpp>

namespace tmux_cxx {
namespace native = protocol::native;
WaitPaneOutput::Request WaitPaneOutput::read(native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}, reader.string(), reader.u32()};
}

Result<WaitPaneOutput::Response> WaitPaneOutput::execute(Server &server, Request request) {
    if (request.needle.empty() || request.timeout_ms == 0 || request.timeout_ms > 750) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument,
                      "pane wait requires a nonempty needle and 1..750 ms deadline"});
    }
    auto raw_output = server.read_pane_output(request.pane);
    if (!raw_output) {
        return std::unexpected(raw_output.error());
    }
    if (raw_output->find(request.needle) != raw_output->npos) {
        native::NativePayloadWriter reply;
        reply.string(*raw_output);
        return native::OperationReply{std::move(reply.bytes)};
    }
    const auto wait_deadline = native::Clock::now() + std::chrono::milliseconds(request.timeout_ms);
    /** @brief Resume from retained pane output; complete only on a match, failure, or the owned
     * deadline. */
    auto await_output_match = [request = std::move(request), wait_deadline](
                                  Server &current_server) -> Result<std::optional<native::Bytes>> {
        auto raw_output = current_server.read_pane_output(request.pane);
        if (!raw_output) {
            return std::unexpected(raw_output.error());
        }
        if (raw_output->find(request.needle) != raw_output->npos) {
            native::NativePayloadWriter reply;
            reply.string(*raw_output);
            return std::optional<native::Bytes>{std::move(reply.bytes)};
        }
        if (native::Clock::now() >= wait_deadline) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::timeout, "pane output deadline expired"});
        }
        return std::optional<native::Bytes>{};
    };
    return native::OperationReply{
        native::PendingReply{wait_deadline, std::move(await_output_match)}};
}

native::OperationReply WaitPaneOutput::write(Response response) { return response; }
} // namespace tmux_cxx
