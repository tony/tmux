#include "paste_buffer/paste_buffer.hpp"

#include <tmux_cxx/server/server.hpp>

#include <gtest/gtest.h>

namespace tmux_cxx {
/** @brief Named paste buffers preserve arbitrary bytes and require explicit deletion. */
TEST(ServerPasteBuffers, NamedStoragePreservesBytesAndDeletion) {
    Server server;
    const std::string bytes{'a', '\0', static_cast<char>(0xff), '\n', 'b'};

    ASSERT_TRUE(server.set_paste_buffer("shared", bytes));
    const auto stored = server.read_paste_buffer("shared");
    ASSERT_TRUE(stored);
    EXPECT_EQ(*stored, bytes);

    ASSERT_TRUE(server.set_paste_buffer("shared", "replacement"));
    EXPECT_EQ(server.read_paste_buffer("shared"), "replacement");
    ASSERT_TRUE(server.delete_paste_buffer("shared"));
    const auto deleted = server.read_paste_buffer("shared");
    ASSERT_FALSE(deleted);
    EXPECT_EQ(deleted.error().code, TmuxErrorCode::not_found);
}

/** @brief Pasting maps line feeds to carriage returns and admits the complete buffer atomically. */
TEST(ServerPasteBuffers, PasteIntoPaneUsesTmuxLineSeparator) {
    Server server;
    const auto session = server.create_session("paste", "/bin/cat");
    ASSERT_TRUE(session);
    ASSERT_TRUE(server.set_paste_buffer("commands", "first\nsecond"));

    ASSERT_TRUE(server.paste_buffer_into_pane("commands", session->pane));
    const auto text = server.read_pane_text(session->pane);
    ASSERT_TRUE(text);
    EXPECT_TRUE(text->input_pending);
    EXPECT_EQ(server.read_paste_buffer("commands"), "first\nsecond");

    const std::string capacity(paste_buffer_byte_limit, 'x');
    ASSERT_TRUE(server.set_paste_buffer("capacity", capacity));
    const auto rejected = server.paste_buffer_into_pane("capacity", session->pane);
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::backpressure);
}
} // namespace tmux_cxx
