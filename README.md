# complexity

`complexity` is one Rust CLI for per-function cognitive complexity in
JavaScript, TypeScript, PHP, Rust, and Python. It scans one file, mixed files,
or directories and gives deterministic text or JSON reports.

Version: `0.3.1`

JSON schema: `2`

Score profile: `core-v1`

The report also gives syntax facts for control depth, condition shape, function
line span, and functions per file. These signals do not change scores or exit
codes.

## Build

```sh
cargo build --release
```

The binary is `target/release/complexity`. To install it from this checkout:

```sh
cargo install --path .
```

## Releases

Push a tag such as `complexity-v0.3.1` to build release archives for Linux
x64 and arm64, macOS Intel and arm64, and Windows x64. The tag version must
match `Cargo.toml`.

Each archive contains the binary, this README, the GPL-2.0 license, and the
`agent` folder. A separate `.sha256` file lets you check the archive before
use. The packager copies only the files listed in `agent/MANIFEST.txt`; local
cache or output files cannot enter a release by accident.

The release flow lives in this repository. It validates the project once,
builds each native target with the locked dependency graph, and creates the
GitHub Release only after all five packages pass.

## Agent support

`agent/skills/complexity-cli` is the explicit-only Codex and Claude skill.
It keeps these policy levels:

| Metric | Target | Hard limit |
| --- | ---: | ---: |
| Cognitive complexity score | 10 | 15 |
| Maximum control depth | 3 | 4 |
| Inclusive line span | 50 | 80 |
| Predicates in one condition | 4 | 6 |

`agent/hooks/` has optional Codex and Claude merge samples for POSIX and
Windows. They check supported files changed during the task and stay silent
for unsupported-only changes. Copying the skill does not enable a hook. See
[`agent/README.md`](agent/README.md) for install and merge steps.

Run the four Promptfoo refactor cases manually against a built binary. The
cases ask Codex to improve one JavaScript, TypeScript, PHP, or Rust function:

```sh
cd agent/eval
npm ci --ignore-scripts
npm test
npm run validate
COMPLEXITY_BIN=../../target/release/complexity npm run eval
```

This live eval needs Node.js 24 or later, Python 3, PHP, `rustc`, network
access, and a Codex login or API key. Each case starts with a measured skill
finding and a passing behavior test. Codex must run the real skill before and
after its edit, change only the source file, keep the behavior test green,
lower the function score, and meet all skill targets. It must also explain the
refactor and name the next useful improvement.

An independent assertion checks the changed files, behavior test, measured
before and after metrics, target limits, and the ordered checker-edit-checker
record. The cases are short and readable in `agent/eval/cases.yaml`. The live
model eval is manual. Release CI runs only the static eval tests and Promptfoo
config validation.

## Usage

```text
complexity [--language javascript|typescript|php|rust|python]...
           [--format text|json]
           [--max-complexity N]
           [--stdin-filename PATH]
           <path...|->
```

Text is the default format. The default maximum complexity is `15`.
`--max-complexity` marks functions above the limit and controls the result
status; it does not hide or change scores.

Scan a mixed project:

```sh
complexity --format json src tests
```

Use repeatable language filters:

```sh
complexity --language javascript --language php .
```

Omit `--language` to select all five language families. A directory scan skips
files outside the selected families. An explicit supported file that a filter
excludes is an error.

## Standard input

Use `-` as the sole input and give exactly one language:

```sh
complexity --language typescript --stdin-filename snippet.tsx -
```

`--stdin-filename` is optional. It sets the virtual report path and parser mode.
For example, use a `.tsx` name for TypeScript JSX. Without this option, the
tool uses `stdin.js`, `stdin.ts`, `stdin.php`, `stdin.rs`, or `stdin.py`.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Analysis finished and no function is above the limit. |
| `1` | Analysis finished and at least one function is above the limit. |
| `2` | Usage or discovery failed, or at least one selected input could not be read or parsed. |

Exit `2` takes priority over exit `1`. A failed file does not get false zero
scores or signals. Valid sibling files stay in the report.

## Input safety

PHP, Rust, and Python reject syntax trees deeper than 512 parent-child edges
before recursive analysis. The file gets the normal `parse_error` result and
exit `2`.

File reads stay inside a capability rooted at the resolved current working
directory. A path swap cannot redirect a selected read outside that root.

JavaScript and TypeScript use iterative visitor paths for logical, unary,
parenthesized, and conditional expression chains. Large logical chains keep
the normal score and signal rules.

A cheap byte count sends delimiter-heavy or question-heavy JavaScript and
TypeScript through an isolated full-analysis probe first. If that probe
aborts, the file gets a normal `parse_error` and exit `2`. If it completes,
the normal analysis keeps valid regex, optional fields, scores, and
diagnostics unchanged.

Text reports and usage errors show control and bidirectional formatting
characters as visible escapes. JSON keeps the original strings.

## Project documents

- [SPEC.md](SPEC.md) defines the CLI, score, signal, and schema contracts.
- [DESIGN.md](DESIGN.md) records design decisions.
- [TASKS.md](TASKS.md) tracks implementation and checks.
- [COMPATIBILITY-RESULTS.md](COMPATIBILITY-RESULTS.md) records old-to-v2 and
  live Sonar evidence.
- [BENCHMARKS.md](BENCHMARKS.md) records speed, memory, and binary size.
- [SONAR-COMPATIBILITY.md](SONAR-COMPATIBILITY.md) states the exact Sonar
  compatibility boundary.

Version `0.3.0` makes no compatibility promise for the old command names, JSON
schema v1, or a public Rust library API.

## License

This project uses the GNU General Public License v2.0 only. See
[`LICENSE`](LICENSE).
