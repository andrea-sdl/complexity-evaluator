import assert from "node:assert/strict";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import test from "node:test";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanupEvalRoot } from "../scripts/eval-lib.mjs";

const evalRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(evalRoot, "../..");
const releaseWorkflow = resolve(
  projectRoot,
  ".github/workflows/complexity-release.yml",
);

function read(relativePath) {
  return readFileSync(resolve(evalRoot, relativePath), "utf8");
}

test("the eval measures Codex refactoring, not checker outcome fixtures", () => {
  const config = read("promptfooconfig.yaml");
  const cases = read("cases.yaml");
  const packageJson = JSON.parse(read("package.json"));

  assert.match(config, /openai:codex-sdk/);
  assert.match(config, /sandbox_mode: workspace-write/);
  assert.match(config, /file:\/\/assertions\/refactor-result\.mjs/);
  assert.doesNotMatch(config, /exec:\s*node provider\.mjs/);
  assert.doesNotMatch(cases, /\.py\b/);
  for (const extension of [".js", ".ts", ".php", ".rs"]) {
    assert.match(cases, new RegExp(`\\${extension}\\b`));
  }
  assert.equal(packageJson.scripts.eval, "node scripts/run-codex-eval.mjs");
});

test("source release CI validates the manual eval without calling Codex", {
  skip: !existsSync(releaseWorkflow),
}, () => {
  const workflow = readFileSync(releaseWorkflow, "utf8");

  assert.match(workflow, /npm run test --prefix agent\/eval/);
  assert.match(workflow, /npm run validate --prefix agent\/eval/);
  assert.doesNotMatch(workflow, /npm run eval --prefix agent\/eval/);
  assert.doesNotMatch(workflow, /working-directory: complexity|complexity\/dist/);
});

test("kept failed eval workspaces do not keep copied Codex auth", () => {
  const tempRoot = mkdtempSync(join(tmpdir(), "complexity-cleanup-test-"));
  const auth = resolve(tempRoot, "codex-home/auth.json");
  const source = resolve(tempRoot, "workspaces/case/subject.js");
  mkdirSync(dirname(auth), { recursive: true });
  mkdirSync(dirname(source), { recursive: true });
  writeFileSync(auth, "secret");
  writeFileSync(source, "source");

  try {
    assert.equal(cleanupEvalRoot(tempRoot, false, true), true);
    assert.equal(existsSync(auth), false);
    assert.equal(existsSync(source), true);
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
});
