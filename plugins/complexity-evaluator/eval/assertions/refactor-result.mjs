import { resolve } from "node:path";

import {
  analyzeFunction,
  caseById,
  changedFiles,
  meetsTargets,
  readBaseline,
  runBehaviorTest,
  sameMetrics,
} from "../scripts/eval-lib.mjs";

function check(label, pass, detail) {
  return {
    pass,
    score: pass ? 1 : 0,
    reason: `${label}: ${detail}`,
    namedScores: { [label]: pass ? 1 : 0 },
  };
}

function parseItems(raw) {
  try {
    const value = typeof raw === "string" ? JSON.parse(raw) : raw;
    return Array.isArray(value?.items) ? value.items : [];
  } catch {
    return [];
  }
}

function runsChecker(command) {
  const invocation = /(?:^|&&|\|\||;|[\n"'])\s*(?:(?:[A-Za-z_][A-Za-z0-9_]*=\S+)\s+)*(?:(?:\S*[\\/])?python(?:3(?:\.\d+)?)?|py\s+-3)(?:\s+-[A-Za-z]+)*\s+\S*check_complexity\.py(?:\s|["']|$)/;
  return invocation.test(command);
}

function commandShows(item, source, outcomes) {
  const command = typeof item?.command === "string" ? item.command : "";
  const output = typeof item?.aggregated_output === "string"
    ? item.aggregated_output
    : "";
  const ranChecker = item?.type === "command_execution"
    && runsChecker(command)
    && command.includes(source);
  const reportedOutcome = outcomes.some((outcome) => output.includes(outcome));
  const passingExit = outcomes.includes("PASS") && item?.exit_code === 0;
  return ranChecker && (reportedOutcome || passingExit);
}

function hasOrderedSkillRun(raw, source) {
  const items = parseItems(raw);
  const before = items.findIndex((item) =>
    commandShows(item, source, ["REVISE", "FAIL"]));
  const change = items.findIndex((item, index) =>
    index > before && item?.type === "file_change");
  const after = items.findIndex((item, index) =>
    index > change && commandShows(item, source, ["PASS"]));
  return before >= 0 && change > before && after > change;
}

function reportMatches(modelReport, expected) {
  return modelReport?.case_id === expected.caseId
    && modelReport?.source === expected.source
    && modelReport?.function === expected.functionName
    && modelReport?.behavior_tests_passed === true
    && typeof modelReport?.refactor === "string"
    && modelReport.refactor.trim().length > 0
    && typeof modelReport?.further_improvement === "string"
    && modelReport.further_improvement.trim().length > 0;
}

function changedOnlySource(files, source) {
  return files.length === 1 && files[0] === source;
}

export function gradeEvidence(evidence) {
  const reportIsExact = reportMatches(evidence.modelReport, evidence)
    && sameMetrics(evidence.modelReport.before, evidence.before)
    && sameMetrics(evidence.modelReport.after, evidence.after);
  const components = [
    check(
      "model report",
      reportIsExact,
      reportIsExact ? "matches measured evidence" : "does not match measured evidence",
    ),
    check(
      "changed files",
      changedOnlySource(evidence.changedFiles, evidence.source),
      changedOnlySource(evidence.changedFiles, evidence.source)
        ? `only ${evidence.source} changed`
        : `changed ${evidence.changedFiles.join(", ") || "nothing"}`,
    ),
    check(
      "behavior test",
      evidence.behaviorPassed,
      evidence.behaviorPassed ? "passed" : "failed",
    ),
    check(
      "lower score",
      evidence.after.score < evidence.before.score,
      `${evidence.before.score} -> ${evidence.after.score}`,
    ),
    check(
      "target limits",
      meetsTargets(evidence.after),
      meetsTargets(evidence.after) ? "all met" : "a target still fails",
    ),
    check(
      "skill rerun",
      hasOrderedSkillRun(evidence.raw, evidence.source),
      "requires check, edit, then passing recheck",
    ),
  ];
  const failed = components.filter((item) => !item.pass);
  const score = components.reduce((sum, item) => sum + item.score, 0)
    / components.length;
  return {
    pass: failed.length === 0,
    score,
    reason: failed.length === 0
      ? `${evidence.functionName} score ${evidence.before.score} -> ${evidence.after.score}`
      : failed.map((item) => item.reason).join("; "),
    componentResults: components,
  };
}

function failedGrade(error) {
  return {
    pass: false,
    score: 0,
    reason: error instanceof Error ? error.message : String(error),
  };
}

export default function grade(output, context) {
  try {
    const caseId = context.vars.caseId;
    const testCase = caseById(caseId);
    const workspace = resolve(process.env.COMPLEXITY_EVAL_WORKSPACES, caseId);
    const baselinePath = resolve(process.env.COMPLEXITY_EVAL_BASELINES, `${caseId}.json`);
    const baseline = readBaseline(baselinePath);
    const modelReport = JSON.parse(output);
    const after = analyzeFunction(
      process.env.COMPLEXITY_BIN,
      workspace,
      testCase.source,
      testCase.functionName,
    );
    runBehaviorTest(workspace, testCase);
    return gradeEvidence({
      modelReport,
      raw: context.providerResponse?.raw,
      before: baseline.metrics,
      after,
      changedFiles: changedFiles(workspace, baseline.files),
      behaviorPassed: true,
      caseId,
      source: testCase.source,
      functionName: testCase.functionName,
    });
  } catch (error) {
    return failedGrade(error);
  }
}
