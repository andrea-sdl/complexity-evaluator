# Sonar compatibility boundary

`complexity` `0.3.0` uses the local `core-v1` profile. The Rust and Python
extension preserves the old language scores. It does not claim general Sonar
or SonarSource compatibility.

## Evidence map

| Language | Frozen source target | Local fixture evidence | Live Sonar evidence |
| --- | --- | --- | --- |
| JS/TS | SonarJS S3776 at `2206d123` | Yes | None |
| PHP | SonarPHP at `cd5c3c244ec1f051ace71e0d07f5313e4c1f9d3e` | Yes | One Agentforce corpus against SonarPHP `3.58.0.16263` |
| Rust | SonarRust at `9539d0c59f8965663fd3efa616a492a4c65315f3` | Yes | None |
| Python | SonarPython at `322e2dcb2a1bcab654614fffa394a8db43b9ef16` | Yes | None |

The source targets and the live PHP analyzer are different evidence. Do not
describe the live run as a run of the frozen source revision.

## JS/TS boundary

The JS/TS part of `core-v1` was designed for per-function score parity with
the named SonarJS source revision. It covers the syntax-local rules in the old
fixtures:

- controls, loops, switch, catch, and ternaries;
- labeled jumps;
- logical `&&` runs;
- direct homogeneous JSX chain suppression;
- separate function ownership and supported syntax containers.

The local fixtures check function order, scores, contribution order,
locations, increments, and selected thresholds. They do not compare a live
SonarJS result.

No percentage claim is valid for JS/TS. Such a claim needs an independent
JS, JSX, TS, and TSX corpus and a runner for a named SonarJS build.

Deferred JS/TS surfaces include file metrics, React aggregation, Vue, Flow,
embedded languages, issue text, remediation cost, and exact parser recovery
parity.

## PHP frozen profile

The PHP fixture set covers 18 behavior families:

1. `if` and structural nesting
2. `elseif`
3. `else` and spaced `else if`
4. `switch`
5. loops
6. `catch`, `try`, and `finally`
7. ternary
8. symbolic logical sequences
9. PHP 8.5 pipe
10. multi-level `break`
11. multi-level `continue`
12. `goto`
13. nested callable ownership
14. recursion
15. `match`
16. other flow statements
17. modern zero-score syntax
18. alternative control syntax

Sixteen families target the frozen SonarPHP source behavior. Two are planned
local differences:

- `match` adds structural complexity.
- Nested callables own their scores and do not add their bodies to a parent.

The ratio `16/18`, or `88.9%`, is only a list of planned rule families. It is
not a measured compatibility rate and not a representative denominator.

## Live PHP result

One live run used:

- run date: 2026-07-27;
- Agentforce commit: `c0f4786da726a1adccc03eaa12081f76fd4c60d8`;
- SonarQube Community Build: `26.7.0.124771`;
- SonarPHP: `3.58.0.16263`;
- SonarScanner CLI: `8.0.1.6346`;
- active rule: only `php:S3776`, with threshold `0`.

The local tool reported 841 functions and 220 positive scores in 56 files.
Sonar reported 216 positive issues. The positive identity union had 237
records. Of these, 193 had the same identity and score.

The strict exact-score rate over that union was `193/237`, or `81.4%`.
For records with the same identity, score agreement was `193/199`, or
`97.0%`.

At threshold 15, both tools flagged the same eight functions. This is one
corpus against one analyzer build. It does not prove broad compatibility.

All measured gaps matched the two planned profile differences:

- Six same-function scores differed.
- Sonar had 17 positive outer-method identities that the local tool did not
  report as positive.
- The local tool had 21 positive nested-callable identities that Sonar did not
  report as separate positive issues.

The Sonar Web API gave positive function identity, score, issue location, and
threshold results. It did not give a stable ordered contribution vector.

## Rust frozen profile

The Rust fixtures cover function and method ownership, closures, Unicode
positions, branches, loops, `match`, labeled jumps, logical sequences, match
guards, let-else, opaque macros, thresholds, signals, and parse failure.

The score rules follow the named SonarRust visitor for those syntax-local
constructs. Nested functions and closures are one planned difference:
`complexity` reports them separately and excludes them from their parent.

Five Rust integration tests pass. This is local fixture evidence. No live
SonarRust run exists, so no compatibility percentage is valid.

## Python frozen profile

The Python fixtures cover sync and async definitions, methods, lambdas,
decorators, nested callable ownership, branches, loops and loop `else`,
exception handlers, try `else`, ternaries, logical sequences, neutral `match`
and `with`, case guards, Unicode positions, thresholds, signals, and parse
failure.

The score rules follow the named SonarPython visitor for those syntax-local
constructs. Nested functions and lambdas use the same planned ownership
difference as Rust. Python `match` remains score-neutral because the reviewed
visitor does not score it.

Six Python integration tests pass. This is local fixture evidence. No live
SonarPython run exists, so no compatibility percentage is valid.

## Parser gaps are separate

The bounded reserved-word class-constant retry fixes a known
`tree-sitter-php 0.24.2` grammar gap. It is not a score-profile difference.

Exact parser diagnostics, token spans, and recovery behavior are also outside
the Sonar score evidence. Parse failures remain fail-closed under `core-v1`.

## Required v2 proof

The merged v2 differential compared old and new results after it removed only
the planned command and schema wrapper changes. For each language it compared:

1. Function identity, kind, name, and range.
2. Score and threshold result.
3. Ordered rule, location, base, nesting, and increment values.
4. File diagnostics and exit result.

All 48 JS/TS runs and all 26 PHP runs matched. Each fixture ran at limits `15`
and `0`. The comparison found no function identity, kind, name, range, score,
threshold result, ordered contribution, diagnostic, stderr, or exit
difference. See [COMPATIBILITY-RESULTS.md](COMPATIBILITY-RESULTS.md).

## Work needed for an 80–90% claim

The Rust and Python MVP does not claim 80–90% Sonar compatibility. Reaching a
measured claim needs:

1. Repeatable runners for named SonarRust and SonarPython builds with only
   S3776 enabled at threshold `0`.
2. Independent Rust and Python corpora that cover common syntax, not only
   project fixtures.
3. Stable mapping from Sonar issue ranges to local callable identities.
4. Exact score comparison over the positive identity union and over matched
   identities.
5. Published parser failures, identity gaps, score gaps, corpus revisions, and
   analyzer versions.
6. A separate threshold-15 comparison.
7. An explicit treatment of the planned nested-callable ownership difference,
   either as a compatibility mode or as a disclosed mismatch class.

The local scorer and report evidence are ready for these runs. The live
analyzer harnesses and independent corpora are not. Adding them is more than a
small parser extension, so they stay incremental work.

JS/TS still needs the same live proof. PHP still needs a saved runner and more
independent corpora. Track later analyzer and parser versions as new targets,
not silent updates.
