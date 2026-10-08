import {
  ServerConnection,
  type PaneTextSnapshot,
  type SessionEventStream,
  type SessionEventStreamItem,
  type SessionSnapshot,
  type StockConnectionCompatibility,
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
  void event;
  void compatibility;
  return sessions[0];
}
void consumer;
