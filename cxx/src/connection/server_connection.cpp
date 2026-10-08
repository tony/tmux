#include "connection/native_server_channel.hpp"

namespace tmux_cxx {
ServerConnection::ServerConnection(std::string socket_path)
    : channel_(std::make_shared<NativeServerChannel>(std::move(socket_path))) {}
ServerConnection::ServerConnection(std::shared_ptr<NativeServerChannel> channel)
    : channel_(std::move(channel)) {}
ServerConnection::~ServerConnection() = default;
void ServerConnection::close() { channel_->request_admission_closed.store(true); }
} // namespace tmux_cxx
