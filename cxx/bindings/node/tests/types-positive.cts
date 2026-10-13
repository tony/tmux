import {
  ServerConnection,
  type PaneSnapshot,
  type PaneTextSnapshot,
  type PasteBufferSnapshot,
  type SessionEventStream,
  type SessionEventStreamItem,
  type SessionSnapshot,
  type StockConnectionCompatibility,
  type WindowSnapshot,
} from "tmux-cxx";

/** Exercise promise results, byte arrays, bigint identities, and session values. */
async function consumer(client: ServerConnection): Promise<SessionSnapshot> {
  const session: SessionSnapshot = await client.createSession("typed", "/bin/sh");
  const pane: bigint = session.pane;
  await client.sendPaneInput(pane, Buffer.from("printf ok\\n\n"));
  const output: Buffer = await client.readPaneOutput(pane);
  const text: PaneTextSnapshot = await client.readPaneText(pane);
  const row: string = text.lines[0];
  const sessions: SessionSnapshot[] = await client.sessions();
  const stream: SessionEventStream = await client.subscribeSessions();
  const event: SessionEventStreamItem | null = await stream.next({ timeoutMs: 500 });
  for await (const observed of stream.events()) {
    const item: SessionEventStreamItem = observed;
    void item;
    break;
  }
  stream.close();
  const compatibility: Promise<StockConnectionCompatibility> = client.stockClientCompatibility(1n);
  await client.renameSession(session.id, row + output.toString("utf8"));
  const renamedWindow: WindowSnapshot = await client.renameWindow(session.window, "typed-window");
  void renamedWindow;
  const arranged: WindowSnapshot = await client.selectLayout(
    session.window,
    "main-horizontal-mirrored",
  );
  void arranged;
  const nextWindow: SessionSnapshot = await client.nextWindow(session.id);
  const previousWindow: SessionSnapshot = await client.previousWindow(session.id);
  const resized: PaneSnapshot = await client.resizePane(pane, "right");
  const automatic: PasteBufferSnapshot | null = await client.addPasteBuffer(Buffer.from("typed"));
  const buffers: PasteBufferSnapshot[] = await client.pasteBuffers();
  await client.pasteTopBuffer(pane);
  if (automatic) {
    const revision: bigint = automatic.revision;
    const byteCount: number = automatic.byteCount;
    const isAutomatic: boolean = automatic.automatic;
    void revision;
    void byteCount;
    void isAutomatic;
  }
  void buffers;
  void resized;
  void nextWindow;
  void previousWindow;
  void event;
  void compatibility;
  return sessions[0];
}
void consumer;
