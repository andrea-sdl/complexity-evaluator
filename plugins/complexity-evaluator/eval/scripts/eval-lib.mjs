import { spawnSync } from "node:child_process";
import {
  accessSync,
  cpSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const evalRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const projectRoot = resolve(evalRoot, "../..");

const rustTestBinary = process.platform === "win32"
  ? ".complexity-eval-test.exe"
  : ".complexity-eval-test";
const rustTestCommand = process.platform === "win32"
  ? rustTestBinary
  : `./${rustTestBinary}`;

export const cases = [
  {
    id: "javascript-score",
    source: "subject.js",
    functionName: "deliveryWindow",
    testSteps: [[process.execPath, ["test.mjs"]]],
  },
  {
    id: "typescript-depth",
    source: "subject.ts",
    functionName: "canReview",
    testSteps: [[process.execPath, ["test.mjs"]]],
  },
  {
    id: "php-predicates",
    source: "subject.php",
    functionName: "canPublish",
    testSteps: [["php", ["test.php"]]],
  },
  {
    id: "rust-span",
    source: "subject.rs",
    functionName: "shipping_zone",
    testSteps: [
      ["rustc", ["--test", "test.rs", "-o", rustTestBinary]],
      [rustTestCommand, []],
    ],
    cleanup: [rustTestBinary],
  },
];

export const targets = {
  score: 10,
  max_control_depth: 3,
  line_span: 50,
  max_condition_predicates: 4,
};

function commandText(command, args) {
  return [command, ...args].join(" ");
}

export function runCommand(command, args, cwd, options = {}) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    env: options.env ?? process.env,
    stdio: options.stdio ?? "pipe",
    timeout: options.timeout ?? 30_000,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    const output = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
    throw new Error(`${commandText(command, args)} failed: ${output}`);
  }
  return result;
}

export function runBehaviorTest(workspace, testCase) {
  try {
    for (const [command, args] of testCase.testSteps) {
      runCommand(command, args, workspace);
    }
  } finally {
    for (const artifact of testCase.cleanup ?? []) {
      rmSync(resolve(workspace, artifact), { force: true });
    }
  }
}

function parseReport(result) {
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0 && result.status !== 1) {
    throw new Error(result.stderr || "complexity analysis failed");
  }
  return JSON.parse(result.stdout);
}

function functionMetrics(result) {
  return {
    score: result.score,
    max_control_depth: result.signals.max_control_depth,
    line_span: result.signals.line_span,
    max_condition_predicates: result.signals.max_condition_predicates,
  };
}

export function analyzeFunction(binary, workspace, source, functionName) {
  const result = spawnSync(binary, ["--format", "json", source], {
    cwd: workspace,
    encoding: "utf8",
    timeout: 30_000,
  });
  const report = parseReport(result);
  const file = report.files.find((item) => item.path === source);
  if (!file || file.status !== "ok") {
    throw new Error(`complexity did not analyze ${source}`);
  }
  const functionResult = file.functions.find((item) => item.name === functionName);
  if (!functionResult) {
    throw new Error(`complexity did not report ${functionName}`);
  }
  return functionMetrics(functionResult);
}

function fileHash(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function visitFiles(root, directory, snapshot) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.name === ".git") {
      continue;
    }
    const path = join(directory, entry.name);
    const name = relative(root, path).replaceAll("\\", "/");
    if (entry.isDirectory()) {
      visitFiles(root, path, snapshot);
    } else if (entry.isSymbolicLink()) {
      snapshot[name] = `symlink:${readlinkSync(path)}`;
    } else {
      snapshot[name] = fileHash(path);
    }
  }
}

export function snapshotFiles(root) {
  const snapshot = {};
  visitFiles(root, root, snapshot);
  return snapshot;
}

export function changedFiles(root, before) {
  const after = snapshotFiles(root);
  const paths = new Set([...Object.keys(before), ...Object.keys(after)]);
  return [...paths].filter((path) => before[path] !== after[path]).sort();
}

export function caseById(id) {
  const testCase = cases.find((item) => item.id === id);
  if (!testCase) {
    throw new Error(`unknown eval case: ${id}`);
  }
  return testCase;
}

export function prepareWorkspace(workspace, binary, testCase) {
  rmSync(workspace, { recursive: true, force: true });
  mkdirSync(workspace, { recursive: true });
  cpSync(resolve(evalRoot, "fixtures", testCase.id), workspace, { recursive: true });
  const skillTarget = resolve(workspace, ".agents/skills/complexity-cli");
  mkdirSync(dirname(skillTarget), { recursive: true });
  cpSync(resolve(projectRoot, "agent/skills/complexity-cli"), skillTarget, {
    recursive: true,
  });
  runCommand("git", ["init", "--quiet"], workspace);
  runBehaviorTest(workspace, testCase);
  const metrics = analyzeFunction(
    binary,
    workspace,
    testCase.source,
    testCase.functionName,
  );
  return {
    caseId: testCase.id,
    source: testCase.source,
    functionName: testCase.functionName,
    metrics,
    files: snapshotFiles(workspace),
  };
}

export function writeBaseline(path, baseline) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(baseline)}\n`);
}

export function readBaseline(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

export function prepareCodexHome(source, target) {
  mkdirSync(target, { recursive: true });
  const auth = resolve(source, "auth.json");
  try {
    accessSync(auth);
  } catch {
    if (process.env.OPENAI_API_KEY || process.env.CODEX_API_KEY) {
      return;
    }
    throw new Error(`Codex login is missing at ${auth}`);
  }
  cpSync(auth, resolve(target, "auth.json"));
  const cloudConfig = resolve(source, "cloud-config-bundle-cache.json");
  try {
    accessSync(cloudConfig);
    cpSync(cloudConfig, resolve(target, "cloud-config-bundle-cache.json"));
  } catch {
    // The cloud model cache is optional.
  }
}

export function cleanupEvalRoot(tempRoot, passed, keepWorkspace) {
  if (passed || !keepWorkspace) {
    rmSync(tempRoot, { recursive: true, force: true });
    return false;
  }
  rmSync(resolve(tempRoot, "codex-home"), { recursive: true, force: true });
  return true;
}

export function resolveBinary(value) {
  const binary = resolve(evalRoot, value ?? "../../target/release/complexity");
  accessSync(binary);
  if (!statSync(binary).isFile()) {
    throw new Error(`${binary} is not a file`);
  }
  return binary;
}

export function meetsTargets(metrics) {
  return Object.entries(targets).every(([metric, limit]) => metrics[metric] <= limit);
}

export function sameMetrics(left, right) {
  return Object.keys(targets).every((metric) => left?.[metric] === right?.[metric]);
}
