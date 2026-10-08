#pragma once

#include "protocol/tmux/imsg_codec.hpp"
#include <tmux_cxx/connection/stock_command_result.hpp>

#include <optional>

namespace tmux_cxx::protocol::tmux {
/** @brief Name a local descriptor attached to one client identification frame. */
enum class StockClientDescriptor { none, standard_input, standard_output };

/** @brief Own one encoded client frame and its optional descriptor source. */
struct StockClientFrame {
    Bytes bytes; ///< Complete encoded frame sent without later mutation.
    StockClientDescriptor descriptor = StockClientDescriptor::none; ///< Descriptor sent with it.
};

/** @brief Supply values consumed by a profile's declared client identification sequence. */
struct StockClientIdentification {
    std::uint64_t client_flags = 0;                 ///< Command-mode flags in profile width.
    std::string terminal_type;                      ///< NUL-terminated TERM value.
    std::uint32_t terminal_features = 0;            ///< Release-sensitive feature mask.
    std::string tty_name;                           ///< Empty for a command-only client.
    std::string working_directory = "/";            ///< Server-side command working directory.
    std::vector<std::string> terminal_capabilities; ///< Repeated terminfo values.
    std::uint32_t process_id = 0;                   ///< Host process identity sent to the server.
    std::vector<std::string> environment;           ///< Explicit environment entries to forward.
};

/** @brief Retain the tmux wire state for one outbound stock command exchange. */
class StockCommandExchange {
  public:
    /** @brief Return client replies and a result when processed server traffic ends the command. */
    struct ServerTrafficOutcome {
        std::vector<StockClientFrame>
            client_replies; ///< Client replies required by received server frames.
        std::optional<StockCommandResult>
            completed_command; ///< Completed command, present exactly once.
    };

    /** @brief Select an explicit profile before any traffic reaches a stock server. */
    explicit StockCommandExchange(TmuxProtocolProfile profile) : selected_profile_(profile) {}
    /** @brief Encode the profile's identification sequence followed by one command request. */
    Result<std::vector<StockClientFrame>>
    begin_command(const StockClientIdentification &identification,
                  std::span<const std::string> arguments);
    /** @brief Consume bounded server traffic and produce required stream acknowledgements. */
    Result<ServerTrafficOutcome> receive_server_traffic(std::span<const std::uint8_t> server_bytes);

  private:
    /** @brief Encode one declared client frame and verify its sender contract. */
    StockClientFrame
    encode_client_frame(TmuxMessage message, const Bytes &payload = {},
                        StockClientDescriptor descriptor = StockClientDescriptor::none) const;
    /** @brief Expand one identification step into zero or more client frames. */
    void append_identification_step(std::vector<StockClientFrame> &frames,
                                    const StockClientIdentification &identification,
                                    const TmuxIdentificationStep &step) const;
    /** @brief Apply one validated server message to command and stream state. */
    Result<void> handle_server_message(const StockFrameHeader &header,
                                       std::span<const std::uint8_t> payload,
                                       ServerTrafficOutcome &outcome);
    /** @brief Validate and acknowledge one stdout or stderr stream opened by the stock server. */
    Result<void> open_output_stream(std::span<const std::uint8_t> payload,
                                    ServerTrafficOutcome &outcome);
    /** @brief Append bounded bytes to one previously opened command-output stream. */
    Result<void> capture_output_stream_data(std::span<const std::uint8_t> payload);
    /** @brief Remove one command-output stream after its validated close message. */
    Result<void> close_output_stream(std::span<const std::uint8_t> payload);
    /** @brief Materialize the command result from one validated exit or shutdown message. */
    void complete_command(std::span<const std::uint8_t> payload, ServerTrafficOutcome &outcome);
    /** @brief Reply to one supported or rejected server output-stream request. */
    void acknowledge_write_stream(std::uint32_t stream_id, std::uint32_t error_number,
                                  ServerTrafficOutcome &outcome) const;
    /** @brief Identify the captured destination for one open stock output stream. */
    enum class StreamTarget { standard_output, standard_error };
    /** @brief Bind one stock stream number to its captured destination. */
    struct OpenStream {
        std::uint32_t id;    ///< Server-selected stream identity.
        StreamTarget target; ///< Captured byte destination.
    };

    TmuxProtocolProfile
        selected_profile_; ///< Explicit profile borrowed from stable declaration spans.
    Bytes inbound_bytes_;  ///< Incomplete server frames retained across reads.
    std::vector<OpenStream> open_streams_; ///< Output streams admitted for byte capture.
    StockCommandResult
        pending_command_result_;    ///< Output and status accumulated before completion.
    bool command_started_ = false;  ///< Identification and command were emitted exactly once.
    bool command_finished_ = false; ///< The completed result has already left this exchange.
};
} // namespace tmux_cxx::protocol::tmux
