#pragma once

#include "platform/posix/owned_fd.hpp"
#include "protocol/tmux/control/channel.hpp"
#include "protocol/tmux/control/selected_session_output_feed.hpp"
#include "protocol/tmux/control/session_notification_feed.hpp"
#include "protocol/tmux/imsg_codec.hpp"
#include <tmux_cxx/client/client_snapshot.hpp>
#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>
#include <tmux_cxx/protocol/tmux/stock_profile_detector.hpp>

#include <chrono>
#include <deque>
#include <variant>

namespace tmux_cxx {
class Server;
namespace commands {
class CommandCatalog;
}
namespace protocol::tmux {
/** @brief Retain tmux wire state for one accepted client without owning its transport. */
class AcceptedClientProtocol {
  public:
    using Clock = std::chrono::steady_clock; ///< Monotonic stock identification deadlines.
    /** @brief Direct the event loop to deliver bytes and optionally close their transport. */
    struct TransportAction {
        Bytes outbound_bytes;   ///< Complete encoded messages awaiting transport delivery.
        bool close_after_write; ///< Close only after every returned byte is delivered.
    };
    /** @brief Describe borrowed descriptor routes for one active control client. */
    struct ControlModeRoute {
        int input_descriptor;  ///< Nonblocking newline-command input route.
        int output_descriptor; ///< Nonblocking guarded-record output route.
        bool accepts_commands; ///< Input begins only after the initial command completes.
        bool output_pending;   ///< Guarded records require writable readiness.
    };
    /** @brief Describe descriptor readiness without coupling protocol state to poll constants. */
    struct ControlModeReadiness {
        bool input_ready;       ///< The input descriptor may be read without waiting.
        bool input_closed;      ///< The input descriptor reached an orderly hangup.
        bool output_ready;      ///< The output descriptor may accept queued records.
        bool output_closed;     ///< The output route can no longer deliver records.
        bool descriptor_failed; ///< A descriptor reported an invalid or error state.
    };

    /** @brief Bind accepted-client wire state to its server-owned client entity. */
    explicit AcceptedClientProtocol(ClientId client_id) : client_id_(client_id) {}
    /** @brief Consume bounded traffic and transferred descriptors; protocol failures close
     * admission before subsequent command effects. */
    Result<TransportAction>
    receive_client_traffic(std::span<const std::uint8_t> client_bytes,
                           std::vector<posix::OwnedFd> transferred_descriptors, Server &server,
                           const commands::CommandCatalog &command_catalog,
                           control::CommandSequence &control_sequence);
    /** @brief Return the next identification or detach-handshake deadline. */
    std::optional<Clock::time_point> next_protocol_deadline() const;
    /** @brief Return the server-owned client corresponding to this accepted protocol lifetime. */
    ClientId client_id() const;
    /** @brief Advance output from current client attachment and committed session events. */
    Result<TransportAction> advance_server_driven_output(Server &server,
                                                         const ClientSnapshot &client_snapshot);
    /** @brief Borrow active control descriptors for one event-loop readiness turn. */
    std::optional<ControlModeRoute> control_mode_route() const;
    /** @brief Execute ready control input and flush guarded output without blocking. */
    Result<TransportAction> advance_control_mode(ControlModeReadiness readiness, Server &server,
                                                 const commands::CommandCatalog &command_catalog,
                                                 control::CommandSequence &control_sequence);

  private:
    /** @brief Own profile detection and descriptors until stock identification completes. */
    struct IdentifyingPhase {
        /** @brief Start the bounded stock-identification interval. */
        IdentifyingPhase() : deadline(Clock::now() + std::chrono::milliseconds(250)) {}
        std::deque<posix::OwnedFd> transferred_descriptors; ///< Unmatched received descriptors.
        posix::OwnedFd stdin_descriptor;         ///< Identified input descriptor, when supplied.
        posix::OwnedFd stdout_descriptor;        ///< Identified output descriptor, when supplied.
        StockProfileDetector profile_detector{}; ///< Framing evidence without release inference.
        std::optional<TmuxProtocolProfile>
            selected_profile;           ///< Profile selected from wire evidence.
        std::uint64_t client_flags = 0; ///< Accumulated flags from validated identification frames.
        Clock::time_point deadline;
        ///< Bound incomplete identification without blocking other connections.
    };
    /** @brief Carry profile and policy evidence after stock admission. */
    struct StockClientAdmission {
        TmuxProtocolProfile profile;                ///< Selected wire contract.
        StockConnectionCompatibility compatibility; ///< Effective tested support contract.
    };
    /** @brief Admit exactly one command after identification. */
    struct AwaitingOneShotCommandPhase {
        StockClientAdmission admission; ///< Admitted stock-client contract.
    };
    /** @brief Retain descriptors until the packed initial control command arrives. */
    struct AwaitingControlStartCommandPhase {
        StockClientAdmission admission;            ///< Admitted control-client contract.
        std::unique_ptr<control::Channel> channel; ///< Descriptor-carried control channel.
        SessionEventCursor session_event_cursor;   ///< Baseline established before initial command.
    };
    /** @brief Retain one command stream until its stock acknowledgement arrives. */
    struct OneShotCommandOutputPhase {
        StockClientAdmission admission; ///< Admitted stock-client contract.
        std::string bytes;              ///< Command bytes awaiting stream acknowledgement.
        int exit_status;                ///< Status emitted after the stream closes.
    };
    /** @brief Route terminal traffic for one attached ordinary stock client. */
    struct AttachedTerminalPhase {
        StockClientAdmission admission; ///< Interactive stock-client contract.
    };
    /** @brief Read commands and deliver guarded records for one stock control client. */
    struct ControlModePhase {
        StockClientAdmission admission;            ///< Admitted control-client contract.
        std::unique_ptr<control::Channel> channel; ///< Descriptor-carried control channel.
        control::SessionNotificationFeed
            session_notifications; ///< Committed session events after admission.
        control::SelectedSessionOutputFeed
            session_output; ///< Selected-session notice and pane cursors.
    };
    /** @brief Flush final guarded records before sending the socket exit message. */
    struct DrainingControlModePhase {
        StockClientAdmission admission;            ///< Admitted control-client contract.
        std::unique_ptr<control::Channel> channel; ///< Channel retained until output drains.
    };
    /** @brief Await EXITING after the server requests stock-client detachment. */
    struct DetachingTerminalPhase {
        StockClientAdmission admission; ///< Interactive stock-client contract.
        Clock::time_point deadline;     ///< Bound the stock exit handshake.
    };
    /** @brief Close the transport after every queued protocol byte is delivered. */
    struct ClosingTransportPhase {};
    using ProtocolPhase =
        std::variant<IdentifyingPhase, AwaitingOneShotCommandPhase,
                     AwaitingControlStartCommandPhase, OneShotCommandOutputPhase,
                     AttachedTerminalPhase, ControlModePhase, DrainingControlModePhase,
                     DetachingTerminalPhase,
                     ClosingTransportPhase>; ///< Closed accepted-client wire phases.

    /** @brief Reject late identification and invoke registered commands under the selected
     * connection policy. */
    void consume_client_message(Server &server, const commands::CommandCatalog &command_catalog,
                                const StockFrameHeader &header,
                                std::span<const std::uint8_t> payload,
                                control::CommandSequence &control_sequence);
    /** @brief Adopt the descriptor paired with one validated terminal identification message. */
    void adopt_identification_descriptor(TmuxMessage message);
    /** @brief Accumulate validated flags so later frames cannot erase an admission constraint. */
    void record_client_flags(TmuxMessage message, std::span<const std::uint8_t> payload);
    /** @brief Admit the identified client entity under its selected profile and connection mode. */
    void complete_identification(Server &server, const commands::CommandCatalog &command_catalog,
                                 std::span<const std::uint8_t> payload);
    /** @brief Invoke one registered tmux command and begin its output or attachment response. */
    void invoke_client_command(Server &server, const commands::CommandCatalog &command_catalog,
                               std::span<const std::uint8_t> payload,
                               control::CommandSequence &control_sequence);
    /** @brief Execute and frame one newline command through an active control descriptor. */
    void invoke_control_command(Server &server, const commands::CommandCatalog &command_catalog,
                                control::CommandSequence &control_sequence,
                                std::string_view command_line);
    /** @brief Deliver retained command output after the stock client acknowledges its stream. */
    void continue_command_output(std::span<const std::uint8_t> payload);
    /** @brief Remeasure one attached ordinary client after a validated resize message. */
    void resize_attached_client(Server &server, std::span<const std::uint8_t> payload);
    /** @brief Complete detach or client-initiated exit and close after the reply is delivered. */
    void complete_client_exit(Server &server, std::span<const std::uint8_t> payload);
    /** @brief Append bounded stock replies; excessive output throws length_error and closes
     * admission. */
    void queue_server_frame(Bytes frame);
    /** @brief Encode command exit status and finish after queued output is delivered. */
    void queue_command_exit(int exit_status);
    /** @brief Return the profile used to validate the current phase's client traffic. */
    const TmuxProtocolProfile &validation_profile() const;
    /** @brief Return the detected wire profile or reject behavior before profile selection. */
    const TmuxProtocolProfile &require_selected_profile() const;
    /** @brief Test whether queued output completes this accepted protocol lifetime. */
    bool closes_after_outbound() const;

    Bytes inbound_bytes_;   ///< Partial stock frames retained across transport reads.
    Bytes outbound_bytes_;  ///< Encoded replies produced by the current receive turn.
    ClientId client_id_;    ///< Server-owned client whose lifetime follows this stock connection.
    ProtocolPhase phase_{}; ///< Exclusive wire phase and its phase-specific ownership.
};
} // namespace protocol::tmux
} // namespace tmux_cxx
