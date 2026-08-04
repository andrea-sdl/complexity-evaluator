use std::io::Write;
use std::process::{Command, Stdio};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_complexity"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("complexity command should run")
}

fn run_with_stdin(args: &[&str], input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_complexity"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("complexity command should start");
    child
        .stdin
        .take()
        .expect("complexity command should accept stdin")
        .write_all(input)
        .expect("complexity command stdin should accept input");
    child
        .wait_with_output()
        .expect("complexity command should finish")
}

fn nested_parentheses(depth: usize) -> String {
    format!(
        "<?php\nfunction nested(): void {{\n    {}$value{};\n}}\n",
        "(".repeat(depth),
        ")".repeat(depth),
    )
}

#[test]
fn php_deep_valid_input_fails_closed_before_recursive_analysis() {
    let output = run_with_stdin(
        &["--language", "php", "--format", "json", "-"],
        nested_parentheses(12_000).as_bytes(),
    );

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
        file["diagnostics"][0]["message"],
        "analysis nesting exceeds the supported limit of 512"
    );

    let control = run_with_stdin(
        &["--language", "php", "--format", "json", "-"],
        nested_parentheses(128).as_bytes(),
    );
    assert_eq!(control.status.code(), Some(0));
    assert!(control.stderr.is_empty());
}

#[test]
fn php_reserved_word_retry_checks_depth_before_recursive_analysis() {
    let source = format!(
        "<?php\nclass DeepRetry {{ private const NAMESPACE = {}1{}; }}\n",
        "(".repeat(600),
        ")".repeat(600),
    );
    let output = run_with_stdin(
        &["--language", "php", "--format", "json", "-"],
        source.as_bytes(),
    );

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
        file["diagnostics"][0]["message"],
        "analysis nesting exceeds the supported limit of 512"
    );
}

#[test]
fn php_empty_function_keeps_its_identity_range_and_zero_score() {
    let output = run(&["--format", "json", "tests/fixtures/php/empty.php"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][0];

    assert_eq!(report["files"][0]["path"], "tests/fixtures/php/empty.php");
    assert_eq!(report["files"][0]["language"], "php");
    assert_eq!(report["files"][0]["status"], "ok");
    assert_eq!(function["id"], "tests/fixtures/php/empty.php:3:1");
    assert_eq!(function["name"], "empty_case");
    assert_eq!(function["kind"], "function");
    assert_eq!(
        function["range"],
        serde_json::json!({
            "start": {"line": 3, "column": 1},
            "end": {"line": 5, "column": 2}
        })
    );
    assert_eq!(function["score"], 0);
    assert_eq!(function["over_limit"], false);
    assert_eq!(function["contributions"], serde_json::json!([]));
    assert_eq!(
        function["signals"],
        serde_json::json!({
            "line_span": 3,
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
fn php_reserved_word_class_constants_do_not_hide_following_methods() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/reserved_constants.php",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let file = &report["files"][0];
    let function = &file["functions"][0];

    assert_eq!(file["status"], "ok");
    assert_eq!(file["signals"]["function_count"], 1);
    assert_eq!(function["name"], "record");
    assert_eq!(function["kind"], "method");
    assert_eq!(function["score"], 1);
    assert_eq!(report["summary"]["errors"], 0);
}

#[test]
fn php_syntax_errors_fail_closed_with_the_original_diagnostic() {
    let output = run(&["--format", "json", "tests/fixtures/php/broken.php"]);

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
            "location": {"line": 3, "column": 1},
            "message": "syntax error near ERROR"
        }])
    );
    assert_eq!(report["summary"]["errors"], 1);
}

#[test]
fn php_reserved_word_retry_does_not_hide_a_second_syntax_error() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/reserved_constant_with_real_error.php",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let file = &report["files"][0];

    assert_eq!(file["status"], "parse_error");
    assert_eq!(file["signals"], serde_json::Value::Null);
    assert_eq!(file["functions"], serde_json::json!([]));
    assert_eq!(
        file["diagnostics"],
        serde_json::json!([{
            "location": {"line": 5, "column": 13},
            "message": "syntax error near ERROR"
        }])
    );
}

#[test]
fn php_mixed_html_keeps_unicode_scalar_function_and_contribution_positions() {
    let output = run(&["--format", "json", "tests/fixtures/php/mixed_unicode.php"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][0];

    assert_eq!(function["id"], "tests/fixtures/php/mixed_unicode.php:2:15");
    assert_eq!(function["name"], "mixed");
    assert_eq!(function["score"], 1);
    assert_eq!(
        function["contributions"],
        serde_json::json!([{
            "rule": "if",
            "location": {"line": 3, "column": 5},
            "base_increment": 1,
            "nesting_increment": 0,
            "increment": 1
        }])
    );
}

#[test]
fn php_nested_callables_get_separate_zero_nesting_results() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/callables_unicode.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");

    assert_eq!(functions.len(), 4);
    assert_eq!(
        (
            functions[0]["id"].as_str(),
            functions[0]["name"].as_str(),
            functions[0]["kind"].as_str(),
            functions[0]["score"].as_u64(),
        ),
        (
            Some("tests/fixtures/php/callables_unicode.php:2:9"),
            Some("outer"),
            Some("function"),
            Some(1),
        )
    );
    assert_eq!(
        (
            functions[1]["id"].as_str(),
            functions[1]["kind"].as_str(),
            functions[1]["score"].as_u64(),
            functions[1]["contributions"][0]["nesting_increment"].as_u64(),
        ),
        (
            Some("tests/fixtures/php/callables_unicode.php:4:20"),
            Some("closure"),
            Some(1),
            Some(0),
        )
    );
    assert_eq!(
        (
            functions[2]["id"].as_str(),
            functions[2]["kind"].as_str(),
            functions[2]["score"].as_u64(),
            functions[2]["signals"]["line_span"].as_u64(),
        ),
        (
            Some("tests/fixtures/php/callables_unicode.php:9:14"),
            Some("arrow"),
            Some(1),
            Some(1),
        )
    );
    assert_eq!(
        (
            functions[3]["id"].as_str(),
            functions[3]["name"].as_str(),
            functions[3]["kind"].as_str(),
            functions[3]["score"].as_u64(),
        ),
        (
            Some("tests/fixtures/php/callables_unicode.php:13:5"),
            Some("method"),
            Some("method"),
            Some(1),
        )
    );
}

#[test]
fn php_reserved_word_retry_stays_at_class_member_scope() {
    for path in [
        "tests/fixtures/php/global_reserved_constant.php",
        "tests/fixtures/php/method_local_reserved_constant.php",
    ] {
        let output = run(&["--format", "json", path]);

        assert_eq!(output.status.code(), Some(2), "{path}");
        assert!(output.stderr.is_empty(), "{path}");

        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let file = &report["files"][0];
        assert_eq!(file["status"], "parse_error", "{path}");
        assert_eq!(file["signals"], serde_json::Value::Null, "{path}");
        assert_eq!(file["functions"], serde_json::json!([]), "{path}");
    }
}

#[test]
fn php_control_depth_counts_each_active_structural_region() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_control_depth.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");

    assert_eq!(functions[0]["name"], "flat_depth");
    assert_eq!(functions[0]["signals"]["max_control_depth"], 1);
    assert_eq!(functions[1]["name"], "nested_depth");
    assert_eq!(functions[1]["signals"]["max_control_depth"], 4);
}

#[test]
fn php_control_depth_follows_catch_ternary_and_match_regions() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_control_depth.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");

    assert_eq!(functions[2]["name"], "catch_depth");
    assert_eq!(functions[2]["signals"]["max_control_depth"], 2);
    assert_eq!(functions[3]["name"], "ternary_depth");
    assert_eq!(functions[3]["signals"]["max_control_depth"], 2);
    assert_eq!(functions[4]["name"], "match_depth");
    assert_eq!(functions[4]["signals"]["max_control_depth"], 2);
}

#[test]
fn php_switch_and_match_enter_depth_only_for_content() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_control_depth.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    let actual = functions[5..9]
        .iter()
        .map(|function| {
            (
                function["name"].as_str(),
                function["signals"]["max_control_depth"].as_u64(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        actual,
        vec![
            (Some("switch_selector_depth"), Some(1)),
            (Some("switch_content_depth"), Some(2)),
            (Some("match_selector_depth"), Some(1)),
            (Some("match_content_depth"), Some(2)),
        ]
    );
    for function in &functions[5..9] {
        let condition_kinds = function["signals"]["conditions"]
            .as_array()
            .expect("conditions should be an array")
            .iter()
            .map(|condition| condition["kind"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(condition_kinds, vec![Some("ternary")]);
    }
}

#[test]
fn php_try_and_finally_do_not_add_control_depth() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_control_depth.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][9];

    assert_eq!(function["name"], "try_finally_depth");
    assert_eq!(function["signals"]["max_control_depth"], 1);
}

#[test]
fn php_flat_boolean_chain_has_one_normalized_depth() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_conditions.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let signals = &report["files"][0]["functions"][0]["signals"];

    assert_eq!(
        signals["conditions"],
        serde_json::json!([{
            "kind": "if",
            "location": {"line": 5, "column": 9},
            "operator_count": 2,
            "predicate_count": 3,
            "max_boolean_depth": 1
        }])
    );
    assert_eq!(signals["condition_count"], 1);
    assert_eq!(signals["max_condition_operators"], 2);
    assert_eq!(signals["max_condition_predicates"], 3);
    assert_eq!(signals["max_boolean_depth"], 1);
}

#[test]
fn php_mixed_boolean_operators_and_not_add_normalized_depth() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_conditions.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let function = &report["files"][0]["functions"][1];
    let signals = &function["signals"];

    assert_eq!(function["name"], "nested_condition");
    assert_eq!(
        signals["conditions"],
        serde_json::json!([{
            "kind": "if",
            "location": {"line": 11, "column": 9},
            "operator_count": 3,
            "predicate_count": 3,
            "max_boolean_depth": 3
        }])
    );
    assert_eq!(signals["max_condition_operators"], 3);
    assert_eq!(signals["max_condition_predicates"], 3);
    assert_eq!(signals["max_boolean_depth"], 3);
    assert_eq!(report["summary"]["conditions"], 20);
    assert_eq!(report["summary"]["max_condition_operators"], 3);
    assert_eq!(report["summary"]["max_condition_predicates"], 3);
    assert_eq!(report["summary"]["max_boolean_depth"], 3);
}

#[test]
fn php_keyword_operators_count_but_pipe_comparison_and_bitwise_do_not() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_conditions.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let conditions = report["files"][0]["functions"][2]["signals"]["conditions"]
        .as_array()
        .expect("conditions should be an array");
    let actual = conditions
        .iter()
        .map(|condition| {
            (
                condition["location"]["line"].as_u64(),
                condition["operator_count"].as_u64(),
                condition["predicate_count"].as_u64(),
                condition["max_boolean_depth"].as_u64(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        actual,
        vec![
            (Some(17), Some(1), Some(2), Some(1)),
            (Some(18), Some(1), Some(2), Some(1)),
            (Some(19), Some(1), Some(2), Some(1)),
            (Some(20), Some(1), Some(2), Some(1)),
            (Some(21), Some(1), Some(2), Some(1)),
            (Some(22), Some(0), Some(1), Some(0)),
        ]
    );
}

#[test]
fn php_elseif_and_spaced_else_if_keep_distinct_kinds_and_depths() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_conditions.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let signals = &report["files"][0]["functions"][3]["signals"];

    assert_eq!(signals["max_control_depth"], 2);
    assert_eq!(
        signals["conditions"],
        serde_json::json!([
            {
                "kind": "if",
                "location": {"line": 27, "column": 9},
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            },
            {
                "kind": "elseif",
                "location": {"line": 28, "column": 15},
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            },
            {
                "kind": "else_if",
                "location": {"line": 29, "column": 16},
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            }
        ])
    );
}

#[test]
fn php_loop_and_ternary_conditions_are_ordered_and_exclusions_stay_out() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_conditions.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let signals = &report["files"][0]["functions"][4]["signals"];
    let actual = signals["conditions"]
        .as_array()
        .expect("conditions should be an array")
        .iter()
        .map(|condition| {
            (
                condition["kind"].as_str(),
                condition["location"]["line"].as_u64(),
                condition["location"]["column"].as_u64(),
                condition["operator_count"].as_u64(),
                condition["predicate_count"].as_u64(),
                condition["max_boolean_depth"].as_u64(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        actual,
        vec![
            (Some("while"), Some(35), Some(12), Some(0), Some(1), Some(0),),
            (
                Some("do_while"),
                Some(36),
                Some(18),
                Some(0),
                Some(1),
                Some(0),
            ),
            (Some("for"), Some(37), Some(22), Some(1), Some(2), Some(1),),
            (
                Some("ternary"),
                Some(41),
                Some(12),
                Some(0),
                Some(1),
                Some(0),
            ),
            (
                Some("ternary"),
                Some(41),
                Some(18),
                Some(0),
                Some(1),
                Some(0),
            ),
        ]
    );
    assert_eq!(signals["condition_count"], 5);
    assert_eq!(signals["max_condition_operators"], 1);
    assert_eq!(signals["max_condition_predicates"], 2);
    assert_eq!(signals["max_boolean_depth"], 1);
}

#[test]
fn php_nested_callable_signal_state_does_not_leak_into_its_parent() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_callable_boundary.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let functions = report["files"][0]["functions"]
        .as_array()
        .expect("functions should be an array");
    let parent_signals = &functions[0]["signals"];
    let closure_signals = &functions[1]["signals"];

    assert_eq!(functions[0]["name"], "callable_condition");
    assert_eq!(parent_signals["max_control_depth"], 1);
    assert_eq!(parent_signals["condition_count"], 1);
    assert_eq!(parent_signals["max_condition_operators"], 1);
    assert_eq!(parent_signals["max_condition_predicates"], 2);
    assert_eq!(parent_signals["max_boolean_depth"], 1);

    assert_eq!(functions[1]["kind"], "closure");
    assert_eq!(closure_signals["max_control_depth"], 1);
    assert_eq!(closure_signals["condition_count"], 1);
    assert_eq!(closure_signals["max_condition_operators"], 1);
    assert_eq!(closure_signals["max_condition_predicates"], 2);
    assert_eq!(closure_signals["max_boolean_depth"], 1);
}

#[test]
fn php_nested_ternary_is_atomic_in_its_containing_condition() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_conditions.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let signals = &report["files"][0]["functions"][5]["signals"];

    assert_eq!(signals["max_control_depth"], 1);
    assert_eq!(
        signals["conditions"],
        serde_json::json!([
            {
                "kind": "if",
                "location": {"line": 46, "column": 9},
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            },
            {
                "kind": "ternary",
                "location": {"line": 46, "column": 9},
                "operator_count": 0,
                "predicate_count": 1,
                "max_boolean_depth": 0
            }
        ])
    );
}

#[test]
fn php_for_and_ternary_locations_keep_explicit_grouping_parentheses() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_conditions.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let conditions = &report["files"][0]["functions"][6]["signals"]["conditions"];

    assert_eq!(
        conditions,
        &serde_json::json!([
            {
                "kind": "for",
                "location": {"line": 51, "column": 22},
                "operator_count": 1,
                "predicate_count": 2,
                "max_boolean_depth": 1
            },
            {
                "kind": "ternary",
                "location": {"line": 52, "column": 12},
                "operator_count": 1,
                "predicate_count": 2,
                "max_boolean_depth": 1
            }
        ])
    );
}

#[test]
fn php_control_conditions_strip_only_their_required_parentheses() {
    let output = run(&[
        "--format",
        "json",
        "tests/fixtures/php/signal_grouped_control_locations.php",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let conditions = report["files"][0]["functions"][0]["signals"]["conditions"]
        .as_array()
        .expect("conditions should be an array");
    let actual = conditions
        .iter()
        .map(|condition| {
            (
                condition["kind"].as_str(),
                condition["location"]["line"].as_u64(),
                condition["location"]["column"].as_u64(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        actual,
        vec![
            (Some("if"), Some(5), Some(9)),
            (Some("elseif"), Some(6), Some(15)),
            (Some("while"), Some(8), Some(12)),
            (Some("do_while"), Some(9), Some(18)),
        ]
    );
}
