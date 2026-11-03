#!/usr/bin/env node
/**
 * Independent JavaScript / TypeScript reference parser for the Perfpal 0.5.0
 * ICC study.
 *
 * It parses the committed example trees (`examples-js/`, `examples-ts/`) and
 * the clean fixtures under `tools/icc/extra/` (`extra.js`, `extra.ts`) with
 * the TypeScript compiler's own AST (`createSourceFile`) and reports the same
 * function-level loop metrics as Perfpal's tree-sitter backends:
 *
 *   loop_count     for / for-in / for-of / while / do-while in the function
 *   loop_body_loc  size of the union of inclusive loop-body line spans
 *   max_loop_depth syntactic loop nesting depth
 *   param_count    node.parameters.length
 *
 * Scope rules (matching Perfpal's JS/TS model):
 *   * every named function-like node is emitted: function declarations,
 *     named function expressions, arrows bound to a variable/property,
 *     methods, getters/setters, and constructors;
 *   * unbound functions are emitted under Perfpal's `<anonymous>` name (this
 *     matches the sampler, which keeps those records); the count is reported;
 *   * declarations without a body (ambient / overload / abstract signatures)
 *     are not real functions and are skipped;
 *   * loops stop at nested function-like nodes (each nested function is its
 *     own record), so an outer record never inherits an inner function's
 *     loops.
 *
 * Compiler resolution
 * -------------------
 * The task provided TypeScript 7.0.2 at
 * `/var/tmp/opencode/icc-js/node_modules/typescript`. That release is the
 * native (Go) compiler and does NOT expose the classic `createSourceFile` AST
 * API. This tool therefore prefers a classic TypeScript compiler and falls
 * back through a small, explicitly documented candidate list:
 *
 *   1. `$ICC_TS_PATH`
 *   2. `/var/tmp/opencode/icc-js/node_modules/typescript`  (7.x; skipped)
 *   3. `/var/tmp/opencode/icc-ts/node_modules/typescript`  (5.9.3; used)
 *
 * Install the classic compiler used by this study with:
 *   mkdir -p /var/tmp/opencode/icc-ts && cd /var/tmp/opencode/icc-ts
 *   npm install --offline typescript@5.9.3
 *
 * If no classic API is available the tool exits 2 instead of silently
 * omitting TypeScript records.
 *
 * Output: `tools/icc/ref/js.json` and `tools/icc/ref/ts.json`, deterministic
 * and sorted by `(path, start_line, start_col, name)`. Paths are relative to
 * the repo root, `start_line` is 1-based and `start_col` is 0-based.
 *
 * Usage:
 *   node tools/icc/js_ref.mjs [--repo ROOT] [--out DIR]
 */

import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const HERE = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_REPO = path.resolve(HERE, "..", "..");

const TS_CANDIDATES = [
  process.env.ICC_TS_PATH,
  "/var/tmp/opencode/icc-js/node_modules/typescript",
  "/var/tmp/opencode/icc-ts/node_modules/typescript",
].filter(Boolean);

function loadTypeScript() {
  const rejected = [];
  for (const candidate of TS_CANDIDATES) {
    try {
      const module = require(candidate);
      if (typeof module.createSourceFile === "function") {
        return { ts: module, source: candidate };
      }
      rejected.push(`${candidate} (no createSourceFile; native compiler?)`);
    } catch (error) {
      rejected.push(`${candidate} (${error.code ?? error.message})`);
    }
  }
  console.error("error: no classic TypeScript compiler with createSourceFile found.");
  for (const line of rejected) console.error(`  tried ${line}`);
  console.error(
    "hint: install one at /var/tmp/opencode/icc-ts with " +
      "`npm install --offline typescript@5.9.3`, or set ICC_TS_PATH.",
  );
  process.exit(2);
}

const { ts } = loadTypeScript();

function parseArgs(argv) {
  let repo = DEFAULT_REPO;
  let out = null;
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--repo") {
      repo = path.resolve(argv[++index]);
    } else if (arg === "--out") {
      out = path.resolve(argv[++index]);
    } else if (arg === "-h" || arg === "--help") {
      console.log(
        "usage: js_ref.mjs [--repo ROOT] [--out DIR]\n" +
          "Parses examples-js/, examples-ts/ and tools/icc/extra/ and writes the JS/TS references.",
      );
      process.exit(0);
    } else {
      console.error(`error: unknown argument ${JSON.stringify(arg)}`);
      process.exit(2);
    }
  }
  return { repo, out: out ?? path.join(repo, "tools", "icc", "ref") };
}

function walkFiles(dir, extensions) {
  if (!fs.existsSync(dir)) return [];
  const result = [];
  const stack = [dir];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
      } else if (extensions.has(path.extname(entry.name))) {
        result.push(full);
      }
    }
  }
  result.sort();
  return result;
}

function scriptKindFor(file) {
  if (file.endsWith(".tsx")) return ts.ScriptKind.TSX;
  if (file.endsWith(".ts")) return ts.ScriptKind.TS;
  return ts.ScriptKind.JS;
}

function isFunctionLike(node) {
  return (
    ts.isFunctionDeclaration(node) ||
    ts.isFunctionExpression(node) ||
    ts.isArrowFunction(node) ||
    ts.isMethodDeclaration(node) ||
    ts.isGetAccessorDeclaration(node) ||
    ts.isSetAccessorDeclaration(node) ||
    ts.isConstructorDeclaration(node)
  );
}

function isLoopNode(node) {
  return (
    ts.isForStatement(node) ||
    ts.isForInStatement(node) ||
    ts.isForOfStatement(node) ||
    ts.isWhileStatement(node) ||
    ts.isDoStatement(node)
  );
}

function nameText(name, sourceFile) {
  if (!name) return null;
  if (ts.isIdentifier(name) || ts.isPrivateIdentifier(name)) return name.text;
  if (ts.isStringLiteral(name) || ts.isNumericLiteral(name)) return name.text;
  return name.getText(sourceFile);
}

/**
 * Resolve a record name. Anonymous functions bound **directly** to a variable
 * or property (the common `export const f = (...) => ...` form) are named
 * after their binding. Anything wrapped (`(arrow) satisfies T`, parentheses)
 * keeps Perfpal's `<anonymous>` name instead of inventing a binding name.
 */
function resolveName(node, sourceFile) {
  if (ts.isConstructorDeclaration(node)) return "constructor";
  const direct = nameText(node.name, sourceFile);
  if (direct !== null) return direct;

  const parent = node.parent;
  if (parent && ts.isVariableDeclaration(parent)) {
    return nameText(parent.name, sourceFile);
  }
  if (
    parent &&
    (ts.isPropertyAssignment(parent) ||
      ts.isPropertyDeclaration(parent) ||
      ts.isPropertySignature(parent))
  ) {
    return nameText(parent.name, sourceFile);
  }
  return "<anonymous>";
}

function hasBody(node) {
  if (ts.isArrowFunction(node)) return true;
  return node.body !== undefined && node.body !== null;
}

function inclusiveLineSpan(node, sourceFile) {
  const start = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
  const endPosition = Math.max(node.getStart(sourceFile), node.getEnd() - 1);
  const end = sourceFile.getLineAndCharacterOfPosition(endPosition);
  return [start.line + 1, end.line + 1];
}

function loopMetrics(fnNode, sourceFile) {
  const spans = [];
  let maxDepth = 0;

  function visit(node, depth) {
    if (node !== fnNode && isFunctionLike(node)) return;
    if (isLoopNode(node)) {
      const nextDepth = depth + 1;
      if (nextDepth > maxDepth) maxDepth = nextDepth;
      if (node.statement) {
        spans.push(inclusiveLineSpan(node.statement, sourceFile));
      }
      ts.forEachChild(node, (child) => visit(child, nextDepth));
      return;
    }
    ts.forEachChild(node, (child) => visit(child, depth));
  }

  if (fnNode.body) visit(fnNode.body, 0);

  const lines = new Set();
  for (const [start, end] of spans) {
    for (let line = start; line <= end; line += 1) lines.add(line);
  }
  return {
    loop_count: spans.length,
    loop_body_loc: lines.size,
    max_loop_depth: maxDepth,
  };
}

function collectFunctions(sourceFile, relPath, counters) {
  const records = [];

  function visit(node) {
    if (isFunctionLike(node)) {
      if (!hasBody(node)) {
        counters.noBody += 1;
      } else {
        const name = resolveName(node, sourceFile);
        if (name === "<anonymous>") counters.anonymous += 1;
        const start = sourceFile.getLineAndCharacterOfPosition(
          node.getStart(sourceFile),
        );
        records.push({
          path: relPath,
          name,
          start_line: start.line + 1,
          start_col: start.character,
          param_count: node.parameters ? node.parameters.length : 0,
          ...loopMetrics(node, sourceFile),
        });
      }
    }
    ts.forEachChild(node, visit);
  }

  visit(sourceFile);
  return records;
}

function addDiagnostics(sourceFile, relPath, diagnostics, sink) {
  for (const diagnostic of diagnostics) {
    const position = diagnostic.start ?? 0;
    const { line, character } = sourceFile.getLineAndCharacterOfPosition(position);
    sink.push(
      `${relPath}:${line + 1}:${character + 1}: ${ts.flattenDiagnosticMessageText(
        diagnostic.messageText,
        " ",
      )}`,
    );
  }
}

function parseVariant(relPath, source, kind) {
  const sourceFile = ts.createSourceFile(
    relPath,
    source,
    ts.ScriptTarget.Latest,
    /* setParentNodes */ true,
    kind,
  );
  const diagnostics = sourceFile.parseDiagnostics ?? [];
  const counters = { anonymous: 0, noBody: 0 };
  const records =
    diagnostics.length === 0 ? collectFunctions(sourceFile, relPath, counters) : [];
  return { sourceFile, diagnostics, records, counters };
}

function functionsForFile(repo, file, counters) {
  const relPath = path.relative(repo, file).split(path.sep).join("/");
  const source = fs.readFileSync(file, "utf8");
  const primaryKind = scriptKindFor(file);
  const variants = [parseVariant(relPath, source, primaryKind)];

  // TSX generic-arrow disambiguation: TypeScript's TSX grammar reads a bare
  // `<T>(...) =>` as the start of a JSX element. The corpus deliberately ships
  // both that form and genuine JSX, so a failing `.tsx` is retried as plain
  // TypeScript. Whichever grammar parses cleanly supplies the records.
  if (
    primaryKind === ts.ScriptKind.TSX &&
    variants[0].diagnostics.length > 0
  ) {
    variants.push(parseVariant(relPath, source, ts.ScriptKind.TS));
  }

  const clean = variants.filter((variant) => variant.diagnostics.length === 0);
  if (clean.length === 0) {
    for (const variant of variants) {
      addDiagnostics(variant.sourceFile, relPath, variant.diagnostics, counters.parseErrors);
    }
    return [];
  }

  // Prefer the primary parse when both grammars succeed, so records are not
  // duplicated. Counters likewise come only from the chosen parse.
  const chosen = clean[0];
  counters.anonymous += chosen.counters.anonymous;
  counters.noBody += chosen.counters.noBody;
  return chosen.records;
}

function build(repo, trees, language, extensions, counters) {
  const files = [];
  for (const tree of trees) {
    files.push(...walkFiles(path.join(repo, tree), extensions));
  }
  files.sort();
  const functions = [];
  for (const file of files) {
    functions.push(...functionsForFile(repo, file, counters));
  }
  functions.sort((a, b) => {
    if (a.path !== b.path) return a.path < b.path ? -1 : 1;
    if (a.start_line !== b.start_line) return a.start_line - b.start_line;
    if (a.start_col !== b.start_col) return a.start_col - b.start_col;
    if (a.name !== b.name) return a.name < b.name ? -1 : 1;
    return 0;
  });
  return { language, functions };
}

function writeJson(outDir, fileName, doc) {
  fs.mkdirSync(outDir, { recursive: true });
  const target = path.join(outDir, fileName);
  fs.writeFileSync(target, `${JSON.stringify(doc, null, 2)}\n`, "utf8");
  return target;
}

function main() {
  const { repo, out } = parseArgs(process.argv.slice(2));
  const counters = { anonymous: 0, noBody: 0, parseErrors: [] };

  const js = build(
    repo,
    ["examples-js", "tools/icc/extra"],
    "javascript",
    new Set([".js", ".cjs", ".mjs"]),
    counters,
  );
  const tsDoc = build(
    repo,
    ["examples-ts", "tools/icc/extra"],
    "typescript",
    new Set([".ts", ".tsx"]),
    counters,
  );

  const jsPath = writeJson(out, "js.json", js);
  const tsPath = writeJson(out, "ts.json", tsDoc);

  console.log(
    `javascript reference: ${js.functions.length} functions -> ${jsPath}`,
  );
  console.log(
    `typescript reference: ${tsDoc.functions.length} functions -> ${tsPath}`,
  );
  console.log(
    `named <anonymous>: ${counters.anonymous}; skipped bodyless ` +
      `(ambient/overload/abstract): ${counters.noBody}`,
  );

  if (counters.parseErrors.length > 0) {
    for (const error of counters.parseErrors) console.error(`parse error: ${error}`);
    process.exitCode = 1;
  }
}

main();
