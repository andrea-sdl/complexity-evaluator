# complexity

`complexity` is one Rust CLI for per-function cognitive complexity in
JavaScript, TypeScript, and PHP. It scans one file, mixed files, or directories
and gives deterministic text or JSON reports.

Version: `0.2.0`

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

## Usage

```text
complexity [--language javascript|typescript|php]...
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

Omit `--language` to select all three language families. A directory scan skips
files outside the selected families. An explicit supported file that a filter
excludes is an error.

## Standard input

Use `-` as the sole input and give exactly one language:

```sh
complexity --language typescript --stdin-filename snippet.tsx -
```

`--stdin-filename` is optional. It sets the virtual report path and parser mode.
For example, use a `.tsx` name for TypeScript JSX. Without this option, the
tool uses `stdin.js`, `stdin.ts`, or `stdin.php`.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Analysis finished and no function is above the limit. |
| `1` | Analysis finished and at least one function is above the limit. |
| `2` | Usage or discovery failed, or at least one selected input could not be read or parsed. |

Exit `2` takes priority over exit `1`. A failed file does not get false zero
scores or signals. Valid sibling files stay in the report.

## Project documents

- [SPEC.md](SPEC.md) defines the CLI, score, signal, and schema contracts.
- [DESIGN.md](DESIGN.md) records design decisions.
- [TASKS.md](TASKS.md) tracks implementation and checks.
- [COMPATIBILITY-RESULTS.md](COMPATIBILITY-RESULTS.md) records old-to-v2 and
  live Sonar evidence.
- [BENCHMARKS.md](BENCHMARKS.md) records speed, memory, and binary size.
- [SONAR-COMPATIBILITY.md](SONAR-COMPATIBILITY.md) states the exact Sonar
  compatibility boundary.

Version `0.2.0` makes no compatibility promise for the old command names, JSON
schema v1, or a public Rust library API.

## License

This project has no license yet.
