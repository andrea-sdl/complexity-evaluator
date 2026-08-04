# `core-v1` compatibility results

Status: v2 differential and v0.3 extension checks passed

These results freeze evidence from `complexity-js` and `complexity-php`
`0.1.0`, record the completed `complexity` `0.2.0` differential, and record
local Rust and Python evidence for `0.3.0`.

## JS/TS local fixtures

Reference target: SonarJS S3776 source revision `2206d123`

| Fixture | Frozen result |
| --- | --- |
| `if_scoring.js` | `score = 5`; `fresh = 1` |
| `function_forms.js` | Nine function forms, each score `1`; parent and nested bodies stay separate |
| `structural_controls.js` | `loops = 6`; `switched = 5`; `recovered = 5`; `ternary = 5`; `jumps = 5` |
| `logical_controls.js` | `chains = 8` from eight maximal `&&` runs |
| `logical_jsx.tsx` | `jsxHomogeneous = 0`; `jsxMixed = 2`; `jsxTernary = 2`; `jsxNested = 1`; `jsxNestedTernary = 1` |
| `js_009_traversal.tsx` | `containers = 18`; `classOwner = 2`; nested `method = 1`; `enumOwner = 1` |
| `js_009_bodyless.ts` | `overloaded = 1`; `complete = 1`; bodyless callables are absent |
| `js_009_comment_anchors.js` | `commentAnchors = 2` |
| `js_009_statement_containers.cjs` | `statementContainers = 4` |

The old public CLI tests also checked callable order, rule names, locations,
increments, and contribution order. Focused fixtures checked the full
contribution object.

This is local fixture evidence. No live SonarJS result exists.

## PHP local fixtures

Reference target:
SonarPHP `cd5c3c244ec1f051ace71e0d07f5313e4c1f9d3e`

| Behavior family | Frozen fixture result | Status |
| --- | --- | --- |
| `if` and nesting | `nested_if = 3`: `if(0,1)`, `if(1,2)` | Pass |
| `elseif` | `elseif_case = 2`: `if(0,1)`, `elseif(0,1)` | Pass |
| `else if` nesting | `else_if_case = 5`: `if(0,1)`, `else_if(0,1)`, `if(2,3)` | Pass |
| `switch` | `switch_case = 3`; `switch_expression_case = 5` | Pass |
| Loops | `loop_case = 4` | Pass |
| Catch versus try/finally | `catch_case = 1` | Pass |
| Ternary | `ternary_case = 3`: `ternary(0,1)`, `ternary(1,2)` | Pass |
| Symbolic logical sequence | `logical_case = 2`; mixed outer sequence also covered | Pass |
| PHP 8.5 pipe | `pipe_case = 2`; `mixed_operator_case = 7` | Pass |
| Multi-level `break` | `break_case = 2` | Pass |
| Multi-level `continue` | `continue_case = 2` | Pass |
| `goto` | `goto_case = 1` | Pass |
| Nested callable ownership | parent `1`, named function `1`, closure `1`, arrow `1` | Planned difference |
| Recursion | `recursive_case = 1`; recursion adds zero | Pass |
| `match` | `match_case = 3` | Planned difference |
| Other flow statements | `zero_flow = 0` | Pass |
| Modern zero-score syntax | `modern_zero = 0` | Pass |
| Alternative syntax | `alternative_case = 4` | Pass |

`rule(nesting, increment)` is short form. Every base increment in this corpus
is `1`. The old PHP tests checked every score and ordered
`(rule, nesting, increment)` list. They also checked threshold zero.

The two planned differences are part of `core-v1`. They are not failures in
the local fixture profile.

## PHP live Agentforce result

Run date: 2026-07-27

| Input or measure | Result |
| --- | --- |
| Agentforce commit | `c0f4786da726a1adccc03eaa12081f76fd4c60d8` |
| SonarPHP | `3.58.0.16263` |
| SonarQube Community Build | `26.7.0.124771` |
| SonarScanner CLI | `8.0.1.6346` |
| Active rule | Only `php:S3776`, threshold `0` |
| Local files | 56 |
| Local functions | 841 |
| Local positive scores | 220 |
| Sonar positive issues | 216 |
| Positive identity union | 237 |
| Same-identity records | 199 |
| Exact same-identity scores | 193 |
| Same-identity score gaps | 6 |
| Raw Sonar-only identities | 17 |
| Raw local-only identities | 21 |
| Strict exact-score rate over union | `81.4%` |
| Same-identity score agreement | `97.0%` |
| Threshold-15 findings | Same eight functions |

All gaps matched `match` scoring or nested-callable ownership. No unexplained
score, identity, or parse gap remained in this corpus.

One tracked hidden file was outside the old local directory scan. It had no
function, so it did not change the function comparison.

The live API did not provide a stable ordered contribution list. The exact
contribution evidence comes from the frozen local fixtures, not the live run.

## Known parser result

The PHP parser has a bounded retry for known reserved-word class-like constant
names. A clean second parse keeps the source ranges and allows analysis. A
failed retry keeps the original parse error and returns no scores for the file.

This result tests parser coverage. It does not change the Sonar score rates.

## V2 differential record

| Check | Result |
| --- | --- |
| JS/TS function identities and ranges | Exact in 48 of 48 runs |
| JS/TS scores and contributions | Exact in 48 of 48 runs |
| JS/TS diagnostics and exits | Exact in 48 of 48 runs |
| PHP function identities and ranges | Exact in 26 of 26 runs |
| PHP scores and contributions | Exact in 26 of 26 runs |
| PHP diagnostics and exits | Exact in 26 of 26 runs |
| Mixed ordering and exit priority | Passed in public CLI tests |

The JS/TS comparison used all 24 migrated fixtures at limits `15` and `0`.
The PHP comparison used all 13 migrated fixtures at the same limits. It
compared process exit, stderr, profile, limit, report status, file path,
language, file status, function identity, name, kind, range, score,
`over_limit`, ordered contributions, diagnostics, and the old summary fields.

The normalization removed only the planned tool metadata, schema v2 wrapper,
file and function signal fields, and added summary signal fields. The 74 runs
had zero differences.

## V0.3 Rust and Python record

Rust and Python have no old local tool for a differential. Their evidence uses
new exact fixtures plus the unchanged JS/TS and PHP suites.

| Check | Result |
| --- | --- |
| Rust callable, score, contribution, signal, position, threshold, parse, and depth cases | 6 of 6 tests passed |
| Python callable, score, contribution, signal, position, threshold, parse, and depth cases | 8 of 8 tests passed |
| Nested decorated Python callable boundary | Parent score and signals remain zero |
| Python `try ... else` control depth | Empty else body reports one active region |
| Five-language ordering, filters, stdin, safe reads, and text escaping | Passed in 44 public CLI tests |
| Rust self-analysis | 7 files, 446 functions, max score 6, zero errors, byte-stable JSON |
| Repository complexity ratchet | Public CLI test pins max score 7, ordered source paths, and per-file function counts |
| Full all-target suite | 116 tests passed |

The existing 48 JS/TS and 26 PHP differential results remain unchanged. No live
SonarRust or SonarPython comparison has run. These local tests do not support a
compatibility percentage.
