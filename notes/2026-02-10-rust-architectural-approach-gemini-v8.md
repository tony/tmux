# TermForge v8 Architecture Specification (DRAFT)

Date: 2026-02-11
Status: DRAFT -- v8, incorporating deep-dive review feedback.
Lineage: v7 (Final) -> **v8** (Gemini-driven refinement).
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the single authoritative architectural reference for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings, CRDT collaboration, and a ratatui-based TUI client.

**Repository locations:**
- tmux C source: `~/study/c/tmux/` (behavioral reference)
- Rust prototype: `~/work/rust/vibe-tmux/` (existing crate structure)
- Python libtmux: `~/work/python/libtmux/` (API design reference)
- ratatui: `~/study/rust/ratatui/` (TUI framework)
- zellij: `~/study/rust/zellij/` (multiplexer reference)
- PyO3: `~/study/rust-python/pyo3/` (Python binding framework)
- Neon: `~/study/rust-node/neon/` (Node.js binding framework)
- OTEL Rust SDK: `~/study/otel/opentelemetry-rust/` (telemetry SDK)

**Settled decisions (not re-argued):**

| # | Decision | Rationale |
|---|---|---|
| S1 | New multiplexer, tmux-compatible | Architectural freedom with wire compatibility. Not a C-to-Rust port. |
| S2 | Protocol version 8 | Wire-compatible with tmux protocol v8 (`tmux-protocol.h:23`). |
| S3 | SlotMap entity IDs | `slotmap::new_key_type!` for all entity IDs (Session, Window, Pane, Client, Job, Buffer). |
| S4 | Flat arena LayoutTree | `Vec<LayoutCell>` with index-based parent/children. Not recursive nesting. |
| S5 | Round-robin resize | One cell at a time, matching `layout.c:448-462`. Not proportional. |
| S6 | flock locking | `flock(LOCK_EX|LOCK_NB)` per `client.c:77-101`. Not PID-based. |
| S7 | Config after identify | Load config only after first client completes identify burst (`server-client.c:3725-3734`). |
| S8 | Protocol violation kills connection | Matching `server-client.c:3472-3475`. Never drop and continue. |
| S9 | Custom VT100 parser | Match tmux's `input.c` state table exactly. Do not use the `vte` crate. |
| S10 | Option FALLTHROUGH | WindowPane falls through to Window scope, matching `options.c:891-903`. |
| S11 | Vec-inside-entity for parent-child | Not SecondaryMap. Reverse lookups computed during snapshot construction. |
| S12 | WASM compilability as CI purity gate | Not production target. `cargo check --target wasm32-unknown-unknown`. |
| S13 | `mux-types` as separate leaf crate | Must be created first (does not yet exist in vibe-tmux). |
| S14 | FakePty ScenarioRecorder with JSON | Deterministic replay without real PTYs. |
| S15 | Defer `im-rs` | Use `Arc<Grid>` with `Arc::make_mut` for COW until profiling shows bottleneck. |

---

## Table of Contents

1. [Vision and Philosophy](#1-vision-and-philosophy)
2. [North Star Acceptance Criteria](#2-north-star-acceptance-criteria)
3. [High-Level Architecture](#3-high-level-architecture)
4. [Workspace Layout](#4-workspace-layout)
5. [Layering Contract](#5-layering-contract)
6. [Entity Model](#6-entity-model)
7. [Event/Effect Engine](#7-eventeffect-engine)
8. [Error Handling](#8-error-handling)
9. [Protocol Codec](#9-protocol-codec)
10. [Configuration System](#10-configuration-system)
11. [Layout Engine](#11-layout-engine)
12. [ORM-like Query API](#12-orm-like-query-api)
13. [Runtime Architecture](#13-runtime-architecture)
14. [Server Lifecycle](#14-server-lifecycle)
15. [Control Mode](#15-control-mode)
16. [Language Bindings](#16-language-bindings)
17. [CRDT Transaction Layer](#17-crdt-transaction-layer)
18. [Security Model](#18-security-model)
19. [OpenTelemetry](#19-opentelemetry)
20. [tmux Version Management](#20-tmux-version-management)
21. [Test Support and Fake PTY](#21-test-support-and-fake-pty)
22. [Binding Test Frameworks](#22-binding-test-frameworks)
23. [Test Framework and Harness Design](#23-test-framework-and-harness-design)
24. [Performance Targets](#24-performance-targets)
25. [Visual Client / TUI](#25-visual-client--tui)
26. [AGENTS.md Rules](#26-agentsmd-rules)
27. [Risks and Mitigations](#27-risks-and-mitigations)
28. [Plan Evolution and Changelog](#28-plan-evolution-and-changelog)
29. [Reference Anchors](#29-reference-anchors)
30. [Appendix: Canonical Type Quick Reference](#30-appendix-canonical-type-quick-reference)
31. [Supplemental Test Matrix](#31-supplemental-test-matrix)

---

## 1. Vision and Philosophy

### 1.1 Core Thesis

TermForge is a **new terminal multiplexer** written in Rust that is **wire-compatible** with tmux protocol v8. It is not a port of tmux's C code. It uses tmux as a behavioral reference while building a clean, layered Rust architecture from scratch.

The internal architecture is Rust-native: algebraic types, ownership-tracked state, pure-functional kernel, and async runtime -- while maintaining bit-for-bit compatibility with the tmux wire format.

### 1.2 Design Principles

1. **Pure/Impure Separation (Sans-IO).** The kernel (`mux-core`) is a pure `fn(graph, event) -> (graph, effects)` reducer. It never touches file descriptors, system calls, timers, or network. All IO happens in the runtime layer that interprets `Effect` variants.

2. **tmux is a Compatibility Profile, Not the Architecture.** tmux's `struct session`, `struct window`, `struct window_pane` inform our entity model but do not dictate it. Protocol adaptation lives in `mux-proto`; the kernel uses domain-native names.

3. **Snapshots are the Read Path.** The authoritative state lives in the `ServerGraph`. Readers (UI, bindings, query API) see immutable snapshots published via `ArcSwap`. Writers go through the event/effect engine. This eliminates read contention.

4. **ORM is a Facade, Not the Engine.** `mux-orm` translates user intent into commands/events. It never implements multiplexer logic. It must not become a second business-logic engine.

5. **Layered Testing.** Every layer has its own test strategy. Pure core uses property testing and snapshot testing. Protocol uses fixture-driven roundtrip tests. Runtime uses hermetic servers with fake PTYs.

6. **WASM as Purity Proof.** Layer 0 crates compile to `wasm32-unknown-unknown` as a CI gate, not a production deployment target. This catches accidental OS dependencies.

7. **Language Bindings as First-Class Citizens.** Python (PyO3), Node (Neon), and C++ (cxx) bindings expose the same ORM API, with snapshot testing support in pytest and vitest.

8. **Observability Built In.** OpenTelemetry tracing from server to client to language bindings, using the `tracing` crate in pure code and OTEL SDK exporters at the runtime boundary.

9. **CRDT-Ready State Model.** The entity graph supports conflict-free replication for distributed/federated scenarios.

### 1.3 Why Not a Direct Port

| Direct Port | TermForge |
|---|---|
| Platform code in the kernel | Platform code quarantined in `mux-os` |
| RB-tree state with manual memory | SlotMap with generation-checked IDs |
| Libevent callback soup | Structured async with tokio tasks |
| CRDTs require invasive surgery | CRDTs compose on top of pure events |
| Testing requires real PTY | Fake PTY backend for deterministic tests |

### 1.4 What TermForge Is Not

- Not a port of tmux C code to Rust.
- Not a wrapper around tmux (like libtmux). It is a standalone server.
- Not limited to tmux's feature set. The architecture supports extensions (CRDT sync, rich TUI, programmatic control) that tmux cannot.

---

## 2. North Star Acceptance Criteria

### 2.1 Compatibility (C)

| ID | Criterion | Verification |
|---|---|---|
| C1 | A stock tmux 3.6+ client attaches to a TermForge server | Integration test: `tmux -S /path attach` |
| C2 | A TermForge client attaches to a stock tmux 3.6+ server | Integration test: `termforge attach -S /path` |
| C3 | Protocol v8 imsg framing roundtrips all 35+ `MsgType` variants | `proptest` with arbitrary payloads |
| C4 | Identify burst (types 100-112) roundtrips byte-exact | Fixture captures from real tmux |
| C5 | SCM_RIGHTS fd passing preserved through sniff proxy | `tmux-sniff` passthrough test |
| C6 | All 200+ tmux commands parse and execute | `tmux-command-audit` tool with coverage report |
| C7 | Format string expansion matches tmux for 200+ variables | `format-audit` parity corpus |
| C8 | Default key bindings match tmux across all 4 key tables | Key table parity tests generated from `key-bindings.c` |

### 2.2 Architectural (A)

| ID | Criterion | Enforcement |
|---|---|---|
| A1 | Layer 0 crates: zero `unsafe`, zero `tokio`, zero `libc` | `#![forbid(unsafe_code)]` in crate root |
| A2 | Layer 0 compiles to `wasm32-unknown-unknown` | CI: `cargo check --target wasm32-unknown-unknown` |
| A3 | `mux-os` is the sole `unsafe` quarantine | CI lint: grep for `unsafe` outside `mux-os` |
| A4 | All state transitions are deterministic | Property: `apply_event(s, e)` is referentially transparent |
| A5 | No panics in protocol decode/encode paths | `#[deny(clippy::unwrap_used)]` in hot crates |
| A6 | Bindings depend only on `mux-api`/`mux-orm` | Cargo dependency check in CI |
| A7 | No `Arc<Mutex<_>>` in the read path | `ArcSwap`-based `StateHandle` |

### 2.3 Product (P)

| ID | Criterion |
|---|---|
| P1 | In-process embedding: create session, split, send keys, read grid -- no process spawning |
| P2 | Python pytest fixture: `server` -> `session` -> `window` -> `pane` in 4 lines |
| P3 | Snapshot test: feed VT100 input, assert grid state via `insta` |
| P4 | CRDT: two servers merge concurrent `new-session` without conflict |
| P5 | TUI client attaches to real tmux server and renders correctly |
| P6 | FakePty scenario replay produces identical grid state to real tmux |

### 2.4 Performance (B)

| ID | Target | Basis |
|---|---|---|
| B1 | VT100 parser, plain ASCII > 300 MB/s | alacritty vte achieves ~500 MB/s |
| B2 | VT100 parser, CSI heavy > 100 MB/s | Parameter parsing overhead |
| B3 | Protocol frame decode > 500 MB/s | 16-byte header + memcpy |
| B4 | Layout resize (20 panes) < 50 us | Tree walk ~60 nodes, no alloc |
| B5 | Graph snapshot < 1 ms | Arc clone + BTreeMap for 50 panes |
| B6 | Option resolve (4-level chain) < 100 ns | 4 BTreeMap lookups |
| B7 | Control notification parse < 200 ns | String split + parse |
| B8 | Format expand (status line) < 50 us | ~10 variable lookups |
| B9 | Config parse (500 lines) < 5 ms | Lexer + parser |

These are conservative targets to be validated after establishing baselines from 3 consecutive median runs. CI regression gate: 130% threshold on nightly only.

### 2.5 Ecosystem (E)

| ID | Criterion | Verification |
|---|---|---|
| E1 | Python bindings: `pip install termforge` | maturin build + PyPI publish |
| E2 | Node bindings: `npm install termforge` | Neon build + npm publish |
| E3 | pytest fixture chain: server -> session -> window -> pane | pytest plugin with hermetic server |
| E4 | vitest fixture chain: same pattern in TypeScript | vitest setup with hermetic server |
| E5 | Snapshot testing from Python/Node against PTY output | `insta`-compatible snapshots via bindings |

---
