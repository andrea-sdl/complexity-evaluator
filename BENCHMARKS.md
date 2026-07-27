# Performance evidence

Status: v2 gates passed

The old `0.1.0` tools were measured on 2026-07-24. These are local baselines,
not speed claims for other machines or source sets.

## Baseline environment

- macOS 26.5.2; the JS/TS record also gave build `25F84`
- Apple M4 Max, arm64, 16 cores
- Rust 1.96.0
- Oxc `0.141.0`
- Tree-sitter `0.26.11`
- tree-sitter-php `0.24.2`

The v2 gate needs the same machine and the same old corpora. Record any change
to the OS, tool chain, power mode, or corpus before comparison.

## Pre-merge JS/TS baseline

The JS/TS generator wrote the same 2,000 functions in two layouts. Each layout
had 1,000 JavaScript and 1,000 TypeScript functions.

| Corpus | Files | Functions | Source bytes |
| --- | ---: | ---: | ---: |
| Many small files | 2,000 | 2,000 | 325,780 |
| Few large files | 8 | 2,000 | 325,780 |

The release binary ran in JSON mode with output sent to the null device. Each
sample used 25 full CLI runs. Two batch warm-ups came before seven measured
batches. The recorded time was divided by 25.

| Corpus | Per-run min / median / max | Median peak RSS |
| --- | ---: | ---: |
| Many small files | 40.0 / 40.0 / 40.4 ms | 6.48 MiB |
| Few large files | 6.4 / 6.8 / 6.8 ms | 5.38 MiB |

Batch time samples before division:

- many small: `1.00, 1.00, 1.00, 1.01, 1.00, 1.00, 1.00` seconds;
- few large: `0.17, 0.16, 0.17, 0.17, 0.17, 0.16, 0.16` seconds.

Peak RSS samples:

- many small: `6.48, 6.48, 6.50, 6.48, 6.50, 6.52, 6.48` MiB;
- few large: `5.38, 5.36, 5.38, 5.34, 5.38, 5.39, 5.36` MiB.

## Pre-merge PHP baseline

The PHP generator wrote 2,000 functions in two layouts.

| Corpus | Files | Functions | Source bytes |
| --- | ---: | ---: | ---: |
| Many small files | 2,000 | 2,000 | 292,890 |
| Few large files | 8 | 2,000 | 280,938 |

The release binary ran in JSON mode with output sent to the null device. Two
warm-up runs came before seven measured runs.

| Corpus | Elapsed min / median / max | Median peak RSS |
| --- | ---: | ---: |
| Many small files | 80 / 80 / 90 ms | 11.41 MiB |
| Few large files | 40 / 40 / 40 ms | 11.39 MiB |

Time samples:

- many small: `0.08, 0.08, 0.08, 0.09, 0.08, 0.08, 0.08` seconds;
- few large: `0.04, 0.04, 0.04, 0.04, 0.04, 0.04, 0.04` seconds.

Peak RSS samples:

- many small: `11.36, 11.34, 11.55, 11.41, 11.61, 11.41, 11.36` MiB;
- few large: `11.38, 11.38, 11.41, 11.41, 11.34, 11.39, 11.42` MiB.

## V2 single-language measurement record

Keep the old method for each matching corpus. Do not fill a median without the
raw samples.

| Language corpus | Warm-ups | Measured unit | Runtime samples | Min / median / max | Peak RSS samples | Median peak RSS |
| --- | ---: | --- | --- | --- | --- | ---: |
| JS/TS, many small | 2 batches | 7 batches of 25 runs | `0.91, 0.91, 0.92, 0.92, 0.93, 0.93, 0.91` s per batch | `36.4 / 36.8 / 37.2` ms per run | `9.14, 9.05, 9.14, 9.05, 9.11, 9.09, 9.05` MiB | `9.09` MiB |
| JS/TS, few large | 2 batches | 7 batches of 25 runs | `0.13, 0.13, 0.13, 0.13, 0.14, 0.13, 0.13` s per batch | `5.2 / 5.2 / 5.6` ms per run | `6.92, 6.92, 6.92, 6.92, 6.92, 6.92, 6.92` MiB | `6.92` MiB |
| PHP, many small | 2 runs | 7 runs | `0.09, 0.09, 0.09, 0.09, 0.09, 0.09, 0.09` s | `90 / 90 / 90` ms | `9.22, 9.22, 9.19, 9.28, 9.28, 9.28, 9.25` MiB | `9.25` MiB |
| PHP, few large | 2 runs | 7 runs | `0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05` s | `50 / 50 / 50` ms | `8.64, 8.66, 8.66, 8.66, 8.67, 8.66, 8.64` MiB | `8.66` MiB |

## V2 runtime gates

`D-011` permits no more than a 25% median runtime increase for each old
single-language corpus. The limits below are arithmetic from the recorded
medians. They are not v2 results.

| Language corpus | Old median | 25% limit | V2 median | Change | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| JS/TS, many small | 40.0 ms | 50.0 ms | 36.8 ms | `-8.0%` | Pass |
| JS/TS, few large | 6.8 ms | 8.5 ms | 5.2 ms | `-23.5%` | Pass |
| PHP, many small | 80 ms | 100 ms | 90 ms | `+12.5%` | Pass |
| PHP, few large, paired batches | 43.2 ms | 54.0 ms | 52.4 ms | `+21.3%` | Pass |

The PHP few-large single-run method uses a timer with `0.01` second output
steps. Its final medians were `40` ms for the old binary and `50` ms for v2,
which cannot resolve the 25% boundary. A paired check used two warm-up batches
and seven measured batches of 25 runs for each binary. Old batch samples were
`1.08, 1.08, 1.08, 1.08, 1.08, 1.08, 1.09` seconds. V2 batch samples were
`1.31, 1.30, 1.31, 1.31, 1.31, 1.32, 1.31` seconds. The paired medians give
the governing `21.3%` result.

Peak memory has no gate in `D-011`, but the new result must record it:

| Language corpus | Old median peak RSS | V2 median peak RSS | Change |
| --- | ---: | ---: | ---: |
| JS/TS, many small | 6.48 MiB | 9.09 MiB | `+2.61 MiB` |
| JS/TS, few large | 5.38 MiB | 6.92 MiB | `+1.54 MiB` |
| PHP, many small | 11.41 MiB | 9.25 MiB | `-2.16 MiB` |
| PHP, few large | 11.39 MiB | 8.66 MiB | `-2.73 MiB` |

## V2 mixed corpus record

There is no pre-merge mixed-command baseline. The mixed run has no 25% gate.
It must still use the release binary and record all fields below.

| Field | Result |
| --- | --- |
| Corpus generator or fixed source revision | `benchmarks/generate_corpus.rs` |
| JavaScript files / functions / bytes | `1 / 10 / 1,580` |
| TypeScript files / functions / bytes | `1 / 10 / 1,600` |
| PHP files / functions / bytes | `1 / 10 / 1,386` |
| Total files / functions / bytes | `3 / 30 / 4,566` |
| Warm-up count | 2 batches of 100 runs |
| Measured sample count | 7 batches of 100 runs |
| Min / median / max runtime | `2.2 / 2.3 / 2.3` ms per run |
| Runtime samples | `0.23, 0.23, 0.23, 0.22, 0.23, 0.23, 0.23` s per batch |
| Median peak RSS | `3.52` MiB |
| Peak RSS samples | `3.52, 3.52, 3.52, 3.52, 3.52, 3.52, 3.52` MiB |
| Exit result | `0` in every run |

## Pre-merge local binary artifacts

The current local release files had these sizes during the document move:

| Artifact | Bytes | MiB |
| --- | ---: | ---: |
| `complexity-js` | 2,849,216 | 2.72 |
| `complexity-php` | 3,817,184 | 3.64 |

These are local built artifacts, not results from a new clean build. The old
benchmark records did not include binary size. Use these values as notes only.
Do not use them as a gate.

## V2 binary size record

Measure the release file, not a debug file.

| Artifact | Bytes | MiB | Build profile |
| --- | ---: | ---: | --- |
| `target/release/complexity` | 4,858,768 | 4.63 | `release` |

There is no binary-size gate. Do not derive a required size change from the
local pre-merge files.

## V2 method

For each single-language corpus:

1. Recreate the exact old corpus.
2. Build `target/release/complexity`.
3. Use the matching language filter and JSON output.
4. Send output to the null device.
5. Use the same warm-up and sample method as its old baseline.
6. Record every runtime and peak RSS sample.
7. Calculate the median and the change from the old median.
8. Mark the gate only after the values are present.

The measured path must include discovery, parsing, scoring, signal work,
report construction, and JSON serialization.

For the mixed corpus, fix its generator or source revision before the run.
Record the count and byte size for each language. Do not compare it to a sum of
old medians; the old tools did not have one mixed invocation.

The v2 run used `/usr/bin/time -lp` and sent report output to the null device.
The generator reproduced all four old corpus trees byte for byte. Its two
tests also lock the file, function, and byte totals and repeatable mixed
output.
