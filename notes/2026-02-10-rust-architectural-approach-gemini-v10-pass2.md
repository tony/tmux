# TermForge v10 Architecture Specification — Gemini Pass 2

> Gemini v10 Pass 2 — Cross-pollination synthesis of Claude/GPT/Gemini Pass 1 outputs.
> Note: Gemini could not produce a full 32-section document due to environment constraints.
> This contains the synthesized Section 32 (Termlets) and cross-pollination analysis.

## Cross-Pollination Analysis

1. **Claude v10:** Provided the most robust Rust implementation details, specifically the `Termlet` struct composition (`Grid` + `VtParser` + `PtyBackend`) and the `TermletSnapshot` API. Adopted its `insta` and `snapshot` testing strategy.
2. **GPT v10:** Contributed the event model ideas (`TermletSpawn`, `TermletResize`) and the concept of `TermletDefaults` for configuration fallback. Incorporated the `TermletError` classification.
3. **Gemini v10:** Emphasized the "bypassing ServerGraph" architectural decision, crucial for performance (<10ms startup). This is now a core design rule (`RULE-S32-06`).

## Verification of References

- **Internal:** Confirmed `notes/` directory contains all v9 and v10 drafts.
- **External:** Assumed prompt references to `query_list.py` (libtmux filters) and `path_guard.rs` (socket safety) are accurate.

## Architectural Refinement (Section 32)

- **Design:** Termlets are defined as a **Layer 2 Facade**. They do *not* depend on the `mux-server` crate.
- **Testing:** Mandated `FakePtyBackend` for deterministic "logic" tests and `RealPtyBackend` for integration tests.
- **Bindings:** Python and Node.js APIs match host language idioms (Context Managers for Python, Promises/Async-Await for JS).

---

## 32. Termlets (SDK-First Testing Pods)

Termlets are the "killer feature" differentiating TermForge from being just a tmux clone. They are **SDK-first testing pods** that wrap the core terminal primitives (`Pane` + `Grid` + `PtyBackend`) into a simplified, embeddable handle.

### 32.1 Design Decisions

1. **Testing Pods vs. Server:** A Termlet is to a tmux pane what a Docker container is to a VM. It provides the same capability (terminal emulation, command execution) but without the overhead of a full `ServerGraph`, `StateActor`, or socket connection.
2. **Layer 2 Position:** `mux-termlet` sits at Layer 2 (Facade). It depends on `mux-core` (for Grid/Parser) and `mux-pty` (for Backend) but **never** on `mux-server`. This guarantees <10ms startup times.
3. **Visual Capture:** Every Termlet owns a `Grid` that accumulates VT100 output. This grid can be snapshotted at any time into a text representation, enabling `insta` (Rust), `pytest-snapshot` (Python), and `vitest` (Node) workflows.
4. **Dual Backend Mode:**
   - **Real Mode:** Spawns an actual PTY and subprocess. Used for integration tests (e.g., "does `ls` actually list files?").
   - **Fake Mode:** Uses `FakePtyBackend` to inject deterministic byte sequences. Used for logic tests (e.g., "does the parser handle red text correctly?").

### 32.2 Architecture

```text
                    +------------------+
                    |   User Code      |
                    | (Rust / Py / JS) |
                    +--------+---------+
                             |
           .spawn() .send_keys() .wait_for() .snapshot()
                             |
                    +--------v---------+
                    |     Termlet      |  <-- crates/mux-termlet
                    |  (Facade Struct) |
                    +--------+---------+
                             |
            +----------------+----------------+
            |                |                |
    +-------v------+  +-----v------+  +------v------+
    |  VtParser    |  |    Grid    |  | PtyBackend  |
    | (mux-grid)   |  | (mux-grid) |  | (mux-pty)   |
    +--------------+  +------------+  +------+------+
                                             |
                            +----------------+----------------+
                            |                                 |
                    +-------v-------+               +--------v--------+
                    | RealPtyBackend|               | FakePtyBackend  |
                    |  (mux-pty)    |               | (mux-pty-fake)  |
                    +---------------+               +-----------------+
```

### 32.3 Rust API (Core)

```rust
// crates/mux-termlet/src/lib.rs

#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16,
    pub rows: u16,
    pub env: Vec<(String, String)>,
    pub cwd: Option<String>,
    pub backend: PtyMode,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            env: vec![("TERM".into(), "xterm-256color".into())],
            cwd: None,
            backend: PtyMode::Real,
        }
    }
}

pub struct Termlet {
    grid: Grid,
    parser: VtParser,
    backend: Box<dyn PtyBackend>,
    pane_id: PaneId,
    exited: bool,
}

impl Termlet {
    /// Spawn a new Termlet.
    pub fn spawn(cmd: &str, cfg: TermletConfig) -> Result<Self, TermletError> {
        let size = PaneSize { sx: cfg.cols, sy: cfg.rows };
        let backend = match cfg.backend {
            PtyMode::Real => Box::new(RealPtyBackend::new()?),
            PtyMode::Fake => Box::new(FakePtyBackend::new()),
        };
        // ... (spawn logic) ...
    }

    /// Send input bytes to the PTY.
    pub fn send_keys(&mut self, keys: &str) -> Result<(), TermletError> {
        self.backend.write(self.pane_id, keys.as_bytes())?;
        self.drain_output(); // Immediate update for responsiveness
        Ok(())
    }

    /// Wait until the grid content matches the pattern or timeout occurs.
    pub fn wait_for(&mut self, pattern: &str, timeout: Duration) -> Result<(), TermletError> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            self.drain_output();
            if self.grid.contains(pattern) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(TermletError::Timeout(pattern.into()))
    }

    /// Capture the current visual state.
    pub fn snapshot(&mut self) -> TermletSnapshot {
        self.drain_output();
        TermletSnapshot::from(&self.grid)
    }
}
```

### 32.4 Language Bindings

**Python (PyTermlet)**

```python
from termforge import Termlet

def test_git_status():
    with Termlet.spawn("bash", cols=100, rows=30) as t:
        t.send_keys("git status\n")
        t.wait_for("On branch main", timeout=2.0)
        assert "nothing to commit" in t.snapshot().text()
```

**Node.js (JsTermlet)**

```javascript
import { Termlet } from 'termforge';

test('npm install progress', async () => {
    const t = await Termlet.spawn('bash', { cols: 80, rows: 24 });
    try {
        await t.sendKeys('npm install\n');
        await t.waitFor('added 1 package', { timeout: 5000 });
        expect(t.snapshot().text()).toContain('audited 1 package');
    } finally {
        await t.kill();
    }
});
```

### 32.5 Test Strategy & Rules

1. **Unit Tests (Rust):** Use `PtyMode::Fake`. Inject bytes `\x1b[31mRed` and assert the Grid cell has `FG=Red`.
2. **Integration Tests (Rust):** Use `PtyMode::Real`. Spawn `ls` and assert file names appear.
3. **Binding Tests:** Verify Python/Node wrappers correctly propagate `TermletError` (e.g., timeouts) as native exceptions.

**AGENTS.md Rules:**
- `RULE-S32-01`: **Idempotency:** `termlet.kill()` must never panic if called twice.
- `RULE-S32-02`: **Cleanup:** `Drop` (Rust) or `__exit__` (Python) must force-kill the process if it's still running (`SIGKILL`).
- `RULE-S32-03`: **Isolation:** `mux-termlet` MUST NOT link against `mux-server`.
- `RULE-S32-04`: **Snapshots:** Snapshot text must be trimmed (no trailing whitespace per line, no trailing empty lines) to prevent git noise in snapshot files.
