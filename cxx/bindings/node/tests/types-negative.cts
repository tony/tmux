import { ServerConnection } from "tmux-cxx";

/** Require argument, return-assignment, and missing-export diagnostics. */
async function consumer(client: ServerConnection): Promise<void> {
  await client.renameSession(1, "bad-number-id");
  await client.selectLayout(1n, "even_horizontal");
  await client.setWindowOption(1, "mode-keys", "vi");
  await client.windowOptions("bad-window");
  await client.enterCopyMode(1);
  await client.moveCopyCursor(1n, "line_start", 2);
  await client.moveCopyCursorTo(1n, "right");
  await client.scrollCopyMode(1n, "page-up");
  await client.copySelection(1n, "cancel");
  const wrongCopy: string = await client.copyMode(1n);
  const stream = await client.subscribeSessions();
  await stream.next({ timeoutMs: "bad-timeout" });
  const wrong: string = await client.sessions();
  client.absentExport();
  console.log(wrong);
  console.log(wrongCopy);
}
void consumer;
