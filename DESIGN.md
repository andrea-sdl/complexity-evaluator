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

D-026 later adds one security dependency and supersedes the dependency count
in this decision.

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

Status: superseded by D-031.

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

A second test discovers Python code below `agent` and `release`. It applies the
same score limit and the skill targets for depth, span, and predicates. The
release and checker code stays under this ratchet. The test also checks the
ordered path list returned by the public CLI, so a selected file cannot
disappear without a test failure. Installed Promptfoo dependencies are
excluded because they are third-party code, not project source.

A third check analyzes project-owned JavaScript below `agent/eval`, excluding
the deliberate refactor fixtures and installed dependencies. It applies the
same score and signal limits. This keeps the eval runner and assertion code
inside the CLI's own policy instead of leaving it untracked.

The implementation target is `5`, but the repository test limit is `7`. This
limit applies only to this project's source. It does not change the public
default, the JSON schema, or the `core-v1` profile.

Why: these tests catch a clear complexity regression through the same command
that users run. A two-point margin lets a parser rule grow when keeping the
rule local is easier to read than splitting it into shallow helpers. The
Python and JavaScript checks keep the release, hook, and eval tools under the
same small-code policy without treating fixtures or dependencies as project
code.

## D-023: Bound recursive Tree-sitter analysis

Decision: before project-owned recursive walks, scan each PHP, Rust, and Python
Tree-sitter tree with an explicit stack. Reject a path deeper than 512
parent-child edges with the existing fail-closed parse-error result. Check both
PHP trees when its reserved-word retry runs.

Why: parser input is untrusted. The parser can build a much deeper tree than
the Rust thread stack can safely process. An iterative precheck gives one clear
limit without changing normal `core-v1` scores, contributions, ranges, or
diagnostics.

## D-024: Keep JavaScript expression work iterative and linear

Decision: use explicit work stacks for JavaScript and TypeScript logical,
unary, parenthesized, and conditional expression chains. Apply the logical
walks in callable discovery, scoring, signal collection, and Boolean-shape
collection. Collect logical contributions with an in-order work stack. Check
homogeneous JSX logical chains iteratively and keep only scoped root spans in
a `HashSet`. Keep sparse source-position checkpoints every 128 Unicode scalars
on each long line.

Why: deeply associated logical trees must not consume the Rust call stack.
JSX span-list membership and removal must not grow quadratically. Recounting
every scalar from the start of a long line for each result must not grow
quadratically. The work stacks preserve source order, and sparse checkpoints
keep one-based Unicode-scalar columns without a full per-character index.

## D-025: Escape untrusted terminal text

Decision: escape Unicode control characters and bidirectional formatting
controls before a text report inserts a function ID, function name,
diagnostic path, or diagnostic message, and before a usage error reaches
standard error. Use Rust default escape forms. Do not alter JSON strings.

Why: source names and paths can contain terminal control characters. Escaping
keeps one text result on one line and blocks terminal control injection. JSON
must retain the exact machine-readable value. Normal text output stays
unchanged.

## D-026: Anchor selected files to directory capabilities

Decision: pin `cap-std 4.0.2` and open the canonical current working directory
as a `cap_std::fs::Dir`. Cache each selected parent directory and its
`DirEntry` values. Open a known nested file through its cached `DirEntry`.
Read root files through the root capability. A missing cached entry, blocked
read, or failed read uses the existing `io_error` result and exit `2`.

Why: checking a canonical path and later reading the absolute path leaves a
race. Another process can replace the selected file with a symlink between
those steps. The cached parent capability anchors each later nested path
lookup. The root capability anchors root-file reads. This fixes the
containment race without changing discovery, ordering, report IDs, parser
selection, or normal source bytes.

## D-027: Probe high-risk Oxc inputs in a child process

Decision: count raw JavaScript and TypeScript opening delimiters and question
marks in one byte pass. When either count exceeds the `2048` trigger in
`SPEC.md`, run the exact source through the same full CLI analysis in a child
process first. Use a private environment marker to prevent recursion. A normal
child exit lets the parent keep normal analysis. An abnormal child result
becomes a structured `parse_error`.

Why: Oxc or a project visitor can abort on extreme syntax before it returns a
report. Raw counts cannot tell code from strings, regex, optional TypeScript
fields, or comments, so they must not reject input. The child proves that the
same full path returned normally before the parent uses it. Normal sources pay
for one simple byte count and no process start. This keeps the old performance
gate while accepting risky-looking valid code. The marker is an internal
recursion guard, not a boundary against a hostile caller-controlled
environment.

## D-028: Release one binary and one agent bundle per native target

Status: partially superseded by D-031 for the repository layout and license.

Decision: use `complexity-vX.Y.Z` tags in the monorepo and require the tag
version to match `Cargo.toml`. Build five common native targets on matching
GitHub-hosted runners. Package each binary with `README.md` and the same
tracked `agent` tree. Add a SHA-256 file for each archive. Transfer build
outputs with GitHub-owned artifact actions, then let one final job create the
release with the GitHub CLI after every build passes.

Keep the skill, hook samples, and eval below `agent/`. The skill stays
explicit-only. Hook samples do not install or enable themselves. D-030
supersedes the model-free eval choice: static eval checks run in release CI,
while the live Codex refactor eval stays manual.

Ship separate POSIX and Windows hook samples because their common Python
launchers differ. Bundled samples call the project-local checker. The skill
reference also keeps examples for a home-directory skill install. This makes
each path explicit and avoids an install script that changes agent settings.

The optional hook checker records dirty-file hashes and the Git head on each
`UserPromptSubmit`. A later `Stop` checks supported files changed from that
snapshot, including commits made in the turn. Every new user prompt replaces
the snapshot. A blocked Stop continues inside the same turn, so its automatic
retry keeps the snapshot without a separate blocked-state flag.

Use `agent/MANIFEST.txt` as the release allowlist. The packager rejects an
agent-directory symlink, any symlink in a listed path, unsafe or duplicate
manifest paths, missing files, and resolved sources outside that directory. It
copies no other file. This keeps local caches, test output, and stray secrets
out of public archives without making packaging depend on a Git checkout.

Why: this repository can release more than one project, so a plain `vX.Y.Z`
tag is ambiguous. Native runners avoid a new cross-compile toolchain. One
archive gives a person or agent the CLI and its optional self-check tools
without adding install side effects. A final release job prevents a partial
release when one platform fails. Package signing and package-manager work can
be added later if users need it.

## D-029: Keep policy evals in Promptfoo and normal YAML

Status: superseded by D-030. This record explains the former deterministic
checker-only eval.

Decision: keep one private Node package under `agent/eval` and pin Promptfoo
`0.121.20`. Do not create a root Node workspace. Put the four policy cases in
one short YAML file. Each case names a fixture, expected outcome, and expected
checker exit. One shared exact-output assertion applies to all cases.

Use one small Node command provider to run the existing Python checker. The
provider converts checker exits `0`, `1`, and `2` into result text because
`REVISE`, `FAIL`, and `BLOCKED` use expected nonzero exits. Promptfoo owns the
case matrix and assertions. The old Python eval runner is removed.

Run with `--no-cache`, `--no-share`, and `--no-write`. A committed
`promptfoo.env` disables telemetry, update checks, and SQLite WAL mode, and
puts Promptfoo's local state below the ignored `agent/eval/.promptfoo` path.
Promptfoo `0.121.20` needs this state path to complete cleanly when telemetry
and writes are off; without it, Node can report an unsettled top-level await
and exit `13` after a passing run.

Keep optional packages because Promptfoo puts its required native SQLite
binding in that group. Pin `ai` to `6.0.237`, `adm-zip` to `0.6.0`, and
`sharp` to `0.35.3`. The lock resolves those exact versions, and a live
`npm audit` of the full install reports zero known vulnerabilities. The
optional model providers are not used by this eval.

Why: Promptfoo gives one common eval command and a clear result table. Native
YAML keeps the rules easy to review. The adapter exists only because a command
provider treats any nonzero process exit as an error, while this checker uses
nonzero exits for valid policy outcomes. Local state avoids network data and a
known shutdown failure without adding a custom eval framework.

## D-030: Use a manual Codex refactor eval

Decision: keep Promptfoo `0.121.20` and the short YAML case list, but replace
the checker-only provider and Python policy fixtures with four refactor cases:
JavaScript score, TypeScript control depth, PHP condition predicates, and Rust
function span. Use the Promptfoo Codex SDK provider in one isolated writable
workspace per case. Keep runs sequential.

The runner first runs each language behavior test and measures the named
function with the supplied real binary. It copies the explicit skill into the
isolated workspace. Codex must use that skill, run the behavior test and
checker before and after its edit, change only the named source file, lower the
measured score, meet all skill targets, and report both the refactor and the
next useful improvement.

Do not use the model as the judge. One shared assertion compares the final
workspace with the saved baseline, reruns the language test and real CLI, and
requires an ordered checker-edit-checker record from the Codex run. It also
checks the structured report against the measured before and after metrics.
Accept only a real Python checker command at a shell command boundary,
including a newline. Reject commands that only print the checker name.

Copy only the Codex auth files needed for the isolated run. If a failed source
workspace is kept for diagnosis, delete the temporary Codex home first. This
keeps login data out of retained eval evidence.

Keep live model runs manual. They need a Codex login or API key, network
access, Node.js 24 or later, Python 3, PHP, and `rustc`. Release CI runs only
the deterministic Node tests and Promptfoo config validation. It must not call
Codex.

Why: the former cases proved that the checker classified fixed inputs. They
did not prove that an AI could use the skill to improve code. These cases test
the intended work: find a measured problem, preserve behavior, make a small
edit, prove a lower score, and explain the next step. The independent assertion
keeps model claims out of the acceptance path.

## D-031: Use the standalone repository and GPL-2.0-only license

Decision: make `andrea-sdl/complexity-evaluator` the canonical repository.
Keep the two available project commits through a subtree history import, then
apply the current project work in one signed merge. Keep the former source tree
unchanged until the owner asks to remove it.

Run the GitHub Actions workflow from the repository root. Keep the existing
`complexity-vX.Y.Z` tag format so release names do not change. Use the target
repository's GNU General Public License v2.0 only, declare it in Cargo metadata,
and include `LICENSE` in each release archive.

Why: the project no longer belongs below an umbrella repository path. Keeping
both histories retains the target's license choice and the available project
record. Shipping the license with each binary archive keeps the distribution
terms next to the program.

## D-032: Retry an unchanged release tag after a workflow fault

Decision: keep tag pushes as the normal release trigger. Set up Python 3.11 in
the validation and native package jobs because the packager uses `tomllib`.
Also allow a manual dispatch with one existing `complexity-vX.Y.Z` tag. The
workflow checks out that tag and applies the normal tag and package gates. It
never moves or replaces the tag.

Why: the first `0.3.1` run used the Ubuntu 22.04 system Python and stopped
before validation because that Python lacked `tomllib`. A workflow fix cannot
change an immutable release tag. A narrow manual retry can run the fixed
workflow against the same tagged source without a tag rewrite or a second
release path.

## D-033: Keep parser and hook-budget tests portable

Decision: a risky JavaScript or TypeScript probe test accepts either complete
normal analysis or a structured fail-closed result. This matches D-027 and the
public parser contract. Debug test binaries get a 15-second timing guard to
avoid shared-runner and parallel-test noise. The release workflow also runs
both hook-budget tests alone, in sequence, with an optimized binary and the
real five-second limit.

The immutable `0.3.1` recovery overlays only the corrected JavaScript and Git
conflict tests during validation. It first confirms that the tag still points
to recorded commit `d1d6309`. The overlay is pinned to repair commit `4d4fadd`,
so a later dispatch ref cannot change either test. The workflow then runs the
full suite and the separate optimized hook-budget checks. Package jobs do not
use the overlay and confirm the same tag commit before they build. This
exception matches only `complexity-v0.3.1`; later manual retries use their
tagged tests unchanged.

Why: parser safety depends on whether the child can complete within the host's
stack. Success and fail-closed are both safe and specified outcomes. A debug
binary running beside other large tests does not measure the installed CLI's
hook cost. A test-only overlay fixes validation without changing the signed
source, skipping a gate, or changing any release artifact.

## D-034: Prove the Git conflict fixture reached the intended state

Decision: the hook test that needs an unmerged file supplies a local identity
and disables commit signing for its merge command. It then checks that Git left
exactly `source.ts` unmerged before it tests the hook response.

Why: a clean Linux runner had no Git identity. Git stopped before it attempted
the merge, but the old helper treated every nonzero merge exit as a conflict.
The hook then had no changed file and returned a valid pass. The test setup
must prove its precondition before it checks product behavior.
