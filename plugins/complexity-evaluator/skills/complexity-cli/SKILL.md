---
name: complexity-cli
description: Run the local complexity CLI on JavaScript, TypeScript, PHP, Rust, and Python only when the user explicitly invokes $complexity-cli or /complexity-cli. Never invoke this skill implicitly.
disable-model-invocation: true
---

# Complexity CLI

Check code complexity before handoff.

## Run the check

1. Use paths supplied with the invocation. If none are supplied, check changed
   supported files in the current Git repository.
2. Run `scripts/check_complexity.py` from this skill directory while keeping the
   target repository as the working directory.
3. Read the outcome and named function evidence.
4. If the outcome is `REVISE` or `FAIL`, make a scoped edit only to the named
   functions when the current task permits edits. Preserve behavior and rerun
   the real CLI. Report the before/after score and metrics, then suggest the
   next useful improvement when a finding remains.
5. If the outcome is `BLOCKED`, fix the tool, input, read, or parse error. Never
   report a pass from an incomplete result.

The checker also tries `$CARGO_HOME/bin/complexity` and
`~/.cargo/bin/complexity`. Use `COMPLEXITY_BIN=/path/to/complexity` for another
install location.

The optional hooks record the Git and file state before each user prompt. At
Stop, they check only supported files changed after that baseline, including
changes committed during the task. Unsupported-only work stays silent.

## Limits and outcomes

| Per-function metric | Target | Hard limit |
| --- | ---: | ---: |
| Cognitive complexity score | 10 | 15 |
| Maximum control depth | 3 | 4 |
| Inclusive line span | 50 | 80 |
| Predicates in one condition | 4 | 6 |

- `PASS`: Every value is at or below its target.
- `REVISE`: At least one target is exceeded, but no hard limit is exceeded.
- `FAIL`: At least one hard limit is exceeded.
- `BLOCKED`: The CLI is missing, analysis is incomplete, or schema v2 cannot be
  read safely.

The checker exits `0` for `PASS`, `1` for `REVISE` or `FAIL`, and `2` for
`BLOCKED`. The score hard limit matches the CLI and Sonar default of 15. The
other values are this skill's small-code policy, not Sonar compatibility
claims.

The checker also enables `--max-cognitive-load 2`. It returns `REVISE` for an
inline conditional return that combines a Boolean test and an explicit cast.
Split it into a guard return and a clear final return, then run the checker
again. This rule uses syntax only. It does not judge variable names or intent.

Checker exit `1` is an expected result for `REVISE` and `FAIL`. Use its named
findings; do not debug that exit as a tool failure.

Treat function count, condition count, operator count, and Boolean depth as
report evidence. Do not gate on them without a project-specific rule.

## Revision guidance

- For excess control depth, use guard clauses when they keep the same behavior.
- For dense predicates, use named Boolean values that express the domain rule.
- For a flagged inline conditional return, use a guard return or compute the
  value before the return when that keeps the behavior clear.
- For excess line span, extract a cohesive domain operation with one clear job.
- Do not split code into shallow helpers only to lower a score. Keep related
  work together and change only the function named by the report.

## Report

Return the outcome, files and functions checked, and each function over a
target. Do not hide existing debt. Separate findings in code changed for the
current task when that boundary is known.

For optional Codex and Claude lifecycle hooks, read
`references/hooks.md`. The skill and sync scripts do not enable those hooks.
