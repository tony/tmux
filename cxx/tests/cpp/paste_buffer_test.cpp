#include "paste_buffer/operations/paste_buffer_command.hpp"
#include "paste_buffer/operations/set_buffer_command.hpp"
#include "paste_buffer/paste_buffer.hpp"
#include "paste_buffer/paste_buffer_native_codec.hpp"
#include "paste_buffer/paste_buffer_store.hpp"

#include <tmux_cxx/server/server.hpp>

#include <gtest/gtest.h>

#include <algorithm>
#include <array>
#include <poll.h>
#include <stdexcept>

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
    ASSERT_TRUE(server.paste_top_buffer(session->pane));
    ASSERT_FALSE(server.read_pane_text(session->pane)->input_pending);
    ASSERT_TRUE(server.set_paste_buffer("commands", "first\nsecond"));
    ASSERT_TRUE(server.paste_top_buffer(session->pane));
    EXPECT_FALSE(server.read_pane_text(session->pane)->input_pending);

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
    const auto automatic = server.add_paste_buffer(capacity);
    ASSERT_TRUE(automatic);
    ASSERT_TRUE(automatic->has_value());
    const auto rejected_top = server.paste_top_buffer(session->pane);
    ASSERT_FALSE(rejected_top);
    EXPECT_EQ(rejected_top.error().code, TmuxErrorCode::backpressure);
    EXPECT_EQ(server.read_paste_buffer((*automatic)->name), capacity);
    const auto stale = server.paste_top_buffer(PaneId{});
    ASSERT_FALSE(stale);
    EXPECT_EQ(stale.error().code, TmuxErrorCode::not_found);
}

/** @brief Replacing or classifying a buffer moves it newest without changing default automatic
 * selection. */
TEST(PasteBufferStore, ReplacementAndDefaultSelectionFollowClassification) {
    PasteBufferStore store;
    ASSERT_TRUE(store.set("buffer0", "reserved", 1));
    const auto first = store.add("automatic", 2);
    ASSERT_TRUE(first);
    ASSERT_TRUE(first->has_value());
    EXPECT_EQ((*first)->name, "buffer1");
    ASSERT_TRUE(store.set("named", "latest named", 3));
    ASSERT_NE(store.top(), nullptr);
    EXPECT_EQ(store.top()->name, "buffer1");
    ASSERT_TRUE(store.set("buffer1", "named replacement", 4));
    EXPECT_EQ(store.top(), nullptr);
    const auto snapshots = store.snapshots();
    ASSERT_EQ(snapshots.size(), 3U);
    EXPECT_EQ(snapshots[0].name, "buffer1");
    EXPECT_FALSE(snapshots[0].automatic);
    EXPECT_EQ(snapshots[0].revision, 4U);
    EXPECT_EQ(snapshots[1].name, "named");
    EXPECT_EQ(snapshots[2].name, "buffer0");
}

/** @brief Automatic eviction preserves all named buffers and rejects named conversion beyond its
 * separate bound. */
TEST(PasteBufferStore, IndependentLimitsPreserveNamedBytes) {
    PasteBufferStore store;
    ASSERT_TRUE(store.set("buffer0", "reserved", 1));
    for (std::size_t index = 1; index < paste_buffer_named_limit; ++index) {
        ASSERT_TRUE(store.set("named" + std::to_string(index), "named", index + 1));
    }
    const auto full_named = store.set("too-many", "rejected", 51);
    ASSERT_FALSE(full_named);
    EXPECT_EQ(full_named.error().code, TmuxErrorCode::backpressure);
    for (std::size_t index = 0; index < paste_buffer_automatic_limit; ++index) {
        ASSERT_TRUE(store.add("automatic", 51 + index));
    }
    const auto newest = store.add("latest", 101);
    ASSERT_TRUE(newest);
    ASSERT_TRUE(newest->has_value());
    EXPECT_EQ((*newest)->name, "buffer51");
    EXPECT_EQ(store.find("buffer1"), nullptr);
    ASSERT_NE(store.find("buffer0"), nullptr);
    EXPECT_EQ(store.find("buffer0")->bytes, "reserved");
    ASSERT_NE(store.find("named49"), nullptr);
    const auto snapshots = store.snapshots();
    EXPECT_EQ(snapshots.size(), 100U);
    EXPECT_EQ(std::ranges::count(snapshots, true, &PasteBufferSnapshot::automatic), 50);
    const auto conversion = store.set("buffer51", "not admitted", 102);
    ASSERT_FALSE(conversion);
    EXPECT_EQ(conversion.error().code, TmuxErrorCode::backpressure);
    ASSERT_NE(store.top(), nullptr);
    EXPECT_EQ(store.top()->name, "buffer51");
    EXPECT_EQ(store.top()->bytes, "latest");
    EXPECT_EQ(store.top()->revision, 101U);
}

/** @brief Empty and rejected automatic input preserves name allocation and the server mutation
 * sequence. */
TEST(ServerPasteBuffers, EmptyAndRejectedCreationPreserveRevision) {
    Server server;
    ASSERT_TRUE(server.set_paste_buffer("named", "retained"));
    const auto empty = server.add_paste_buffer("");
    ASSERT_TRUE(empty);
    EXPECT_FALSE(empty->has_value());
    const auto rejected = server.add_paste_buffer(std::string(paste_buffer_byte_limit + 1, 'x'));
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::invalid_argument);
    const std::string bytes{'a', '\0', static_cast<char>(0xff)};
    const auto added = server.add_paste_buffer(bytes);
    ASSERT_TRUE(added);
    ASSERT_TRUE(added->has_value());
    EXPECT_EQ((*added)->name, "buffer0");
    EXPECT_EQ((*added)->revision, 2U);
    EXPECT_EQ((*added)->byte_count, bytes.size());
    EXPECT_EQ(server.read_paste_buffer((*added)->name), bytes);
    ASSERT_TRUE(server.set_paste_buffer("named", ""));
    const auto listed = server.paste_buffers();
    ASSERT_TRUE(listed);
    ASSERT_EQ(listed->size(), 2U);
    EXPECT_EQ(listed->back().revision, 1U);
    EXPECT_EQ(server.read_paste_buffer("named"), "retained");
}

/** @brief Default paste validates an ended target even when no automatic buffer exists. */
TEST(ServerPasteBuffers, ClosedDefaultTargetRejectsBeforeNoop) {
    Server server;
    const auto session = server.create_session("ended", "/bin/true");
    ASSERT_TRUE(session);
    const auto descriptors = server.pane_program_readiness_descriptors();
    ASSERT_EQ(descriptors.size(), 1U);
    pollfd exited{descriptors.front(), POLLIN, 0};
    ASSERT_EQ(::poll(&exited, 1, 500), 1);
    server.advance_pane_pty_io(session->pane);
    const auto text = server.read_pane_text(session->pane);
    ASSERT_TRUE(text);
    EXPECT_FALSE(text->input_open);
    const auto pasted = server.paste_top_buffer(session->pane);
    ASSERT_FALSE(pasted);
    EXPECT_EQ(pasted.error().code, TmuxErrorCode::closed);
    EXPECT_TRUE(server.paste_buffers()->empty());
}

/** @brief Stock argument variants distinguish default automatic operations from explicit named
 * requests. */
TEST(PasteBufferCommands, DefaultAndNamedRequestsRemainTyped) {
    const auto automatic = SetBufferCommand::read(std::array<std::string, 1>{"data"});
    ASSERT_TRUE(automatic);
    EXPECT_TRUE(std::holds_alternative<AddPasteBuffer::Request>(*automatic));
    const auto named = SetBufferCommand::read(std::array<std::string, 3>{"-b", "named", "data"});
    ASSERT_TRUE(named);
    EXPECT_TRUE(std::holds_alternative<SetPasteBuffer::Request>(*named));
    const auto escaped = SetBufferCommand::read(std::array<std::string, 2>{"--", "-data"});
    ASSERT_TRUE(escaped);
    EXPECT_EQ(std::get<AddPasteBuffer::Request>(*escaped).bytes, "-data");
    EXPECT_FALSE(SetBufferCommand::read(std::array<std::string, 1>{"-w"}));
    const auto top = PasteBufferCommand::read(std::array<std::string, 2>{"-t", "%1"});
    ASSERT_TRUE(top);
    EXPECT_EQ(std::get<PasteTopBuffer::Request>(*top).pane, PaneId{1});
    EXPECT_FALSE(PasteBufferCommand::read(std::array<std::string, 2>{"-t", "1"}));
    EXPECT_FALSE(PasteBufferCommand::read(std::array<std::string, 3>{"-t", "%1", "-r"}));
}

/** @brief Paste metadata codecs reject impossible flags, byte counts, revisions, and names. */
TEST(PasteBufferProtocol, SnapshotRejectsMalformedMetadata) {
    struct InvalidSnapshot {
        std::string_view name;
        std::uint32_t byte_count;
        std::uint16_t automatic;
        std::uint64_t revision;
    };
    const std::array invalid{
        InvalidSnapshot{"buffer0", 1, 2, 1},      InvalidSnapshot{"buffer0", 0, 1, 1},
        InvalidSnapshot{"buffer0", 262145, 1, 1}, InvalidSnapshot{"buffer0", 1, 1, 0},
        InvalidSnapshot{"invalid\n", 1, 1, 1},
    };
    for (const auto &snapshot : invalid) {
        protocol::native::NativePayloadWriter writer;
        writer.string(snapshot.name);
        writer.u32(snapshot.byte_count);
        writer.u16(snapshot.automatic);
        writer.u64(snapshot.revision);
        protocol::native::NativePayloadReader reader(writer.bytes);
        EXPECT_THROW((void)protocol::native::read_paste_buffer_snapshot(reader),
                     std::invalid_argument);
    }
    protocol::native::NativePayloadWriter writer;
    protocol::native::write_paste_buffer_snapshot(writer, {"named", 3, false, 1});
    protocol::native::NativePayloadReader reader(writer.bytes);
    const auto valid = protocol::native::read_paste_buffer_snapshot(reader);
    EXPECT_EQ(valid.name, "named");
    EXPECT_FALSE(valid.automatic);
    EXPECT_TRUE(reader.finished());
}
} // namespace tmux_cxx
