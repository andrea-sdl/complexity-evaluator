import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import test from "node:test";
import { resolve } from "node:path";

import {
  analyzeFunction,
  cases,
  evalRoot,
  meetsTargets,
  resolveBinary,
  runBehaviorTest,
} from "../scripts/eval-lib.mjs";

const expectedFindings = {
  "javascript-score": ["score", 11],
  "typescript-depth": ["max_control_depth", 4],
  "php-predicates": ["max_condition_predicates", 5],
  "rust-span": ["line_span", 54],
};

test("each readable fixture starts complex and has a separate passing test", () => {
  const binary = resolveBinary(process.env.COMPLEXITY_BIN);
  for (const testCase of cases) {
    const fixture = resolve(evalRoot, "fixtures", testCase.id);
    assert.ok(existsSync(resolve(fixture, testCase.source)));
    runBehaviorTest(fixture, testCase);
    const metrics = analyzeFunction(
      binary,
      fixture,
      testCase.source,
      testCase.functionName,
    );
    const [metric, value] = expectedFindings[testCase.id];
    assert.equal(metrics[metric], value);
    assert.ok(metrics.score > 0, `${testCase.id} must have a score to lower`);
    assert.equal(meetsTargets(metrics), false, `${testCase.id} needs no revision`);
  }
});
