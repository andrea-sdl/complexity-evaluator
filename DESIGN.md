# `complexity` design record

Every public or architecture change must update this file, `SPEC.md`, and
`TASKS.md`.

## D-001: One clean v2 project

Decision: replace `complexity-js` and `complexity-php` with one top-level Rust
project and one `complexity` binary at version `0.2.0`.

Why: one command can scan mixed repositories and stdin. The old projects are
unreleased `0.1.0` experiments, so preserving two serializers and wrapper
commands would retain the split inside the merge.

The old folders were removed only after code, fixtures, evidence, benchmarks,
documents, tests, and reviews moved and passed.

## D-002: One shared shell, four parser engines

Decision: share CLI parsing, discovery, report types, sorting, formatting,
summaries, and exits. Keep the JavaScript and TypeScript, PHP, Rust, and Python
parsing, function discovery, scoring, diagnostics, positions, and signal walks
in separate language modules. Dispatch with an explicit language match, not a
parser trait.

Why: the public flow is stable and shared. Parser ASTs and rules are materially
different. A generic framework would hide those differences and add indirection.

## D-003: Add only the two required grammar crates

Decision: use the exact existing versions of `ignore`, `oxc`, `serde`,
`serde_json`, `tree-sitter`, and `tree-sitter-php`. Add only
`tree-sitter-rust 0.24.2` and `tree-sitter-python 0.25.0`. Do not add a CLI
framework, semantic engine, parallel runtime, or signal library.

Why: the existing Tree-sitter runtime needs compiled grammar functions for the
two new languages. Both grammar crates use the existing
`tree-sitter-language` and C build stack. Manual option parsing and sequential
dispatch keep the new surface small. The owner approved both production
dependencies on 2026-07-28.

## D-004: Preserve language scores

Decision: keep the existing JavaScript, TypeScript, and PHP `core-v1` score,
contribution, callable, position, parser-mode, and fail-closed contracts
unchanged.

Why: the merge changes delivery and adds evidence. It must not silently change
the validated measures.

The JS/TS profile stays tied to the reviewed SonarJS S3776 revision in
`SONAR-COMPATIBILITY.md`. The PHP profile keeps two deliberate differences
from the reviewed SonarPHP source: `match` is structural, and nested callables
own separate scores instead of adding to a parent. Project fixtures were
written from the documented behavior; they do not copy Sonar source or
fixtures.

Rust follows the reviewed SonarRust cognitive-complexity visitor at revision
`9539d0c59f8965663fd3efa616a492a4c65315f3`. Python follows the reviewed
SonarPython visitor at revision
`322e2dcb2a1bcab654614fffa394a8db43b9ef16`. Project fixtures describe the
public rules without copying analyzer fixtures or source.

## D-005: Schema v2 is a deliberate break

Decision: use one compact deterministic JSON schema v2 and remove JSON v1.

Why: v1 fixes the old tool names and exact fields. Signals and a combined tool
identity require a new contract. A clean break is smaller and more honest than
compatibility modes.

## D-006: Repeatable language filters

Decision: omitted language filters select all five supported families.
Repeated filters select a union. Extensions still choose exact parser modes.

Why: one invocation must handle both focused checks and mixed repositories
without an `auto` mode that conflicts with repeated values.

## D-007: Stdin is one explicit virtual file

Decision: `-` is the sole input, requires one language, and accepts a safe
relative `--stdin-filename`. Rust defaults to `stdin.rs`; Python defaults to
`stdin.py`.

Why: one stream has one parser mode. A virtual filename preserves stable IDs
and enables `.tsx` without mixing filesystem and stream ordering.

## D-008: Signals are syntax facts

Decision: report active control depth, normalized condition shape, inclusive
function span, and file function count. Signals never affect score or exit.

Why: these facts help an AI review nested flow and dense predicates without
claiming to detect responsibility, coupling, or architecture quality.

The signal walk follows callable boundaries but remains separate from
language-specific score contributions. Score-specific exceptions such as JSX
logical suppression or PHP pipe must not alter syntax facts. A control test or
selector runs before its structural region, so only the selected body or
result raises active control depth.

## D-009: Fail closed without false zeroes

Decision: a failed file reports `signals: null`, no functions, incomplete
status, and exit `2`.

Why: zero would make an unmeasured file look simple. Successful sibling files
remain useful for diagnosis.

## D-010: Compact text, full JSON evidence

Decision: text shows per-function score and signal maxima. JSON includes every
condition record.

Why: text stays scannable while JSON gives AI and CI callers complete stable
evidence.

## D-011: Sequential execution and measured performance

Decision: keep one sorted sequential analysis flow. The release must benchmark
the old JS/TS and PHP corpora plus a five-language mixed corpus and Rust
self-analysis. The target is no more than a 25% median runtime regression
against each old single-language baseline on the same machine.

Why: current baselines are already below one tenth of a second for 2,000
functions. Parallelism would add coordination and ordering cost without a
measured need.

## D-012: CLI-only compatibility promise

Decision: the CLI is the supported interface. Internal library entry points
exist for tests but are not a stable public Rust API.

Why: the product is a portable command and deterministic report. Preserving the
old PHP library surface would add a second compatibility contract with no known
consumer.

## D-013: License remains unset

Decision: do not declare a project license until the owner chooses one.

Why: dependency licenses do not choose the project license.

## D-014: Condition kinds name source syntax

Decision: condition records use `if`, `elseif`, `else_if`, `while`,
`do_while`, `for`, and `ternary`. PHP keeps `elseif` separate from spaced
`else if`.

Why: stable syntax labels let an AI distinguish forms that have different
parser and control-region boundaries without reading source text.

## D-015: Benchmarks keep old corpora and one small mixed case

Decision: one dependency-free Rust generator reproduces the four old
2,000-function corpus trees byte for byte. It also writes one JavaScript, one
TypeScript, one PHP, one Rust, and one Python file with ten functions each.
Single-language runs keep the old warm-up and sample methods. The small mixed
case uses two warm-up batches and seven measured batches of 100 runs. If the
old single-run timer cannot resolve the 25% boundary, a paired old/new 25-run
batch result controls the gate while the original samples stay in the record.
Cargo registers the generator as a test target so `cargo test --all-targets`
locks its corpus counts and bytes.

Why: exact old source makes the runtime gates comparable. The mixed case tests
one full combined command without treating it as a language-throughput
baseline. Paired batches reduce timer rounding without hiding the old method.

## D-016: Discovery uses project-owned ignore files

Decision: directory discovery uses `.ignore` and `.gitignore` files only.
`.ignore` has higher priority than `.gitignore`, and deeper files win within
each type. Global Git ignore files and `.git/info/exclude` do not apply.
Canonical targets of discovered file symlinks use the same rules. Explicit
files override them.

Why: the same project input must produce the same file set on each machine.
The separate matchers also keep discovered symlinks from bypassing or changing
the normal ignore rules.

## D-017: PHP retries only one known grammar gap

Decision: when the first PHP tree has an error, mask a known reserved-word
class-like constant name in a same-length parser copy and parse the full file
again. The retry applies only to class, trait, interface, or enum members and
the names frozen in `SPEC.md`. Accept it only when the second tree is clean.
Use the original source for all scores and positions.

Why: PHP permits these names, but `tree-sitter-php 0.24.2` rejects them in the
first constant declarator. The narrow clean-tree check fixes that grammar gap
without scoring a recovered or unrelated invalid tree.

## D-018: PHP score flow operators stay one sequence

Decision: the PHP score walker flattens parenthesized `&&`, `||`, and `|>`
operators into one sequence. The outermost operator selects the flat logical
or nested pipe increment rule in `SPEC.md`. Signal operators stay independent
from this score-only rule.

Why: this preserves the frozen `core-v1` handling for mixed logical and PHP
pipe expressions. Splitting pipe into a separate subtree changes nesting and
breaks the old contribution vectors.

## D-019: Nested callables keep one cross-language ownership rule

Decision: Rust nested functions and closures, and Python nested functions and
lambdas, get separate results. Parent score and signal walks stop at each
nested callable.

Why: one callable should own only the code that runs as that callable. This
keeps the existing JavaScript, TypeScript, and PHP rule and gives an AI a
specific function to revise.

This differs from the reviewed SonarRust and SonarPython visitors, which add
nested callable content to an outer function in some cases and do not report
each nested callable as a separate rule issue.

## D-020: Python match stays score-neutral

Decision: Python `match` adds no score or control-depth region in `core-v1`.
Case guards can still produce condition records and `and` or `or` score
contributions.

Why: the reviewed SonarPython cognitive-complexity visitor does not score
`match`. Rust `match` remains structural because the reviewed SonarRust visitor
does score it. The language-specific modules keep this difference visible.

## D-021: Version 0.3 extends schema v2

Decision: add Rust and Python in version `0.3.0` without changing JSON schema
version `2` or the `core-v1` profile name.

Why: new language labels and function records fit the existing schema. Existing
fields, ordering, and meanings do not change. A schema bump would add migration
work without a data-shape change.

## D-022: Self-analysis keeps a max-score ratchet

Decision: an integration test runs the built public CLI against all Rust files
in `src` with `--max-complexity 7`. It requires complete analysis, no
violations, and a summary maximum at or below `7`. The test pins the ordered
source paths and each file's function count. A source change must update those
counts explicitly.

The implementation target is `5`, but the repository test limit is `7`. This
limit applies only to this project's source. It does not change the public
default, the JSON schema, or the `core-v1` profile.

Why: the test catches a clear complexity regression through the same command
that users run. A two-point margin lets a parser rule grow when keeping the
rule local is easier to read than splitting it into shallow helpers.
