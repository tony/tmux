#include <tmux_cxx/connection/server_connection.hpp>

#include <napi.h>

#ifndef NODE_ADDON_API_CPP_EXCEPTIONS_ALL
#error "Node callback boundaries must translate all C++ exceptions"
#endif

#include <cmath>
#include <concepts>
#include <functional>
#include <limits>
#include <memory>
#include <optional>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

namespace {
/** @brief Retain one native session stream and its JavaScript-thread admission state. */
struct NodeSessionStreamState {
    /** @brief Adopt a native stream without copying its observer-owned cursor. */
    explicit NodeSessionStreamState(std::shared_ptr<tmux_cxx::SessionEventStream> stream_value)
        : stream(std::move(stream_value)) {}

    std::shared_ptr<tmux_cxx::SessionEventStream> stream; ///< Native cursor and cancellation owner.
    bool next_pending = false; ///< JavaScript-thread guard against overlapping reads.
};

/** @brief Own environment-local constructors and close live streams during addon teardown. */
struct NodeAddonState {
    /** @brief Cancel streams before releasing references tied to the JavaScript environment. */
    ~NodeAddonState() {
        for (auto &stream : session_streams) {
            if (auto retained = stream.lock()) {
                retained->stream->close();
            }
        }
    }

    /** @brief Retain a weak cleanup route without extending a stream's JavaScript lifetime. */
    void observe(const std::shared_ptr<NodeSessionStreamState> &stream) {
        std::erase_if(session_streams,
                      /** @brief Discard cleanup routes whose JavaScript wrappers are gone. */
                      [](const auto &candidate) { return candidate.expired(); });
        session_streams.emplace_back(stream);
    }

    Napi::FunctionReference session_stream_constructor; ///< Environment-local native wrapper.
    std::vector<std::weak_ptr<NodeSessionStreamState>>
        session_streams; ///< Cleanup routes for live native subscriptions.
};

/** @brief Wrap one native session stream in its environment-local JavaScript object. */
Napi::Object
session_event_stream_object(Napi::Env env,
                            const std::shared_ptr<tmux_cxx::SessionEventStream> &stream);

/** @brief Closed set of materialized native results awaiting JavaScript translation. */
using NodeOperationValue = std::variant<
    std::monostate, tmux_cxx::SessionSnapshot, std::vector<tmux_cxx::SessionSnapshot>,
    tmux_cxx::SessionObservation, tmux_cxx::ClientSnapshot, std::vector<tmux_cxx::ClientSnapshot>,
    tmux_cxx::StockConnectionCompatibility, tmux_cxx::WindowSnapshot, tmux_cxx::WindowLinkSnapshot,
    std::vector<tmux_cxx::WindowLinkSnapshot>, tmux_cxx::PaneSnapshot,
    std::vector<tmux_cxx::PaneSnapshot>, tmux_cxx::PaneTextSnapshot, tmux_cxx::SessionEventBatch,
    std::shared_ptr<tmux_cxx::SessionEventStream>, std::string>;
/** @brief Move-owned native work that never captures JavaScript handles. */
using NodeOperation =
    std::move_only_function<tmux_cxx::Result<NodeOperationValue>(tmux_cxx::ServerConnection &)>;

/** @brief Materialize snapshot identities as exact bigint values on the JavaScript thread. */
Napi::Object session_object(Napi::Env env, const tmux_cxx::SessionSnapshot &session) {
    auto object = Napi::Object::New(env);
    object.Set("id", Napi::BigInt::New(env, session.id.value));
    object.Set("window", Napi::BigInt::New(env, session.window.value));
    object.Set("link", Napi::BigInt::New(env, session.link.value));
    object.Set("pane", Napi::BigInt::New(env, session.pane.value));
    object.Set("name", Napi::String::New(env, session.name));
    object.Set("revision", Napi::BigInt::New(env, session.revision));
    return object;
}
/** @brief Materialize an owner-qualified session event position as exact JavaScript values. */
Napi::Object session_event_cursor_object(Napi::Env env,
                                         const tmux_cxx::SessionEventCursor &cursor) {
    auto object = Napi::Object::New(env);
    object.Set("serverInstance", cursor.server_instance);
    object.Set("nextSequence", Napi::BigInt::New(env, cursor.next_sequence));
    return object;
}
/** @brief Materialize a session catalog and its following event position on the JavaScript thread.
 */
Napi::Object session_observation_object(Napi::Env env,
                                        const tmux_cxx::SessionObservation &observation) {
    auto object = Napi::Object::New(env);
    auto sessions = Napi::Array::New(env, observation.sessions.size());
    for (std::size_t index = 0; index < observation.sessions.size(); ++index) {
        sessions.Set(static_cast<std::uint32_t>(index),
                     session_object(env, observation.sessions[index]));
    }
    object.Set("sessions", sessions);
    object.Set("nextEvent", session_event_cursor_object(env, observation.next_event));
    return object;
}
/** @brief Materialize one typed committed session event with a stable discriminator. */
Napi::Object session_event_object(Napi::Env env, const tmux_cxx::SessionEvent &session_event) {
    return std::visit(
        /** @brief Materialize the active typed event alternative with its stable discriminator. */
        [env](const auto &event) {
            using Event = std::remove_cvref_t<decltype(event)>;
            auto object = Napi::Object::New(env);
            if constexpr (std::same_as<Event, tmux_cxx::SessionCreatedEvent>) {
                object.Set("kind", "session_created");
                object.Set("session", session_object(env, event.session));
            } else if constexpr (std::same_as<Event, tmux_cxx::SessionRenamedEvent>) {
                object.Set("kind", "session_renamed");
                object.Set("session", Napi::BigInt::New(env, event.session.value));
                object.Set("name", event.name);
                object.Set("revision", Napi::BigInt::New(env, event.revision));
            } else if constexpr (std::same_as<Event, tmux_cxx::SessionClosedEvent>) {
                object.Set("kind", "session_closed");
                object.Set("session", Napi::BigInt::New(env, event.session.value));
                object.Set("name", event.name);
                object.Set("revision", Napi::BigInt::New(env, event.revision));
            }
            return object;
        },
        session_event);
}
/** @brief Materialize one committed session record without adding a stream discriminator. */
Napi::Object session_event_record_object(Napi::Env env,
                                         const tmux_cxx::SessionEventRecord &record) {
    auto object = Napi::Object::New(env);
    object.Set("sequence", Napi::BigInt::New(env, record.sequence));
    object.Set("event", session_event_object(env, record.event));
    return object;
}
/** @brief Materialize ordered committed events and the cursor following their batch. */
Napi::Object session_event_batch_object(Napi::Env env, const tmux_cxx::SessionEventBatch &batch) {
    auto object = Napi::Object::New(env);
    auto records = Napi::Array::New(env, batch.records.size());
    for (std::size_t index = 0; index < batch.records.size(); ++index) {
        records.Set(static_cast<std::uint32_t>(index),
                    session_event_record_object(env, batch.records[index]));
    }
    object.Set("records", records);
    object.Set("nextCursor", session_event_cursor_object(env, batch.next_cursor));
    return object;
}
/** @brief Materialize one stream event, recovery gap, or empty wait on the JavaScript thread. */
Napi::Value
session_event_stream_item_value(Napi::Env env,
                                const std::optional<tmux_cxx::SessionEventStreamItem> &item) {
    if (!item) {
        return env.Null();
    }
    return std::visit(
        /** @brief Add the stable stream discriminator to the active item alternative. */
        [env](const auto &value) -> Napi::Value {
            using Item = std::remove_cvref_t<decltype(value)>;
            if constexpr (std::same_as<Item, tmux_cxx::SessionEventRecord>) {
                auto object = session_event_record_object(env, value);
                object.Set("kind", "event");
                return object;
            } else {
                auto object = Napi::Object::New(env);
                object.Set("kind", "gap");
                object.Set("missedFrom", session_event_cursor_object(env, value.missed_from));
                object.Set("replacement", session_observation_object(env, value.replacement));
                return object;
            }
        },
        *item);
}
/** @brief Materialize copied client attachment state without exposing terminal descriptors. */
Napi::Object client_object(Napi::Env env, const tmux_cxx::ClientSnapshot &client) {
    auto object = Napi::Object::New(env);
    object.Set("id", Napi::BigInt::New(env, client.id.value));
    if (client.session) {
        object.Set("session", Napi::BigInt::New(env, client.session->value));
    } else {
        object.Set("session", env.Null());
    }
    object.Set("detachedFrom", Napi::String::New(env, client.detached_from));
    object.Set("identified", Napi::Boolean::New(env, client.identified));
    object.Set("hasTerminal", Napi::Boolean::New(env, client.has_terminal));
    object.Set("rows", Napi::Number::New(env, client.rows));
    object.Set("columns", Napi::Number::New(env, client.columns));
    object.Set("revision", Napi::BigInt::New(env, client.revision));
    return object;
}
/** @brief Copy UTF-8 report entries into one JavaScript array on the JavaScript thread. */
Napi::Array text_array(Napi::Env env, const std::vector<std::string> &values) {
    auto array = Napi::Array::New(env, values.size());
    for (std::size_t index = 0; index < values.size(); ++index) {
        array.Set(static_cast<std::uint32_t>(index), Napi::String::New(env, values[index]));
    }
    return array;
}
/** @brief Materialize one stock connection's profile, support, evidence, quirks, and limits. */
Napi::Object
stock_compatibility_object(Napi::Env env,
                           const tmux_cxx::StockConnectionCompatibility &compatibility) {
    namespace stock = tmux_cxx::protocol::tmux;
    auto object = Napi::Object::New(env);
    object.Set("profile", compatibility.profile);
    object.Set("release", compatibility.release);
    object.Set("releaseUncertain", compatibility.release_uncertain);
    object.Set("direction", std::string(stock::protocol_direction_name(compatibility.direction)));
    object.Set("mode", std::string(stock::connection_mode_name(compatibility.mode)));
    object.Set("tier", std::string(stock::support_tier_name(compatibility.tier)));
    object.Set("support", std::string(stock::capability_support_name(compatibility.support)));
    object.Set("verification",
               std::string(stock::compatibility_verification_name(compatibility.verification)));
    object.Set("commands", text_array(env, compatibility.commands));
    object.Set("quirks", text_array(env, compatibility.quirks));
    object.Set("evidence", text_array(env, compatibility.evidence));
    object.Set("limitations", text_array(env, compatibility.limitations));
    return object;
}
/** @brief Materialize a shared window value with exact bigint identities and revision. */
Napi::Object window_object(Napi::Env env, const tmux_cxx::WindowSnapshot &window) {
    auto object = Napi::Object::New(env);
    object.Set("id", Napi::BigInt::New(env, window.id.value));
    object.Set("pane", Napi::BigInt::New(env, window.pane.value));
    object.Set("name", Napi::String::New(env, window.name));
    object.Set("rows", Napi::Number::New(env, window.rows));
    object.Set("columns", Napi::Number::New(env, window.columns));
    object.Set("revision", Napi::BigInt::New(env, window.revision));
    return object;
}
/** @brief Materialize membership identities as bigints and the bounded session index as a number.
 */
Napi::Object window_link_object(Napi::Env env, const tmux_cxx::WindowLinkSnapshot &link) {
    auto object = Napi::Object::New(env);
    object.Set("id", Napi::BigInt::New(env, link.id.value));
    object.Set("session", Napi::BigInt::New(env, link.session.value));
    object.Set("window", Napi::BigInt::New(env, link.window.value));
    object.Set("index", Napi::Number::New(env, link.index));
    object.Set("revision", Napi::BigInt::New(env, link.revision));
    return object;
}
/** @brief Materialize one pane's window-owned geometry and active selection. */
Napi::Object pane_object(Napi::Env env, const tmux_cxx::PaneSnapshot &pane) {
    auto object = Napi::Object::New(env);
    object.Set("id", Napi::BigInt::New(env, pane.id.value));
    object.Set("window", Napi::BigInt::New(env, pane.window.value));
    object.Set("top", Napi::Number::New(env, pane.top));
    object.Set("left", Napi::Number::New(env, pane.left));
    object.Set("rows", Napi::Number::New(env, pane.rows));
    object.Set("columns", Napi::Number::New(env, pane.columns));
    object.Set("active", Napi::Boolean::New(env, pane.active));
    object.Set("revision", Napi::BigInt::New(env, pane.revision));
    return object;
}
/** @brief Materialize copied pane text with exact owner identity and terminal freshness on the
 * JavaScript thread. */
Napi::Object pane_text_object(Napi::Env env, const tmux_cxx::PaneTextSnapshot &text) {
    auto object = Napi::Object::New(env);
    object.Set("serverInstance", Napi::String::New(env, text.server_instance));
    object.Set("pane", Napi::BigInt::New(env, text.pane.value));
    object.Set("revision", Napi::BigInt::New(env, text.revision));
    object.Set("rows", text.rows);
    object.Set("columns", text.columns);
    object.Set("cursorRow", text.cursor_row);
    object.Set("cursorColumn", text.cursor_column);
    object.Set("cursorVisible", text.cursor_visible);
    object.Set("alternateScreen", text.alternate_screen);
    object.Set("complete", text.complete);
    object.Set("historyLines", text.history_lines);
    object.Set("historyGap", text.history_gap);
    object.Set("inputOpen", text.input_open);
    object.Set("inputPending", text.input_pending);
    auto lines = Napi::Array::New(env, text.lines.size());
    for (std::size_t row = 0; row < text.lines.size(); ++row) {
        lines.Set(static_cast<std::uint32_t>(row), Napi::String::New(env, text.lines[row]));
    }
    object.Set("lines", lines);
    return object;
}
/** @brief Convert completed native values on the JavaScript thread, preserving raw bytes as
 * Buffers. */
Napi::Value to_javascript(Napi::Env env, const NodeOperationValue &operation_value) {
    if (auto stream =
            std::get_if<std::shared_ptr<tmux_cxx::SessionEventStream>>(&operation_value)) {
        return session_event_stream_object(env, *stream);
    }
    if (auto batch = std::get_if<tmux_cxx::SessionEventBatch>(&operation_value)) {
        return session_event_batch_object(env, *batch);
    }
    if (auto observation = std::get_if<tmux_cxx::SessionObservation>(&operation_value)) {
        return session_observation_object(env, *observation);
    }
    if (auto text = std::get_if<tmux_cxx::PaneTextSnapshot>(&operation_value)) {
        return pane_text_object(env, *text);
    }
    if (auto session = std::get_if<tmux_cxx::SessionSnapshot>(&operation_value)) {
        return session_object(env, *session);
    }
    if (auto sessions = std::get_if<std::vector<tmux_cxx::SessionSnapshot>>(&operation_value)) {
        auto array = Napi::Array::New(env, sessions->size());
        for (std::size_t i = 0; i < sessions->size(); ++i) {
            array.Set(static_cast<std::uint32_t>(i), session_object(env, (*sessions)[i]));
        }
        return array;
    }
    if (auto client = std::get_if<tmux_cxx::ClientSnapshot>(&operation_value)) {
        return client_object(env, *client);
    }
    if (auto clients = std::get_if<std::vector<tmux_cxx::ClientSnapshot>>(&operation_value)) {
        auto array = Napi::Array::New(env, clients->size());
        for (std::size_t index = 0; index < clients->size(); ++index) {
            array.Set(static_cast<std::uint32_t>(index), client_object(env, (*clients)[index]));
        }
        return array;
    }
    if (auto compatibility =
            std::get_if<tmux_cxx::StockConnectionCompatibility>(&operation_value)) {
        return stock_compatibility_object(env, *compatibility);
    }
    if (auto text = std::get_if<std::string>(&operation_value)) {
        return Napi::Buffer<char>::Copy(env, text->data(), text->size());
    }
    if (auto window = std::get_if<tmux_cxx::WindowSnapshot>(&operation_value)) {
        return window_object(env, *window);
    }
    if (auto link = std::get_if<tmux_cxx::WindowLinkSnapshot>(&operation_value)) {
        return window_link_object(env, *link);
    }
    if (auto links = std::get_if<std::vector<tmux_cxx::WindowLinkSnapshot>>(&operation_value)) {
        auto array = Napi::Array::New(env, links->size());
        for (std::size_t i = 0; i < links->size(); ++i) {
            array.Set(static_cast<std::uint32_t>(i), window_link_object(env, (*links)[i]));
        }
        return array;
    }
    if (auto pane = std::get_if<tmux_cxx::PaneSnapshot>(&operation_value)) {
        return pane_object(env, *pane);
    }
    if (auto panes = std::get_if<std::vector<tmux_cxx::PaneSnapshot>>(&operation_value)) {
        auto array = Napi::Array::New(env, panes->size());
        for (std::size_t index = 0; index < panes->size(); ++index) {
            array.Set(static_cast<std::uint32_t>(index), pane_object(env, (*panes)[index]));
        }
        return array;
    }
    return env.Undefined();
}
/** @brief Move successful native values without discarding structured failures. */
template <class T> tmux_cxx::Result<NodeOperationValue> to_node_result(tmux_cxx::Result<T> result) {
    if (!result) {
        return std::unexpected(result.error());
    }
    return NodeOperationValue(std::move(*result));
}
/** @brief Move a successful subscription into shared binding lifetime without sharing its cursor.
 */
tmux_cxx::Result<NodeOperationValue>
to_node_result(tmux_cxx::Result<tmux_cxx::SessionEventStream> result) {
    if (!result) {
        return std::unexpected(result.error());
    }
    return NodeOperationValue(std::make_shared<tmux_cxx::SessionEventStream>(std::move(*result)));
}
/** @brief Preserve mutation failures and represent successful completion without a value. */
tmux_cxx::Result<NodeOperationValue> to_node_result(tmux_cxx::Result<void> result) {
    if (!result) {
        return std::unexpected(result.error());
    }
    return NodeOperationValue(std::monostate{});
}

/** @brief Hold one admitted request until its promise settles on the JavaScript thread. */
class NodeOperationWorker : public Napi::AsyncWorker {
  public:
    /** @brief Own one move-only operation and retain its connection until completion. */
    NodeOperationWorker(Napi::Env env, std::shared_ptr<tmux_cxx::ServerConnection> connection,
                        NodeOperation operation)
        : Napi::AsyncWorker(env), deferred_(Napi::Promise::Deferred::New(env)),
          connection_(std::move(connection)), operation_(std::move(operation)) {}
    /** @brief Expose the promise before admitting the worker to Node's queue. */
    Napi::Promise promise() const { return deferred_.Promise(); }
    /** @brief Run native work off the JavaScript thread and retain its failure context. */
    void Execute() override {
        try {
            auto operation_result = operation_(*connection_);
            if (operation_result) {
                operation_value_ = std::move(*operation_result);
            } else {
                error_code_ = operation_result.error().code;
                operation_name_ = operation_result.error().operation;
                server_instance_ = operation_result.error().server_instance;
                SetError(operation_result.error().message);
            }
        } catch (const std::exception &error) {
            error_code_ = tmux_cxx::TmuxErrorCode::io;
            SetError(error.what());
        }
    }
    /** @brief Resolve completed native values on the JavaScript thread. */
    void OnOK() override { deferred_.Resolve(to_javascript(Env(), operation_value_)); }
    /** @brief Reject the promise with the native classification, operation, and server identity. */
    void OnError(const Napi::Error &error) override {
        error.Value().Set("name", "TmuxError");
        error.Value().Set("code", std::string(tmux_cxx::tmux_error_code_name(error_code_)));
        error.Value().Set("operation", operation_name_);
        error.Value().Set("serverInstance", server_instance_);
        deferred_.Reject(error.Value());
    }

  private:
    Napi::Promise::Deferred deferred_; ///< Promise resolved or rejected on the JavaScript thread.
    std::shared_ptr<tmux_cxx::ServerConnection>
        connection_;          ///< Keeps admitted work alive without sharing server entity pointers.
    NodeOperation operation_; ///< Move-owned request run off the JavaScript thread.
    NodeOperationValue operation_value_; ///< Owned result awaiting JavaScript materialization.
    tmux_cxx::TmuxErrorCode error_code_ =
        tmux_cxx::TmuxErrorCode::none; ///< Classification retained for promise rejection.
    std::string operation_name_;       ///< Requested operation retained for rejection context.
    std::string server_instance_;      ///< Observed server identity retained when available.
};

/** @brief Copy required JavaScript text without coercing non-string arguments. */
std::string string_arg(const Napi::CallbackInfo &info, std::size_t index) {
    if (index >= info.Length() || !info[index].IsString()) {
        throw Napi::TypeError::New(info.Env(), "expected a string argument");
    }
    return info[index].As<Napi::String>().Utf8Value();
}
/** @brief Translate an exact unsigned 64-bit bigint before shared C++ identity resolution. */
std::uint64_t id_arg(const Napi::CallbackInfo &info, std::size_t index) {
    if (index >= info.Length() || !info[index].IsBigInt()) {
        throw Napi::TypeError::New(info.Env(), "identities must be bigint");
    }
    bool lossless = false;
    auto identity = info[index].As<Napi::BigInt>().Uint64Value(&lossless);
    if (!lossless) {
        throw Napi::RangeError::New(info.Env(), "identity must be an unsigned 64-bit value");
    }
    return identity;
}
/** @brief Copy and validate one owner-qualified event cursor before admitting worker execution. */
tmux_cxx::SessionEventCursor session_event_cursor_arg(const Napi::CallbackInfo &info,
                                                      std::size_t index) {
    if (index >= info.Length() || !info[index].IsObject()) {
        throw Napi::TypeError::New(info.Env(), "event cursor must be an object");
    }
    const auto object = info[index].As<Napi::Object>();
    const auto server_instance = object.Get("serverInstance");
    const auto next_sequence = object.Get("nextSequence");
    if (!server_instance.IsString() || !next_sequence.IsBigInt()) {
        throw Napi::TypeError::New(info.Env(),
                                   "event cursor requires serverInstance and nextSequence");
    }
    bool lossless = false;
    const auto sequence = next_sequence.As<Napi::BigInt>().Uint64Value(&lossless);
    if (!lossless) {
        throw Napi::RangeError::New(info.Env(), "event cursor sequence exceeds uint64");
    }
    return {server_instance.As<Napi::String>().Utf8Value(), sequence};
}
/** @brief Translate an exact unsigned 32-bit JavaScript integer before operation-specific
 * validation. */
std::uint32_t uint32_arg(const Napi::CallbackInfo &info, std::size_t position) {
    if (position >= info.Length() || !info[position].IsNumber()) {
        throw Napi::TypeError::New(info.Env(), "expected an unsigned 32-bit number");
    }
    const auto number = info[position].As<Napi::Number>().DoubleValue();
    if (!std::isfinite(number) || number < 0 ||
        number > std::numeric_limits<std::uint32_t>::max() || number != std::trunc(number)) {
        throw Napi::RangeError::New(info.Env(), "expected an integer in 0..4294967295");
    }
    return static_cast<std::uint32_t>(number);
}

/** @brief Translate the declared pane split spelling without accepting implicit variants. */
tmux_cxx::PaneSplitOrientation pane_split_orientation_arg(const Napi::CallbackInfo &info,
                                                          std::size_t index) {
    const auto orientation = string_arg(info, index);
    if (orientation == "left_right") {
        return tmux_cxx::PaneSplitOrientation::left_right;
    }
    if (orientation == "top_bottom") {
        return tmux_cxx::PaneSplitOrientation::top_bottom;
    }
    throw Napi::RangeError::New(info.Env(),
                                "pane split orientation must be left_right or top_bottom");
}

/** @brief Copy a byte array on the JavaScript thread before admitting asynchronous work. */
std::string bytes_arg(const Napi::CallbackInfo &info, std::size_t index) {
    if (index >= info.Length() || !info[index].IsTypedArray()) {
        throw Napi::TypeError::New(info.Env(), "expected a Uint8Array or Buffer");
    }
    auto typed_array = info[index].As<Napi::TypedArray>();
    if (typed_array.TypedArrayType() != napi_uint8_array) {
        throw Napi::TypeError::New(info.Env(), "expected a Uint8Array or Buffer");
    }
    auto bytes = info[index].As<Napi::Uint8Array>();
    if (bytes.ElementLength() == 0) {
        return {};
    }
    return {reinterpret_cast<const char *>(bytes.Data()), bytes.ElementLength()};
}

/** @brief Run one stream wait off the JavaScript thread and settle its promise once. */
class NodeSessionEventWorker : public Napi::AsyncWorker {
  public:
    /** @brief Retain the stream state until its one admitted wait settles. */
    NodeSessionEventWorker(Napi::Env env, std::shared_ptr<NodeSessionStreamState> state,
                           std::uint32_t timeout_ms)
        : Napi::AsyncWorker(env), deferred_(Napi::Promise::Deferred::New(env)),
          state_(std::move(state)), timeout_ms_(timeout_ms) {}
    /** @brief Expose the promise before admitting the worker to Node's queue. */
    Napi::Promise promise() const { return deferred_.Promise(); }
    /** @brief Wait for one native stream item without touching JavaScript handles. */
    void Execute() override {
        try {
            auto result = state_->stream->next(timeout_ms_);
            if (result) {
                item_ = std::move(*result);
            } else {
                error_code_ = result.error().code;
                operation_name_ = result.error().operation;
                server_instance_ = result.error().server_instance;
                SetError(result.error().message);
            }
        } catch (const std::exception &error) {
            error_code_ = tmux_cxx::TmuxErrorCode::io;
            SetError(error.what());
        }
    }
    /** @brief Resolve one stream item and reopen JavaScript-thread admission. */
    void OnOK() override {
        state_->next_pending = false;
        deferred_.Resolve(session_event_stream_item_value(Env(), item_));
    }
    /** @brief Reject one stream wait with its native classification and reopen admission. */
    void OnError(const Napi::Error &error) override {
        state_->next_pending = false;
        error.Value().Set("name", "TmuxError");
        error.Value().Set("code", std::string(tmux_cxx::tmux_error_code_name(error_code_)));
        error.Value().Set("operation", operation_name_);
        error.Value().Set("serverInstance", server_instance_);
        deferred_.Reject(error.Value());
    }

  private:
    Napi::Promise::Deferred deferred_; ///< Promise settled only on the JavaScript thread.
    std::shared_ptr<NodeSessionStreamState> state_; ///< Stream retained through worker completion.
    std::uint32_t timeout_ms_;                      ///< Bounded native wait in milliseconds.
    std::optional<tmux_cxx::SessionEventStreamItem> item_; ///< Completed item or empty wait.
    tmux_cxx::TmuxErrorCode error_code_ =
        tmux_cxx::TmuxErrorCode::none; ///< Classification retained for promise rejection.
    std::string operation_name_;       ///< Native operation retained for rejection context.
    std::string server_instance_;      ///< Observed server identity retained when available.
};

/** @brief Expose one native session observer without blocking the JavaScript thread. */
class NodeSessionEventStream : public Napi::ObjectWrap<NodeSessionEventStream> {
  public:
    /** @brief Adopt the internal shared state supplied by subscription completion. */
    explicit NodeSessionEventStream(const Napi::CallbackInfo &info)
        : Napi::ObjectWrap<NodeSessionEventStream>(info) {
        if (info.Length() != 1 || !info[0].IsExternal()) {
            throw Napi::TypeError::New(info.Env(),
                                       "SessionEventStream is created by subscribeSessions");
        }
        auto holder = info[0].As<Napi::External<std::shared_ptr<NodeSessionStreamState>>>().Data();
        state_ = *holder;
    }
    /** @brief Cancel an admitted wait before JavaScript releases this wrapper. */
    ~NodeSessionEventStream() override { state_->stream->close(); }

    /** @brief Define the internal stream methods used by the JavaScript ergonomic facade. */
    static Napi::Function define(Napi::Env env) {
        return DefineClass(
            env, "SessionEventStream",
            {
                InstanceAccessor("initial", &NodeSessionEventStream::initial, nullptr),
                InstanceMethod("readNext", &NodeSessionEventStream::read_next),
                InstanceMethod("closeNative", &NodeSessionEventStream::close_native),
            });
    }

  private:
    /** @brief Materialize the atomic creation baseline on the JavaScript thread. */
    Napi::Value initial(const Napi::CallbackInfo &info) {
        return session_observation_object(info.Env(), state_->stream->initial());
    }
    /** @brief Queue one bounded wait and reject overlapping reads of the same cursor. */
    Napi::Value read_next(const Napi::CallbackInfo &info) {
        if (state_->next_pending) {
            throw Napi::Error::New(info.Env(), "SessionEventStream already has a pending read");
        }
        const auto timeout_ms = info.Length() > 0 ? uint32_arg(info, 0) : 500;
        state_->next_pending = true;
        try {
            auto worker = std::make_unique<NodeSessionEventWorker>(info.Env(), state_, timeout_ms);
            auto promise = worker->promise();
            worker->Queue();
            // Napi::AsyncWorker owns and deletes itself after completion.
            worker.release();
            return promise;
        } catch (...) {
            state_->next_pending = false;
            throw;
        }
    }
    /** @brief Cancel the stream synchronously without changing persistent server state. */
    Napi::Value close_native(const Napi::CallbackInfo &info) {
        state_->stream->close();
        return info.Env().Undefined();
    }

    std::shared_ptr<NodeSessionStreamState> state_; ///< Native lifetime and JS admission guard.
};

Napi::Object
session_event_stream_object(Napi::Env env,
                            const std::shared_ptr<tmux_cxx::SessionEventStream> &stream) {
    auto *addon = env.GetInstanceData<NodeAddonState>();
    if (addon == nullptr) {
        throw Napi::Error::New(env, "tmux-cxx addon state is unavailable");
    }
    auto state = std::make_shared<NodeSessionStreamState>(stream);
    addon->observe(state);
    auto *holder = new std::shared_ptr<NodeSessionStreamState>(state);
    auto external = Napi::External<std::shared_ptr<NodeSessionStreamState>>::New(
        env, holder,
        /** @brief Release the constructor handoff after the wrapper copies shared state. */
        [](Napi::Env, std::shared_ptr<NodeSessionStreamState> *value) { delete value; });
    return addon->session_stream_constructor.New({external});
}

/** @brief Adapt JavaScript values and promises to the shared native connection without owning
 * server entities. */
class NodeServerConnection : public Napi::ObjectWrap<NodeServerConnection> {
  public:
    /** @brief Create a connection facade without opening traffic or starting a daemon. */
    explicit NodeServerConnection(const Napi::CallbackInfo &info)
        : Napi::ObjectWrap<NodeServerConnection>(info),
          connection_(std::make_shared<tmux_cxx::ServerConnection>(string_arg(info, 0))) {}

    /** @brief Publish the declared connection methods without connecting during addon loading. */
    static Napi::Object initialize(Napi::Env env, Napi::Object exports) {
        exports.Set(
            "ServerConnection",
            DefineClass(
                env, "ServerConnection",
                {
                    InstanceMethod("createSession", &NodeServerConnection::create_session),
                    InstanceMethod("sessions", &NodeServerConnection::sessions),
                    InstanceMethod("observeSessions", &NodeServerConnection::observe_sessions),
                    InstanceMethod("subscribeSessions", &NodeServerConnection::subscribe_sessions),
                    InstanceMethod("readSessionEvents", &NodeServerConnection::read_session_events),
                    InstanceMethod("waitSessionEvents", &NodeServerConnection::wait_session_events),
                    InstanceMethod("renameSession", &NodeServerConnection::rename_session),
                    InstanceMethod("killSession", &NodeServerConnection::kill_session),
                    InstanceMethod("clients", &NodeServerConnection::clients),
                    InstanceMethod("stockClientCompatibility",
                                   &NodeServerConnection::stock_client_compatibility),
                    InstanceMethod("attachClient", &NodeServerConnection::attach_client),
                    InstanceMethod("detachClient", &NodeServerConnection::detach_client),
                    InstanceMethod("resizeClient", &NodeServerConnection::resize_client),
                    InstanceMethod("linkWindow", &NodeServerConnection::link_window),
                    InstanceMethod("unlinkWindow", &NodeServerConnection::unlink_window),
                    InstanceMethod("windowLinks", &NodeServerConnection::window_links),
                    InstanceMethod("window", &NodeServerConnection::window),
                    InstanceMethod("splitPane", &NodeServerConnection::split_pane),
                    InstanceMethod("selectPane", &NodeServerConnection::select_pane),
                    InstanceMethod("panes", &NodeServerConnection::panes),
                    InstanceMethod("setPasteBuffer", &NodeServerConnection::set_paste_buffer),
                    InstanceMethod("readPasteBuffer", &NodeServerConnection::read_paste_buffer),
                    InstanceMethod("deletePasteBuffer", &NodeServerConnection::delete_paste_buffer),
                    InstanceMethod("pasteBufferIntoPane",
                                   &NodeServerConnection::paste_buffer_into_pane),
                    InstanceMethod("sendPaneInput", &NodeServerConnection::send_pane_input),
                    InstanceMethod("readPaneOutput", &NodeServerConnection::read_pane_output),
                    InstanceMethod("readPaneText", &NodeServerConnection::read_pane_text),
                    InstanceMethod("waitPaneOutput", &NodeServerConnection::wait_pane_output),
                    InstanceMethod("close", &NodeServerConnection::close),
                }));
        return exports;
    }

  private:
    /** @brief Transfer one task to Node's worker lifecycle and return its promise. */
    Napi::Value queue_operation(const Napi::CallbackInfo &info, NodeOperation operation) {
        auto worker =
            std::make_unique<NodeOperationWorker>(info.Env(), connection_, std::move(operation));
        auto promise = worker->promise();
        worker->Queue();
        // Napi::AsyncWorker owns and deletes itself after completion.
        worker.release();
        return promise;
    }
    /** @brief Copy creation arguments on the JavaScript thread before queuing native work. */
    Napi::Value create_session(const Napi::CallbackInfo &info) {
        auto name = string_arg(info, 0);
        auto command = info.Length() > 1 ? string_arg(info, 1) : "/bin/sh";
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::create_session */
                               [name, command](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.create_session(name, command));
                               });
    }
    /** @brief Queue materialized session observation through the shared connection. */
    Napi::Value sessions(const Napi::CallbackInfo &info) {
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::sessions */
                               [](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.sessions());
                               });
    }
    /** @brief Queue an atomic session catalog and following event cursor observation. */
    Napi::Value observe_sessions(const Napi::CallbackInfo &info) {
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::observe_sessions */
                               [](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.observe_sessions());
                               });
    }
    /** @brief Queue atomic baseline creation and return one owner-scoped native stream. */
    Napi::Value subscribe_sessions(const Napi::CallbackInfo &info) {
        return queue_operation(
            info, /** @copybrief tmux_cxx::ServerConnection::subscribe_sessions */
            [](tmux_cxx::ServerConnection &connection) {
                return to_node_result(connection.subscribe_sessions());
            });
    }
    /** @brief Copy one event cursor before queuing a retained committed-event read. */
    Napi::Value read_session_events(const Napi::CallbackInfo &info) {
        auto cursor = session_event_cursor_arg(info, 0);
        return queue_operation(
            info, /** @copybrief tmux_cxx::ServerConnection::read_session_events */
            [cursor = std::move(cursor)](tmux_cxx::ServerConnection &connection) mutable {
                return to_node_result(connection.read_session_events(std::move(cursor)));
            });
    }
    /** @brief Copy one event cursor and bounded deadline before queuing its asynchronous wait. */
    Napi::Value wait_session_events(const Napi::CallbackInfo &info) {
        auto cursor = session_event_cursor_arg(info, 0);
        const auto timeout_ms = info.Length() > 1 ? uint32_arg(info, 1) : 500;
        return queue_operation(
            info, /** @copybrief tmux_cxx::ServerConnection::wait_session_events */
            [cursor = std::move(cursor),
             timeout_ms](tmux_cxx::ServerConnection &connection) mutable {
                return to_node_result(
                    connection.wait_session_events(std::move(cursor), timeout_ms));
            });
    }
    /** @brief Copy a session identity and replacement name before queuing rename. */
    Napi::Value rename_session(const Napi::CallbackInfo &info) {
        auto id = tmux_cxx::SessionId{id_arg(info, 0)};
        auto name = string_arg(info, 1);
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::rename_session */
                               [id, name](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.rename_session(id, name));
                               });
    }
    /** @brief Queue explicit session deletion after validating its bigint identity. */
    Napi::Value kill_session(const Napi::CallbackInfo &info) {
        auto id = tmux_cxx::SessionId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::kill_session */
                               [id](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.kill_session(id));
                               });
    }
    /** @brief Queue copied client lifecycle observation through the shared connection. */
    Napi::Value clients(const Napi::CallbackInfo &info) {
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::clients */
                               [](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.clients());
                               });
    }
    /** @brief Validate one client identity before queuing its copied stock compatibility report. */
    Napi::Value stock_client_compatibility(const Napi::CallbackInfo &info) {
        const auto client = tmux_cxx::ClientId{id_arg(info, 0)};
        return queue_operation(
            info, /** @copybrief tmux_cxx::ServerConnection::stock_client_compatibility */
            [client](tmux_cxx::ServerConnection &connection) {
                return to_node_result(connection.stock_client_compatibility(client));
            });
    }
    /** @brief Validate client and session identities before queuing attachment. */
    Napi::Value attach_client(const Napi::CallbackInfo &info) {
        const auto client = tmux_cxx::ClientId{id_arg(info, 0)};
        const auto session = tmux_cxx::SessionId{id_arg(info, 1)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::attach_client */
                               [client, session](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.attach_client(client, session));
                               });
    }
    /** @brief Validate one client identity before queuing terminal restoration. */
    Napi::Value detach_client(const Napi::CallbackInfo &info) {
        const auto client = tmux_cxx::ClientId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::detach_client */
                               [client](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.detach_client(client));
                               });
    }
    /** @brief Validate one client identity before queuing retained-TTY remeasurement. */
    Napi::Value resize_client(const Napi::CallbackInfo &info) {
        const auto client = tmux_cxx::ClientId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::resize_client */
                               [client](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.resize_client(client));
                               });
    }
    /** @brief Validate shared-window and destination identities before queuing membership creation.
     */
    Napi::Value link_window(const Napi::CallbackInfo &info) {
        const auto window = tmux_cxx::WindowId{id_arg(info, 0)};
        const auto session = tmux_cxx::SessionId{id_arg(info, 1)};
        const auto index = uint32_arg(info, 2);
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::link_window */
                               [window, session, index](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(
                                       connection.link_window(window, session, index));
                               });
    }
    /** @brief Queue explicit removal of a validated window membership. */
    Napi::Value unlink_window(const Napi::CallbackInfo &info) {
        const auto id = tmux_cxx::WindowLinkId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::unlink_window */
                               [id](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.unlink_window(id));
                               });
    }
    /** @brief Queue ordered membership observation for a validated session identity. */
    Napi::Value window_links(const Napi::CallbackInfo &info) {
        const auto session = tmux_cxx::SessionId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::window_links */
                               [session](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.window_links(session));
                               });
    }
    /** @brief Queue shared-window observation for a validated bigint identity. */
    Napi::Value window(const Napi::CallbackInfo &info) {
        const auto id = tmux_cxx::WindowId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::window */
                               [id](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.window(id));
                               });
    }
    /** @brief Validate split arguments before queuing pane creation and layout mutation. */
    Napi::Value split_pane(const Napi::CallbackInfo &info) {
        const auto pane = tmux_cxx::PaneId{id_arg(info, 0)};
        const auto orientation = pane_split_orientation_arg(info, 1);
        const auto command = info.Length() > 2 ? string_arg(info, 2) : "/bin/sh";
        return queue_operation(
            info, /** @copybrief tmux_cxx::ServerConnection::split_pane */
            [pane, orientation, command](tmux_cxx::ServerConnection &connection) {
                return to_node_result(connection.split_pane(pane, orientation, command));
            });
    }
    /** @brief Queue shared-window pane selection for a validated pane identity. */
    Napi::Value select_pane(const Napi::CallbackInfo &info) {
        const auto pane = tmux_cxx::PaneId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::select_pane */
                               [pane](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.select_pane(pane));
                               });
    }
    /** @brief Queue window-owned pane geometry for a validated window identity. */
    Napi::Value panes(const Napi::CallbackInfo &info) {
        const auto window = tmux_cxx::WindowId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::panes */
                               [window](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.panes(window));
                               });
    }
    /** @brief Copy one name and byte array before queuing server-global buffer storage. */
    Napi::Value set_paste_buffer(const Napi::CallbackInfo &info) {
        auto name = string_arg(info, 0);
        auto bytes = bytes_arg(info, 1);
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::set_paste_buffer */
                               [name, bytes](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.set_paste_buffer(name, bytes));
                               });
    }
    /** @brief Queue exact byte observation for one copied paste-buffer name. */
    Napi::Value read_paste_buffer(const Napi::CallbackInfo &info) {
        auto name = string_arg(info, 0);
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::read_paste_buffer */
                               [name](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.read_paste_buffer(name));
                               });
    }
    /** @brief Queue explicit deletion for one copied paste-buffer name. */
    Napi::Value delete_paste_buffer(const Napi::CallbackInfo &info) {
        auto name = string_arg(info, 0);
        return queue_operation(
            info, /** @copybrief tmux_cxx::ServerConnection::delete_paste_buffer */
            [name](tmux_cxx::ServerConnection &connection) {
                return to_node_result(connection.delete_paste_buffer(name));
            });
    }
    /** @brief Validate a pane identity and queue one copied named buffer for program input. */
    Napi::Value paste_buffer_into_pane(const Napi::CallbackInfo &info) {
        auto name = string_arg(info, 0);
        const auto pane = tmux_cxx::PaneId{id_arg(info, 1)};
        return queue_operation(
            info, /** @copybrief tmux_cxx::ServerConnection::paste_buffer_into_pane */
            [name, pane](tmux_cxx::ServerConnection &connection) {
                return to_node_result(connection.paste_buffer_into_pane(name, pane));
            });
    }
    /** @brief Copy raw program-input bytes before queuing their delivery. */
    Napi::Value send_pane_input(const Napi::CallbackInfo &info) {
        auto id = tmux_cxx::PaneId{id_arg(info, 0)};
        auto bytes = bytes_arg(info, 1);
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::send_pane_input */
                               [id, bytes](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.send_pane_input(id, bytes));
                               });
    }
    /** @brief Queue a raw-byte observation for a validated pane identity. */
    Napi::Value read_pane_output(const Napi::CallbackInfo &info) {
        auto id = tmux_cxx::PaneId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::read_pane_output */
                               [id](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.read_pane_output(id));
                               });
    }
    /** @brief Queue interpreted pane text without exposing terminal or server storage. */
    Napi::Value read_pane_text(const Napi::CallbackInfo &info) {
        const auto id = tmux_cxx::PaneId{id_arg(info, 0)};
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::read_pane_text */
                               [id](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(connection.read_pane_text(id));
                               });
    }
    /** @brief Copy wait arguments on the JavaScript thread; shared C++ validates the deadline and
     * pattern. */
    Napi::Value wait_pane_output(const Napi::CallbackInfo &info) {
        auto id = tmux_cxx::PaneId{id_arg(info, 0)};
        auto needle = bytes_arg(info, 1);
        std::uint32_t timeout_ms = 500;
        if (info.Length() > 2) {
            timeout_ms = uint32_arg(info, 2);
        }
        return queue_operation(info, /** @copybrief tmux_cxx::ServerConnection::wait_pane_output */
                               [id, needle, timeout_ms](tmux_cxx::ServerConnection &connection) {
                                   return to_node_result(
                                       connection.wait_pane_output(id, needle, timeout_ms));
                               });
    }
    /** @brief Close admission synchronously while preserving persistent server sessions. */
    Napi::Value close(const Napi::CallbackInfo &info) {
        connection_->close();
        return info.Env().Undefined();
    }
    std::shared_ptr<tmux_cxx::ServerConnection>
        connection_; ///< Shared facade lifetime; no server entity ownership crosses the binding.
};
/** @brief Export the native connection adapter on the JavaScript thread. */
Napi::Object initialize_addon(Napi::Env env, Napi::Object exports) {
    auto addon = std::make_unique<NodeAddonState>();
    auto session_stream = NodeSessionEventStream::define(env);
    addon->session_stream_constructor = Napi::Persistent(session_stream);
    exports.Set("SessionEventStream", session_stream);
    env.SetInstanceData(addon.release());
    return NodeServerConnection::initialize(env, exports);
}
} // namespace

NODE_API_MODULE(tmux_cxx, initialize_addon)
