use std::{
    io::Write,
    process::{Command, Stdio},
};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_complexity"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("complexity command should run")
}

fn run_stdin(args: &[&str], source: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_complexity"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("complexity command should start");
    child
        .stdin
        .take()
        .expect("complexity command should accept standard input")
        .write_all(source.as_bytes())
        .expect("source should be written to standard input");
    child
        .wait_with_output()
        .expect("complexity command should finish")
}

fn nested_block_source(depth: usize) -> String {
    format!(
        "fn nested() -> i32 {}0{}",
        "{".repeat(depth),
        "}".repeat(depth)
    )
}

#[test]
fn rust_rejects_unsafe_ast_depth_without_losing_safe_input() {
    let safe_output = run_stdin(
        &["--format", "json", "--language", "rust", "-"],
        &nested_block_source(64),
    );
    assert_eq!(safe_output.status.code(), Some(0));

    let deep_output = run_stdin(
        &["--format", "json", "--language", "rust", "-"],
        &nested_block_source(5_000),
    );

    assert_eq!(deep_output.status.code(), Some(2));
    let report: serde_json::Value =
        serde_json::from_slice(&deep_output.stdout).expect("output should be valid JSON");
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["files"][0]["status"], "parse_error");
    assert_eq!(report["files"][0]["signals"], serde_json::Value::Null);
    assert_eq!(report["files"][0]["functions"], serde_json::json!([]));
    assert_eq!(
        report["files"][0]["diagnostics"][0]["message"],
        "analysis nesting exceeds the supported limit of 512"
    );
}

#[test]
fn rust_empty_function_keeps_its_identity_range_and_zero_score() {
    let output = run(&["--format", "json", "tests/fixtures/rust/empty.rs"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][0];

    assert_eq!(report["files"][0]["path"], "tests/fixtures/rust/empty.rs");
    assert_eq!(report["files"][0]["language"], "rust");
    assert_eq!(report["files"][0]["status"], "ok");
    assert_eq!(function["id"], "tests/fixtures/rust/empty.rs:1:1");
    assert_eq!(function["name"], "empty_case");
    assert_eq!(function["kind"], "function");
    assert_eq!(
        function["range"],
        serde_json::json!({
            "start": {"line": 1, "column": 1},
            "end": {"line": 2, "column": 2}
        })
    );
    assert_eq!(function["score"], 0);
    assert_eq!(function["over_limit"], false);
    assert_eq!(function["contributions"], serde_json::json!([]));
    assert_eq!(
        function["signals"],
        serde_json::json!({
            "line_span": 2,
            "max_control_depth": 0,
            "condition_count": 0,
            "max_condition_operators": 0,
            "max_condition_predicates": 0,
            "max_boolean_depth": 0,
            "conditions": []
        })
    );
}

#[test]
fn rust_scores_structural_flat_and_logical_contributions() {
    let output = run(&["--format", "json", "tests/fixtures/rust/scoring.rs"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][0];

    assert_eq!(function["name"], "scoring");
    assert_eq!(function["score"], 16);
    assert_eq!(function["over_limit"], true);
    assert_eq!(
        function["contributions"],
        serde_json::json!([
            {"rule": "if", "location": {"line": 2, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "loop", "location": {"line": 3, "column": 9}, "base_increment": 1, "nesting_increment": 1, "increment": 2},
            {"rule": "logical_sequence", "location": {"line": 3, "column": 17}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "logical_sequence", "location": {"line": 3, "column": 22}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "else_if", "location": {"line": 6, "column": 12}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "match", "location": {"line": 7, "column": 9}, "base_increment": 1, "nesting_increment": 1, "increment": 2},
            {"rule": "logical_sequence", "location": {"line": 8, "column": 20}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "loop", "location": {"line": 9, "column": 17}, "base_increment": 1, "nesting_increment": 2, "increment": 3},
            {"rule": "labeled_jump", "location": {"line": 10, "column": 21}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "else", "location": {"line": 15, "column": 7}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "loop", "location": {"line": 16, "column": 9}, "base_increment": 1, "nesting_increment": 1, "increment": 2}
        ])
    );
    assert_eq!(
        function["signals"],
        serde_json::json!({
            "line_span": 19,
            "max_control_depth": 3,
            "condition_count": 4,
            "max_condition_operators": 2,
            "max_condition_predicates": 3,
            "max_boolean_depth": 2,
            "conditions": [
                {"kind": "if", "location": {"line": 2, "column": 8}, "operator_count": 0, "predicate_count": 1, "max_boolean_depth": 0},
                {"kind": "while", "location": {"line": 3, "column": 15}, "operator_count": 2, "predicate_count": 3, "max_boolean_depth": 2},
                {"kind": "if", "location": {"line": 6, "column": 15}, "operator_count": 0, "predicate_count": 1, "max_boolean_depth": 0},
                {"kind": "match_guard", "location": {"line": 8, "column": 18}, "operator_count": 2, "predicate_count": 2, "max_boolean_depth": 2}
            ]
        })
    );
    assert_eq!(report["summary"]["max_score"], 16);

    let threshold_output = run(&[
        "--format",
        "json",
        "--max-complexity",
        "16",
        "tests/fixtures/rust/scoring.rs",
    ]);
    assert_eq!(threshold_output.status.code(), Some(0));
    let threshold_report: serde_json::Value =
        serde_json::from_slice(&threshold_output.stdout).expect("output should be valid JSON");
    assert_eq!(
        threshold_report["files"][0]["functions"][0]["over_limit"],
        false
    );
}

#[test]
fn rust_nested_callables_reset_score_and_signals_and_keep_unicode_positions() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/rust/callables_unicode.rs",
    ]);

    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");

    assert_eq!(report["files"][0]["signals"]["function_count"], 4);
    assert_eq!(
        functions
            .iter()
            .map(|function| (
                function["id"].as_str(),
                function["name"].as_str(),
                function["kind"].as_str(),
                function["score"].as_u64(),
                function["signals"]["max_control_depth"].as_u64(),
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                Some("tests/fixtures/rust/callables_unicode.rs:1:1"),
                Some("outer"),
                Some("function"),
                Some(1),
                Some(1)
            ),
            (
                Some("tests/fixtures/rust/callables_unicode.rs:3:19"),
                Some("<anonymous>"),
                Some("closure"),
                Some(1),
                Some(1)
            ),
            (
                Some("tests/fixtures/rust/callables_unicode.rs:4:5"),
                Some("nested"),
                Some("function"),
                Some(1),
                Some(1)
            ),
            (
                Some("tests/fixtures/rust/callables_unicode.rs:10:1"),
                Some("café"),
                Some("function"),
                Some(1),
                Some(1)
            ),
        ]
    );
    assert_eq!(
        functions[3]["contributions"][0]["location"],
        serde_json::json!({"line": 10, "column": 13})
    );
}

#[test]
fn rust_macros_are_opaque_and_syntax_errors_fail_closed() {
    let macro_output = run(&["--format", "json", "tests/fixtures/rust/macros.rs"]);
    assert_eq!(macro_output.status.code(), Some(0));
    let macro_report: serde_json::Value =
        serde_json::from_slice(&macro_output.stdout).expect("output should be valid JSON");
    assert_eq!(macro_report["files"][0]["signals"]["function_count"], 1);
    assert_eq!(macro_report["files"][0]["functions"][0]["name"], "visible");
    assert_eq!(macro_report["files"][0]["functions"][0]["score"], 0);

    let broken_output = run(&["--format", "json", "tests/fixtures/rust/broken.rs"]);
    assert_eq!(broken_output.status.code(), Some(2));
    let broken_report: serde_json::Value =
        serde_json::from_slice(&broken_output.stdout).expect("output should be valid JSON");
    assert_eq!(broken_report["status"], "incomplete");
    assert_eq!(broken_report["files"][0]["status"], "parse_error");
    assert_eq!(
        broken_report["files"][0]["signals"],
        serde_json::Value::Null
    );
    assert_eq!(
        broken_report["files"][0]["functions"],
        serde_json::json!([])
    );
    assert_eq!(broken_report["summary"]["errors"], 1);
}

#[test]
fn rust_methods_are_methods_and_bodyless_trait_signatures_are_not_results() {
    let output = run(&["--format", "json", "tests/fixtures/rust/methods.rs"]);

    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");

    assert_eq!(report["files"][0]["signals"]["function_count"], 1);
    assert_eq!(functions[0]["name"], "measure");
    assert_eq!(functions[0]["kind"], "method");
    assert_eq!(functions[0]["score"], 1);
}
