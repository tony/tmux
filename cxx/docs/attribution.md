# Source provenance

The surrounding read-only upstream tmux tree started at immutable revision
`d1fedcc9a0e98dbad57f92f923db46c6f4b6e893`. It is not a build dependency.

`src/protocol/tmux/imsg_codec.hpp` translates wire values and structures from upstream tmux 3.4,
commit `9ae69c3795ab5ef6b4d760f6398cd9281151f632`, original paths
`tmux-protocol.h` and `compat/imsg.h`. The copyright and permission notice is
preserved in the derived header. The selected imsg layout is specific to
that upstream release; protocol integer 8 alone does not identify it.

`include/tmux_cxx/protocol/tmux/protocol_profile.hpp` and the dual-layout codec additionally reference tmux 3.7, immutable commit `81f88f8517c9fc5371b56cf117530c6b477c96ac`, original paths `compat/imsg.h` and `compat/imsg.c`. The descriptor marker's copyright and permission notice is preserved in the profile header. Version masking follows `proc.c` in both recorded releases; control flags follow their `tmux.h` declarations. The independent C++ codecs use validated host-order words and copied metadata.

The tmux 3.7 reference fixture is built only during explicit provisioning from its locked source archive. Its binary, generated source, and producer record remain under `_build/`; it is not linked or invoked by the production engine or bindings.

`src/protocol/tmux/imsg_codec.cpp` and the stock stream response handling in `src/protocol/tmux/accepted_client_protocol.cpp`
were written with read-only reference to `client.c` and `file.c` at the same
tmux 3.4 commit. No upstream C file or object enters the production build.

The limited control attachment, pane-output, and session-notification behavior
was checked against `control.c`, `control-notify.c`, `cmd-new-session.c`,
`cmd-rename-session.c`, and `session.c` at the recorded tmux 3.7 commit. The C++
server uses independent bounded pane retention and a committed-event journal;
no upstream source or object enters the production build.

Window membership behavior was checked against `server_link_window`,
`server_unlink_window`, and `session_has` in the surrounding read-only
`server-fn.c` and `session.c` at the recorded parent revision. The C++
entity graph uses independent value types and ownership; those C files
are not production inputs.

Named paste-buffer storage and default paste delivery were checked against the
surrounding read-only `paste.c`, `cmd-set-buffer.c`, `cmd-delete-buffer.c`, and
`cmd-paste-buffer.c` at the recorded parent revision. The C++ implementation is
independent and intentionally exposes only the documented explicit-name
subset. No upstream C file or object enters the production build.

`terminal/` adapts the external C99 [libvterm 0.3.3 library](https://www.leonerd.org.uk/code/libvterm/). Its canonical source archive has SHA-256 `09156f43dd2128bd347cbeebe50d9a571d32c64e0cf18d211197946aff7226e0`. The archive is retained under `third_party/libvterm/`; CMake verifies its digest, extracts it into the selected build tree, and compiles its named C sources with that build's compiler and sanitizer configuration. The C++ terminal component owns parser lifetimes, callback failure containment, retained history, and copied values. Installed native products include libvterm's original MIT license. libvterm does not introduce a dependency on upstream tmux.

The build backports [Vim's UTF-8 continuation correction](https://github.com/vim/vim/commit/b24a1d9291b2d7fe202237a5d531076a095887d6) into a generated `state.c`. One decoder owns UTF-8 continuation, then designated GL character sets map decoded ASCII. This project extension prevents callback batching around malformed UTF-8 from changing character-set selection. The build also adapts [Neovim's right-edge combining predicate](https://github.com/neovim/neovim/commit/814f2629cbcf02b18ff08d394d15835419e563e4) to libvterm's combining loop: a combining mark updates the previous glyph without consuming pending wrap. [Vim's control-width correction](https://github.com/vim/vim/commit/54ffef7eb83f2592833f422b89bf5f30de6b4bb0) prevents encoded C1 controls from moving the cursor before the screen. [Vim's REP correction](https://github.com/vim/vim/commit/785bb3e9b271eb377dbd4157ce281eac7f16a021) ignores repetition before any graphic character exists. Native regressions and retained fuzz inputs reproduce the original failures.

The generated `parser.c` includes unchanged upstream bounds for [numeric CSI arguments](https://github.com/vim/vim/commit/77e7a40af2cab8c0f89a33553af42428b20af233) and [argument count](https://github.com/vim/vim/commit/fe05143f5d70c89e4a14cbf61fee091dc6ba791c). Numeric arguments saturate below the missing-argument sentinel, and argument storage remains capped at 16 entries. CMake checks original and corrected file hashes recorded in the toolchain lock; the extracted external sources remain unchanged. These corrections retain libvterm's MIT license and do not introduce first-party C engine code.

Untrusted OSC numbers and cursor offsets use two C-linkage functions implemented in C++23. OSC commands saturate instead of wrapping into another command; screen coordinates remain representable until libvterm applies its geometry bounds. The generated external C copies contain only their declarations and call sites.
