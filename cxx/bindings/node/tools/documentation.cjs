"use strict";

const { readFileSync } = require("node:fs");
const { execFileSync } = require("node:child_process");
const { parseSync, Visitor } = require("oxc-parser");

/**
 * Inventory JavaScript and TypeScript function contracts through Oxc's syntax tree.
 * @param {string} filename
 * @returns {{file: string, line: number, symbol: string, documentation: string}[]}
 */
function inventory(filename) {
  const text = readFileSync(filename, "utf8");
  const parsed = parseSync(filename, text);
  if (parsed.errors.length) throw new Error(`cannot parse documentation input: ${filename}`);
  /** @type {{file: string, line: number, symbol: string, documentation: string}[]} */
  const entries = [];
  const methodBodies = new Set();

  /**
   * Associate the immediately preceding JSDoc with one explicit callable.
   * @param {import('oxc-parser').Node} node
   */
  function record(node) {
    if (methodBodies.has(node.start)) return;
    const earlier = parsed.comments.filter(
      /** Retain comments ending before this callable's source range. */
      (comment) => comment.end <= node.start,
    );
    const comment = earlier.at(-1);
    const documentation =
      comment &&
      text.slice(comment.start, comment.start + 3) === "/**" &&
      !text.slice(comment.end, node.start).trim()
        ? text.slice(comment.start, comment.end)
        : "";
    entries.push({
      file: filename,
      line: text.slice(0, node.start).split("\n").length,
      symbol: node.type,
      documentation,
    });
  }

  /**
   * Count a method once while retaining traversal into its nested callbacks.
   * @param {import('oxc-parser').MethodDefinition} node
   */
  function method(node) {
    record(node);
    methodBodies.add(node.value.start);
  }

  new Visitor({
    FunctionDeclaration: record,
    FunctionExpression: record,
    ArrowFunctionExpression: record,
    TSDeclareFunction: record,
    TSEmptyBodyFunctionExpression: record,
    MethodDefinition: method,
    TSAbstractMethodDefinition: method,
    TSMethodSignature: record,
    TSConstructSignatureDeclaration: record,
  }).visit(parsed.program);
  return entries;
}

const supplied = process.argv.slice(2);
const files = supplied.length
  ? supplied
  : execFileSync(
      "rg",
      [
        "--files",
        "bindings/node",
        "-g",
        "!node_modules/**",
        "-g",
        "!dist/**",
        "-g",
        "*.cjs",
        "-g",
        "*.mjs",
        "-g",
        "*.js",
        "-g",
        "*.cts",
        "-g",
        "*.mts",
        "-g",
        "*.ts",
      ],
      { encoding: "utf8" },
    )
      .trim()
      .split("\n");
console.log(JSON.stringify(files.flatMap(inventory)));
