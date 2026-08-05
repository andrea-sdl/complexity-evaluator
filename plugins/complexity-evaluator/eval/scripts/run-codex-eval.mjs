import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";

import {
  analyzeFunction,
  caseById,
  cases,
  cleanupEvalRoot,
  evalRoot,
  meetsTargets,
  prepareCodexHome,
  prepareWorkspace,
  resolveBinary,
  writeBaseline,
} from "./eval-lib.mjs";

function selectedCases(arguments_) {
  if (arguments_.length === 0) {
    return cases;
  }
  return arguments_.map(caseById);
}

function run(executable, args, options = {}) {
  const result = spawnSync(executable, args, {
    cwd: evalRoot,
    env: options.env ?? process.env,
    stdio: "inherit",
    timeout: options.timeout ?? 60_000,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${args[0] ?? executable} exited ${result.status}`);
  }
}

function promptfooCommand() {
  return resolve(evalRoot, "node_modules/promptfoo/dist/src/entrypoint.js");
}

function runPreflight(binary) {
  const env = { ...process.env, COMPLEXITY_BIN: binary };
  run(process.execPath, ["--test"], { env });
  run(process.execPath, [
    promptfooCommand(),
    "validate",
    "config",
    "-c",
    "promptfooconfig.yaml",
    "--env-file",
    "promptfoo.env",
  ], { env });
}

function prepareCases(selected, binary, workspaces, baselines) {
  for (const testCase of selected) {
    const workspace = resolve(workspaces, testCase.id);
    const baseline = prepareWorkspace(workspace, binary, testCase);
    if (baseline.metrics.score <= 0 || meetsTargets(baseline.metrics)) {
      throw new Error(`${testCase.id} does not start with a useful finding`);
    }
    writeBaseline(resolve(baselines, `${testCase.id}.json`), baseline);
  }
}

function filterArguments(selected) {
  if (selected.length === cases.length) {
    return [];
  }
  const names = selected.map((testCase) => testCase.id).join("|");
  return ["--filter-pattern", `^\\[(${names})\\]`];
}

function evalEnvironment(tempRoot, binary, workspaces, baselines, codexHome) {
  const localBin = resolve(evalRoot, "node_modules/.bin");
  return {
    ...process.env,
    PATH: `${localBin}${delimiter}${process.env.PATH}`,
    COMPLEXITY_BIN: binary,
    COMPLEXITY_EVAL_WORKSPACES: workspaces,
    COMPLEXITY_EVAL_BASELINES: baselines,
    COMPLEXITY_EVAL_CODEX_HOME: codexHome,
    PROMPTFOO_CACHE_ENABLED: "false",
    PROMPTFOO_CACHE_PATH: resolve(tempRoot, "promptfoo-cache"),
    PROMPTFOO_CONFIG_DIR: resolve(tempRoot, "promptfoo-state"),
    PROMPTFOO_DISABLE_TELEMETRY: "1",
    PROMPTFOO_DISABLE_UPDATE: "1",
    PROMPTFOO_PASS_RATE_THRESHOLD: "100",
  };
}

function runPromptfoo(selected, environment) {
  const args = [
    promptfooCommand(),
    "eval",
    "-c",
    "promptfooconfig.yaml",
    "--no-cache",
    "--no-share",
    "--no-progress-bar",
    ...filterArguments(selected),
  ];
  run(process.execPath, args, { env: environment, timeout: 20 * 60_000 });
}

function printResults(selected, binary, workspaces, baselines) {
  for (const testCase of selected) {
    const baseline = JSON.parse(
      readFileSync(resolve(baselines, `${testCase.id}.json`), "utf8"),
    );
    const after = analyzeFunction(
      binary,
      resolve(workspaces, testCase.id),
      testCase.source,
      testCase.functionName,
    );
    console.log(
      `PASS ${testCase.id}: ${testCase.functionName} score ${baseline.metrics.score} -> ${after.score}`,
    );
  }
}

function main() {
  const selected = selectedCases(process.argv.slice(2));
  const binary = resolveBinary(process.env.COMPLEXITY_BIN);
  runPreflight(binary);
  const tempRoot = mkdtempSync(join(tmpdir(), "complexity-codex-eval-"));
  const workspaces = resolve(tempRoot, "workspaces");
  const baselines = resolve(tempRoot, "baselines");
  const codexHome = resolve(tempRoot, "codex-home");
  let passed = false;
  try {
    prepareCases(selected, binary, workspaces, baselines);
    const sourceCodexHome = process.env.CODEX_HOME ?? resolve(homedir(), ".codex");
    prepareCodexHome(sourceCodexHome, codexHome);
    const environment = evalEnvironment(
      tempRoot,
      binary,
      workspaces,
      baselines,
      codexHome,
    );
    runPromptfoo(selected, environment);
    printResults(selected, binary, workspaces, baselines);
    passed = true;
  } finally {
    const keepWorkspace = process.env.COMPLEXITY_EVAL_KEEP_WORKSPACE === "1";
    if (cleanupEvalRoot(tempRoot, passed, keepWorkspace)) {
      console.error(`Kept failed eval workspace at ${tempRoot}`);
    }
  }
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
