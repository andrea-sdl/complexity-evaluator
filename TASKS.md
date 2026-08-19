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

## CX-016: Close the validated security findings

- Status: done
- Owner: root
- Scope: close the eight findings from Codex Security scan
  `c49cbf92-4a41-4856-8193-035c118f24fa`: bounded analysis for PHP,
  JavaScript and TypeScript, Rust, and Python; linear JSX cleanup and
  JavaScript position lookup; race-safe contained file reads; and safe text
  output.
- Success: each original exploit or bounded equivalent fails closed without a
  process abort, quadratic runtime, containment bypass, or terminal-control
  injection. Existing scores, contributions, ranges, JSON, valid text, and
  exit rules remain stable. Focused regressions, formatting, strict Clippy,
  all-target tests, a locked release build, self-analysis, and independent
  security review pass.
- Work split: `php.rs` and PHP tests, `javascript.rs` and JavaScript tests,
  `rust.rs` and Rust tests, `python.rs` and Python tests, and `lib.rs` and CLI
  tests have separate owners. Root owns shared documents, integration,
  exploit reruns, and final checks.
- Evidence: all eight saved exploit cases now fail to reproduce. PHP, Rust,
  and Python reject analysis trees deeper than `512` with a structured parse
  error. JavaScript and TypeScript use iterative hot-path walkers and a
  process probe for risky parser input. JSX suppression uses direct span
  membership, and source positions use sparse Unicode checkpoints. Selected
  files open through a cached directory capability, and text output escapes
  control and bidirectional format characters. All 116 tests, formatting,
  strict Clippy, the locked release build, and the dependency audit pass.
  Public self-analysis reports 7 files, 446 functions, maximum score `6`, and
  no violation. The four required runtime changes are `+5.40%`, `+6.30%`,
  `+16.96%`, and `+14.74%`; all stay below the 25% gate. The release
  SHA-256 is
  `73ee3294505d5b99d37f713433159c2a881e4c71389d1c4aad5725839f88b8c3`.
  Independent reviews found no remaining issue in the changed paths.

## CX-017: Package releases and agent support

- Status: done
- Owner: root
- Scope: add a tag-driven GitHub Actions release flow, portable release
  archives, and one tracked `agent` folder with the explicit-only skill,
  Codex and Claude hook samples, a deterministic policy eval, and a score-7
  ratchet for the new Python support code.
- Success: a `complexity-vX.Y.Z` tag must match `Cargo.toml`; Linux x64 and
  arm64, macOS Intel and arm64, and Windows x64 builds must use the locked
  dependency graph. Each archive must contain the native binary, root README,
  and the same `agent` tree. SHA-256 files, package tests, hook tests, skill
  validation, eval cases, Rust gates, release build, and the self-complexity
  ratchet must pass. The release job must publish only after all builds pass.
- Work split: `agent/**` belongs to `agent_bundle_worker`.
  `release/**` and `.github/workflows/complexity-release.yml` belong to
  `release_flow_worker`. Root owns shared documents, integration, and final
  checks. Workers must preserve all other uncommitted work.
- Evidence: the tag workflow uses five native hosted runners, locked Cargo
  builds, SHA-pinned GitHub actions, read-only checkout credentials, and one
  final `contents: write` release job. Seven release and ratchet tests, 15
  checker tests, 7 agent eval tests, and the real 4-case policy eval pass. A
  real macOS arm64 archive passed its SHA-256 check, its 18 packaged agent
  files matched `agent/MANIFEST.txt`, and its unpacked eval passed. Manifest
  traversal, extra-file, and symlink-parent regressions fail closed.
  Formatting, strict Clippy, all 116 Rust tests, and the locked release build
  pass. Rust self-analysis covers 7 files and 447 functions with maximum score
  `6`, depth `3`, span `50`, and 3 predicates. The Python ratchet covers 7
  files and 131 functions with maximum score `6`, depth `2`, span `46`, and 3
  predicates. The Skill Creator validator could not import host `PyYAML`; the
  same frontmatter checks passed with the system YAML parser, and the skill's
  own tests passed without a new dependency. Final independent reviews found
  no remaining issue. The hosted five-runner matrix and `gh release create`
  remain unproved until a real release tag runs.

## CX-018: Use Promptfoo for readable policy evals

- Status: done
- Owner: root
- Depends on: CX-017
- Scope: replace the custom policy-eval runner with one local Promptfoo setup,
  four short YAML cases, and the smallest adapter needed to preserve checker
  exit-code evidence. Update the release bundle, CI, tests, and documents.
- Success: a reader can understand the `PASS`, `REVISE`, `FAIL`, and `BLOCKED`
  cases from one short YAML file. Promptfoo validates and runs all four cases
  against the real binary. No model is called, no root Node workspace is
  created, and no duplicate eval runner remains.
- Dependency approval: the owner explicitly requested Promptfoo on 2026-08-04.
- Historical evidence at CX-018 completion: `cases.yaml` held four short native
  YAML cases and `provider.mjs` was the only adapter. Bundle tests blocked the
  removed JSON and Python runner.
  Promptfoo `0.121.20` validates the config and passes all four cases against
  debug, release, and unpacked release binaries. The full locked npm install
  reports zero known vulnerabilities after the three recorded overrides.
  Eleven release tests, 15 checker tests, formatting, strict Clippy, all 116
  Rust tests, and the locked release build pass. The Python ratchet covers 6
  files and 119 functions with maximum score `6`, depth `3`, span `46`, and 3
  predicates. The JavaScript ratchet covers the two provider functions with
  maximum score `4`, depth `1`, span `31`, and 1 predicate. Independent
  correctness and readability reviews found no remaining issue. The hosted
  five-runner release matrix remains unproved until a real tag runs. CX-019
  later removed `provider.mjs` and the Python policy fixtures; D-029 keeps the
  former design record.

## CX-019: Evaluate Codex refactoring behavior

- Status: done
- Owner: root
- Depends on: CX-018
- Scope: replace the Python policy fixtures with short JavaScript, TypeScript,
  PHP, and Rust refactoring cases. Run Codex manually in an isolated workspace,
  let it use the real skill and binary, and require a behavior-preserving edit,
  a lower measured score, and a useful next improvement. Keep release CI
  deterministic and model-free.
- Success: each readable Promptfoo case proves the starting function is over a
  target, Codex edits only the source fixture, its language test still passes,
  and the real CLI reports a lower score after the edit and all targets pass.
  Codex must report the refactor and one next useful improvement. The
  independent assertion must verify the exact changed file, behavior test,
  before and after metrics, and ordered checker runs. Deterministic assertion
  tests, Promptfoo validation, all four live Codex cases, release tests, skill
  tests, Rust gates, self-analysis, and independent review must pass.
- Work split: `agent/eval/fixtures/**` and the case contract belong to
  `eval_cases_worker`. `agent/skills/complexity-cli/**` belongs to
  `skill_guidance_worker`. Root owns the Promptfoo runner, remaining agent and
  release files, documents, integration, and final checks. Workers must
  preserve all other uncommitted work.
- Evidence: the four readable cases target JavaScript score, TypeScript control
  depth, PHP condition predicates, and Rust function span. Their separate tests
  cover all listed day results, all 16 TypeScript flag sets, all 32 PHP flag
  sets, and the Rust zone and gate boundaries. Clean manual Codex runs passed:
  `eval-dQj-2026-08-04T14:14:50` changed score `11` to `1`,
  `eval-uPK-2026-08-04T14:14:41` changed `10` to `1`,
  `eval-YFC-2026-08-04T14:21:54` changed `2` to `1`, and
  `eval-NE9-2026-08-04T14:21:55` changed `3` to `1`. Each independent grade
  accepted one source edit, the behavior proof, exact before and after metrics,
  all targets, and the ordered checker runs. Regression tests accept multiline
  checker commands, reject printed checker names, and prove that a retained
  failed workspace drops copied Codex login data. Ten eval tests, Promptfoo
  validation, 16 skill tests, 11 release tests, formatting, strict Clippy, all
  116 Rust tests, and the locked release build pass. Self-analysis covers 447
  Rust functions at maximum score `6`, 121 Python functions at maximum score
  `6`, and 64 eval-support functions at maximum score `7`, with no violation.
  A real macOS arm64 archive passed its SHA-256 check and contained all 30
  manifest files. Final read-only review found no remaining actionable issue.

## CX-020: Move to the canonical repository

- Status: completed
- Owner: root
- Depends on: CX-019
- Scope: move the complete project and its available Git history from the
  umbrella repository to `andrea-sdl/complexity-evaluator`. Preserve the
  current uncommitted project work, keep the old source tree unchanged, and
  adapt repository-root paths. Keep the target repository's GPL-2.0 license
  and include it in every release archive.
- Success: the target `main` branch contains the two available project commits
  plus one signed migration merge. All source files and current work are
  present. Repository-root tests, release package tests, Rust gates,
  self-analysis, and a real local package check pass. The pushed target and
  local canonical checkout must point to the same commit.
- Evidence: the target repository keeps its initial GPL-2.0 commit and imports
  the two available project commits through a subtree merge. The current source
  work is present, the repository-root release workflow replaced the monorepo
  path assumptions, the package now includes `LICENSE`, and source-only diff
  checks left the old tree unchanged. Validation passed with `cargo fmt --all
  -- --check`, `cargo clippy --locked --all-targets --all-features -- -D
  warnings`, `cargo test --locked --all-targets`, `cargo build --locked --bin
  complexity`, `cargo build --release --locked`, `python3 -m unittest discover
  -s release/tests -p "test_*.py"`, `python3 -m unittest discover -s
  agent/skills/complexity-cli/scripts -p "test_*.py"`, `npm test`, `npm run
  validate`, and a real local package run for
  `complexity-v0.3.0` on `aarch64-apple-darwin`. The archive passed its
  SHA-256 check and contained `LICENSE`, `README.md`, the binary, and the full
  manifest-listed `agent` tree.

## CX-021: Package the skill and hooks for Claude Code and Codex

- Status: in-progress
- Owner: root
- Depends on: CX-020
- Scope: add installable Claude Code and Codex plugin packages. Generate both
  from `agent/skills/complexity-cli/**`, `agent/hooks/*`, and the current manual
  eval files; do not create a second hand-maintained skill or checker. Do not
  implement this work as part of CX-020.
- Expected repository layout: use one self-contained
  `plugins/complexity-evaluator/` payload with
  `.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`,
  `skills/complexity-cli/**`, host and OS hook samples, `README.md`, and
  `LICENSE`. Add `.claude-plugin/marketplace.json` and
  `.agents/plugins/marketplace.json` at the repository root; both catalogs
  must point to that payload. Each installed or archived payload must work
  without files outside its own directory.
- Install rules: the base plugin exposes the skill but does not enable hooks.
  Keep hook JSON as an explicit opt-in sample unless a host supports a separate
  opt-in hook component. Add `disable-model-invocation: true` for Claude Code
  and keep `policy.allow_implicit_invocation: false` for Codex. Use host-root
  variables or documented install paths in hook commands; never write a local
  machine path. Require a separately installed `complexity` binary and fail
  with `BLOCKED` when it is absent.
- Manifest rules: use the project version, GPL-2.0-only license, canonical
  repository URL, real author and interface fields, and only fields accepted
  by each host validator. The Codex marketplace entry must include install and
  auth policy plus a category. The Claude marketplace entry must use the
  relative plugin source and must not duplicate the version in two places.
- Success: a clean Claude Code install and a clean Codex install each expose
  only the explicit skill. Enabling the matching POSIX or Windows hook sample
  must check supported changes, ignore unsupported-only changes, filter mixed
  changes, keep the per-prompt baseline, and stop on `REVISE`, `FAIL`, or
  `BLOCKED`. Generated plugin copies must match the canonical `agent` files,
  and release archives must contain no caches, credentials, symlinks, or
  unlisted files.
- Validation: add drift and archive contract tests; validate both manifests
  and marketplaces; run `claude plugin validate .` plus a local marketplace
  add/install/reload smoke test; run the Codex plugin validator and a clean
  marketplace install smoke test; invoke the skill manually in each host;
  enable and test each hook sample separately; run the short Promptfoo eval,
  skill and release tests, Rust gates, self-analysis, checksum checks, and an
  independent security and correctness review.
- Evidence: the generated `0.4.0` payload and both repository marketplaces
  pass exact drift, allowlist, source-containment, symlink, and archive
  contracts. `claude plugin validate` passes for the payload and marketplace.
  Clean local Codex and Claude Code marketplace installs each report version
  `0.4.0`; their cached payloads match the source bytes, expose one skill, and
  enable no hook. The current Codex CLI accepts the marketplace and plugin, but
  its separate plugin-creator preflight rejects Claude's required
  `disable-model-invocation` field; D-036 records why the real shared-payload
  install is the Codex ingestion proof. Both POSIX hook samples run the packed
  checker against mixed changes. The Windows commands have exact contract
  tests. The Windows release build is configured and contract-tested to run
  both against its native binary for releases after the immutable `0.3.1`
  source; that hosted step has not run yet.

  Formatting, strict Clippy, all 116 Rust tests, the locked release build, 31
  release tests with one Windows-only skip, 19 skill tests, 10 eval tests, and
  Promptfoo config validation pass. Project-owned source stays at or below
  score 7. A real macOS arm64 archive reports `complexity 0.4.0`, passes its
  SHA-256 check, and includes the plugin, both marketplaces, manual agent
  bundle, and four watercolor images. Tar and zip outputs stay byte-identical
  when only source timestamps change. An initial review found one plugin
  manifest traversal fault and unstable archive metadata; focused red-green
  fixes closed both. Final security and correctness reviews found no remaining
  issue.

  Canonical delivery validation on 2026-08-05 copied the generated payload,
  marketplaces, release flow, documentation, and four watercolor images into
  `andrea-sdl/complexity-evaluator` without deleting the target's existing
  work. The target matched the verified source after excluding local settings
  and build caches. It passed formatting, strict Clippy, all 116 Rust tests,
  the locked release build, 31 release tests with one Windows-only skip, 19
  skill tests, 10 static eval tests, Promptfoo config validation, plugin drift
  checks, and a new arm64 archive checksum check. Claude Code validates both
  manifests and installs the local plugin with one skill and no hooks. Codex
  installs the local plugin as version `0.4.0` with `AVAILABLE` and
  `ON_INSTALL` policy.

  Live evidence on 2026-08-05: an explicit Codex invocation scored the public
  JavaScript fixture `deliveryWindow` at 11 and returned `REVISE` with a lookup
  table refactor. It made no edit. That first run also read local repository
  guidance, so later checks use an isolated workspace with only the fixture and
  required skill files. Claude Code installed the plugin but its explicit run
  stopped before it read the fixture because its OAuth session had expired.
  The short Promptfoo run `eval-KkI-2026-08-05T15:04:12` passed all four
  refactor cases: JavaScript 11 to 1, TypeScript 10 to 1, PHP 2 to 1, and Rust
  3 to 2. Temporary plugin installs, marketplaces, and workspaces were removed.

  Remaining: refresh Claude Code authentication, then run one explicit Claude
  invocation in an isolated workspace. This is an external account blocker.

## CX-024: Add an optional cognitive-load gate

- Status: done
- Owner: root
- Depends on: CX-021
- Scope: add `--max-cognitive-load N` without changing `core-v1` scores,
  existing output, or exit status when the option is absent. Report and gate a
  deterministic inline conditional return that combines a conditional return,
  a compound Boolean test, and an explicit branch cast. Add the rule for all
  five supported languages, using each parser's native conditional-return and
  cast syntax. Update the canonical skill, generated plugin, hook behavior,
  docs, and focused tests.
- Success: the TypeScript `record` regression returns score 2 and exits 0 by
  default; with `--max-cognitive-load 2`, it returns one stable diagnostic and
  exits 1. A guard-return refactor passes. Parse and input failures keep exit 2
  priority. The checker asks an AI to simplify and retry.
- Evidence: focused red tests first rejected the unknown CLI option. The final
  Rust suite passes 46 CLI tests, 32 JavaScript tests, 22 PHP tests, 8 Python
  tests, 6 Rust tests, 1 PHP compatibility test, and the benchmark tests.
  Formatting, strict Clippy, and the locked release build pass. Release tests
  pass 30 cases with one Windows-only skip. The skill checker passes 20 tests;
  the static Promptfoo eval passes 10 tests and its config validates. Plugin
  synchronization and drift checks pass.

  The final release binary proves the requested TypeScript case: its score is
  2, its load is 3, and `--max-cognitive-load 2` reports
  `cognitive_load.inline_conditional_return` at line 2, column 10. The
  guard-return refactor has score 1 and no readability violation. A Rust
  self-check with `--max-complexity 7 --max-cognitive-load 2 src` reports
  maximum score 6 and no cognitive-load violations.

## CX-022: Publish test release 0.3.1

- Status: done
- Owner: root
- Depends on: CX-020
- Scope: make the smallest version-only update from `0.3.0` to `0.3.1`, run
  the documented release checks, and publish the signed annotated
  `complexity-v0.3.1` tag. Do not change scores, schemas, dependencies, or the
  tagged source. Fix only release workflow behavior needed to publish that
  unchanged tag.
- Success: current version contracts and CLI expectations report `0.3.1`;
  historical `0.3.0` evidence stays unchanged; the signed release commit and
  tag reach `main`; and the tag-triggered release workflow starts. If the
  hosted workflow itself fails, fix it without moving the tag and rerun the
  same tagged source through the workflow's manual recovery input.
- Preparation evidence: the exact tag validator, formatting, strict Clippy,
  all 116 Rust tests, the locked release build, 11 release tests, 16 skill and
  hook tests, 10 eval tests, and Promptfoo config validation pass. The release
  binary reports `0.3.1`. A real macOS arm64 archive passed its SHA-256 check
  and contains the binary, `README.md`, `LICENSE`, and all manifest-listed
  agent files. Signed commit `d1d6309` and signed annotated tag
  `complexity-v0.3.1` are on GitHub and verified. Actions run `30927294187`
  started for that tag but failed before validation because the Ubuntu 22.04
  system Python lacked `tomllib`. A focused red-green workflow contract now
  requires Python 3.11 and an existing-tag manual recovery path. Release
  recovery checks pass: YAML parsing, the exact tag validator, formatting,
  strict Clippy, all 116 Rust tests, the locked release build, 12 release
  tests, 16 skill and hook tests, 10 eval tests, and Promptfoo config
  validation. Recovery run `30928244207` passed the fixed Python and tag checks
  but exposed three Ubuntu-only test faults: two assertions rejected normal
  probe completion that the spec allows, and a parallel debug timing check
  exceeded the release hook budget. The portable tests now accept both safe
  probe outcomes, use a wider debug guard, and keep a separate five-second
  optimized gate. The immutable `0.3.1` validation confirms tag commit
  `d1d6309` and runs the full suite. Package jobs verify and build the unchanged
  tag without the overlay. Recovery run `30929482127` passed all Rust and
  optimized hook-budget gates, then found that the Git conflict fixture lacked
  a committer identity on Linux and never created its conflict. Repair commit
  `4d4fadd` gives that merge a local test identity, disables signing, and proves
  that `source.ts` is unmerged. The pinned validation overlay includes both
  corrected test files. Recovery run `30930233848` passed the full validation,
  all five native package builds, package checksum checks, and GitHub release
  creation. GitHub published five archives and their five SHA-256 files at
  `complexity-v0.3.1`. The signed tag still points to the unchanged `d1d6309`
  release commit.

## CX-023: Clarify agent setup and add README visuals

- Status: done
- Owner: root
- Depends on: CX-020
- Scope: state the current manual Codex and Claude Code install paths without
  claiming that plugin packaging exists; enforce the explicit-only skill
  contract in both hosts; and add one hero plus three small README images.
  Keep `CX-021` ready and do not add plugin manifests in this task.
- Success: the root and agent docs name the current host paths and the plugin
  gap; Codex and Claude Code metadata both block implicit invocation; focused
  tests fail before and pass after the metadata and path fixes; the four image
  files are small, valid, text-free JPEGs with useful alt text; release
  archives contain the images used by their README; bundled hook docs name the
  project-root requirement; and all project gates pass.
- Evidence: current OpenAI and Claude Code docs confirm the manual skill paths,
  explicit-only metadata, and plugin-manifest gap. Focused tests first failed
  for the missing Claude metadata, stale Codex path, absent archive images, and
  weak hook working-directory text, then passed after each fix. The final run
  passed 18 skill tests, 14 release tests, 10 eval tests, Promptfoo config
  validation, formatting, strict Clippy, all 116 Rust tests, and the release
  build. The CLI self-check reports maximum score 6 and Python span 50. A real
  macOS arm64 archive passed its SHA-256 check and contained the README, all
  four matching images, and the agent bundle. The final watercolor set has one
  1600 by 900 JPEG at 692,664 bytes and three 720 by 720 JPEGs from 252,733 to
  291,326 bytes. Independent review found no remaining actionable issue.

## CX-025: Publish release 0.4.0

- Status: in-progress
- Owner: root
- Depends on: CX-024
- Scope: validate the current `0.4.0` source, publish a signed release
  preparation commit, and push one signed annotated `complexity-v0.4.0` tag.
  Do not change scores, schemas, dependencies, or release contents.
- Success: all documented release checks pass; the signed commit and tag reach
  `main`; the tag-triggered workflow publishes all five archives and checksum
  files; and the GitHub release points to the unchanged signed tag.
- Preparation evidence: the first release test run found that
  `readability_findings` exceeded the project score limit. A small split kept
  the same schema checks and output while lowering each function to the limit.
  The tag validator, plugin drift check, formatting, strict Clippy, all 118
  Rust tests, the locked release build, 31 release tests with one Windows-only
  skip, 20 skill tests, 10 static eval tests, and Promptfoo config validation
  pass. A real macOS arm64 archive reports `complexity 0.4.0`, passes its
  SHA-256 check, and contains the binary, docs, manual agent bundle, plugin,
  and both marketplace files.
