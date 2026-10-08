import { afterEach, expect, test } from "vitest";
import tmux from "tmux-cxx";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

let process: ChildProcess | undefined;
let directory: string | undefined;
const { ServerConnection } = tmux;

/** Stop the owned daemon and remove only its temporary socket directory. */
async function disposeServer(): Promise<void> {
  if (process && process.exitCode === null) {
    const exited = new Promise<number | null>(
      /** Subscribe before signalling so a fast exit cannot be missed. */
      (resolve) => {
        process?.once("exit", resolve);
      },
    );
    process.kill("SIGTERM");
    const code = await exited;
    if (code !== 0) throw new Error(`owned daemon exited with status ${code}`);
  }
  if (directory) rmSync(directory, { recursive: true });
}
afterEach(disposeServer);

/** Observe explicit readiness and reject startup failure without polling. */
function awaitServerReady(child: ChildProcess): Promise<void> {
  return new Promise(
    /** Install readiness, failure, and exit handlers before awaiting startup. */
    (resolve, reject) => {
      const ready = child.stdio[3];
      if (!ready) {
        reject(new Error("owned daemon has no readiness pipe"));
        return;
      }
      /** Remove startup subscriptions once readiness or failure is known. */
      function cleanup(): void {
        child.off("exit", exited);
        child.off("error", failed);
        ready?.off("data", signalled);
      }
      /** Reject a child that exits before its readiness byte arrives. */
      function exited(code: number | null): void {
        cleanup();
        reject(new Error(`owned daemon exited before readiness: ${code}`));
      }
      /** Preserve a process-start failure and detach startup subscriptions. */
      function failed(error: Error): void {
        cleanup();
        reject(error);
      }
      /** Validate the daemon's readiness byte before admitting requests. */
      function signalled(data: Buffer): void {
        cleanup();
        if (data.equals(Buffer.from("R"))) resolve();
        else reject(new Error("invalid daemon readiness byte"));
      }
      child.once("exit", exited);
      child.once("error", failed);
      ready.once("data", signalled);
    },
  );
}

/** Pass fixture runtime settings without injecting ASan into the daemon's shell programs. */
function serverEnvironment(): Record<string, string> {
  const allowed = new Set([
    "PATH",
    "TMPDIR",
    "LANG",
    "LC_ALL",
    "TERM",
    "LD_LIBRARY_PATH",
    "ASAN_SYMBOLIZER_PATH",
    "ASAN_OPTIONS",
    "UBSAN_OPTIONS",
    "LSAN_OPTIONS",
  ]);
  const environment: Record<string, string> = {};
  for (const [name, value] of Object.entries(globalThis.process.env)) {
    if (value !== undefined && (allowed.has(name) || name.startsWith("TMUX_CXX_"))) {
      environment[name] = value;
    }
  }
  return environment;
}

/** Exercise live PTY bytes, structured errors, and persistent session ownership. */
async function serverOperations(): Promise<void> {
  directory = mkdtempSync(join(tmpdir(), "tmux-cxx-node-"));
  const socket = join(directory, "server");
  const daemon = globalThis.process.env.TMUX_CXX_DAEMON;
  if (!daemon) throw new Error("TMUX_CXX_DAEMON is required");
  process = spawn(daemon, ["serve", "--socket", socket, "--ready-fd", "3"], {
    stdio: ["ignore", "pipe", "pipe", "pipe"],
    env: serverEnvironment(),
  });
  await awaitServerReady(process);
  const client = new ServerConnection(socket);
  const session = await client.createSession("node", "/bin/sh");
  expect(typeof session.id).toBe("bigint");
  const observation = await client.observeSessions();
  expect(
    observation.sessions.map(
      /** Compare copied session identities at the observation boundary. */
      (value) => value.id,
    ),
  ).toEqual([session.id]);
  expect(observation.nextEvent.serverInstance).toMatch(/^[0-9a-f]{32}$/);
  expect(observation.nextEvent.nextSequence).toBeGreaterThanOrEqual(1n);
  await client.renameSession(session.id, "renamed");
  const changes = await client.readSessionEvents(observation.nextEvent);
  expect(changes.nextCursor.nextSequence).toBe(observation.nextEvent.nextSequence + 1n);
  expect(changes.records).toHaveLength(1);
  expect(changes.records[0].sequence).toBe(observation.nextEvent.nextSequence);
  expect(changes.records[0].event).toMatchObject({
    kind: "session_renamed",
    session: session.id,
    name: "renamed",
  });
  const pendingChanges = client.waitSessionEvents(changes.nextCursor, 500);
  await client.renameSession(session.id, "renamed-after-wait");
  expect((await pendingChanges).records[0].event).toMatchObject({
    kind: "session_renamed",
    session: session.id,
    name: "renamed-after-wait",
  });
  const stream = await client.subscribeSessions();
  expect(
    stream.initial.sessions.map(
      /** Compare stream-baseline session identities. */
      (value) => value.id,
    ),
  ).toEqual([session.id]);
  await expect(stream.next({ timeoutMs: 0 })).rejects.toThrow(
    new RangeError("timeoutMs must be an integer from 1 through 750"),
  );
  await expect(stream.next({ timeoutMs: 751 })).rejects.toThrow(
    new RangeError("timeoutMs must be an integer from 1 through 750"),
  );
  const nextEvent = stream.next({ timeoutMs: 500 });
  await expect(stream.next({ timeoutMs: 500 })).rejects.toThrow(
    "SessionEventStream already has a pending read",
  );
  await client.renameSession(session.id, "renamed-through-stream");
  expect(await nextEvent).toMatchObject({
    kind: "event",
    event: { kind: "session_renamed", name: "renamed-through-stream" },
  });
  const iterator = stream.events()[Symbol.asyncIterator]();
  const iterated = iterator.next();
  await client.renameSession(session.id, "renamed-through-iterator");
  expect(await iterated).toMatchObject({
    done: false,
    value: {
      kind: "event",
      event: { kind: "session_renamed", name: "renamed-through-iterator" },
    },
  });
  stream.close();
  await expect(stream.next({ timeoutMs: 500 })).rejects.toMatchObject({ code: "closed" });
  const cancellable = await client.subscribeSessions();
  const cancellation = new AbortController();
  const cancellationReason = new Error("cancel session observation");
  const cancelled = cancellable.next({ timeoutMs: 500, signal: cancellation.signal });
  cancellation.abort(cancellationReason);
  await expect(cancelled).rejects.toBe(cancellationReason);
  expect((await client.sessions())[0].name).toBe("renamed-through-iterator");
  await client.sendPaneInput(session.pane, Buffer.from("printf 'node-%s\\n' 'pty-token'\n"));
  expect(
    (await client.waitPaneOutput(session.pane, Buffer.from("node-pty-token"), 500)).includes(
      Buffer.from("node-pty-token"),
    ),
  ).toBe(true);
  await expect(client.renameSession(99999n, "stale")).rejects.toMatchObject({
    name: "TmuxError",
    code: "not_found",
    operation: "session.rename",
  });
  await expect(client.renameSession(0n, "missing")).rejects.toMatchObject({
    name: "TmuxError",
    code: "not_found",
    operation: "session.rename",
  });
  await expect(client.stockClientCompatibility(99999n)).rejects.toMatchObject({
    name: "TmuxError",
    code: "not_found",
    operation: "client.read_stock_compatibility",
  });
  const binary = await client.createSession("bytes", "printf '\\377\\000binary-end'; exec cat");
  const output = await client.waitPaneOutput(binary.pane, Buffer.from("binary-end"), 500);
  expect(Buffer.isBuffer(output)).toBe(true);
  expect(output.includes(Buffer.from([0xff, 0x00, ...Buffer.from("binary-end")]))).toBe(true);
  const text = await client.readPaneText(binary.pane);
  expect(text.pane).toBe(binary.pane);
  expect(text.serverInstance).toMatch(/^[0-9a-f]{32}$/);
  expect(text.lines[0]).toContain("binary-end");
  expect(text.complete).toBe(true);
  expect(typeof text.revision).toBe("bigint");
  const splitSession = await client.createSession("split", "/bin/cat");
  const split = await client.splitPane(
    splitSession.pane,
    "left_right",
    "printf 'right-ready\\n'; exec cat",
  );
  expect(
    (await client.panes(splitSession.window)).map(
      /** Materialize exact pane geometry independently of the live window tree. */
      (pane) => [pane.id, pane.left, pane.columns, pane.active],
    ),
  ).toEqual([
    [splitSession.pane, 0, 40, false],
    [split.id, 41, 39, true],
  ]);
  expect(
    (await client.waitPaneOutput(split.id, Buffer.from("right-ready"), 500)).includes(
      Buffer.from("right-ready"),
    ),
  ).toBe(true);
  expect(await client.selectPane(splitSession.pane)).toMatchObject({
    id: splitSession.pane,
    active: true,
  });
  expect(
    (await client.panes(splitSession.window)).map(
      /** Observe selection through copied pane values. */
      (pane) => pane.active,
    ),
  ).toEqual([true, false]);
  const pasteBytes = Buffer.from([0x62, 0x00, 0xff]);
  await client.setPasteBuffer("node-binary", pasteBytes);
  expect(await client.readPasteBuffer("node-binary")).toEqual(pasteBytes);
  const pasteCommand = Buffer.from("printf 'node-paste-%s\\n' 'token'\n");
  await client.setPasteBuffer("node-command", pasteCommand);
  await client.pasteBufferIntoPane("node-command", session.pane);
  expect(
    (await client.waitPaneOutput(session.pane, Buffer.from("node-paste-token"), 500)).includes(
      Buffer.from("node-paste-token"),
    ),
  ).toBe(true);
  await client.deletePasteBuffer("node-binary");
  await expect(client.readPasteBuffer("node-binary")).rejects.toMatchObject({
    code: "not_found",
    operation: "paste_buffer.read",
  });
  const shared = await client.createSession("shared", "/bin/cat");
  await expect(client.linkWindow(session.window, shared.id, 2147483648)).rejects.toMatchObject({
    code: "invalid_argument",
    operation: "window.link",
  });
  await expect(
    client.waitPaneOutput(session.pane, Buffer.from("missing"), 751),
  ).rejects.toMatchObject({
    code: "invalid_argument",
    operation: "pane.wait_output",
  });
  const link = await client.linkWindow(session.window, shared.id, 7);
  expect(link).toMatchObject({ session: shared.id, window: session.window, index: 7 });
  expect(
    (await client.windowLinks(shared.id)).map(
      /** Compare ordered indices owned by the destination session. */
      (membership) => membership.index,
    ),
  ).toEqual([0, 7]);
  client.close();
  const observer = new ServerConnection(socket);
  expect((await observer.sessions())[0].id).toBe(session.id);
  await observer.killSession(session.id);
  expect((await observer.window(session.window)).pane).toBe(session.pane);
  await observer.unlinkWindow(shared.link);
  expect(
    (await observer.sessions()).find(
      /** Select the destination session retaining the shared pane. */
      (value) => value.id === shared.id,
    )?.pane,
  ).toBe(session.pane);
  await observer.unlinkWindow(link.id);
  await expect(observer.window(session.window)).rejects.toMatchObject({ code: "not_found" });
  await expect(observer.renameSession(session.id, "deleted")).rejects.toThrow(
    "session no longer exists",
  );
  observer.close();
}
test("Node controls real PTY state and disposing preserves the session", serverOperations);
