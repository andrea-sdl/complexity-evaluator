import assert from "node:assert/strict";
import test from "node:test";

import { gradeEvidence } from "../assertions/refactor-result.mjs";

const source = "src/score.js";
const before = {
  score: 11,
  max_control_depth: 1,
  line_span: 15,
  max_condition_predicates: 1,
};
const after = {
  score: 0,
  max_control_depth: 0,
  line_span: 4,
  max_condition_predicates: 0,
};

function report(overrides = {}) {
  return {
    case_id: "javascript-score",
    source,
    function: "deliveryWindow",
    before,
    after,
    behavior_tests_passed: true,
    refactor: "Replace repeated branches with a direct day lookup.",
    further_improvement: "No further complexity change is needed.",
    ...overrides,
  };
}

function rawItems(includeRerun = true) {
  const items = [
    {
      type: "command_execution",
      command: `/bin/zsh -lc 'node test.mjs && python3 .agents/skills/complexity-cli/scripts/check_complexity.py ${source}'`,
      exit_code: 1,
      aggregated_output: "REVISE complexity: deliveryWindow score=11>10",
    },
    { type: "file_change", changes: [{ path: source, kind: "update" }] },
  ];
  if (includeRerun) {
    items.push({
      type: "command_execution",
      command: `/bin/zsh -lc 'node test.mjs\npython3 .agents/skills/complexity-cli/scripts/check_complexity.py ${source}'`,
      exit_code: 0,
      aggregated_output: "",
    });
  }
  return JSON.stringify({ items });
}

test("passes only with a scoped edit, preserved behavior, and a measured score drop", () => {
  const result = gradeEvidence({
    modelReport: report(),
    raw: rawItems(),
    before,
    after,
    changedFiles: [source],
    behaviorPassed: true,
    caseId: "javascript-score",
    source,
    functionName: "deliveryWindow",
  });

  assert.equal(result.pass, true, result.reason);
  assert.equal(result.score, 1);
});

test("fails when Codex does not rerun the skill or changes another file", () => {
  const result = gradeEvidence({
    modelReport: report(),
    raw: rawItems(false),
    before,
    after,
    changedFiles: [source, "test.mjs"],
    behaviorPassed: true,
    caseId: "javascript-score",
    source,
    functionName: "deliveryWindow",
  });

  assert.equal(result.pass, false);
  assert.match(result.reason, /skill rerun|changed files/i);
});

test("fails when the real score does not fall", () => {
  const result = gradeEvidence({
    modelReport: report({ after: before }),
    raw: rawItems(),
    before,
    after: before,
    changedFiles: [source],
    behaviorPassed: true,
    caseId: "javascript-score",
    source,
    functionName: "deliveryWindow",
  });

  assert.equal(result.pass, false);
  assert.match(result.reason, /lower score/i);
});

test("fails when output is fabricated without running the checker", () => {
  const raw = JSON.stringify({
    items: [
      {
        type: "command_execution",
        command: `echo check_complexity.py ${source}`,
        exit_code: 1,
        aggregated_output: "REVISE",
      },
      { type: "file_change", changes: [{ path: source, kind: "update" }] },
      {
        type: "command_execution",
        command: `echo check_complexity.py ${source}`,
        exit_code: 0,
        aggregated_output: "PASS",
      },
    ],
  });
  const result = gradeEvidence({
    modelReport: report(),
    raw,
    before,
    after,
    changedFiles: [source],
    behaviorPassed: true,
    caseId: "javascript-score",
    source,
    functionName: "deliveryWindow",
  });

  assert.equal(result.pass, false);
  assert.match(result.reason, /skill rerun/i);
});
