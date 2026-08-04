use std::process::Command;

fn run(max_complexity: u32) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_complexity"))
        .args([
            "--format",
            "json",
            "--max-complexity",
            &max_complexity.to_string(),
            "tests/fixtures/php/compatibility.php",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("complexity command should run")
}

#[test]
fn frozen_php_core_v1_corpus_keeps_exact_scores_and_contribution_vectors() {
    let output = run(15);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(report["files"][0]["status"], "ok");

    let actual = report["files"][0]["functions"]
        .as_array()
        .expect("function array")
        .iter()
        .map(|function| {
            let contributions = function["contributions"]
                .as_array()
                .expect("contribution array")
                .iter()
                .map(|contribution| {
                    let base = contribution["base_increment"]
                        .as_u64()
                        .expect("integer base");
                    let nesting = contribution["nesting_increment"]
                        .as_u64()
                        .expect("integer nesting");
                    let increment = contribution["increment"]
                        .as_u64()
                        .expect("integer increment");
                    assert_eq!(increment, base + nesting);
                    (
                        contribution["rule"].as_str().expect("rule"),
                        nesting,
                        increment,
                    )
                })
                .collect::<Vec<_>>();
            (
                function["name"].as_str().expect("name"),
                function["kind"].as_str().expect("kind"),
                function["score"].as_u64().expect("integer score"),
                contributions,
            )
        })
        .collect::<Vec<_>>();

    let expected = vec![
        ("nested_if", "function", 3, vec![("if", 0, 1), ("if", 1, 2)]),
        (
            "elseif_case",
            "function",
            2,
            vec![("if", 0, 1), ("elseif", 0, 1)],
        ),
        (
            "else_if_case",
            "function",
            5,
            vec![("if", 0, 1), ("else_if", 0, 1), ("if", 2, 3)],
        ),
        (
            "switch_case",
            "function",
            3,
            vec![("switch", 0, 1), ("if", 1, 2)],
        ),
        (
            "switch_expression_case",
            "function",
            5,
            vec![("switch", 0, 1), ("ternary", 1, 2), ("ternary", 1, 2)],
        ),
        (
            "loop_case",
            "function",
            4,
            vec![
                ("loop", 0, 1),
                ("loop", 0, 1),
                ("loop", 0, 1),
                ("loop", 0, 1),
            ],
        ),
        ("catch_case", "function", 1, vec![("catch", 0, 1)]),
        (
            "ternary_case",
            "function",
            3,
            vec![("ternary", 0, 1), ("ternary", 1, 2)],
        ),
        (
            "logical_case",
            "function",
            2,
            vec![("logical_sequence", 0, 1), ("logical_sequence", 0, 1)],
        ),
        (
            "pipe_case",
            "function",
            2,
            vec![("pipe", 0, 1), ("pipe", 0, 1)],
        ),
        (
            "mixed_operator_case",
            "function",
            7,
            vec![
                ("loop", 0, 1),
                ("logical_sequence", 0, 1),
                ("pipe", 0, 1),
                ("ternary", 1, 2),
                ("logical_sequence", 1, 2),
            ],
        ),
        (
            "break_case",
            "function",
            2,
            vec![("loop", 0, 1), ("numbered_jump", 0, 1)],
        ),
        (
            "continue_case",
            "function",
            2,
            vec![("loop", 0, 1), ("numbered_jump", 0, 1)],
        ),
        ("goto_case", "function", 1, vec![("goto", 0, 1)]),
        ("callable_parent", "function", 1, vec![("if", 0, 1)]),
        ("nested_named", "function", 1, vec![("if", 0, 1)]),
        ("<anonymous>", "closure", 1, vec![("if", 0, 1)]),
        ("<anonymous>", "arrow", 1, vec![("ternary", 0, 1)]),
        ("recursive_case", "function", 1, vec![("if", 0, 1)]),
        (
            "match_case",
            "function",
            3,
            vec![("match", 0, 1), ("ternary", 1, 2)],
        ),
        ("zero_flow", "function", 0, vec![]),
        ("modern_zero", "function", 0, vec![]),
        (
            "alternative_case",
            "function",
            4,
            vec![("if", 0, 1), ("loop", 1, 2), ("else", 0, 1)],
        ),
    ];

    assert_eq!(actual, expected);

    let threshold_output = run(0);
    assert_eq!(threshold_output.status.code(), Some(1));
    let threshold_report: serde_json::Value =
        serde_json::from_slice(&threshold_output.stdout).expect("valid threshold JSON report");
    for function in threshold_report["files"][0]["functions"]
        .as_array()
        .expect("function array")
    {
        let score = function["score"].as_u64().expect("integer score");
        assert_eq!(function["over_limit"], score > 0);
    }
}
