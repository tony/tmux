# Build only the explicitly locked external C library; production C++ never borrows tmux sources.
set(vterm_source_archive
  "${CMAKE_SOURCE_DIR}/third_party/libvterm/libvterm-0.3.3.tar.gz")
set(vterm_digest "09156f43dd2128bd347cbeebe50d9a571d32c64e0cf18d211197946aff7226e0")
if(NOT EXISTS "${vterm_source_archive}")
  message(FATAL_ERROR "The locked libvterm source archive is missing")
endif()
file(SHA256 "${vterm_source_archive}" vterm_source_archive_digest)
if(NOT vterm_source_archive_digest STREQUAL vterm_digest)
  message(FATAL_ERROR "The vendored libvterm source archive does not match its lock")
endif()

set(vterm_source_root "${CMAKE_BINARY_DIR}/third_party/libvterm")
set(vterm_source "${vterm_source_root}/libvterm-0.3.3")
set(vterm_source_stamp "${vterm_source_root}/.archive-sha256")
set(vterm_extract_sources ON)
if(EXISTS "${vterm_source_stamp}" AND EXISTS "${vterm_source}/src/state.c")
  file(READ "${vterm_source_stamp}" vterm_extracted_digest)
  string(STRIP "${vterm_extracted_digest}" vterm_extracted_digest)
  if(vterm_extracted_digest STREQUAL vterm_digest)
    set(vterm_extract_sources OFF)
  endif()
endif()
if(vterm_extract_sources)
  file(REMOVE_RECURSE "${vterm_source_root}")
  file(MAKE_DIRECTORY "${vterm_source_root}")
  file(ARCHIVE_EXTRACT INPUT "${vterm_source_archive}" DESTINATION "${vterm_source_root}")
  file(WRITE "${vterm_source_stamp}" "${vterm_digest}\n")
endif()

# Backport Vim b24a1d9291b2d7fe202237a5d531076a095887d6 and keep designated character
# sets independent of input-write boundaries without altering provisioned sources.
file(SHA256 "${vterm_source}/src/state.c" vterm_state_digest)
if(NOT vterm_state_digest STREQUAL "8f6504f091a91194fa4e972a142d586fb96bd8e7a15b6d26747323f1fe48ff38")
  message(FATAL_ERROR "The locked libvterm UTF-8 backport requires an unchanged state.c")
endif()
file(READ "${vterm_source}/src/state.c" vterm_state)
string(REPLACE "#include \"vterm_internal.h\"\n"
  "#include \"vterm_internal.h\"\n\nextern int tmux_cxx_vterm_saturating_add(int value, int delta);\n"
  vterm_state "${vterm_state}")
set(vterm_decoder_before [=[  VTermEncodingInstance *encoding =
    state->gsingle_set     ? &state->encoding[state->gsingle_set] :
    !(bytes[eaten] & 0x80) ? &state->encoding[state->gl_set] :
    state->vt->mode.utf8   ? &state->encoding_utf8 :
                             &state->encoding[state->gr_set];

  (*encoding->enc->decode)(encoding->enc, encoding->data,
      codepoints, &npoints, state->gsingle_set ? 1 : maxpoints,
      bytes, &eaten, len);]=])
set(vterm_decoder_after [=[  VTermEncodingInstance *gl_encoding = &state->encoding[state->gl_set];
  VTermEncodingInstance *encoding =
    state->gsingle_set   ? &state->encoding[state->gsingle_set] :
    state->vt->mode.utf8 ? &state->encoding_utf8 :
    !(bytes[eaten] & 0x80) ? gl_encoding : &state->encoding[state->gr_set];

  if(encoding->enc == state->encoding_utf8.enc)
    encoding = &state->encoding_utf8;  // Only use one UTF-8 encoding state.
  (*encoding->enc->decode)(encoding->enc, encoding->data,
      codepoints, &npoints, state->gsingle_set ? 1 : maxpoints,
      bytes, &eaten, len);

  /* Decode UTF-8 as one stream, then apply the active GL character set to
   * raw ASCII codepoints so callback batching cannot change their meaning. */
  if(state->vt->mode.utf8 && !state->gsingle_set &&
      gl_encoding->enc != state->encoding_utf8.enc) {
    for(int point = 0; point < npoints; point++) {
      if(codepoints[point] >= 0x20 && codepoints[point] < 0x7f) {
        char ascii = codepoints[point];
        uint32_t mapped = codepoints[point];
        int mapped_count = 0;
        size_t ascii_pos = 0;
        (*gl_encoding->enc->decode)(gl_encoding->enc, gl_encoding->data,
            &mapped, &mapped_count, 1, &ascii, &ascii_pos, 1);
        if(mapped_count == 1 && ascii_pos == 1)
          codepoints[point] = mapped;
      }
    }
  }]=])
string(REPLACE "${vterm_decoder_before}" "${vterm_decoder_after}" vterm_state "${vterm_state}")
# Adapt Neovim 814f2629cbcf02b18ff08d394d15835419e563e4's right-edge predicate to libvterm's combining loop.
string(REPLACE
  "state->pos.row == state->combine_pos.row && state->pos.col == state->combine_pos.col + state->combine_width"
  "state->pos.row == state->combine_pos.row &&\n        state->pos.col >= state->combine_pos.col &&\n        state->pos.col <= state->combine_pos.col + state->combine_width"
  vterm_state "${vterm_state}")
# Backport Vim 54ffef7eb83f2592833f422b89bf5f30de6b4bb0's control-width clamp.
string(REPLACE
  "      width += this_width;\n    }\n\n    while(i < npoints && vterm_unicode_is_combining(codepoints[i]))"
  "      width += this_width;\n    }\n\n    if(width < 0)\n      width = 0;\n\n    while(i < npoints && vterm_unicode_is_combining(codepoints[i]))"
  vterm_state "${vterm_state}")
# Backport Vim 785bb3e9b271eb377dbd4157ce281eac7f16a021's empty REP guard.
string(REPLACE
  "  case 0x62: { // REP - ECMA-48 8.3.103\n    const int row_width = THISROWWIDTH(state);\n    count = CSI_ARG_COUNT(args[0]);"
  "  case 0x62: { // REP - ECMA-48 8.3.103\n    const int row_width = THISROWWIDTH(state);\n\n    // ECMA-48 repeats the preceding graphic character; when none was\n    // printed yet \"combine_width\" is zero and the loop below would never\n    // advance the cursor.  Ignore the control then, like xterm does.\n    if(state->combine_width < 1)\n      break;\n\n    count = CSI_ARG_COUNT(args[0]);"
  vterm_state "${vterm_state}")
# Keep untrusted terminal counts representable until libvterm applies screen bounds.
string(REPLACE "state->pos.row += count;"
  "state->pos.row = tmux_cxx_vterm_saturating_add(state->pos.row, count);"
  vterm_state "${vterm_state}")
string(REPLACE "state->pos.col += count;"
  "state->pos.col = tmux_cxx_vterm_saturating_add(state->pos.col, count);"
  vterm_state "${vterm_state}")
string(REPLACE "state->pos.row += state->scrollregion_top;"
  "state->pos.row = tmux_cxx_vterm_saturating_add(state->pos.row, state->scrollregion_top);"
  vterm_state "${vterm_state}")
string(REPLACE "state->pos.col += SCROLLREGION_LEFT(state);"
  "state->pos.col = tmux_cxx_vterm_saturating_add(state->pos.col, SCROLLREGION_LEFT(state));"
  vterm_state "${vterm_state}")
string(REPLACE "rect.end_col   = state->pos.col + count;"
  "rect.end_col   = tmux_cxx_vterm_saturating_add(state->pos.col, count);"
  vterm_state "${vterm_state}")
string(REPLACE "col = state->pos.col + count;"
  "col = tmux_cxx_vterm_saturating_add(state->pos.col, count);"
  vterm_state "${vterm_state}")
string(SHA256 vterm_corrected_digest "${vterm_state}")
if(NOT vterm_corrected_digest STREQUAL "4503192bae1c2998070fff109fe600b2202dab35afdc4bd7b41bf0c7b89fa215")
  message(FATAL_ERROR
    "The generated libvterm UTF-8 backport differs from its locked result: ${vterm_corrected_digest}")
endif()
file(CONFIGURE OUTPUT "${CMAKE_BINARY_DIR}/vterm/state.c" CONTENT "@vterm_state@" @ONLY)

# Backport Vim 77e7a40af2cab8c0f89a33553af42428b20af233 and fe05143f5d70c89e4a14cbf61fee091dc6ba791c.
file(SHA256 "${vterm_source}/src/parser.c" vterm_parser_digest)
if(NOT vterm_parser_digest STREQUAL "237220e912ab95795f78c013d6734b88080b00595010dab87d11980c24d025af")
  message(FATAL_ERROR "The locked libvterm argument backports require an unchanged parser.c")
endif()
file(READ "${vterm_source}/src/parser.c" vterm_parser)
string(REPLACE "#include \"vterm_internal.h\"\n"
  "#include \"vterm_internal.h\"\n\nextern int tmux_cxx_vterm_append_decimal_digit(int value, int digit);\n"
  vterm_parser "${vterm_parser}")
set(vterm_osc_digits_before [=[      if(c >= '0' && c <= '9') {
        if(vt->parser.v.osc.command == -1)
          vt->parser.v.osc.command = 0;
        else
          vt->parser.v.osc.command *= 10;
        vt->parser.v.osc.command += c - '0';
        break;
      }]=])
set(vterm_osc_digits_after [=[      if(c >= '0' && c <= '9') {
        vt->parser.v.osc.command = tmux_cxx_vterm_append_decimal_digit(
            vt->parser.v.osc.command, c - '0');
        break;
      }]=])
string(REPLACE "${vterm_osc_digits_before}" "${vterm_osc_digits_after}"
  vterm_parser "${vterm_parser}")
set(vterm_csi_digits_before [=[        if(vt->parser.v.csi.args[vt->parser.v.csi.argi] == CSI_ARG_MISSING)
          vt->parser.v.csi.args[vt->parser.v.csi.argi] = 0;
        vt->parser.v.csi.args[vt->parser.v.csi.argi] *= 10;
        vt->parser.v.csi.args[vt->parser.v.csi.argi] += c - '0';]=])
set(vterm_csi_digits_after [=[        long arg_max = CSI_ARG_MISSING - 1;
        long *arg = &vt->parser.v.csi.args[vt->parser.v.csi.argi];
        int digit = c - '0';

        if(*arg == CSI_ARG_MISSING)
          *arg = 0;
        if(*arg > (arg_max - digit) / 10)
          *arg = arg_max;
        else
          *arg = *arg * 10 + digit;]=])
string(REPLACE "${vterm_csi_digits_before}" "${vterm_csi_digits_after}" vterm_parser "${vterm_parser}")
string(REPLACE "      if(c == ';') {\n        vt->parser.v.csi.argi++;"
  "      if(c == ';') {\n        if(vt->parser.v.csi.argi >= CSI_ARGS_MAX - 1)\n          break; /* drop excess args */\n        vt->parser.v.csi.argi++;"
  vterm_parser "${vterm_parser}")
string(SHA256 vterm_corrected_digest "${vterm_parser}")
if(NOT vterm_corrected_digest STREQUAL "08ab071ef14f9b7632b608b3fe18450f399774dfa1aac13db278fde9d529c4c8")
  message(FATAL_ERROR "The generated libvterm argument backports differ from their locked result")
endif()
file(CONFIGURE OUTPUT "${CMAKE_BINARY_DIR}/vterm/parser.c" CONTENT "@vterm_parser@" @ONLY)

set(vterm_compile_flags -x c -std=c99 -fPIC -Wall -Wextra -Wpedantic
  "$<$<CONFIG:Debug>:-g>" "$<$<NOT:$<CONFIG:Debug>>:-O2>")
if(TMUX_CXX_SANITIZE)
  list(APPEND vterm_compile_flags -fsanitize=address,undefined,fuzzer-no-link
    -fno-sanitize-recover=all -fno-omit-frame-pointer)
endif()
set(vterm_headers
  "${vterm_source}/include/vterm.h" "${vterm_source}/include/vterm_keycodes.h"
  "${vterm_source}/src/vterm_internal.h" "${vterm_source}/src/rect.h"
  "${vterm_source}/src/utf8.h" "${vterm_source}/src/fullwidth.inc"
  "${vterm_source}/src/encoding/DECdrawing.inc" "${vterm_source}/src/encoding/uk.inc")
set(vterm_objects)
foreach(vterm_name encoding keyboard mouse parser pen screen state unicode vterm)
  set(vterm_input "${vterm_source}/src/${vterm_name}.c")
  if(vterm_name STREQUAL "state" OR vterm_name STREQUAL "parser")
    set(vterm_input "${CMAKE_BINARY_DIR}/vterm/${vterm_name}.c")
  endif()
  set(vterm_object "${CMAKE_BINARY_DIR}/vterm/${vterm_name}.o")
  add_custom_command(OUTPUT "${vterm_object}"
    COMMAND "${CMAKE_COMMAND}" -E make_directory "${CMAKE_BINARY_DIR}/vterm"
    COMMAND "${CMAKE_CXX_COMPILER}" ${vterm_compile_flags}
      "-I${vterm_source}/include" "-I${vterm_source}/src" -c "${vterm_input}" -o "${vterm_object}"
    DEPENDS "${vterm_input}" ${vterm_headers}
    COMMAND_EXPAND_LISTS VERBATIM)
  list(APPEND vterm_objects "${vterm_object}")
endforeach()
add_library(tmux_cxx_vterm_arithmetic OBJECT
  "${CMAKE_SOURCE_DIR}/src/terminal/vterm_arithmetic.cpp")
tmux_cxx_target(tmux_cxx_vterm_arithmetic)
set(vterm_archive "${CMAKE_BINARY_DIR}/vterm/libtmux_cxx_vterm.a")
add_custom_command(OUTPUT "${vterm_archive}"
  COMMAND "${CMAKE_AR}" rcs "${vterm_archive}" ${vterm_objects}
    $<TARGET_OBJECTS:tmux_cxx_vterm_arithmetic>
  COMMAND "${CMAKE_RANLIB}" "${vterm_archive}"
  DEPENDS ${vterm_objects} tmux_cxx_vterm_arithmetic COMMAND_EXPAND_LISTS VERBATIM)
add_custom_target(tmux_cxx_vterm_build DEPENDS "${vterm_archive}")
add_library(tmux_cxx::vterm STATIC IMPORTED GLOBAL)
set_target_properties(tmux_cxx::vterm PROPERTIES IMPORTED_LOCATION "${vterm_archive}")
add_dependencies(tmux_cxx::vterm tmux_cxx_vterm_build)
