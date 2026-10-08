import tmux from "tmux-cxx";

/** Observe Python's mutation, rename the same session, and preserve it after disposal. */
async function inspectSharedServer(): Promise<void> {
  const socket = process.argv[2];
  if (!socket) throw new Error("an owned socket is required");
  const connection = new tmux.ServerConnection(socket);
  try {
    const sessions = await connection.sessions();
    if (sessions.length !== 1 || sessions[0].name !== "python") {
      throw new Error("Node did not observe Python's session mutation");
    }
    await connection.renameSession(sessions[0].id, "node");
  } finally {
    connection.close();
  }
}

await inspectSharedServer();
