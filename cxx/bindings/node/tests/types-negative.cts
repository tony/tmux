import { ServerConnection } from "tmux-cxx";

/** Require argument, return-assignment, and missing-export diagnostics. */
async function consumer(client: ServerConnection): Promise<void> {
  await client.renameSession(1, "bad-number-id");
  const stream = await client.subscribeSessions();
  await stream.next({ timeoutMs: "bad-timeout" });
  const wrong: string = await client.sessions();
  client.absentExport();
  console.log(wrong);
}
void consumer;
