# Complexity v2 task board

This file is the handoff record for humans and subagents.

Statuses: `blocked`, `ready`, `in-progress`, `done`.

Protocol:

1. Read `SPEC.md`, `DESIGN.md`, and this file.
2. Claim a ready task and record its owner.
3. Work only in the owned files and behavior.
4. Add one public failing test before each behavior change.
5. Record checks and decisions before setting a task to `done`.
6. Parallel tasks are allowed only when their file ownership does not overlap.

## CX-000: Freeze the merged contract

- Status: done
- Owner: root
- Scope: schema v2, CLI, signals, migration rules, design decisions, and nested
  agent instructions.
- Success: another agent can implement the public behavior without choosing
  names, defaults, edge cases, or output meanings.
- Evidence: `SPEC.md` freezes CLI, stdin, score preservation, signal meanings,
  schema v2, text, errors, and exclusions. `DESIGN.md` records 18 merge
  decisions. `AGENTS.md`, `README.md`, and this board route implementation.

## CX-001: Shared CLI and report

- Status: done
- Owner: shared_cli_worker
- Depends on: CX-000
- Scope: command parsing, mixed discovery, stdin, schema v2, text and JSON,
  summaries, and exit codes.
- Success: public CLI tests cover empty mixed reports, filtering, stdin
  validation, deterministic output, and exit priority.
- Evidence: 14 public CLI tests, formatting, strict Clippy, all-target tests,
  and a release build pass. Independent review found and fixed discovered file
  symlink handling with a red-green test.
- Budget: this task exceeded the 4,000-token task limit because its frozen
  scope combined the command, discovery, stdin, schema, output, and exit
  layers. The overrun was reported before final checks; no extra scope was
  added.

## CX-002: JavaScript and TypeScript engine

- Status: done
- Owner: js_ts_port
- Depends on: CX-001
- Scope: port Oxc modes, function boundaries, scoring, diagnostics, positions,
  and frozen fixtures without score changes.
- Success: old and new JS/TS function identities, scores, contributions,
  diagnostics, and exits match after schema normalization.
- Evidence: 10 focused tests, formatting, and strict Clippy pass. The
  old-to-new differential matched 38 of 38 fixture and threshold runs after
  removing only v2 wrapper and signal fields. Full project gates remain in
  CX-008.

## CX-003: PHP engine

- Status: done
- Owner: php_port
- Depends on: CX-001
- Scope: port Tree-sitter parsing, reserved-word retry, function boundaries,
  scoring, diagnostics, positions, and frozen fixtures without score changes.
- Success: old and new PHP function identities, scores, contributions,
  diagnostics, and exits match after schema normalization.
- Evidence: 8 focused tests, strict Clippy, the release build, and owned-file
  formatting pass. All 23 frozen functions match the old binary at limits 15
  and 0 after schema normalization. Full project gates remain in CX-008.

## CX-004: JavaScript and TypeScript signals

- Status: done
- Owner: js_ts_port
- Depends on: CX-002
- Scope: control depth, condition shape, line span, and file function count.
- Success: public fixtures cover every signal definition and nested-callable
  reset.
- Evidence: 22 JS/TS public tests pass. The old score and contribution
  differential still matches across 48 fixture and threshold runs after
  signals. Formatting and a release build pass.

## CX-005: PHP signals

- Status: done
- Owner: php_port
- Depends on: CX-003
- Scope: control depth, condition shape, line span, and file function count.
- Success: public fixtures use the same cross-language signal meanings and
  preserve PHP-specific syntax.
- Evidence: 20 PHP tests pass, including keyword operators, pipe exclusion,
  selector depth, nested-callable barriers, and exact grouped locations. The
  23-function score differential remains exact at limits 15 and 0. Formatting,
  strict Clippy, all 57 tests, and a release build pass.

## CX-006: Mixed and stdin integration

- Status: done
- Owner: shared_cli_worker
- Depends on: CX-004, CX-005
- Scope: mixed global ordering, repeated filters, stdin modes, exact compact
  JSON v2, stable text, and fail-closed mixed reports.
- Success: all public interface and signal cases in `SPEC.md` pass.
- Evidence: 35 public CLI tests pass. They cover exact JSON and text, mixed
  ordering, filters, all stdin modes, usage failures, exit priority, explicit
  skip overrides, and project-owned ignore and symlink target boundaries.
  Independent review found and fixed rooted virtual paths, skip-rule bypasses,
  and ignore precedence with red-green tests.

## CX-007: Compatibility differential

- Status: done
- Owner: root
- Depends on: CX-006
- Scope: compare old and new language results after removing expected wrapper
  and schema changes.
- Success: no unexplained identity, range, score, contribution, diagnostic, or
  exit difference remains.
- Evidence: all 48 JS/TS runs and all 26 PHP runs matched at limits `15` and
  `0`. The comparison kept identities, kinds, names, ranges, scores,
  thresholds, ordered contributions, diagnostics, stderr, exits, and old
  summary fields. It removed only planned v2 metadata and signal fields.

## CX-008: Benchmark, review, docs, and retirement

- Status: done
- Owner: root
- Depends on: CX-007
- Scope: legacy and mixed benchmarks, final documentation, cognitive-load and
  correctness reviews, then removal of the old project folders.
- Success: format, strict Clippy, all tests, release build, benchmark target,
  docs, and reviews pass before either old folder is removed.
- Evidence: format, strict Clippy, 80 all-target tests, and the release build
  pass. The four single-language runtime gates pass; the closest paired result
  is PHP few-large at `+21.3%` against a `+25%` limit. The mixed median is
  `2.3 ms`. Correctness and cognitive-load reviews have no unresolved finding.
  A mixed Agentforce run completed 94 files and 1,121 functions with zero
  errors and byte-stable repeated JSON. The two old folders were removed only
  after these checks and the 74-run differential passed.
- Budget: CX-008 exceeded the 4,000-token task limit because the approved task
  combined five benchmark corpora, migration audit, two independent reviews,
  final documentation, full gates, a real-repository run, and retirement. No
  extra product feature was added.
