use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_complexity"))
        .args(args)
        .output()
        .expect("complexity command should run")
}

fn run_stdin(args: &[&str], source: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_complexity"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("complexity command should run");
    child
        .stdin
        .take()
        .expect("complexity stdin should be available")
        .write_all(source.as_bytes())
        .expect("test source should be written");
    child
        .wait_with_output()
        .expect("complexity command should complete")
}

fn assert_risky_parser_outcome(output: &std::process::Output) {
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");

    match output.status.code() {
        Some(0) => {
            assert_eq!(report["files"][0]["status"], "ok");
            assert_eq!(report["files"][0]["signals"]["function_count"], 1);
            let functions = report["files"][0]["functions"]
                .as_array()
                .expect("functions should be an array");
            assert_eq!(functions.len(), 1);
            assert_eq!(functions[0]["score"], 1);
        }
        Some(2) => {
            assert_eq!(report["files"][0]["status"], "parse_error");
            assert_eq!(report["files"][0]["signals"], serde_json::Value::Null);
            assert!(
                report["files"][0]["functions"]
                    .as_array()
                    .expect("functions should be an array")
                    .is_empty()
            );
            assert!(
                report["files"][0]["diagnostics"][0]["message"]
                    .as_str()
                    .expect("diagnostic message")
                    .contains("analysis probe")
            );
        }
        code => panic!("risky analysis returned unexpected exit {code:?}"),
    }
}

fn hook_budget() -> Duration {
    if cfg!(debug_assertions) {
        return Duration::from_secs(15);
    }
    Duration::from_secs(5)
}

fn balanced_logical_expression(leaves: usize) -> String {
    let mut expressions = vec!["value".to_string(); leaves];
    while expressions.len() > 1 {
        expressions = expressions
            .chunks(2)
            .map(|pair| match pair {
                [left, right] => format!("({left} && {right})"),
                [left] => left.clone(),
                _ => unreachable!("chunks never yields an empty pair"),
            })
            .collect();
    }
    expressions.pop().expect("at least one leaf")
}

fn left_associated_logical_expression(leaves: usize) -> String {
    vec!["value"; leaves].join(" && ")
}

#[test]
fn javascript_handles_a_large_left_associated_logical_chain() {
    let source = format!(
        "function chain() {{ return {}; }}\n",
        left_associated_logical_expression(65_537)
    );
    let output = run_stdin(
        &["--format", "json", "--language", "javascript", "-"],
        &source,
    );

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(report["files"][0]["functions"][0]["score"], 1);
    assert_eq!(
        report["files"][0]["functions"][0]["contributions"],
        serde_json::json!([{
            "rule": "logical_and",
            "location": { "line": 1, "column": 33 },
            "base_increment": 1,
            "nesting_increment": 0,
            "increment": 1
        }])
    );
}

#[test]
fn javascript_collects_condition_signals_for_a_large_left_associated_logical_chain() {
    let source = format!(
        "function condition() {{ if ({}) {{ return value; }} }}\n",
        left_associated_logical_expression(65_537)
    );
    let output = run_stdin(
        &["--format", "json", "--language", "javascript", "-"],
        &source,
    );

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let function = &report["files"][0]["functions"][0];
    assert_eq!(function["score"], 2);
    assert_eq!(function["signals"]["max_condition_operators"], 65_536);
    assert_eq!(function["signals"]["max_condition_predicates"], 65_537);
    assert_eq!(function["signals"]["max_boolean_depth"], 1);
}

#[test]
fn javascript_handles_deep_unary_expressions() {
    let source = format!(
        "function unary() {{ return {}value; }}\n",
        "!".repeat(32_768)
    );
    let output = run_stdin(
        &["--format", "json", "--language", "javascript", "-"],
        &source,
    );

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(report["files"][0]["functions"][0]["score"], 0);
}

#[test]
fn typescript_accepts_many_optional_fields() {
    let fields = (0..2_050)
        .map(|index| format!("  field{index}?: number;"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("type Large = {{\n{fields}\n}};\n");
    let output = run_stdin(
        &[
            "--format",
            "json",
            "--language",
            "typescript",
            "--stdin-filename",
            "large.ts",
            "-",
        ],
        &source,
    );

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(report["files"][0]["status"], "ok");
    assert_eq!(report["files"][0]["signals"]["function_count"], 0);
}

#[test]
fn javascript_accepts_a_regex_with_many_raw_delimiters_and_questions() {
    let source = format!(
        "function regex() {{ return /{}/; }}\n",
        "(?:a)?".repeat(2_050)
    );
    let output = run_stdin(
        &["--format", "json", "--language", "javascript", "-"],
        &source,
    );

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(report["files"][0]["status"], "ok");
    assert_eq!(report["files"][0]["functions"][0]["score"], 0);
}

#[test]
fn javascript_extreme_parser_nesting_completes_or_fails_closed() {
    let source = format!(
        "function nested() {{ return {}value ? yes : no{}; }}\n",
        "(".repeat(4_096),
        ")".repeat(4_096)
    );
    let output = run_stdin(
        &["--format", "json", "--language", "javascript", "-"],
        &source,
    );

    assert_risky_parser_outcome(&output);
}

#[test]
fn javascript_extreme_template_parser_nesting_completes_or_fails_closed() {
    let source = format!(
        "function nested() {{ return `value: ${{{}value ? yes : no{}}}`; }}\n",
        "(".repeat(4_096),
        ")".repeat(4_096)
    );
    let output = run_stdin(
        &["--format", "json", "--language", "javascript", "-"],
        &source,
    );

    assert_risky_parser_outcome(&output);
}

#[test]
fn javascript_accepts_a_deep_right_associated_ternary_chain() {
    let source = format!(
        "function ternary() {{ return {}no; }}\n",
        "value ? yes : ".repeat(4_096)
    );
    let output = run_stdin(
        &["--format", "text", "--language", "javascript", "-"],
        &source,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stdout)
            .expect("text output should be utf-8")
            .contains("score=8390656")
    );
}

#[test]
fn typescript_jsx_handles_large_balanced_logical_chains_within_the_hook_budget() {
    let expression = balanced_logical_expression(262_144);
    let source = format!("function view() {{ return <>{{{expression}}}</>; }}\n");
    let started = Instant::now();
    let output = run_stdin(
        &[
            "--format",
            "json",
            "--language",
            "typescript",
            "--stdin-filename",
            "view.tsx",
            "-",
        ],
        &source,
    );

    assert!(
        started.elapsed() < hook_budget(),
        "analysis exceeded the hook budget"
    );
    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(report["files"][0]["functions"][0]["score"], 0);
}

#[test]
fn javascript_reports_many_same_line_controls_within_the_hook_budget() {
    let source = format!(
        "function positions() {{ {} }}\n",
        "if (value) {}".repeat(262_144)
    );
    let started = Instant::now();
    let output = run_stdin(
        &["--format", "text", "--language", "javascript", "-"],
        &source,
    );

    assert!(
        started.elapsed() < hook_budget(),
        "analysis exceeded the hook budget"
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stdout)
            .expect("text output should be utf-8")
            .contains("score=262144"),
        "the full result should preserve the expected score"
    );
}

type ContributionFact = (String, u64, u64, u64, u64, u64);
type FunctionScoreFact = (String, u64, Vec<ContributionFact>);

fn score_facts(report: &serde_json::Value) -> Vec<FunctionScoreFact> {
    report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array")
        .iter()
        .map(|function| {
            let contributions = function["contributions"]
                .as_array()
                .expect("contributions should be an array")
                .iter()
                .map(|contribution| {
                    (
                        contribution["rule"].as_str().expect("rule").to_string(),
                        contribution["location"]["line"].as_u64().expect("line"),
                        contribution["location"]["column"].as_u64().expect("column"),
                        contribution["base_increment"].as_u64().expect("base"),
                        contribution["nesting_increment"].as_u64().expect("nesting"),
                        contribution["increment"].as_u64().expect("increment"),
                    )
                })
                .collect();
            (
                function["name"].as_str().expect("name").to_string(),
                function["score"].as_u64().expect("score"),
                contributions,
            )
        })
        .collect()
}

#[test]
fn javascript_if_chains_keep_the_core_v1_score_and_contributions() {
    let output = run(&[
        "--format",
        "json",
        "--max-complexity",
        "4",
        "tests/fixtures/javascript/if_scoring.js",
    ]);

    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(report["files"][0]["language"], "javascript");
    let mut function = report["files"][0]["functions"][0].clone();
    function
        .as_object_mut()
        .expect("function should be an object")
        .remove("signals");
    assert_eq!(
        function,
        serde_json::json!({
            "id": "tests/fixtures/javascript/if_scoring.js:1:1",
            "name": "score",
            "kind": "function",
            "range": {
                "start": { "line": 1, "column": 1 },
                "end": { "line": 7, "column": 2 }
            },
            "score": 5,
            "over_limit": true,
            "contributions": [
                {
                    "rule": "if",
                    "location": { "line": 2, "column": 3 },
                    "base_increment": 1,
                    "nesting_increment": 0,
                    "increment": 1
                },
                {
                    "rule": "if",
                    "location": { "line": 3, "column": 5 },
                    "base_increment": 1,
                    "nesting_increment": 1,
                    "increment": 2
                },
                {
                    "rule": "else_if",
                    "location": { "line": 4, "column": 5 },
                    "base_increment": 1,
                    "nesting_increment": 0,
                    "increment": 1
                },
                {
                    "rule": "else",
                    "location": { "line": 5, "column": 5 },
                    "base_increment": 1,
                    "nesting_increment": 0,
                    "increment": 1
                }
            ]
        })
    );
}

#[test]
fn javascript_callables_keep_names_kinds_ranges_and_unicode_columns() {
    let output = run(&[
        "--format",
        "json",
        "--max-complexity",
        "99",
        "tests/fixtures/javascript/function_forms.js",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    let facts = functions
        .iter()
        .map(|function| {
            (
                function["id"].as_str().expect("id"),
                function["name"].as_str().expect("name"),
                function["kind"].as_str().expect("kind"),
                function["range"]["start"]["line"]
                    .as_u64()
                    .expect("start line"),
                function["range"]["start"]["column"]
                    .as_u64()
                    .expect("start column"),
                function["range"]["end"]["line"].as_u64().expect("end line"),
                function["range"]["end"]["column"]
                    .as_u64()
                    .expect("end column"),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        facts,
        vec![
            (
                "tests/fixtures/javascript/function_forms.js:1:1",
                "declared",
                "function",
                1,
                1,
                6,
                2,
            ),
            (
                "tests/fixtures/javascript/function_forms.js:3:3",
                "nested",
                "function",
                3,
                3,
                5,
                4,
            ),
            (
                "tests/fixtures/javascript/function_forms.js:8:20",
                "expressed",
                "function",
                8,
                20,
                10,
                2,
            ),
            (
                "tests/fixtures/javascript/function_forms.js:12:15",
                "arrow",
                "arrow",
                12,
                15,
                14,
                2,
            ),
            (
                "tests/fixtures/javascript/function_forms.js:17:3",
                "constructor",
                "constructor",
                17,
                3,
                19,
                4,
            ),
            (
                "tests/fixtures/javascript/function_forms.js:21:3",
                "method",
                "method",
                21,
                3,
                23,
                4,
            ),
            (
                "tests/fixtures/javascript/function_forms.js:25:3",
                "value",
                "getter",
                25,
                3,
                27,
                4,
            ),
            (
                "tests/fixtures/javascript/function_forms.js:29:3",
                "value",
                "setter",
                29,
                3,
                31,
                4,
            ),
            (
                "tests/fixtures/javascript/function_forms.js:34:11",
                "λ",
                "arrow",
                34,
                11,
                36,
                2,
            ),
        ]
    );
    assert!(
        functions
            .iter()
            .all(|function| function["score"] == 1 && function["contributions"][0]["rule"] == "if")
    );
}

#[test]
fn javascript_structural_controls_keep_core_v1_nesting() {
    let output = run(&[
        "--format",
        "json",
        "--max-complexity",
        "99",
        "tests/fixtures/javascript/structural_controls.js",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(
        score_facts(&report),
        vec![
            (
                "loops".to_string(),
                6,
                vec![
                    ("loop".to_string(), 2, 3, 1, 0, 1),
                    ("loop".to_string(), 3, 5, 1, 1, 2),
                    ("loop".to_string(), 5, 3, 1, 0, 1),
                    ("loop".to_string(), 6, 3, 1, 0, 1),
                    ("loop".to_string(), 7, 3, 1, 0, 1),
                ],
            ),
            (
                "switched".to_string(),
                5,
                vec![
                    ("switch".to_string(), 11, 3, 1, 0, 1),
                    ("if".to_string(), 13, 7, 1, 1, 2),
                    ("if".to_string(), 16, 7, 1, 1, 2),
                ],
            ),
            (
                "recovered".to_string(),
                5,
                vec![
                    ("if".to_string(), 22, 5, 1, 0, 1),
                    ("catch".to_string(), 23, 5, 1, 0, 1),
                    ("if".to_string(), 24, 5, 1, 1, 2),
                    ("if".to_string(), 26, 5, 1, 0, 1),
                ],
            ),
            (
                "ternary".to_string(),
                5,
                vec![
                    ("ternary".to_string(), 31, 16, 1, 0, 1),
                    ("ternary".to_string(), 31, 29, 1, 1, 2),
                    ("ternary".to_string(), 31, 51, 1, 1, 2),
                ],
            ),
            (
                "jumps".to_string(),
                5,
                vec![
                    ("loop".to_string(), 35, 10, 1, 0, 1),
                    ("if".to_string(), 36, 5, 1, 1, 2),
                    ("labeled_jump".to_string(), 37, 7, 1, 0, 1),
                    ("labeled_jump".to_string(), 39, 5, 1, 0, 1),
                ],
            ),
        ]
    );
}

#[test]
fn javascript_logical_runs_and_jsx_suppression_keep_core_v1_scores() {
    let logical = run(&[
        "--format",
        "json",
        "--max-complexity",
        "99",
        "tests/fixtures/javascript/logical_controls.js",
    ]);
    assert!(logical.status.success());
    let logical: serde_json::Value =
        serde_json::from_slice(&logical.stdout).expect("output should be JSON");
    assert_eq!(
        score_facts(&logical),
        vec![(
            "chains".to_string(),
            8,
            vec![
                ("logical_and".to_string(), 2, 24, 1, 0, 1),
                ("logical_and".to_string(), 3, 21, 1, 0, 1),
                ("logical_and".to_string(), 3, 31, 1, 0, 1),
                ("logical_and".to_string(), 4, 27, 1, 0, 1),
                ("logical_and".to_string(), 4, 39, 1, 0, 1),
                ("logical_and".to_string(), 5, 25, 1, 0, 1),
                ("logical_and".to_string(), 6, 19, 1, 0, 1),
                ("logical_and".to_string(), 6, 31, 1, 0, 1),
            ],
        )]
    );

    let jsx = run(&[
        "--format",
        "json",
        "--max-complexity",
        "99",
        "tests/fixtures/javascript/logical_jsx.tsx",
    ]);
    assert!(jsx.status.success());
    let jsx: serde_json::Value =
        serde_json::from_slice(&jsx.stdout).expect("output should be JSON");
    assert_eq!(
        score_facts(&jsx),
        vec![
            ("jsxHomogeneous".to_string(), 0, vec![]),
            (
                "jsxMixed".to_string(),
                2,
                vec![
                    ("logical_and".to_string(), 6, 19, 1, 0, 1),
                    ("logical_and".to_string(), 6, 29, 1, 0, 1),
                ],
            ),
            (
                "jsxTernary".to_string(),
                2,
                vec![
                    ("logical_and".to_string(), 10, 19, 1, 0, 1),
                    ("ternary".to_string(), 10, 24, 1, 0, 1),
                ],
            ),
            (
                "jsxNested".to_string(),
                1,
                vec![("logical_and".to_string(), 14, 24, 1, 0, 1)],
            ),
            (
                "jsxNestedTernary".to_string(),
                1,
                vec![("ternary".to_string(), 18, 29, 1, 0, 1)],
            ),
        ]
    );
}

#[test]
fn javascript_oxc_containers_keep_executable_region_ownership() {
    let output = run(&[
        "--format",
        "json",
        "--max-complexity",
        "99",
        "tests/fixtures/javascript/traversal.tsx",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(
        score_facts(&report),
        vec![
            (
                "containers".to_string(),
                18,
                vec![
                    ("ternary".to_string(), 2, 17, 1, 0, 1),
                    ("logical_and".to_string(), 3, 17, 1, 0, 1),
                    ("loop".to_string(), 5, 3, 1, 0, 1),
                    ("logical_and".to_string(), 5, 26, 1, 0, 1),
                    ("logical_and".to_string(), 5, 40, 1, 0, 1),
                    ("logical_and".to_string(), 5, 61, 1, 0, 1),
                    ("loop".to_string(), 6, 3, 1, 0, 1),
                    ("logical_and".to_string(), 6, 28, 1, 0, 1),
                    ("loop".to_string(), 7, 3, 1, 0, 1),
                    ("logical_and".to_string(), 7, 30, 1, 0, 1),
                    ("logical_and".to_string(), 8, 30, 1, 0, 1),
                    ("logical_and".to_string(), 8, 47, 1, 0, 1),
                    ("logical_and".to_string(), 8, 64, 1, 0, 1),
                    ("logical_and".to_string(), 9, 44, 1, 0, 1),
                    ("logical_and".to_string(), 9, 67, 1, 0, 1),
                    ("logical_and".to_string(), 10, 17, 1, 0, 1),
                    ("logical_and".to_string(), 11, 27, 1, 0, 1),
                    ("ternary".to_string(), 11, 73, 1, 0, 1),
                ],
            ),
            (
                "classOwner".to_string(),
                2,
                vec![
                    ("logical_and".to_string(), 16, 23, 1, 0, 1),
                    ("if".to_string(), 18, 7, 1, 0, 1),
                ],
            ),
            (
                "method".to_string(),
                1,
                vec![("if".to_string(), 21, 7, 1, 0, 1)],
            ),
            (
                "enumOwner".to_string(),
                1,
                vec![("logical_and".to_string(), 29, 22, 1, 0, 1)],
            ),
        ]
    );
}

#[test]
fn typescript_bodyless_callables_do_not_create_results() {
    let output = run(&[
        "--format",
        "json",
        "--max-complexity",
        "99",
        "tests/fixtures/javascript/bodyless.ts",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    assert_eq!(functions.len(), 2);
    assert_eq!(
        functions[0]["id"],
        "tests/fixtures/javascript/bodyless.ts:5:1"
    );
    assert_eq!(functions[0]["name"], "overloaded");
    assert_eq!(
        functions[0]["range"]["end"],
        serde_json::json!({"line": 7, "column": 2})
    );
    assert_eq!(
        functions[1]["id"],
        "tests/fixtures/javascript/bodyless.ts:12:3"
    );
    assert_eq!(functions[1]["name"], "complete");
    assert_eq!(functions[1]["kind"], "method");
    assert_eq!(
        functions[1]["range"]["end"],
        serde_json::json!({"line": 14, "column": 4})
    );
}

#[test]
fn javascript_contribution_tokens_skip_comments_and_container_syntax() {
    let comments = run(&[
        "--format",
        "json",
        "--max-complexity",
        "99",
        "tests/fixtures/javascript/comment_anchors.js",
    ]);
    assert!(comments.status.success());
    let comments: serde_json::Value =
        serde_json::from_slice(&comments.stdout).expect("output should be JSON");
    assert_eq!(
        score_facts(&comments),
        vec![(
            "commentAnchors".to_string(),
            2,
            vec![
                ("ternary".to_string(), 2, 24, 1, 0, 1),
                ("logical_and".to_string(), 2, 40, 1, 0, 1),
            ],
        )]
    );

    let statements = run(&[
        "--format",
        "json",
        "--max-complexity",
        "99",
        "tests/fixtures/javascript/statement_containers.cjs",
    ]);
    assert!(statements.status.success());
    let statements: serde_json::Value =
        serde_json::from_slice(&statements.stdout).expect("output should be JSON");
    assert_eq!(
        score_facts(&statements),
        vec![(
            "statementContainers".to_string(),
            4,
            vec![
                ("loop".to_string(), 2, 3, 1, 0, 1),
                ("logical_and".to_string(), 2, 16, 1, 0, 1),
                ("logical_and".to_string(), 3, 15, 1, 0, 1),
                ("logical_and".to_string(), 4, 10, 1, 0, 1),
            ],
        )]
    );
}

#[test]
fn javascript_and_typescript_extensions_keep_their_oxc_source_modes() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/modes/component.js",
        "tests/fixtures/javascript/modes/component.jsx",
        "tests/fixtures/javascript/modes/component.mjs",
        "tests/fixtures/javascript/modes/component.cjs",
        "tests/fixtures/javascript/modes/plain.ts",
        "tests/fixtures/javascript/modes/plain.mts",
        "tests/fixtures/javascript/modes/plain.cts",
        "tests/fixtures/javascript/modes/component.tsx",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let files = report["files"]
        .as_array()
        .expect("files should be an array");
    assert_eq!(files.len(), 8);
    assert!(files.iter().all(|file| file["status"] == "ok"));
    assert_eq!(
        files
            .iter()
            .map(|file| file["language"].as_str().expect("language"))
            .collect::<Vec<_>>(),
        vec![
            "javascript",
            "javascript",
            "jsx",
            "javascript",
            "tsx",
            "typescript",
            "typescript",
            "typescript",
        ]
    );
}

#[test]
fn typescript_without_tsx_extension_rejects_jsx() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/modes/jsx-in-ts.ts",
    ]);

    assert_eq!(output.status.code(), Some(2));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(report["files"][0]["status"], "parse_error");
    assert_eq!(report["files"][0]["signals"], serde_json::Value::Null);
    assert_eq!(report["files"][0]["functions"], serde_json::json!([]));
    assert!(
        !report["files"][0]["diagnostics"]
            .as_array()
            .expect("diagnostics should be an array")
            .is_empty()
    );
}

#[test]
fn javascript_parse_errors_keep_oxc_diagnostic_locations() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/parse_error.js",
    ]);

    assert_eq!(output.status.code(), Some(2));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    assert_eq!(
        report["files"][0]["diagnostics"],
        serde_json::json!([{
            "location": { "line": 2, "column": 17 },
            "message": "Unexpected token"
        }])
    );
}

#[test]
fn javascript_function_signals_use_inclusive_parser_ranges_for_line_span() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_span.js",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    assert_eq!(functions[0]["signals"]["line_span"], 1);
    assert_eq!(functions[1]["signals"]["line_span"], 5);
}

#[test]
fn javascript_control_depth_is_zero_without_control_and_one_for_a_flat_region() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_control.js",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    assert_eq!(functions[0]["signals"]["max_control_depth"], 0);
    assert_eq!(functions[1]["signals"]["max_control_depth"], 1);
}

#[test]
fn javascript_control_depth_tracks_loop_switch_catch_and_ternary_regions() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_control.js",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    let depths = functions
        .iter()
        .filter_map(|function| {
            let name = function["name"].as_str().expect("name");
            matches!(
                name,
                "none" | "flat" | "boundaries" | "tryFinally" | "nested"
            )
            .then_some((
                name,
                function["signals"]["max_control_depth"]
                    .as_u64()
                    .expect("control depth"),
            ))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        depths,
        vec![
            ("none", 0),
            ("flat", 1),
            ("boundaries", 1),
            ("tryFinally", 0),
            ("nested", 5),
        ]
    );
}

#[test]
fn javascript_nested_callables_reset_control_depth_and_stay_out_of_the_parent() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_control.js",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    let outer = functions
        .iter()
        .find(|function| function["name"] == "outerReset")
        .expect("outer function");
    let nested = functions
        .iter()
        .find(|function| function["name"] == "nestedReset")
        .expect("nested function");
    assert_eq!(outer["signals"]["max_control_depth"], 1);
    assert_eq!(nested["signals"]["max_control_depth"], 2);
    assert_eq!(outer["signals"]["condition_count"], 1);
    assert_eq!(nested["signals"]["condition_count"], 2);
}

#[test]
fn javascript_empty_control_regions_each_reach_depth_one() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_control.js",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    for name in [
        "loopBoundary",
        "switchBoundary",
        "catchBoundary",
        "ternaryBoundary",
    ] {
        let function = functions
            .iter()
            .find(|function| function["name"] == name)
            .expect("boundary function");
        assert_eq!(
            function["signals"]["max_control_depth"], 1,
            "wrong depth for {name}"
        );
    }
}

#[test]
fn javascript_atomic_if_condition_reports_one_predicate_and_no_boolean_operator() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_control.js",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    let none = &functions[0]["signals"];
    assert_eq!(none["condition_count"], 0);
    assert_eq!(none["conditions"], serde_json::json!([]));

    let flat = &functions[1]["signals"];
    assert_eq!(flat["condition_count"], 1);
    assert_eq!(
        flat["conditions"],
        serde_json::json!([{
            "kind": "if",
            "location": { "line": 4, "column": 7 },
            "operator_count": 0,
            "predicate_count": 1,
            "max_boolean_depth": 0
        }])
    );
    assert_eq!(flat["max_condition_operators"], 0);
    assert_eq!(flat["max_condition_predicates"], 1);
    assert_eq!(flat["max_boolean_depth"], 0);
}

#[test]
fn javascript_flat_boolean_chain_has_two_operators_three_predicates_and_depth_one() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_conditions.ts",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let signals = &report["files"][0]["functions"][0]["signals"];
    assert_eq!(
        signals["conditions"],
        serde_json::json!([{
            "kind": "if",
            "location": { "line": 2, "column": 7 },
            "operator_count": 2,
            "predicate_count": 3,
            "max_boolean_depth": 1
        }])
    );
    assert_eq!(signals["max_condition_operators"], 2);
    assert_eq!(signals["max_condition_predicates"], 3);
    assert_eq!(signals["max_boolean_depth"], 1);
}

#[test]
fn javascript_mixed_boolean_tree_adds_depth_for_mixed_operators_and_not() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_conditions.ts",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let condition = &report["files"][0]["functions"][1]["signals"]["conditions"][0];
    assert_eq!(
        condition,
        &serde_json::json!({
            "kind": "while",
            "location": { "line": 6, "column": 10 },
            "operator_count": 3,
            "predicate_count": 3,
            "max_boolean_depth": 3
        })
    );
}

#[test]
fn javascript_condition_shape_finds_nested_operators_but_stops_at_nested_callables() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_conditions.ts",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let conditions = &report["files"][0]["functions"][2]["signals"]["conditions"];
    assert_eq!(
        conditions,
        &serde_json::json!([
            {
                "kind": "if",
                "location": { "line": 10, "column": 7 },
                "operator_count": 1,
                "predicate_count": 2,
                "max_boolean_depth": 1
            },
            {
                "kind": "if",
                "location": { "line": 11, "column": 7 },
                "operator_count": 1,
                "predicate_count": 2,
                "max_boolean_depth": 1
            },
            {
                "kind": "if",
                "location": { "line": 12, "column": 7 },
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            }
        ])
    );
}

#[test]
fn typescript_condition_shape_ignores_parentheses_and_type_wrappers() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_wrappers.ts",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let conditions = report["files"][0]["functions"][0]["signals"]["conditions"]
        .as_array()
        .expect("conditions should be an array");
    let shapes = conditions
        .iter()
        .map(|condition| {
            (
                condition["operator_count"].as_u64().expect("operators"),
                condition["predicate_count"].as_u64().expect("predicates"),
                condition["max_boolean_depth"]
                    .as_u64()
                    .expect("boolean depth"),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        shapes,
        vec![(2, 3, 1), (2, 3, 1), (2, 3, 1), (2, 3, 1), (0, 1, 0)]
    );
}

#[test]
fn javascript_nullish_coalescing_counts_as_a_distinct_boolean_operator() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_conditions.ts",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let function = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array")
        .iter()
        .find(|function| function["name"] == "nullish")
        .expect("nullish function");
    let condition = &function["signals"]["conditions"][0];
    assert_eq!(condition["operator_count"], 2);
    assert_eq!(condition["predicate_count"], 3);
    assert_eq!(condition["max_boolean_depth"], 2);
}

#[test]
fn javascript_condition_records_keep_kinds_order_maxima_and_exclusions() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/javascript/signals_records.js",
    ]);

    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let signals = &report["files"][0]["functions"][0]["signals"];
    assert_eq!(
        signals["conditions"],
        serde_json::json!([
            {
                "kind": "if",
                "location": { "line": 2, "column": 7 },
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            },
            {
                "kind": "else_if",
                "location": { "line": 3, "column": 14 },
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            },
            {
                "kind": "while",
                "location": { "line": 4, "column": 10 },
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            },
            {
                "kind": "do_while",
                "location": { "line": 5, "column": 16 },
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            },
            {
                "kind": "for",
                "location": { "line": 6, "column": 23 },
                "operator_count": 1,
                "predicate_count": 2,
                "max_boolean_depth": 1
            },
            {
                "kind": "ternary",
                "location": { "line": 10, "column": 17 },
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            },
            {
                "kind": "ternary",
                "location": { "line": 10, "column": 22 },
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            }
        ])
    );
    assert_eq!(signals["condition_count"], 7);
    assert_eq!(signals["max_condition_operators"], 1);
    assert_eq!(signals["max_condition_predicates"], 2);
    assert_eq!(signals["max_boolean_depth"], 1);
}
