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

## CX-009: Freeze the Rust and Python contract

- Status: done
- Owner: root
- Scope: version, filters, extensions, stdin defaults, parser choices,
  callable boundaries, score rules, signals, compatibility limits, and task
  ownership.
- Success: Rust and Python tests can use exact expected results without making
  new product choices.
- Dependency approval: the owner approved `tree-sitter-rust 0.24.2` and
  `tree-sitter-python 0.25.0` on 2026-07-28.
- Evidence: `SPEC.md` defines all new public behavior. `DESIGN.md` records the
  dependency, compatibility, callable, Python match, and version choices.

## CX-010: Extend the shared CLI

- Status: done
- Owner: root
- Depends on: CX-009
- Scope: pinned dependencies, version, language filters, extensions, stdin
  defaults, direct dispatch, mixed ordering, and shared CLI tests.
- Success: the public CLI selects and dispatches all five language families
  without changing existing JavaScript, TypeScript, or PHP output.
- Evidence: 39 CLI tests pass, including Rust and Python stdin, repeated
  filters, five-language stable ordering, version, help, and error cases.

## CX-011: Add the Rust engine

- Status: done
- Owner: rust_engine
- Depends on: CX-009, CX-010
- Scope: `src/rust.rs`, Rust fixtures, and Rust integration tests.
- Success: exact callable, score, contribution, signal, position, threshold,
  and parse-error tests pass for `.rs` source.
- Evidence: all 5 Rust integration tests pass. At CX-011 completion, the
  release binary analyzed all 7 Rust files in `src` and reported 350 functions
  with zero errors.

## CX-012: Add the Python engine

- Status: done
- Owner: python_engine
- Depends on: CX-009, CX-010
- Scope: `src/python.rs`, Python fixtures, and Python integration tests.
- Success: exact callable, score, contribution, signal, position, threshold,
  and parse-error tests pass for `.py` source.
- Evidence: the initial 6 Python integration tests pass. A red-green
  regression test proves Boolean and ternary decorators on nested functions
  do not leak into their parent.

## CX-013: Self-analysis and final validation

- Status: done
- Owner: root
- Depends on: CX-010, CX-011, CX-012
- Scope: Rust self-analysis, five-language deterministic integration, skill
  and hook support, compatibility notes, benchmarks, full gates, and reviews.
- Success: `complexity` analyzes its own Rust source without errors, all
  required checks pass, and no existing language result changes.
- Evidence: formatting, strict Clippy, 95 all-target tests, and a release build
  pass. The five-language mixed corpus reports 50 functions with exit `0`.
  Rust self-analysis was complete, deterministic, and had zero errors. At
  CX-013 completion, the explicit skill checker reported 18 policy findings
  in 350 functions; it did not hide existing debt. The two new Rust hard-limit
  findings were reduced to target-only span findings. Two independent reviews
  found one Python decorator boundary bug and two small reader-load issues.
  The bug was fixed with a red-green test, both reader-load changes were
  applied, and re-review found no remaining defect.

## CX-014: Make the CLI pass its own readability policy

- Status: done
- Owner: root
- Depends on: CX-013
- Scope: simplify the 18 functions named by the `complexity-cli` skill. Keep
  related logic in each existing module and add no dependency. Also fix any
  confirmed defect found by the required independent review.
- Success: the installed skill reports `PASS` for all Rust source in `src`;
  exact JS/TS/PHP/Rust/Python fixtures, formatting, strict Clippy, all-target
  tests, and the locked release build still pass.
- Work split: `lib.rs`, `php.rs`, `python.rs`, `rust.rs`, and `javascript.rs`
  have separate owners. Each owner must preserve concurrent work and may edit
  only its assigned source file.
- Evidence: the explicit skill checker reports `PASS` for all 7 Rust source
  files and 398 functions. The maxima are score 10, control depth 3, line span
  47, and 3 predicates in one condition. Old fixture reports at limits 15 and
  0 match the pre-change JSON byte for byte. Their SHA-256 values are
  `1564cd54f94cca80b62366f4eea320c0ee05cd4acc830d316fd6cf80566058eb`
  and
  `d448e1c95084251d2ed4809454ce125f7e8ed21b7d4a49850174c119eb212dec`.
  An independent review found one Python `try ... else` depth defect. A
  red-green public test fixed it, and focused re-review found no remaining
  defect. A second review found no reader-load issue or metric gaming.
  Formatting, strict Clippy, all 96 tests, the locked release build, and the
  diff check pass. Two self-analysis reports match byte for byte. The refreshed
  median is `3.185` ms for the mixed corpus and `23.114` ms for self-analysis.

## CX-015: Add the max-score ratchet and reach five

- Status: done
- Owner: root
- Depends on: CX-014
- Scope: add one public self-analysis test with a maximum score of `7`, then
  reduce the 21 current functions above `5` without changing CLI results or
  adding a dependency.
- Success: the new test fails against the starting maximum of `10`, passes
  after the first refactor batch, and stays green when every Rust source
  function reaches score `5` or less. Exact fixtures, formatting, strict
  Clippy, all-target tests, the locked release build, benchmarks, and
  independent correctness and reader-load reviews must pass.
- Work split: `tests/cli.rs` and project documents belong to root. Refactor
  owners may edit only their assigned language module. Each change must keep a
  syntax rule local or create a real domain boundary; shallow score-moving
  helpers are out of scope.
- Evidence: the new public test failed first because the source maximum was
  `10`, then passed after the first batch reduced it to `7`. The final
  self-analysis covers all 7 source files and 407 functions with maximum score
  `5`, control depth `2`, line span `44`, and 3 predicates in one condition.
  The ratchet pins limit `7`, the ordered source paths, and each file's
  function count. Fixture JSON at limits `15` and `0` remains byte-identical;
  the SHA-256 values are
  `5572e4b67325c63a38a2aadb23c1167ecd156548808ae03f67b90be3cf238fff`
  and
  `99edc2ec46f858e077a75b9fe75da30daa31d3cd5617543596faeef51495d256`.
  Formatting, strict Clippy, all 97 tests, the locked release build, and the
  diff check pass. Independent correctness and newcomer reviews have no
  remaining finding. A paired pre/post benchmark measured `+1.41%` on the
  five-language mixed corpus and `-0.05%` on Rust self-analysis.
