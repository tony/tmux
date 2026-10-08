"use strict";

const { copyFileSync, mkdirSync } = require("node:fs");
const { resolve } = require("node:path");
const { spawnSync } = require("node:child_process");

const project = resolve(__dirname, "../../..");
const preset = process.env.TMUX_CXX_PACKAGE_PRESET ?? "release";
if (!/^[A-Za-z0-9_-]+$/.test(preset)) {
  throw new Error("TMUX_CXX_PACKAGE_PRESET must name one CMake preset");
}
const cmake = process.env.TMUX_CXX_CMAKE ?? resolve(project, ".venv/bin/cmake");
const build = spawnSync(cmake, ["--build", "--preset", preset, "--target", "tmux_cxx_node"], {
  cwd: project,
  stdio: "inherit",
});
if (build.error) throw build.error;
if (build.status !== 0) throw new Error(`release addon build exited with status ${build.status}`);

const source = resolve(project, `_build/${preset}/node/tmux_cxx.node`);
const destination = resolve(__dirname, "../dist/tmux_cxx.node");
mkdirSync(resolve(__dirname, "../dist"), { recursive: true });
copyFileSync(source, destination);
