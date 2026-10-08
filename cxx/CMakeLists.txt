cmake_minimum_required(VERSION 4.4)
project(tmux_cxx VERSION 0.1.0 LANGUAGES CXX)

include(CTest)
include(GNUInstallDirs)
include(CheckCXXSourceCompiles)
include(CMakePackageConfigHelpers)

set(TMUX_CXX_STANDARD 23 CACHE STRING "Required native language standard")
if(TMUX_CXX_STANDARD LESS 23)
  message(FATAL_ERROR "tmux-cxx requires C++23 or newer")
endif()
set(TMUX_CXX_STDLIB "libstdc++" CACHE STRING "C++ standard library")
option(TMUX_CXX_SANITIZE "Instrument every native target with ASan and UBSan" OFF)
option(TMUX_CXX_BUILD_PYTHON_BINDING "Build the nanobind Python adapter" ON)
option(TMUX_CXX_BUILD_NODE_BINDING "Build the Node-API JavaScript adapter" ON)
set(TMUX_CXX_NODE_ROOT "${CMAKE_SOURCE_DIR}/_build/tools/node_current" CACHE PATH "Locked Node distribution")
find_package(Python 3.12 REQUIRED COMPONENTS Interpreter)

set(sanitizer_environment)
set(sanitizer_stub_options)
if(TMUX_CXX_SANITIZE)
  if(NOT CMAKE_CXX_COMPILER_ID MATCHES "Clang" OR NOT CMAKE_SYSTEM_NAME STREQUAL "Linux")
    message(FATAL_ERROR "The binding sanitizer preset requires the declared Linux Clang runtime")
  endif()
  execute_process(COMMAND "${CMAKE_CXX_COMPILER}" -print-file-name=libclang_rt.asan.so
    OUTPUT_VARIABLE sanitizer_runtime OUTPUT_STRIP_TRAILING_WHITESPACE
    COMMAND_ERROR_IS_FATAL ANY)
  if(NOT EXISTS "${sanitizer_runtime}")
    message(FATAL_ERROR "The selected compiler's shared ASan runtime is required")
  endif()
  get_filename_component(sanitizer_runtime_dir "${sanitizer_runtime}" DIRECTORY)
  get_filename_component(sanitizer_llvm_bin "${CMAKE_CXX_COMPILER}" DIRECTORY)
  set(sanitizer_environment "ASAN_SYMBOLIZER_PATH=${sanitizer_llvm_bin}/llvm-symbolizer"
    "ASAN_OPTIONS=detect_leaks=1:halt_on_error=1"
    "UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1")
  set(sanitizer_binding_environment "LD_PRELOAD=${sanitizer_runtime}"
    "LSAN_OPTIONS=suppressions=${CMAKE_SOURCE_DIR}/support/sanitizers/external_leaks.supp:print_suppressions=1")
  set(sanitizer_stub_options --sanitizer-runtime "${sanitizer_runtime}")
endif()

if(TMUX_CXX_STDLIB STREQUAL "libc++")
  if(NOT CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    message(FATAL_ERROR "The libc++ preset requires Clang")
  endif()
  get_filename_component(llvm_bin "${CMAKE_CXX_COMPILER}" DIRECTORY)
  get_filename_component(llvm_root "${llvm_bin}" DIRECTORY)
  set(llvm_runtime "${llvm_root}/lib/x86_64-unknown-linux-gnu")
  set(CMAKE_REQUIRED_FLAGS "-std=c++23 -stdlib=libc++")
else()
  set(CMAKE_REQUIRED_FLAGS "-std=c++23")
endif()
check_cxx_source_compiles("#include <expected>\n#include <span>\nint main() { std::expected<int,int> r(1); int a[1]{}; std::span<int> s(a); return *r - 1 + s[0]; }" TMUX_CXX_REQUIRED_LIBRARY)
if(NOT TMUX_CXX_REQUIRED_LIBRARY)
  message(FATAL_ERROR "Required C++23 std::expected/std::span facilities are unavailable")
endif()
check_cxx_source_compiles("#include <functional>\nint main() { std::move_only_function<int()> next = [] { return 0; }; return next(); }" TMUX_CXX_REQUIRED_CONTINUATION)
if(NOT TMUX_CXX_REQUIRED_CONTINUATION)
  message(FATAL_ERROR "The selected standard library must provide C++23 std::move_only_function")
endif()

# Apply coherent C++23, warning, runtime, and instrumentation settings to one native target.
function(tmux_cxx_target target)
  set_target_properties(${target} PROPERTIES CXX_STANDARD ${TMUX_CXX_STANDARD}
    CXX_STANDARD_REQUIRED YES CXX_EXTENSIONS NO POSITION_INDEPENDENT_CODE YES)
  target_compile_features(${target} PUBLIC cxx_std_23)
  target_compile_options(${target} PRIVATE -Wall -Wextra -Wpedantic -Werror=return-type
    -Werror=sign-compare)
  if(TMUX_CXX_STDLIB STREQUAL "libc++")
    target_compile_options(${target} PRIVATE -stdlib=libc++)
    target_link_options(${target} PRIVATE -stdlib=libc++)
    set_property(TARGET ${target} APPEND PROPERTY BUILD_RPATH "${llvm_runtime}")
  endif()
  if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    target_link_options(${target} PRIVATE -fuse-ld=lld)
    set_property(TARGET ${target} PROPERTY CXX_LINKER_LAUNCHER
      "${CMAKE_COMMAND};-E;env;LD_LIBRARY_PATH=${CMAKE_SOURCE_DIR}/_build/tools/llvm_runtime/usr/lib/x86_64-linux-gnu:$ENV{LD_LIBRARY_PATH}")
  endif()
  if(TMUX_CXX_SANITIZE)
    target_compile_options(${target} PRIVATE -fsanitize=address,undefined,fuzzer-no-link
      -fno-sanitize-recover=all -fno-omit-frame-pointer)
    target_link_options(${target} PRIVATE -fsanitize=address,undefined -shared-libasan)
    set_property(TARGET ${target} APPEND PROPERTY BUILD_RPATH "${sanitizer_runtime_dir}")
  endif()
endfunction()

include(cmake/VTerm.cmake)
add_library(tmux_cxx_terminal STATIC src/terminal/terminal_state.cpp)
add_library(tmux_cxx::terminal ALIAS tmux_cxx_terminal)
target_include_directories(tmux_cxx_terminal PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
target_include_directories(tmux_cxx_terminal PRIVATE src)
target_include_directories(tmux_cxx_terminal SYSTEM PRIVATE "${vterm_source}/include")
target_link_libraries(tmux_cxx_terminal PRIVATE tmux_cxx::vterm)
set_target_properties(tmux_cxx_terminal PROPERTIES EXPORT_NAME terminal)
tmux_cxx_target(tmux_cxx_terminal)

add_library(tmux_cxx_tmux_protocol STATIC
  src/protocol/tmux/imsg_codec.cpp
  src/protocol/tmux/payload_codec.cpp
  src/protocol/tmux/stock_profile_detector.cpp
  src/protocol/tmux/compatibility_policy.cpp
  src/protocol/tmux/protocol_quirk.cpp
  src/protocol/tmux/stock_command_exchange.cpp)
add_library(tmux_cxx::tmux_protocol ALIAS tmux_cxx_tmux_protocol)
target_include_directories(tmux_cxx_tmux_protocol PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
target_include_directories(tmux_cxx_tmux_protocol PRIVATE src)
set_target_properties(tmux_cxx_tmux_protocol PROPERTIES EXPORT_NAME tmux_protocol)
tmux_cxx_target(tmux_cxx_tmux_protocol)

add_library(tmux_cxx_connection STATIC
  src/protocol/native/framing.cpp src/connection/native_request_transport.cpp
  src/protocol/native/handshake.cpp
  src/protocol/native/reply_envelope.cpp
  src/protocol/native/session_event_codec.cpp
  src/connection/server_connection.cpp src/connection/stock_server_connection.cpp
  src/connection/stock_server_operation_catalog.cpp
  src/connection/stock_server_operation_registry.cpp
  src/connection/native_server_channel.cpp
  src/session/session_event_stream.cpp
  src/session/register_stock_server_operations.cpp
  src/session/operations/create_session_native_adapter.cpp
  src/session/operations/list_sessions_native_adapter.cpp
  src/session/operations/observe_sessions_native_adapter.cpp
  src/session/operations/read_session_events_native_adapter.cpp
  src/session/operations/wait_session_events_native_adapter.cpp
  src/session/operations/list_stock_server_session_names.cpp
  src/session/operations/rename_session_native_adapter.cpp
  src/session/operations/kill_session_native_adapter.cpp
  src/client/operations/list_clients_native_adapter.cpp
  src/client/operations/attach_client_native_adapter.cpp
  src/client/operations/detach_client_native_adapter.cpp
  src/client/operations/resize_client_native_adapter.cpp
  src/client/operations/read_stock_client_compatibility_native_adapter.cpp
  src/window_link/operations/link_window_native_adapter.cpp
  src/window_link/operations/unlink_window_native_adapter.cpp
  src/window_link/operations/list_window_links_native_adapter.cpp
  src/window/operations/read_window_native_adapter.cpp
  src/pane/operations/send_pane_input_native_adapter.cpp
  src/pane/operations/read_pane_output_native_adapter.cpp
  src/pane/operations/read_pane_text_native_adapter.cpp
  src/pane/operations/wait_pane_output_native_adapter.cpp
  src/pane/operations/split_pane_native_adapter.cpp
  src/pane/operations/select_pane_native_adapter.cpp
  src/pane/operations/list_panes_native_adapter.cpp
  src/paste_buffer/operations/set_paste_buffer_native_adapter.cpp
  src/paste_buffer/operations/read_paste_buffer_native_adapter.cpp
  src/paste_buffer/operations/delete_paste_buffer_native_adapter.cpp
  src/paste_buffer/operations/paste_buffer_into_pane_native_adapter.cpp)
add_library(tmux_cxx::connection ALIAS tmux_cxx_connection)
target_include_directories(tmux_cxx_connection PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
target_include_directories(tmux_cxx_connection PRIVATE src)
target_link_libraries(tmux_cxx_connection PRIVATE tmux_cxx_tmux_protocol)
set_target_properties(tmux_cxx_connection PROPERTIES EXPORT_NAME connection)
tmux_cxx_target(tmux_cxx_connection)

set(exposure_contract "${CMAKE_BINARY_DIR}/generated/exposure.cpp")
add_custom_command(OUTPUT "${exposure_contract}"
  COMMAND "${Python_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/support/codegen/native_contract.py"
    --output "${exposure_contract}"
  DEPENDS api/exposure.toml support/codegen/exposure.py support/codegen/native_contract.py
  VERBATIM)
add_library(tmux_cxx_exposure OBJECT "${exposure_contract}")
target_include_directories(tmux_cxx_exposure PRIVATE include)
tmux_cxx_target(tmux_cxx_exposure)
target_sources(tmux_cxx_connection PRIVATE $<TARGET_OBJECTS:tmux_cxx_exposure>)

add_library(tmux_cxx_server STATIC
  src/server/api_catalogs.cpp src/server/state.cpp src/server/daemon.cpp
  src/session/operations/read_session_events.cpp
  src/session/operations/wait_session_events.cpp
  src/session/session_event_journal.cpp src/session/session_event_reader.cpp
  src/client/client_terminal.cpp src/client/client_terminal_routing.cpp
  src/client/client_key_input.cpp
  src/client/client_view.cpp src/client/client_view_refresh.cpp src/client/stock_client_lifecycle.cpp
  src/client/register_operations.cpp
  src/client/register_key_bindings.cpp src/client/operations/detach_client_key_binding.cpp
  src/client/register_commands.cpp src/client/operations/attach_session_command.cpp
  src/client/operations/list_clients.cpp src/client/operations/attach_client.cpp
  src/client/operations/detach_client.cpp src/client/operations/resize_client.cpp
  src/client/operations/read_stock_client_compatibility.cpp
  src/protocol/native/operation_catalog.cpp src/protocol/native/operation_registry.cpp
  src/protocol/tmux/accepted_client_protocol.cpp src/protocol/tmux/control/channel.cpp
  src/protocol/tmux/control/session_notification_feed.cpp
  src/protocol/tmux/control/selected_session_output_feed.cpp
  src/protocol/tmux/control/command_line.cpp src/protocol/tmux/control/command_result.cpp
  src/protocol/tmux/control/pane_output.cpp src/protocol/tmux/control/session_changed.cpp
  src/protocol/tmux/control/session_renamed.cpp
  src/protocol/tmux/control/sessions_changed.cpp
  src/protocol/native/server_request.cpp
  src/commands/command_catalog.cpp src/commands/command_registry.cpp
  src/commands/session_target.cpp src/commands/session_format.cpp
  src/key_table/single_byte_key_parser.cpp
  src/key_table/key_table_catalog.cpp src/key_table/key_binding_registry.cpp
  src/option/option_catalog.cpp src/option/option_table.cpp
  src/session/register_operations.cpp
  src/session/register_commands.cpp
  src/session/session_options.cpp
  src/session/operations/create_session_command.cpp src/session/operations/list_sessions_command.cpp
  src/session/operations/rename_session_command.cpp src/session/operations/set_option_command.cpp
  src/session/operations/create_session.cpp src/session/operations/list_sessions.cpp
  src/session/operations/observe_sessions.cpp
  src/session/operations/rename_session.cpp src/session/operations/kill_session.cpp
  src/session/operations/set_session_option.cpp
  src/pane/register_operations.cpp
  src/pane/register_key_bindings.cpp src/pane/operations/send_prefix_key_binding.cpp
  src/pane/operations/split_pane_key_bindings.cpp
  src/pane/operations/select_next_pane_key_binding.cpp
  src/pane/register_commands.cpp src/pane/operations/split_window_command.cpp
  src/pane/operations/select_pane_command.cpp src/pane/pane_target.cpp
  src/pane/program.cpp src/pane/pane_output_buffer.cpp src/pane/pane_output_reader.cpp
  src/pane/pane_program_input.cpp src/pane/pty_io.cpp src/pane/terminal_resize.cpp
  src/pane/spawn_pane.cpp
  src/pane/operations/split_pane.cpp src/pane/operations/select_pane.cpp
  src/pane/operations/list_panes.cpp
  src/pane/operations/send_pane_input.cpp src/pane/operations/read_pane_output.cpp
  src/pane/operations/read_pane_text.cpp
  src/pane/operations/wait_pane_output.cpp
  src/paste_buffer/register_operations.cpp
  src/paste_buffer/register_commands.cpp
  src/paste_buffer/operations/set_paste_buffer.cpp
  src/paste_buffer/operations/read_paste_buffer.cpp
  src/paste_buffer/operations/delete_paste_buffer.cpp
  src/paste_buffer/operations/paste_buffer_into_pane.cpp
  src/paste_buffer/operations/set_buffer_command.cpp
  src/paste_buffer/operations/delete_buffer_command.cpp
  src/paste_buffer/operations/paste_buffer_command.cpp
  src/window/register_operations.cpp
  src/window/layout.cpp src/window/window_size.cpp
  src/window/operations/read_window.cpp
  src/window_link/register_operations.cpp
  src/window_link/operations/link_window.cpp src/window_link/operations/unlink_window.cpp
  src/window_link/operations/list_window_links.cpp)
add_library(tmux_cxx::server ALIAS tmux_cxx_server)
target_link_libraries(tmux_cxx_server PUBLIC tmux_cxx_connection
  PRIVATE tmux_cxx_terminal tmux_cxx_tmux_protocol util)
target_include_directories(tmux_cxx_server PRIVATE src)
set_target_properties(tmux_cxx_server PROPERTIES EXPORT_NAME server)
tmux_cxx_target(tmux_cxx_server)

add_library(tmux_cxx_cli STATIC src/cli/action_registry.cpp src/cli/action_catalog.cpp
  src/session/register_cli_actions.cpp
  src/session/operations/create_session_cli_action.cpp
  src/session/operations/list_sessions_cli_action.cpp)
target_link_libraries(tmux_cxx_cli PRIVATE tmux_cxx_connection)
target_include_directories(tmux_cxx_cli PRIVATE src)
tmux_cxx_target(tmux_cxx_cli)

add_executable(tmux-cxx src/main.cpp)
target_link_libraries(tmux-cxx PRIVATE tmux_cxx_server tmux_cxx_cli)
tmux_cxx_target(tmux-cxx)

if(TMUX_CXX_BUILD_PYTHON_BINDING)
  set(binding_documentation "${CMAKE_BINARY_DIR}/generated/binding_documentation.hpp")
  add_custom_command(OUTPUT "${binding_documentation}"
    COMMAND "${Python_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/support/codegen/binding_documentation.py"
      --output "${binding_documentation}"
    DEPENDS api/exposure.toml support/codegen/exposure.py
      support/codegen/binding_documentation.py
      include/tmux_cxx/connection/server_connection.hpp
      include/tmux_cxx/client/client_snapshot.hpp
      include/tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp
      include/tmux_cxx/session/session_event_stream.hpp
      include/tmux_cxx/session/session_snapshot.hpp
      include/tmux_cxx/window/window_snapshot.hpp
      include/tmux_cxx/window_link/window_link_snapshot.hpp
      include/tmux_cxx/pane/pane_snapshot.hpp
      include/tmux_cxx/pane/pane_text_snapshot.hpp
    VERBATIM)
  add_custom_target(binding_documentation_header ALL DEPENDS "${binding_documentation}")
endif()

if(TMUX_CXX_BUILD_NODE_BINDING)
  add_custom_command(OUTPUT "${CMAKE_SOURCE_DIR}/bindings/node/index.d.ts"
    COMMAND "${Python_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/support/codegen/node/generate.py"
    DEPENDS api/exposure.toml support/codegen/exposure.py support/codegen/node/generate.py
      include/tmux_cxx/connection/server_connection.hpp
      include/tmux_cxx/client/client_snapshot.hpp
      include/tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp
      include/tmux_cxx/session/session_event_stream.hpp
      include/tmux_cxx/session/session_snapshot.hpp
      include/tmux_cxx/window/window_snapshot.hpp
      include/tmux_cxx/window_link/window_link_snapshot.hpp
      include/tmux_cxx/pane/pane_snapshot.hpp
      include/tmux_cxx/pane/pane_text_snapshot.hpp
    VERBATIM)
  add_custom_target(node_declarations ALL
    DEPENDS "${CMAKE_SOURCE_DIR}/bindings/node/index.d.ts")
endif()

if(TMUX_CXX_BUILD_PYTHON_BINDING)
  find_package(Python 3.12 REQUIRED COMPONENTS Interpreter Development.Module)
  execute_process(COMMAND "${Python_EXECUTABLE}" -m nanobind --cmake_dir
    OUTPUT_VARIABLE nanobind_dir OUTPUT_STRIP_TRAILING_WHITESPACE
    COMMAND_ERROR_IS_FATAL ANY)
  find_package(nanobind CONFIG REQUIRED PATHS "${nanobind_dir}" NO_DEFAULT_PATH)
  nanobind_add_module(_native NOMINSIZE NOSTRIP bindings/python/module.cpp)
  target_link_libraries(_native PRIVATE tmux_cxx_connection)
  tmux_cxx_target(_native)
  # Nanobind's runtime must use the same standard library as the extension.
  if(TMUX_CXX_STDLIB STREQUAL "libc++")
    target_compile_options(nanobind-static PRIVATE -stdlib=libc++)
  endif()
  if(TMUX_CXX_SANITIZE)
    target_compile_options(nanobind-static PRIVATE -fsanitize=address,undefined,fuzzer-no-link
      -fno-sanitize-recover=all -fno-omit-frame-pointer)
  endif()
  set(python_dir "${CMAKE_BINARY_DIR}/python/tmux_cxx")
  file(MAKE_DIRECTORY "${python_dir}")
  foreach(python_module IN ITEMS __init__.py server_connection.py session_event_stream.py)
    configure_file("bindings/python/tmux_cxx/${python_module}" "${python_dir}/${python_module}"
      COPYONLY)
  endforeach()
  set_target_properties(_native PROPERTIES LIBRARY_OUTPUT_DIRECTORY "${python_dir}")
  file(WRITE "${python_dir}/py.typed" "")
  add_custom_command(OUTPUT "${python_dir}/_native.pyi"
    COMMAND "${Python_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/support/codegen/python/generate.py"
      --python-path "${CMAKE_BINARY_DIR}/python" --output "${python_dir}/_native.pyi"
      ${sanitizer_stub_options}
    DEPENDS _native api/exposure.toml support/codegen/exposure.py support/codegen/python/generate.py
    VERBATIM)
  add_custom_target(python_stubs ALL DEPENDS "${python_dir}/_native.pyi")

  add_dependencies(_native binding_documentation_header)
  target_include_directories(_native PRIVATE "${CMAKE_BINARY_DIR}/generated")

  install(TARGETS _native LIBRARY DESTINATION tmux_cxx)
  install(FILES "${python_dir}/_native.pyi" "${python_dir}/py.typed" DESTINATION tmux_cxx)
endif()

if(TMUX_CXX_BUILD_NODE_BINDING)
  set(node_headers "${TMUX_CXX_NODE_ROOT}/include/node")
  set(node_api_headers "${CMAKE_SOURCE_DIR}/bindings/node/node_modules/node-addon-api")
  if(NOT EXISTS "${node_headers}/node_api.h" OR NOT EXISTS "${node_api_headers}/napi.h")
    message(FATAL_ERROR "Node and node-addon-api are required; run the documented bootstrap")
  endif()
  add_library(tmux_cxx_node MODULE bindings/node/addon.cpp)
  target_include_directories(tmux_cxx_node SYSTEM PRIVATE "${node_headers}" "${node_api_headers}")
  target_compile_definitions(tmux_cxx_node PRIVATE NAPI_VERSION=8
    NODE_ADDON_API_CPP_EXCEPTIONS NODE_ADDON_API_CPP_EXCEPTIONS_ALL)
  target_link_libraries(tmux_cxx_node PRIVATE tmux_cxx_connection)
  tmux_cxx_target(tmux_cxx_node)
  add_dependencies(tmux_cxx_node node_declarations)
  set_target_properties(tmux_cxx_node PROPERTIES PREFIX "" SUFFIX ".node"
    OUTPUT_NAME tmux_cxx LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/node")
endif()

if(BUILD_TESTING)
  if(NOT EXISTS "${CMAKE_SOURCE_DIR}/_build/tools/googletest/CMakeLists.txt")
    message(FATAL_ERROR "Locked GoogleTest sources are required; run bootstrap --provision")
  endif()
  set(INSTALL_GTEST OFF CACHE BOOL "Keep test dependencies out of installed products" FORCE)
  set(BUILD_GMOCK OFF CACHE BOOL "Use a single native test framework" FORCE)
  add_subdirectory(_build/tools/googletest _build/googletest EXCLUDE_FROM_ALL)
  tmux_cxx_target(gtest)
  tmux_cxx_target(gtest_main)
  include(GoogleTest)
  add_executable(server_test tests/cpp/server_test.cpp tests/cpp/operation_registry_test.cpp
    tests/cpp/command_registry_test.cpp tests/cpp/control_command_line_test.cpp
    tests/cpp/control_channel_test.cpp
    tests/cpp/key_table_test.cpp
    tests/cpp/option_test.cpp
    tests/cpp/stock_server_operation_catalog_test.cpp
    tests/cpp/cli_action_registry_test.cpp
    tests/cpp/native_protocol_test.cpp
    tests/cpp/tmux_protocol_test.cpp tests/cpp/terminal_state_test.cpp tests/cpp/pane_program_input_test.cpp
    tests/cpp/pane_program_test.cpp tests/cpp/pane_pty_io_test.cpp tests/cpp/client_test.cpp
    tests/cpp/paste_buffer_test.cpp)
  target_link_libraries(server_test PRIVATE tmux_cxx_server tmux_cxx_cli GTest::gtest_main)
  target_include_directories(server_test PRIVATE src)
  tmux_cxx_target(server_test)
  add_executable(sanitizer_probe tests/cpp/sanitizer_probe.cpp)
  tmux_cxx_target(sanitizer_probe)
  add_executable(stock_server_client_test tests/cpp/stock_server_client_test.cpp)
  target_link_libraries(stock_server_client_test PRIVATE tmux_cxx_connection)
  tmux_cxx_target(stock_server_client_test)
  foreach(fuzz_component native_protocol tmux_protocol terminal_state)
    set(fuzz_input "${fuzz_component}_fuzz_input")
    add_library(${fuzz_input} OBJECT "tests/cpp/fuzz/${fuzz_component}.cpp")
    target_include_directories(${fuzz_input} PRIVATE src include)
    tmux_cxx_target(${fuzz_input})
    if(TMUX_CXX_SANITIZE)
      set(fuzz_driver "${fuzz_component}_fuzz")
      add_executable(${fuzz_driver} $<TARGET_OBJECTS:${fuzz_input}>)
      target_link_libraries(${fuzz_driver} PRIVATE tmux_cxx_server)
      tmux_cxx_target(${fuzz_driver})
      target_link_options(${fuzz_driver} PRIVATE -fsanitize=fuzzer)
    endif()
  endforeach()
  gtest_discover_tests(server_test DISCOVERY_MODE PRE_TEST
    PROPERTIES TIMEOUT 1 LABELS "inner" ENVIRONMENT "${sanitizer_environment}")
  if(TMUX_CXX_BUILD_PYTHON_BINDING AND TMUX_CXX_BUILD_NODE_BINDING)
    set(test_environment "PYTHONPATH=${CMAKE_BINARY_DIR}/python"
      "TMUX_CXX_DAEMON=$<TARGET_FILE:tmux-cxx>"
      "TMUX_CXX_STOCK_CLIENT=${CMAKE_SOURCE_DIR}/_build/fixtures/tmux-3.4/bin/tmux"
      "TMUX_CXX_MODERN_STOCK_CLIENT=${CMAKE_SOURCE_DIR}/_build/fixtures/tmux-3.7/bin/tmux"
      "TMUX_CXX_STOCK_SERVER_CLIENT=$<TARGET_FILE:stock_server_client_test>"
      "TMUX_CXX_ADDON=$<TARGET_FILE:tmux_cxx_node>"
      "TMPDIR=${CMAKE_BINARY_DIR}/test_tmp" ${sanitizer_environment} ${sanitizer_binding_environment})
    file(MAKE_DIRECTORY "${CMAKE_BINARY_DIR}/test_tmp")
    add_test(NAME python-server COMMAND "${Python_EXECUTABLE}" -m pytest -q
      --basetemp "${CMAKE_BINARY_DIR}/test_tmp/py-server"
      "${CMAKE_SOURCE_DIR}/tests/integration/test_server.py"
      "${CMAKE_SOURCE_DIR}/tests/integration/test_shared_server.py")
    add_test(NAME python-protocol-compatibility COMMAND "${Python_EXECUTABLE}" -m pytest -q
      --basetemp "${CMAKE_BINARY_DIR}/test_tmp/py-compatibility"
      "${CMAKE_SOURCE_DIR}/tests/compat/test_native_replies.py"
      "${CMAKE_SOURCE_DIR}/tests/compat/test_stock_clients.py")
    add_test(NAME python-tooling COMMAND "${Python_EXECUTABLE}" -m pytest -q
      --basetemp "${CMAKE_BINARY_DIR}/test_tmp/py-tooling"
      "${CMAKE_SOURCE_DIR}/tests/tooling/test_bootstrap.py")
    add_test(NAME node-server COMMAND "${TMUX_CXX_NODE_ROOT}/bin/node"
      "${CMAKE_SOURCE_DIR}/bindings/node/node_modules/vitest/vitest.mjs" run
      --root "${CMAKE_SOURCE_DIR}/bindings/node")
    add_test(NAME node-server-lts COMMAND "${CMAKE_SOURCE_DIR}/_build/tools/node_lts/bin/node"
      "${CMAKE_SOURCE_DIR}/bindings/node/node_modules/vitest/vitest.mjs" run
      --root "${CMAKE_SOURCE_DIR}/bindings/node")
    set_tests_properties(python-server python-protocol-compatibility python-tooling
      node-server node-server-lts PROPERTIES
      ENVIRONMENT "${test_environment}" TIMEOUT 5 LABELS "mid"
      WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  endif()
endif()

install(TARGETS tmux_cxx_connection tmux_cxx_server tmux_cxx_terminal tmux_cxx_tmux_protocol tmux-cxx
  EXPORT tmux_cxxTargets
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(FILES "${vterm_archive}" DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(FILES "${vterm_source}/LICENSE" DESTINATION ${CMAKE_INSTALL_DATADIR}/tmux-cxx/licenses
  RENAME libvterm.txt)
install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(EXPORT tmux_cxxTargets NAMESPACE tmux_cxx:: DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/tmux_cxx)
configure_package_config_file(cmake/tmux_cxxConfig.cmake.in
  "${CMAKE_BINARY_DIR}/tmux_cxxConfig.cmake"
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/tmux_cxx)
write_basic_package_version_file("${CMAKE_BINARY_DIR}/tmux_cxxConfigVersion.cmake"
  VERSION ${PROJECT_VERSION} COMPATIBILITY SameMajorVersion)
install(FILES "${CMAKE_BINARY_DIR}/tmux_cxxConfig.cmake"
  "${CMAKE_BINARY_DIR}/tmux_cxxConfigVersion.cmake"
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/tmux_cxx)
