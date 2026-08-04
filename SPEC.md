# `complexity` specification

Version: `0.3.0`

Schema: `2`

Score profile: `core-v1`

## Goal

Give an AI and a human one deterministic CLI for JavaScript, TypeScript, PHP,
Rust, and Python function complexity. Preserve the existing language score
rules and add syntax-only signals for control depth, condition shape, function
span, and functions per file.

Signals are evidence, not quality judgments. They never affect exit status.

## Command

```text
complexity [--language javascript|typescript|php|rust|python]...
           [--format text|json]
           [--max-complexity N]
           [--stdin-filename PATH]
           <path...|->
```

- At least one path or `-` is required.
- `--format` defaults to `text`.
- `--max-complexity` defaults to `15` and accepts a non-negative integer.
- `--language` is repeatable. Omit it to select all five language families.
- Duplicate language values are accepted once.
- `--help` and `--version` are valid only as the sole argument.
- Unknown options, duplicate scalar options, missing values, and unsupported
  language or format values are usage errors.

Exit codes:

| Code | Meaning |
| --- | --- |
| `0` | Analysis completed and every function score is at or below the limit. |
| `1` | Analysis completed and at least one function score is above the limit. |
| `2` | Usage or discovery failed, or at least one selected input could not be read or parsed. |

Exit `2` takes priority over exit `1`.

## Language selection

| Filter | Extensions |
| --- | --- |
| `javascript` | `.js`, `.jsx`, `.mjs`, `.cjs` |
| `typescript` | `.ts`, `.tsx`, `.mts`, `.cts` |
| `php` | `.php` |
| `rust` | `.rs` |
| `python` | `.py` |

Extension matching is case-sensitive. File reports keep the source label
`javascript`, `jsx`, `typescript`, `tsx`, `php`, `rust`, or `python`.

An explicit supported file excluded by the active language filters is an
error. Directory discovery skips files outside the selected families. A run
with no selected files is an error.

## Path discovery

- Paths must resolve inside the current working directory.
- Directory scans are recursive and honor `.gitignore` and `.ignore`.
- Global Git ignore files and `.git/info/exclude` do not affect discovery.
- Hidden entries, `node_modules`, and `vendor` are skipped.
- Directory symlinks are not followed.
- Explicit supported files override ignore, hidden-entry, and
  dependency-directory rules when their canonical target remains inside the
  working directory.
- Explicit directories do not override hidden or dependency-directory skips.
- Repeated and overlapping inputs produce one result per canonical file.
- Results sort by slash-separated canonical path relative to the current
  working directory.
- Unsupported explicit files and non-UTF-8 output paths are errors.
- After discovery, the CLI reads each selected relative path through a
  capability rooted at the resolved current working directory. A path swap
  cannot redirect the read outside that directory. A blocked or failed read
  gives that file an `io_error` result and exit `2`.

## Standard input

`-` reads one source from standard input.

- `-` must be the sole input.
- Exactly one `--language` value is required.
- `--stdin-filename` is valid only with `-`.
- The virtual filename must be a relative UTF-8 path with no `.` or `..`
  component and an extension matching the selected language.
- Defaults are `stdin.js`, `stdin.ts`, `stdin.php`, `stdin.rs`, and `stdin.py`.
- The virtual filename selects parser mode and supplies report paths and IDs.
- TypeScript JSX requires a `.tsx` virtual filename.
- PHP input follows normal file mode and requires normal PHP tags.
- Invalid UTF-8 or a read failure produces an incomplete file report and exit
  `2`, not a usage error.

## Parse and function boundaries

JavaScript and TypeScript use Oxc with the existing extension-specific source
modes. PHP uses Tree-sitter `LANGUAGE_PHP` and retains the bounded reserved-word
class-constant retry. Rust uses `tree-sitter-rust 0.24.2`. Python uses
`tree-sitter-python 0.25.0`.

PHP, Rust, and Python check Tree-sitter depth with an iterative walk before any
recursive project analysis. A root-to-node path can have at most 512
parent-child edges. A deeper tree fails with:

```text
analysis nesting exceeds the supported limit of 512
```

This uses the normal `parse_error` result: no functions, `signals: null`,
incomplete status, and exit `2`. PHP applies the same check to the first tree
and to an accepted reserved-word retry tree.

The PHP retry applies only inside a class, trait, interface, or enum member
list. It recognizes `array`, `bool`, `callable`, `false`, `float`, `int`,
`iterable`, `mixed`, `namespace`, `null`, `object`, `string`, `true`, and
`void`. It masks the known name in a same-length parser-only copy and accepts
the retry only when the full second tree has no `ERROR` or `MISSING` node.
Analysis and positions still use the original source. It never retries a
global or function-local constant.

Any parser diagnostic makes the file `parse_error`, gives it no functions or
signals, marks the run incomplete, and causes exit `2`. A read or UTF-8 failure
does the same with `io_error`. Valid sibling files remain in the report.

Each executable function keeps its existing language-specific kind, best-effort
name, and source range. Its stable ID is:

```text
<relative-or-virtual-path>:<start-line>:<start-column>
```

Nested callables receive separate results, are excluded from their parent, and
start score and signal state from zero. Top-level script code is not reported.
Positions are one-based Unicode-scalar line and column numbers.

JavaScript and TypeScript visitor paths for logical expressions, unary chains,
parentheses, and conditional chains are iterative. Logical-expression handling
uses this rule in callable discovery, score contributions, signals,
Boolean-shape facts, and direct homogeneous JSX chain checks. It adds no
input-depth error and does not change score, contribution, condition, or
source-order rules.

Before normal JavaScript or TypeScript analysis, the CLI counts raw opening
delimiters and question marks in one byte pass. More than `2048` raw `(`, `[`,
and `{` bytes in total, or more than `2048` raw `?` bytes, marks the source as
risky. This count is only a probe trigger. It does not reject the source.

A risky source runs once through the same full CLI analysis in a child process
with the same language, virtual path, and complexity limit. Normal child exits
`0`, `1`, and `2` let the parent run normal analysis and keep the usual score,
diagnostic, and exit rules. A spawn, input, wait, signal, or unexpected-exit
failure makes the file `parse_error` at `1:1`, with no functions or signals.
This can differ by build profile: a valid source stays valid when the probe
completes, while a build whose probe aborts fails closed. A private environment
marker prevents probe recursion. The supported threat model trusts the process
environment set by the CLI caller.

Rust reports each `function_item` with a body. A function with a `self`
parameter has kind `method`; other function items have kind `function`.
Closures have kind `closure` and name `<anonymous>`. Bodyless trait function
signatures do not produce results. Function and closure ranges use their exact
parser node. Macro definitions and invocations are opaque; the CLI does not
expand or score generated syntax.

Python reports synchronous and asynchronous function definitions. A definition
in a class body has kind `method`; other definitions have kind `function`.
Lambdas have kind `lambda` and name `<anonymous>`. A decorated function range
starts at `def` or `async`, not at its decorators. Decorators, annotations, and
default values are outside the callable score and signals. Function, method,
and lambda bodies are callable barriers. Class bodies are otherwise
transparent.

## Text output safety

Human-readable reports and usage errors escape control characters and Unicode
bidirectional formatting controls. This applies to function IDs, function
names, diagnostic paths, diagnostic messages, and usage-error text. Escapes
use visible Rust default forms such as `\n`, `\t`, `\u{1b}`, and
`\u{202e}`. This keeps one result on one terminal line. JSON reports keep the
original strings. Inputs without these characters keep the same text output.

## Cognitive complexity

The rules below are the `core-v1` score contract. The merge must not change any
score, contribution, function range, threshold result, or parse-error policy.

Each contribution remains:

```text
rule
location
base_increment
nesting_increment
increment
```

The function score is the sum of its ordered contribution increments. A score
is over the limit only when `score > max_complexity`.

### JavaScript and TypeScript score rules

A structural contribution is `1 + current control nesting`. Its selected body
or result then raises nesting by one.

| Construct | Contribution |
| --- | --- |
| `if` | Structural |
| `else if` | Flat `+1`; no new nesting level for the condition |
| `else` | Flat `+1` |
| `for`, `for in`, `for of`, `while`, `do while` | Structural |
| `switch` | Structural once; cases run as nested content |
| `catch` | Structural |
| `try`, `finally` | `0` |
| Ternary | Structural; both result arms are nested |
| Labeled `break` or `continue` | Flat `+1` |
| Unlabeled `break` or `continue` | `0` |
| Each contiguous `&&` run | Flat `+1` |
| `||` and `??` | `0`; each splits an `&&` run |
| Parentheses | `0`; they do not split an `&&` run |
| Direct homogeneous JSX logical chain | Suppressed to `0` |
| Recursion and other constructs | `0` |

A direct homogeneous JSX logical chain is the top-level logical expression in
a JSX expression container. Suppression does not apply to mixed chains,
non-JSX expressions, or nested unrelated expressions.

Default parameter expressions belong to their callable. Class field
initializers and static blocks belong to their containing callable. Nested
method or function bodies remain separate. Contribution rules are `if`,
`else_if`, `else`, `loop`, `switch`, `catch`, `ternary`, `labeled_jump`, and
`logical_and`.

### PHP score rules

A structural contribution is `1 + current control nesting`. Flat
contributions add `1` without nesting.

| Construct | Contribution | Nested region |
| --- | --- | --- |
| `if` | Structural | Body |
| `elseif` | Flat `+1` | Body |
| `else` | Flat `+1` | Body |
| Spaced `else if` | Inner `if` is flat `+1`; no separate `else` point | The `else` region and inner body both add nesting |
| `for`, `foreach`, `while`, `do` | Structural | Body |
| `switch` | Structural once; cases add no point | Selector, case values, and case bodies |
| `match` | Structural once; arms add no point | Each arm result |
| `catch` | Structural for each catch | Catch body |
| `try`, `finally` | `0` | No added nesting |
| Ternary, including shorthand | Structural | Present result arms |
| `break N`, `continue N` | Flat `+1` when an argument is present | None |
| `goto` | Flat `+1` | None |

PHP score flow operators use one flattened parenthesized chain of `&&`, `||`,
and `|>`.

- When the outermost operator is `&&` or `||`, add a flat `1` for the first
  operator and each operator-type change.
- When the outermost operator is `|>`, add `1 + current nesting` for the first
  operator and each later operator whose type matches the prior operator.
- `&&` and `||` contributions use `logical_sequence`; `|>` uses `pipe`.
- Keyword `and`, `or`, and `xor`, and null coalescing `??`, add zero to the
  score. Their signal rules remain separate.

Plain `break`, plain `continue`, `return`, `throw`, `yield`, recursion,
nullsafe access, enums, attributes, named arguments, and Fiber API calls add
zero by themselves. Alternative colon syntax follows the same rules as brace
syntax.

PHP contribution rules are `if`, `elseif`, `else`, `else_if`, `loop`,
`switch`, `match`, `catch`, `ternary`, `logical_sequence`, `pipe`,
`numbered_jump`, and `goto`.

### Rust score rules

A structural contribution is `1 + current control nesting`. Flat
contributions add `1` without nesting.

| Construct | Contribution | Nested region |
| --- | --- | --- |
| `if` and `if let` | Structural | Consequence |
| `else if` | Flat `+1`; no separate `else` point | Consequence at the current branch depth |
| `else` | Flat `+1` | Alternative |
| `loop`, `while`, `while let`, `for` | Structural | Body |
| `match` | Structural once | Each arm value |
| Labeled `break` or `continue` | Flat `+1` | None |
| Unlabeled `break` or `continue` | `0` | None |
| Flattened `&&` and `||` sequence | Flat `+1` for the first operator and each operator-type change | None |

Calls, recursion, `return`, `?`, `await`, `yield`, unsafe blocks, async blocks,
const blocks, let-else, match arms, and match guards add zero by themselves.
Parentheses do not split a logical sequence.

Rust contribution rules are `if`, `else_if`, `else`, `loop`, `match`,
`labeled_jump`, and `logical_sequence`.

### Python score rules

A structural contribution is `1 + current control nesting`. Flat
contributions add `1` without nesting.

| Construct | Contribution | Nested region |
| --- | --- | --- |
| `if` | Structural | Body |
| `elif` | Flat `+1` | Body at the current branch depth |
| `else` on `if`, `for`, `while`, or `try` | Flat `+1` | Else body |
| `for`, `async for`, `while` | Structural | Body |
| Each `except` or `except*` | Structural | Handler body |
| Conditional expression | Structural | Both result arms |
| Flattened `and` and `or` sequence | Flat `+1` for the first operator and each operator-type change | None |

`try`, `finally`, `with`, `async with`, `match`, comprehensions, `not`,
`break`, `continue`, `return`, `raise`, `yield`, `await`, `assert`, recursion,
and assignment expressions add zero by themselves. A `finally` body adds no
nesting. Python `match` is intentionally zero in this profile because the
reviewed SonarPython visitor does not score it. Parentheses do not split a
logical sequence.

Python contribution rules are `if`, `elif`, `else`, `loop`, `except`,
`ternary`, and `logical_sequence`.

## Deterministic signals

### Function span

`line_span` is:

```text
range.end.line - range.start.line + 1
```

It uses the existing parser-specific function range without changing its
meaning.

### Maximum control depth

`max_control_depth` counts active structural control regions:

- No structural control is `0`.
- A top-level structural region is `1`.
- Each nested structural region adds `1`.
- Entering a region updates the maximum even when the region has no scored
  child.

Regions follow each language scorer's executable nested regions for `if`
branches, loops, switch or match content, catch or except bodies, and ternary
arms. Tests and switch or match selectors stay at the incoming depth; the
selected body or result enters the region. Rust match arm values enter one
region. Python match adds no region. Python loop and try else bodies enter one
region. Zero-cost `try`, `finally`, and `with` do not add depth. Nested
callables reset depth.

### Condition records

Record the test expression for:

- `if`
- `elseif`
- spaced `else if`
- Python `elif`
- `while`
- `do while`
- a present classic `for` test
- ternary
- Rust match guards
- Python case guards

Do not record `foreach`, Rust or Python `for`, JavaScript `for in` or `for of`,
switch selectors, match selectors, Python comprehension filters, or free
Boolean expressions.

Each record has:

```text
kind
location
operator_count
predicate_count
max_boolean_depth
```

`kind` uses these stable values:

| Syntax | Kind |
| --- | --- |
| `if` | `if` |
| PHP `elseif` | `elseif` |
| Spaced `else if` | `else_if` |
| Python `elif` | `elif` |
| `while` | `while` |
| `do while` | `do_while` |
| Classic `for` test | `for` |
| Ternary test | `ternary` |
| Rust match guard | `match_guard` |
| Python case guard | `case_guard` |

The location is the start of the test expression. Records sort by location and
then kind.

Count these operators:

- JavaScript and TypeScript: `!`, `&&`, `||`, `??`
- PHP: `!`, `&&`, `||`, `??`, `and`, `or`, `xor`
- Rust: `!`, `&&`, `||`
- Python: `not`, `and`, `or`

Do not count comparisons, bitwise operators, PHP pipe, or ternary as Boolean
operators in the containing condition. A nested ternary gets its own record.

`operator_count` is the number of listed operator tokens.
`predicate_count` is the number of atomic leaves after splitting on those
operators and is at least `1`.

`max_boolean_depth` is the normalized operator-tree depth:

- No listed operator is `0`.
- A flat chain of the same binary operator is `1`.
- Mixed nested binary operators and unary `!` each add a level.
- Parentheses, TypeScript wrappers, and parser associativity do not add depth.

Examples:

| Test | Operators | Predicates | Depth |
| --- | ---: | ---: | ---: |
| `a` | 0 | 1 | 0 |
| `a && b && c` | 2 | 3 | 1 |
| `a && (b || !c)` | 3 | 3 | 3 |

Function signal maxima use `0` when a function has no condition records.

### File signal

An `ok` file reports `function_count`, including zero. A failed file reports
`signals: null`; it must not claim that the source has zero functions.

## JSON schema v2

JSON is one compact object followed by one newline. Keys use this order:

```text
schema_version
tool
profile
max_complexity
status
files
summary
```

`tool` is `{"name":"complexity","version":"0.3.0"}`.

Each file uses:

```text
path
language
status
signals
functions
diagnostics
```

Each function uses:

```text
id
name
kind
range
score
over_limit
contributions
signals
```

Function signals use:

```text
line_span
max_control_depth
condition_count
max_condition_operators
max_condition_predicates
max_boolean_depth
conditions
```

The summary uses:

```text
files
functions
violations
errors
max_score
max_control_depth
max_function_line_span
max_functions_per_file
conditions
max_condition_operators
max_condition_predicates
max_boolean_depth
```

`violations` counts only functions above `max_complexity`. Signal maxima include
only successfully analyzed files. Empty observed sets use zero.

Arrays and diagnostics keep deterministic source ordering. JSON contains no
source text, absolute path, host data, duration, or timestamp.

## Text output

Each function emits:

```text
PASS|FAIL <id> <name> score=<n> lines=<n> control-depth=<n> conditions=<n> condition-operators=<n> condition-predicates=<n> boolean-depth=<n>
```

The three condition values are per-function maxima. Full condition records are
JSON-only. Diagnostics precede the summary.

The summary reports all JSON summary values with explicit field names.

## Manual Codex refactor eval

The bundled Promptfoo eval has four short cases:

| Case | Source | Starting focus |
| --- | --- | --- |
| `javascript-score` | `subject.js` | score `11` |
| `typescript-depth` | `subject.ts` | control depth `4` |
| `php-predicates` | `subject.php` | `5` predicates in one condition |
| `rust-span` | `subject.rs` | span `54` and score `3` |

Each case uses a separate behavior test. Before Codex starts, the runner must
prove that the test passes, measure the named function with the supplied real
`complexity` binary, and save an exact file and metric baseline. The starting
function must have a positive score and fail at least one skill target.

Codex must invoke the explicit `complexity-cli` skill, run the behavior test
and checker before and after its edit, and edit only the named source file. The
final source must keep the behavior test green, have a lower measured score,
and meet these targets:

| Metric | Target |
| --- | ---: |
| Cognitive complexity score | `10` |
| Maximum control depth | `3` |
| Inclusive line span | `50` |
| Predicates in one condition | `4` |

Codex returns the exact case, source, function, before metrics, and after
metrics. It also describes the refactor and names one more useful improvement,
or says why no more change is needed.

One independent assertion checks the model report against the saved baseline
and a new CLI result. It requires exactly one changed file, a passing behavior
test, a lower score, all targets, and an ordered checker-edit-checker record.
It rejects a result based only on the model's claims. A checker record must
contain a real Python checker invocation at a shell command boundary. Printed
or echoed checker text is not evidence.

The live eval is manual and sequential. It needs Node.js 24 or later, Python
3, PHP, `rustc`, network access, and Codex credentials. Release CI runs the
static eval tests and Promptfoo config validation only. It does not call a
model. A retained failed workspace never keeps the copied temporary Codex
home or its login data.

## Release packages

A pushed tag named `complexity-vX.Y.Z` starts the release flow. `X.Y.Z` must
match the package version in `Cargo.toml`. A mismatch fails before packaging
or release creation.

The release builds these native targets with `Cargo.lock`:

| Target | Archive |
| --- | --- |
| `x86_64-unknown-linux-gnu` | `.tar.gz` |
| `aarch64-unknown-linux-gnu` | `.tar.gz` |
| `x86_64-apple-darwin` | `.tar.gz` |
| `aarch64-apple-darwin` | `.tar.gz` |
| `x86_64-pc-windows-msvc` | `.zip` |

Each archive is named `complexity-X.Y.Z-TARGET` plus its archive suffix. It
contains one directory with the same base name and these paths:

```text
complexity or complexity.exe
README.md
LICENSE
agent/
```

The tracked `agent` tree contains:

```text
MANIFEST.txt
README.md
skills/complexity-cli/
hooks/codex.json
hooks/codex-windows.json
hooks/claude.json
hooks/claude-windows.json
eval/
```

The skill is explicit-only. Packaging or copying it must not enable a hook.
Hook files are merge samples. POSIX samples use `python3`; Windows samples use
`py -3`. Bundled samples call the checker in the project-local `agent` tree.
Installed-skill examples call the matching home skill path. The eval uses the
project-local pinned Promptfoo package and the four manual Codex refactor cases
defined above. `COMPLEXITY_BIN` must name the supplied `complexity` binary.
The runner disables Promptfoo telemetry and update checks, bypasses its result
cache, and does not share results.

`agent/MANIFEST.txt` is a sorted, unique list of archive paths and includes
itself. Packaging copies only regular files named there. Each resolved source
must stay below the real `agent` directory, and no path component can be a
symlink. An absent, unsafe, duplicate, or unsorted entry fails packaging.

Each archive has a separate SHA-256 file. The release is created only after
validation and all five package jobs pass.

## Out of scope

- Signal thresholds or exit effects
- Core CLI config files, baselines, Git-diff mode, SARIF, or editor services
- Parallel analysis
- Inheritance, import, dependency, or call graphs
- Duplication, comment, naming, framework, or architecture judgments
- Compatibility aliases for the old commands or JSON schema v1
- A stable public Rust library API
- Package signing, installers, package-manager publication, or automatic skill
  and hook installation
