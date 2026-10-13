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
    await connection.renameSession(session, "installed-node");
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
