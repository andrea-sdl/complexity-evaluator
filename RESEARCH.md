# Research record for `complexity`

Research dates: 2026-07-24 and 2026-07-27

This file moves the source research from `complexity-js` and
`complexity-php` into the merged v2 project. [SPEC.md](SPEC.md) is the product
contract. [DESIGN.md](DESIGN.md) records the choices for v2.

## Evidence levels

The research has four evidence levels:

1. Frozen source review of a named revision.
2. Local tests against sources written for this project.
3. A live run against a named Sonar analyzer and source corpus.
4. Vendor data used only to choose a parser.

These levels are not equal. A source review is not a live comparison. A local
test is not proof of Sonar results. A vendor benchmark is not a result for this
CLI.

## Product reference

[complexipy](https://github.com/rohaquinlop/complexipy) was a product
reference for both old tools. Its useful product shape was:

- file and directory input;
- stable per-function results;
- a threshold;
- text and machine output.

It was not a score source. Its score guide and code did not agree on all
`with` and `match` behavior when reviewed on 2026-07-24. The old projects
therefore used explicit `core-v1` rules and fixtures.

Relevant source material:

- [complexipy score guide](https://github.com/rohaquinlop/complexipy/blob/main/docs/understanding-scores.md)
- [complexipy scorer](https://github.com/rohaquinlop/complexipy/blob/main/src/cognitive_complexity.rs)
- [complexipy Boolean handling](https://github.com/rohaquinlop/complexipy/blob/main/src/utils.rs)

## JavaScript and TypeScript parser

V2 keeps Oxc `0.141.0`. Oxc gives the Rust tool one AST for JavaScript,
JSX, TypeScript, and TSX. It also gives spans and parser diagnostics.

The old review compared these options:

| Parser | Main fit | Reason not selected |
| --- | --- | --- |
| Oxc | Batch Rust analysis of JS, JSX, TS, and TSX | Selected. |
| SWC | Rust JS and TS parser | Oxc had the narrower local fit and direct diagnostics used by the CLI. |
| Tree-sitter | Error-tolerant and incremental trees | The batch CLI does not need incremental parsing. |
| Biome | Full JS and TS tool chain | The CLI does not need format-preserving edits. |
| esbuild | Fast Go parser and bundler | It would add a Go boundary and bundler scope. |

The parser mode still comes from the file extension. JavaScript extensions
allow JSX where the old contract allowed it. TypeScript JSX needs `.tsx`.
Parser errors fail closed.

Oxc published parser conformance and speed data during the 2026-07-24 review.
Those are vendor results. They do not support a speed or syntax-coverage claim
for `complexity`.

Primary parser material:

- [Oxc parser use](https://www.oxcjs.com/guide/usage/parser.html)
- [Oxc parser design](https://www.oxcjs.com/learn/architecture/parser.html)

## PHP parser

V2 keeps `tree-sitter 0.26.11` and `tree-sitter-php 0.24.2`.
`LANGUAGE_PHP` accepts normal `.php` files with PHP tags and mixed HTML.
PHP-only snippets without tags are not a separate file mode.

The old review compared these options:

| Parser | Main fit | Reason not selected |
| --- | --- | --- |
| tree-sitter-php | Rust binding, source ranges, and mixed PHP files | Selected. |
| Mago syntax | Typed Rust PHP AST | It had a larger linked package set and a less stable local API. |
| VKCOM and Go forks | PHP ASTs in Go | They would add a Go process or a port, and needed new PHP-version checks. |

Tree-sitter can make a tree from bad syntax. The CLI does not score such a
tree. An `ERROR` or `MISSING` node makes the file incomplete, apart from the
one bounded retry below.

Primary parser material:

- [tree-sitter-php](https://github.com/tree-sitter/tree-sitter-php)
- [tree-sitter-php Rust binding](https://github.com/tree-sitter/tree-sitter-php/blob/master/bindings/rust/lib.rs)
- [Tree-sitter](https://tree-sitter.github.io/tree-sitter/)

### Known parser gap

The 2026-07-27 review found one gap in `tree-sitter-php 0.24.2`. PHP permits
most reserved words as class-like constant names. The grammar rejects a small
set when such a name is the first constant name. Upstream issue
[#295](https://github.com/tree-sitter/tree-sitter-php/issues/295) was open at
the review date.

V2 keeps the old narrow retry. It masks only the known names in a same-length
parser copy, then parses the full file again. It accepts the retry only when
the new tree has no `ERROR` or `MISSING` node. Scores and positions still use
the source text. [SPEC.md](SPEC.md) and `D-017` preserve this rule.

This is a parser work-around. It is not a Sonar profile difference.

## Score references

The JS/TS profile was designed for per-function score parity with SonarJS
S3776 at source revision `2206d123`. There was no live SonarJS run.

The PHP rules used a frozen review of SonarPHP revision
[`cd5c3c2`](https://github.com/SonarSource/sonar-php/tree/cd5c3c244ec1f051ace71e0d07f5313e4c1f9d3e).
The PHP profile has two planned differences:

1. It scores `match` as structural flow.
2. It gives nested callables their own result and excludes them from parents.

One live PHP run measured these choices. See
[SONAR-COMPATIBILITY.md](SONAR-COMPATIBILITY.md) and
[COMPATIBILITY-RESULTS.md](COMPATIBILITY-RESULTS.md).

## Performance rule

Parser language does not prove CLI speed. A valid result must include the
fixed corpus, source size, parser versions, build mode, warm-up method,
samples, median, range, machine data, and full CLI work. It must include
discovery, parsing, scoring, report work, and JSON output.

[BENCHMARKS.md](BENCHMARKS.md) keeps the pre-merge results and records the
passed v2 JS/TS, PHP, and mixed runs.

## License facts

Oxc, Tree-sitter, and tree-sitter-php declared MIT licenses in the reviewed
sources. Mago declared Apache-2.0 or MIT. These facts allow dependency review;
they do not choose the project license or complete a release license check.
The v2 project license stays unset under `D-013`.

## Work still needed

- Add a live, repeatable SonarJS comparison.
- Make the SonarPHP runner repeatable and test more independent corpora.
- Recheck parser releases before any dependency update.
- Review the full locked dependency set and notices before release.
