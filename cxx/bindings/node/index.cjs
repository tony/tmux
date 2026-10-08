"use strict";

const { resolve } = require("node:path");
const { installSessionEventStream } = require("./session_event_stream.cjs");

/** @type {Parameters<typeof installSessionEventStream>[0]} */
// oxlint-disable-next-line typescript/no-unsafe-assignment -- Node has no typed native loader; generated declarations and real-addon tests verify the boundary.
const native = require(
  process.env.TMUX_CXX_ADDON ? resolve(process.env.TMUX_CXX_ADDON) : "./dist/tmux_cxx.node",
);

installSessionEventStream(native);
module.exports = native;
