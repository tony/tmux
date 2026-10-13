import assert from "node:assert/strict";
import { readFileSync, realpathSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import tmux from "tmux-cxx";

/** Verify installed addon ownership before mutating the Python-created persistent session. */
async function exerciseInstalledServer(): Promise<void> {
  const [socket, record, consumer, expectedName, outputToken] = process.argv.slice(2);
  assert(
    socket && record && consumer && expectedName && outputToken,
    "consumer arguments required",
  );
  const require = createRequire(import.meta.url);
  const entry = realpathSync(require.resolve("tmux-cxx"));
  const root = realpathSync(resolve(consumer, "node_modules/tmux-cxx"));
  assert.equal(dirname(entry), root, "Node loaded a facade outside the installed package");
  const addon = realpathSync(
    require.resolve("tmux-cxx").replace(/index\.cjs$/, "dist/tmux_cxx.node"),
  );
  const within = relative(root, addon);
  assert(!isAbsolute(within) && !within.startsWith(".."), "addon is outside the installed package");
  assert(require.cache[addon], "installed addon was not loaded");
  const [sessionText, paneText, owner] = readFileSync(record, "utf8").trim().split(" ");
  assert(sessionText && paneText && owner, "invalid shared session record");
  const session = BigInt(sessionText);
  const pane = BigInt(paneText);
  const connection = new tmux.ServerConnection(socket);
  try {
    const observation = await connection.observeSessions();
    assert.equal(observation.nextEvent.serverInstance, owner);
    assert.equal(observation.sessions.length, 1);
    assert.equal(observation.sessions[0].id, session);
    assert.equal(observation.sessions[0].pane, pane);
    assert.equal(observation.sessions[0].name, expectedName);
    const window = observation.sessions[0].window;
    const previousPanes = await connection.panes(window);
    assert.equal(previousPanes.length, 3);
    const preset = outputToken === "node-current-pty" ? "even-horizontal" : "tiled";
    const arranged = await connection.selectLayout(window, preset);
    assert.equal(arranged.pane, pane);
    const arrangedPanes = await connection.panes(window);
    assert.deepEqual(
      arrangedPanes.map(
        /** Retain logical identities while applying an installed layout operation. */
        (value) => value.id,
      ),
      previousPanes.map(
        /** Copy the three persistent pane identities before rearrangement. */
        (value) => value.id,
      ),
    );
    assert.deepEqual(
      arrangedPanes.map(
        /** Preserve the original active pane across both installed Node runtimes. */
        (value) => value.active,
      ),
      [true, false, false],
    );
    assert.deepEqual(
      arrangedPanes.map(
        /** Compare pane rectangles from the arranged layout snapshot. */
        (value) => [value.top, value.left, value.rows, value.columns],
      ),
      preset === "even-horizontal"
        ? [
            [0, 0, 24, 26],
            [0, 27, 24, 26],
            [0, 54, 24, 26],
          ]
        : [
            [0, 0, 11, 39],
            [0, 40, 11, 40],
            [12, 0, 12, 80],
          ],
    );
    await connection.renameSession(session, "installed-node");
    const buffers = await connection.pasteBuffers();
    assert(
      buffers.some(
        /** Observe the automatic buffer created by the independently installed Python product. */
        (buffer) => buffer.automatic && buffer.name === "buffer0",
      ),
    );
    const automaticBytes = Buffer.from(`printf 'installed-buffer-%s\\n' '${outputToken}'\n`);
    const automatic = await connection.addPasteBuffer(automaticBytes);
    assert(automatic && automatic.automatic);
    assert.equal(automatic.byteCount, automaticBytes.length);
    assert.equal(typeof automatic.revision, "bigint");
    assert.deepEqual(await connection.readPasteBuffer(automatic.name), automaticBytes);
    await connection.pasteTopBuffer(pane);
    assert(
      (
        await connection.waitPaneOutput(pane, Buffer.from(`installed-buffer-${outputToken}`))
      ).includes(Buffer.from(`installed-buffer-${outputToken}`)),
    );
    const output = Buffer.from(`installed-${outputToken}`);
    await connection.sendPaneInput(
      pane,
      Buffer.from(`printf 'installed-%s\\n' '${outputToken}'\n`),
    );
    assert((await connection.waitPaneOutput(pane, output, 500)).includes(output));
    await assert.rejects(connection.renameSession(99999n, "missing"), {
      name: "TmuxError",
      code: "not_found",
      operation: "session.rename",
    });
  } finally {
    connection.close();
  }
  await assert.rejects(connection.sessions(), { name: "TmuxError", code: "closed" });
}

await exerciseInstalledServer();
