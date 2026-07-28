use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_complexity"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("complexity command should run")
}

#[test]
fn python_empty_function_keeps_its_identity_range_and_zero_score() {
    let output = run(&["--format", "json", "tests/fixtures/python/empty.py"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][0];

    assert_eq!(report["files"][0]["path"], "tests/fixtures/python/empty.py");
    assert_eq!(report["files"][0]["language"], "python");
    assert_eq!(report["files"][0]["status"], "ok");
    assert_eq!(function["id"], "tests/fixtures/python/empty.py:1:1");
    assert_eq!(function["name"], "empty_case");
    assert_eq!(function["kind"], "function");
    assert_eq!(
        function["range"],
        serde_json::json!({
            "start": {"line": 1, "column": 1},
            "end": {"line": 2, "column": 9}
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
fn python_scores_structural_flat_and_logical_contributions() {
    let output = run(&[
        "--format",
        "json",
        "--max-complexity",
        "13",
        "tests/fixtures/python/scoring.py",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][0];

    assert_eq!(function["score"], 14);
    assert_eq!(function["over_limit"], true);
    assert_eq!(
        function["contributions"],
        serde_json::json!([
            {"rule": "if", "location": {"line": 2, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "if", "location": {"line": 3, "column": 9}, "base_increment": 1, "nesting_increment": 1, "increment": 2},
            {"rule": "elif", "location": {"line": 5, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "else", "location": {"line": 7, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "loop", "location": {"line": 9, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "if", "location": {"line": 10, "column": 9}, "base_increment": 1, "nesting_increment": 1, "increment": 2},
            {"rule": "else", "location": {"line": 12, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "except", "location": {"line": 16, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "else", "location": {"line": 18, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "ternary", "location": {"line": 22, "column": 16}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "logical_sequence", "location": {"line": 23, "column": 14}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "logical_sequence", "location": {"line": 23, "column": 21}, "base_increment": 1, "nesting_increment": 0, "increment": 1}
        ])
    );
    assert_eq!(
        function["signals"],
        serde_json::json!({
            "line_span": 23,
            "max_control_depth": 2,
            "condition_count": 5,
            "max_condition_operators": 0,
            "max_condition_predicates": 1,
            "max_boolean_depth": 0,
            "conditions": [
                {"kind": "if", "location": {"line": 2, "column": 8}, "operator_count": 0, "predicate_count": 1, "max_boolean_depth": 0},
                {"kind": "if", "location": {"line": 3, "column": 12}, "operator_count": 0, "predicate_count": 1, "max_boolean_depth": 0},
                {"kind": "elif", "location": {"line": 5, "column": 10}, "operator_count": 0, "predicate_count": 1, "max_boolean_depth": 0},
                {"kind": "if", "location": {"line": 10, "column": 12}, "operator_count": 0, "predicate_count": 1, "max_boolean_depth": 0},
                {"kind": "ternary", "location": {"line": 22, "column": 19}, "operator_count": 0, "predicate_count": 1, "max_boolean_depth": 0}
            ]
        })
    );
}

#[test]
fn python_nested_callables_use_separate_ranges_and_reset_nesting() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/python/callables_unicode.py",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");

    let facts = functions
        .iter()
        .map(|function| {
            (
                function["id"].as_str(),
                function["name"].as_str(),
                function["kind"].as_str(),
                function["range"].clone(),
                function["score"].as_u64(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        facts,
        vec![
            (
                Some("tests/fixtures/python/callables_unicode.py:2:1"),
                Some("decorated"),
                Some("function"),
                serde_json::json!({"start": {"line": 2, "column": 1}, "end": {"line": 4, "column": 13}}),
                Some(1)
            ),
            (
                Some("tests/fixtures/python/callables_unicode.py:9:5"),
                Some("method"),
                Some("method"),
                serde_json::json!({"start": {"line": 9, "column": 5}, "end": {"line": 10, "column": 51}}),
                Some(0)
            ),
            (
                Some("tests/fixtures/python/callables_unicode.py:10:16"),
                Some("<anonymous>"),
                Some("lambda"),
                serde_json::json!({"start": {"line": 10, "column": 16}, "end": {"line": 10, "column": 51}}),
                Some(1)
            ),
            (
                Some("tests/fixtures/python/callables_unicode.py:13:1"),
                Some("outer"),
                Some("function"),
                serde_json::json!({"start": {"line": 13, "column": 1}, "end": {"line": 18, "column": 55}}),
                Some(1)
            ),
            (
                Some("tests/fixtures/python/callables_unicode.py:15:9"),
                Some("nested"),
                Some("function"),
                serde_json::json!({"start": {"line": 15, "column": 9}, "end": {"line": 17, "column": 21}}),
                Some(1)
            ),
            (
                Some("tests/fixtures/python/callables_unicode.py:18:20"),
                Some("<anonymous>"),
                Some("lambda"),
                serde_json::json!({"start": {"line": 18, "column": 20}, "end": {"line": 18, "column": 55}}),
                Some(1)
            ),
        ]
    );
    assert_eq!(report["files"][0]["signals"]["function_count"], 6);
    assert_eq!(functions[3]["signals"]["condition_count"], 1);
    assert_eq!(functions[3]["signals"]["max_control_depth"], 1);
    assert_eq!(functions[4]["contributions"][0]["nesting_increment"], 0);
    assert_eq!(functions[5]["contributions"][0]["nesting_increment"], 0);
}

#[test]
fn python_nested_decorators_stay_out_of_parent_score_and_signals() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/python/nested_decorated.py",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0]["name"], "outer");
    assert_eq!(functions[0]["score"], 0);
    assert_eq!(functions[0]["contributions"], serde_json::json!([]));
    assert_eq!(functions[0]["signals"]["max_control_depth"], 0);
    assert_eq!(functions[0]["signals"]["condition_count"], 0);
    assert_eq!(functions[1]["name"], "inner");
    assert_eq!(functions[1]["score"], 0);
    assert_eq!(functions[2]["name"], "inner_two");
    assert_eq!(functions[2]["score"], 0);
}

#[test]
fn python_try_else_counts_the_else_region_without_nested_control() {
    let output = run(&["--format", "json", "tests/fixtures/python/try_else.py"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][0];

    assert_eq!(function["score"], 1);
    assert_eq!(function["signals"]["max_control_depth"], 1);
}

#[test]
fn python_match_and_with_are_neutral_but_case_guards_and_async_for_apply_rules() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/python/neutral_controls.py",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][0];

    assert_eq!(function["score"], 4);
    assert_eq!(
        function["contributions"],
        serde_json::json!([
            {"rule": "logical_sequence", "location": {"line": 4, "column": 29}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "logical_sequence", "location": {"line": 4, "column": 40}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "loop", "location": {"line": 6, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1},
            {"rule": "else", "location": {"line": 8, "column": 5}, "base_increment": 1, "nesting_increment": 0, "increment": 1}
        ])
    );
    assert_eq!(
        function["signals"],
        serde_json::json!({
            "line_span": 9,
            "max_control_depth": 1,
            "condition_count": 1,
            "max_condition_operators": 3,
            "max_condition_predicates": 3,
            "max_boolean_depth": 3,
            "conditions": [
                {"kind": "case_guard", "location": {"line": 4, "column": 23}, "operator_count": 3, "predicate_count": 3, "max_boolean_depth": 3}
            ]
        })
    );
}

#[test]
fn python_syntax_errors_fail_closed() {
    let output = run(&["--format", "json", "tests/fixtures/python/broken.py"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let file = &report["files"][0];

    assert_eq!(report["status"], "incomplete");
    assert_eq!(file["status"], "parse_error");
    assert_eq!(file["signals"], serde_json::Value::Null);
    assert_eq!(file["functions"], serde_json::json!([]));
    assert_eq!(
        file["diagnostics"],
        serde_json::json!([{
            "location": {"line": 1, "column": 1},
            "message": "syntax error near ERROR"
        }])
    );
    assert_eq!(report["summary"]["errors"], 1);
}
