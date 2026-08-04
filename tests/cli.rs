use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PROJECT_ID: AtomicU64 = AtomicU64::new(0);

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new() -> Self {
        let id = NEXT_PROJECT_ID.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("complexity-cli-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).expect("test project should be created");
        Self { root }
    }

    fn write(&self, path: &str, source: &str) {
        self.write_bytes(path, source.as_bytes());
    }

    fn write_bytes(&self, path: &str, source: &[u8]) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().expect("file should have a parent"))
            .expect("test directory should be created");
        fs::write(path, source).expect("test file should be written");
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_complexity"))
            .current_dir(&self.root)
            .args(args)
            .output()
            .expect("complexity command should run")
    }

    fn run_with_stdin(&self, args: &[&str], input: &[u8]) -> std::process::Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_complexity"))
            .current_dir(&self.root)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("complexity command should start");
        child
            .stdin
            .take()
            .expect("stdin should be piped")
            .write_all(input)
            .expect("stdin should be written");
        child.wait_with_output().expect("complexity should finish")
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("test project should be removed");
    }
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_complexity"))
        .args(args)
        .output()
        .expect("complexity command should run")
}

#[test]
fn public_command_describes_the_merged_cli() {
    let help = run(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(help.stdout).expect("help should be UTF-8"),
        "Usage: complexity [--language javascript|typescript|php|rust|python]... \
[--format text|json] [--max-complexity N] [--stdin-filename PATH] <path...|->\n"
    );
    assert!(help.stderr.is_empty());

    let version = run(&["--version"]);
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(version.stdout).expect("version should be UTF-8"),
        "complexity 0.3.1\n"
    );
    assert!(version.stderr.is_empty());
}

#[test]
fn command_requires_an_input_path() {
    let output = run(&[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: at least one path is required\n"
    );
}

#[test]
fn json_reports_one_valid_file_with_no_functions() {
    let project = TestProject::new();
    project.write("src/a.js", "let answer = 42;\n");

    let output = project.run(&["--format", "json", "src/a.js"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).expect("report should be UTF-8"),
        concat!(
            "{\"schema_version\":2,\"tool\":{\"name\":\"complexity\",\"version\":\"0.3.1\"},",
            "\"profile\":\"core-v1\",\"max_complexity\":15,\"status\":\"complete\",",
            "\"files\":[{\"path\":\"src/a.js\",\"language\":\"javascript\",\"status\":\"ok\",",
            "\"signals\":{\"function_count\":0},\"functions\":[],\"diagnostics\":[]}],",
            "\"summary\":{\"files\":1,\"functions\":0,\"violations\":0,\"errors\":0,",
            "\"max_score\":0,\"max_control_depth\":0,\"max_function_line_span\":0,",
            "\"max_functions_per_file\":0,\"conditions\":0,\"max_condition_operators\":0,",
            "\"max_condition_predicates\":0,\"max_boolean_depth\":0}}\n"
        )
    );
}

#[test]
fn exact_mixed_json_includes_nonzero_signals_and_all_summary_maxima() {
    let project = TestProject::new();
    project.write("a.php", "<?php\n");
    project.write(
        "z.js",
        "function js(a, b, c) {\n  if (a && (b || !c)) {}\n}\n",
    );

    let output = project.run(&["--format", "json", "z.js", "a.php"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).expect("report should be UTF-8"),
        concat!(
            "{\"schema_version\":2,\"tool\":{\"name\":\"complexity\",\"version\":\"0.3.1\"},",
            "\"profile\":\"core-v1\",\"max_complexity\":15,\"status\":\"complete\",\"files\":[",
            "{\"path\":\"a.php\",\"language\":\"php\",\"status\":\"ok\",",
            "\"signals\":{\"function_count\":0},\"functions\":[],\"diagnostics\":[]},",
            "{\"path\":\"z.js\",\"language\":\"javascript\",\"status\":\"ok\",",
            "\"signals\":{\"function_count\":1},\"functions\":[{\"id\":\"z.js:1:1\",",
            "\"name\":\"js\",\"kind\":\"function\",\"range\":{\"start\":{\"line\":1,\"column\":1},",
            "\"end\":{\"line\":3,\"column\":2}},\"score\":2,\"over_limit\":false,",
            "\"contributions\":[{\"rule\":\"if\",\"location\":{\"line\":2,\"column\":3},",
            "\"base_increment\":1,\"nesting_increment\":0,\"increment\":1},",
            "{\"rule\":\"logical_and\",\"location\":{\"line\":2,\"column\":9},",
            "\"base_increment\":1,\"nesting_increment\":0,\"increment\":1}],",
            "\"signals\":{\"line_span\":3,\"max_control_depth\":1,\"condition_count\":1,",
            "\"max_condition_operators\":3,\"max_condition_predicates\":3,",
            "\"max_boolean_depth\":3,\"conditions\":[{\"kind\":\"if\",",
            "\"location\":{\"line\":2,\"column\":7},\"operator_count\":3,",
            "\"predicate_count\":3,\"max_boolean_depth\":3}]}}],\"diagnostics\":[]}],",
            "\"summary\":{\"files\":2,\"functions\":1,\"violations\":0,\"errors\":0,",
            "\"max_score\":2,\"max_control_depth\":1,\"max_function_line_span\":3,",
            "\"max_functions_per_file\":1,\"conditions\":1,\"max_condition_operators\":3,",
            "\"max_condition_predicates\":3,\"max_boolean_depth\":3}}\n"
        )
    );
}

#[test]
fn directory_discovery_reports_mixed_files_in_path_order() {
    let project = TestProject::new();
    project.write("src/z.php", "<?php\n");
    project.write("src/a.tsx", "const answer = 42;\n");
    project.write("src/ignored.txt", "not source\n");

    let output = project.run(&["--format", "json", "src"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("report should be UTF-8");
    let typescript = stdout
        .find("\"path\":\"src/a.tsx\"")
        .expect("TypeScript file should be reported");
    let php = stdout
        .find("\"path\":\"src/z.php\"")
        .expect("PHP file should be reported");
    assert!(typescript < php, "files should use portable path order");
    assert!(!stdout.contains("ignored.txt"));
}

#[test]
fn mixed_overlapping_inputs_use_one_global_canonical_path_order() {
    let project = TestProject::new();
    project.write("z/last.php", "<?php\n");
    project.write("a/first.js", "const first = true;\n");
    project.write("m/middle.ts", "const middle: boolean = true;\n");

    let output = project.run(&[
        "--language",
        "typescript",
        "--language",
        "javascript",
        "--language",
        "php",
        "--language",
        "javascript",
        "--format",
        "json",
        "z",
        "a/first.js",
        "m",
        "a",
    ]);

    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    let paths = report["files"]
        .as_array()
        .expect("files should be an array")
        .iter()
        .map(|file| file["path"].as_str().expect("path should be text"))
        .collect::<Vec<_>>();
    assert_eq!(paths, ["a/first.js", "m/middle.ts", "z/last.php"]);
}

#[test]
fn five_language_reports_are_byte_stable_and_globally_sorted() {
    let project = TestProject::new();
    project.write("e/function.js", "function javascript() {}\n");
    project.write(
        "d/function.ts",
        "function typescript(value: number): number { return value; }\n",
    );
    project.write("c/function.php", "<?php\nfunction php(): void {}\n");
    project.write("b/function.rs", "fn rust() {}\n");
    project.write("a/function.py", "def python():\n    pass\n");

    let first_json = project.run(&["--format", "json", "."]);
    let second_json = project.run(&["--format", "json", "."]);
    let first_text = project.run(&["--format", "text", "."]);
    let second_text = project.run(&["--format", "text", "."]);

    assert_eq!(first_json.status.code(), Some(0));
    assert_eq!(second_json.status.code(), Some(0));
    assert_eq!(first_text.status.code(), Some(0));
    assert_eq!(second_text.status.code(), Some(0));
    assert_eq!(first_json.stdout, second_json.stdout);
    assert_eq!(first_text.stdout, second_text.stdout);

    let report: serde_json::Value =
        serde_json::from_slice(&first_json.stdout).expect("report should be valid JSON");
    let files = report["files"]
        .as_array()
        .expect("files should be an array");
    let paths_and_languages = files
        .iter()
        .map(|file| {
            (
                file["path"].as_str().expect("path should be text"),
                file["language"].as_str().expect("language should be text"),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        paths_and_languages,
        [
            ("a/function.py", "python"),
            ("b/function.rs", "rust"),
            ("c/function.php", "php"),
            ("d/function.ts", "typescript"),
            ("e/function.js", "javascript"),
        ]
    );
    assert_eq!(report["summary"]["functions"], 5);
    assert_eq!(report["summary"]["errors"], 0);
}

#[test]
fn repeated_language_filters_select_a_union_for_directory_scans() {
    let project = TestProject::new();
    project.write("src/a.js", "const javascript = true;\n");
    project.write("src/b.ts", "const typescript: boolean = true;\n");
    project.write("src/c.php", "<?php\n");
    project.write("src/d.rs", "fn rust() {}\n");
    project.write("src/e.py", "def python():\n    pass\n");

    let output = project.run(&[
        "--language",
        "php",
        "--language",
        "rust",
        "--language",
        "php",
        "--language",
        "python",
        "--format",
        "json",
        "src",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("report should be UTF-8");
    assert!(!stdout.contains("\"path\":\"src/a.js\""));
    assert!(!stdout.contains("\"path\":\"src/b.ts\""));
    assert!(stdout.contains("\"path\":\"src/c.php\""));
    assert!(stdout.contains("\"path\":\"src/d.rs\""));
    assert!(stdout.contains("\"language\":\"rust\""));
    assert!(stdout.contains("\"path\":\"src/e.py\""));
    assert!(stdout.contains("\"language\":\"python\""));
}

#[test]
fn an_explicit_file_excluded_by_filters_is_an_error() {
    let project = TestProject::new();
    project.write("source.ts", "const answer: number = 42;\n");

    let output = project.run(&["--language", "php", "source.ts"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: explicit file is excluded by language filters: source.ts\n"
    );
}

#[test]
fn usage_errors_escape_untrusted_path_control_characters() {
    let project = TestProject::new();
    let filename = "bad\n\r\t\u{1b}[31m\u{202e}.txt";
    project.write(filename, "not source\n");

    let output = project.run(&[filename]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: unsupported explicit file: bad\\n\\r\\t\\u{1b}[31m\\u{202e}.txt\n"
    );
}

#[test]
fn unknown_language_names_list_all_supported_filters() {
    let output = run(&["--language", "ruby", "."]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: --language requires javascript, typescript, php, rust, or python\n"
    );
}

#[test]
fn explicit_files_override_hidden_and_dependency_directory_skips() {
    let project = TestProject::new();
    project.write(".hidden/package.js", "const hidden = true;\n");
    project.write("node_modules/package.js", "const dependency = true;\n");
    project.write("vendor/package.php", "<?php\n");
    project.write("visible.js", "const visible = true;\n");

    let directory = project.run(&["--format", "json", "."]);
    assert_eq!(directory.status.code(), Some(0));
    let directory: serde_json::Value =
        serde_json::from_slice(&directory.stdout).expect("directory report should be valid JSON");
    assert_eq!(directory["summary"]["files"], 1);
    assert_eq!(directory["files"][0]["path"], "visible.js");

    let explicit = project.run(&[
        "--format",
        "json",
        "vendor/package.php",
        "node_modules/package.js",
        ".hidden/package.js",
    ]);
    assert_eq!(explicit.status.code(), Some(0));
    let explicit: serde_json::Value =
        serde_json::from_slice(&explicit.stdout).expect("explicit report should be valid JSON");
    let paths = explicit["files"]
        .as_array()
        .expect("files should be an array")
        .iter()
        .map(|file| file["path"].as_str().expect("path should be text"))
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            ".hidden/package.js",
            "node_modules/package.js",
            "vendor/package.php"
        ]
    );
}

#[test]
fn a_directory_with_no_selected_files_is_an_error() {
    let project = TestProject::new();
    project.write("src/source.ts", "const answer: number = 42;\n");

    let output = project.run(&["--language", "php", "src"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: no selected files found\n"
    );
}

#[test]
fn stdin_uses_the_selected_language_default_filename() {
    let project = TestProject::new();
    let output = project.run_with_stdin(
        &["--language", "typescript", "--format", "json", "-"],
        b"const answer: number = 42;\n",
    );

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("report should be UTF-8");
    assert!(stdout.contains("\"path\":\"stdin.ts\""));
    assert!(stdout.contains("\"language\":\"typescript\""));
}

#[test]
fn stdin_dispatches_rust_and_python_with_default_filenames() {
    let project = TestProject::new();

    let rust = project.run_with_stdin(
        &["--language", "rust", "--format", "json", "-"],
        b"fn rust_stdin() {}\n",
    );
    assert_eq!(rust.status.code(), Some(0));
    assert!(rust.stderr.is_empty());
    let rust: serde_json::Value =
        serde_json::from_slice(&rust.stdout).expect("Rust report should be JSON");
    assert_eq!(rust["files"][0]["path"], "stdin.rs");
    assert_eq!(rust["files"][0]["language"], "rust");
    assert_eq!(rust["files"][0]["functions"][0]["id"], "stdin.rs:1:1");

    let python = project.run_with_stdin(
        &["--language", "python", "--format", "json", "-"],
        b"def python_stdin():\n    pass\n",
    );
    assert_eq!(python.status.code(), Some(0));
    assert!(python.stderr.is_empty());
    let python: serde_json::Value =
        serde_json::from_slice(&python.stdout).expect("Python report should be JSON");
    assert_eq!(python["files"][0]["path"], "stdin.py");
    assert_eq!(python["files"][0]["language"], "python");
    assert_eq!(python["files"][0]["functions"][0]["id"], "stdin.py:1:1");
}

#[test]
fn rust_engine_analyzes_this_cli_source_deterministically() {
    let first = run(&[
        "--language",
        "rust",
        "--format",
        "json",
        "--max-complexity",
        "4294967295",
        "src",
    ]);
    let second = run(&[
        "--language",
        "rust",
        "--format",
        "json",
        "--max-complexity",
        "4294967295",
        "src",
    ]);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    assert!(first.stderr.is_empty());
    assert!(second.stderr.is_empty());
    assert_eq!(first.stdout, second.stdout);

    let report: serde_json::Value =
        serde_json::from_slice(&first.stdout).expect("self-analysis should return JSON");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["summary"]["errors"], 0);
    assert!(
        report["summary"]["functions"]
            .as_u64()
            .expect("function count should be numeric")
            > 0
    );

    let files = report["files"]
        .as_array()
        .expect("files should be an array");
    assert!(
        files
            .iter()
            .all(|file| file["language"].as_str() == Some("rust"))
    );
    assert!(files.iter().any(|file| file["path"] == "src/lib.rs"));
    assert!(files.iter().any(|file| file["path"] == "src/main.rs"));
}

#[test]
fn cli_source_cannot_regress_above_score_seven() {
    let output = run(&[
        "--language",
        "rust",
        "--format",
        "json",
        "--max-complexity",
        "7",
        "src",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("self-analysis should return JSON");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["summary"]["errors"], 0);
    assert_eq!(report["summary"]["violations"], 0);
    assert_eq!(report["summary"]["files"], 7);
    assert_eq!(report["summary"]["functions"], 447);
    assert!(
        report["summary"]["max_score"]
            .as_u64()
            .expect("maximum score should be numeric")
            <= 7
    );

    let analyzed_source = report["files"]
        .as_array()
        .expect("files should be an array")
        .iter()
        .map(|file| {
            (
                file["path"].as_str().expect("path should be text"),
                file["signals"]["function_count"]
                    .as_u64()
                    .expect("function count should be numeric"),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        analyzed_source,
        [
            ("src/javascript.rs", 136),
            ("src/lib.rs", 72),
            ("src/main.rs", 1),
            ("src/model.rs", 1),
            ("src/php.rs", 92),
            ("src/python.rs", 77),
            ("src/rust.rs", 68),
        ]
    );
}

#[test]
fn stdin_dispatches_javascript_typescript_tsx_and_php_modes() {
    let project = TestProject::new();

    let javascript = project.run_with_stdin(
        &["--language", "javascript", "--format", "json", "-"],
        b"function js_stdin() {}\n",
    );
    assert_eq!(javascript.status.code(), Some(0));
    let javascript: serde_json::Value =
        serde_json::from_slice(&javascript.stdout).expect("JavaScript report should be JSON");
    assert_eq!(javascript["files"][0]["path"], "stdin.js");
    assert_eq!(javascript["files"][0]["language"], "javascript");
    assert_eq!(javascript["files"][0]["functions"][0]["id"], "stdin.js:1:1");

    let typescript = project.run_with_stdin(
        &["--language", "typescript", "--format", "json", "-"],
        b"function ts_stdin(value: number): number { return value; }\n",
    );
    assert_eq!(typescript.status.code(), Some(0));
    let typescript: serde_json::Value =
        serde_json::from_slice(&typescript.stdout).expect("TypeScript report should be JSON");
    assert_eq!(typescript["files"][0]["path"], "stdin.ts");
    assert_eq!(typescript["files"][0]["language"], "typescript");
    assert_eq!(typescript["files"][0]["functions"][0]["id"], "stdin.ts:1:1");

    let tsx = project.run_with_stdin(
        &[
            "--language",
            "typescript",
            "--stdin-filename",
            "snippets/view.tsx",
            "--format",
            "json",
            "-",
        ],
        b"const View = () => <div />;\n",
    );
    assert_eq!(tsx.status.code(), Some(0));
    let tsx: serde_json::Value =
        serde_json::from_slice(&tsx.stdout).expect("TSX report should be JSON");
    assert_eq!(tsx["files"][0]["path"], "snippets/view.tsx");
    assert_eq!(tsx["files"][0]["language"], "tsx");
    assert_eq!(tsx["files"][0]["functions"][0]["name"], "View");

    let php = project.run_with_stdin(
        &["--language", "php", "--format", "json", "-"],
        b"<?php\nfunction php_stdin(): void {}\n",
    );
    assert_eq!(php.status.code(), Some(0));
    let php: serde_json::Value =
        serde_json::from_slice(&php.stdout).expect("PHP report should be JSON");
    assert_eq!(php["files"][0]["path"], "stdin.php");
    assert_eq!(php["files"][0]["language"], "php");
    assert_eq!(php["files"][0]["functions"][0]["id"], "stdin.php:2:1");
}

#[test]
fn stdin_accepts_one_repeated_language_value_once() {
    let project = TestProject::new();
    let output = project.run_with_stdin(
        &[
            "--language",
            "javascript",
            "--language",
            "javascript",
            "--format",
            "json",
            "-",
        ],
        b"function repeated_filter() {}\n",
    );

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert_eq!(report["files"][0]["path"], "stdin.js");
    assert_eq!(report["summary"]["functions"], 1);
}

#[test]
fn duplicate_scalar_options_are_usage_errors() {
    let project = TestProject::new();
    project.write("a.js", "const answer = 42;\n");

    let duplicate_format = project.run(&["--format", "text", "--format", "json", "a.js"]);
    assert_eq!(duplicate_format.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(duplicate_format.stderr).expect("error should be UTF-8"),
        "error: --format cannot be repeated\n"
    );

    let duplicate_limit =
        project.run(&["--max-complexity", "10", "--max-complexity", "20", "a.js"]);
    assert_eq!(duplicate_limit.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(duplicate_limit.stderr).expect("error should be UTF-8"),
        "error: --max-complexity cannot be repeated\n"
    );

    let duplicate_filename = project.run_with_stdin(
        &[
            "--language",
            "javascript",
            "--stdin-filename",
            "one.js",
            "--stdin-filename",
            "two.js",
            "-",
        ],
        b"",
    );
    assert_eq!(duplicate_filename.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(duplicate_filename.stderr).expect("error should be UTF-8"),
        "error: --stdin-filename cannot be repeated\n"
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_explicit_path_fails_cleanly() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let project = TestProject::new();
    let filename = OsString::from_vec(b"invalid-\xff.js".to_vec());

    let output = Command::new(env!("CARGO_BIN_EXE_complexity"))
        .current_dir(&project.root)
        .arg(filename)
        .output()
        .expect("complexity command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: selected path is not valid UTF-8\n"
    );
}

#[test]
fn incomplete_analysis_takes_exit_priority_over_a_violation() {
    let project = TestProject::new();
    project.write(
        "src/violation.js",
        "function violation(value) {\n  if (value) return true;\n}\n",
    );
    project.write_bytes("src/unreadable.php", &[0xff]);

    let output = project.run(&["--format", "json", "--max-complexity", "0", "src"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["summary"]["violations"], 1);
    assert_eq!(report["summary"]["errors"], 1);
    assert_eq!(report["files"][0]["path"], "src/unreadable.php");
    assert_eq!(report["files"][0]["status"], "io_error");
    assert!(report["files"][0]["signals"].is_null());
    assert_eq!(report["files"][0]["functions"], serde_json::json!([]));
}

#[cfg(unix)]
#[test]
fn read_error_takes_exit_priority_over_a_violation() {
    use std::os::unix::fs::PermissionsExt;

    let project = TestProject::new();
    project.write("a-unreadable.php", "<?php\n");
    project.write(
        "z-violation.js",
        "function violation(value) {\n  if (value) return true;\n}\n",
    );
    fs::set_permissions(
        project.root.join("a-unreadable.php"),
        fs::Permissions::from_mode(0o000),
    )
    .expect("read permissions should be removed");

    let output = project.run(&[
        "--format",
        "json",
        "--max-complexity",
        "0",
        "a-unreadable.php",
        "z-violation.js",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert_eq!(report["summary"]["violations"], 1);
    assert_eq!(report["summary"]["errors"], 1);
    assert_eq!(report["files"][0]["status"], "io_error");
    assert!(report["files"][0]["signals"].is_null());
}

#[test]
fn parse_error_takes_exit_priority_and_keeps_a_valid_sibling() {
    let project = TestProject::new();
    project.write("a-broken.php", "<?php\nfunction broken( {\n");
    project.write(
        "z-violation.js",
        "function violation(value) {\n  if (value) return true;\n}\n",
    );

    let output = project.run(&[
        "--format",
        "json",
        "--max-complexity",
        "0",
        "z-violation.js",
        "a-broken.php",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["summary"]["violations"], 1);
    assert_eq!(report["summary"]["errors"], 1);
    assert_eq!(report["files"][0]["path"], "a-broken.php");
    assert_eq!(report["files"][0]["status"], "parse_error");
    assert!(report["files"][0]["signals"].is_null());
    assert_eq!(report["files"][0]["functions"], serde_json::json!([]));
    assert_eq!(report["files"][1]["path"], "z-violation.js");
    assert_eq!(report["files"][1]["status"], "ok");
    assert_eq!(report["files"][1]["functions"][0]["over_limit"], true);
}

#[test]
fn stdin_validation_rejects_ambiguous_or_unsafe_inputs() {
    let project = TestProject::new();

    let no_language = project.run_with_stdin(&["-"], b"");
    assert_eq!(no_language.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(no_language.stderr).expect("error should be UTF-8"),
        "error: stdin requires exactly one language\n"
    );

    let mixed_inputs = project.run_with_stdin(&["--language", "javascript", "-", "other.js"], b"");
    assert_eq!(mixed_inputs.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(mixed_inputs.stderr).expect("error should be UTF-8"),
        "error: - must be the sole input\n"
    );

    let unsafe_filename = project.run_with_stdin(
        &[
            "--language",
            "typescript",
            "--stdin-filename",
            "../snippet.tsx",
            "-",
        ],
        b"",
    );
    assert_eq!(unsafe_filename.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(unsafe_filename.stderr).expect("error should be UTF-8"),
        "error: --stdin-filename must be a safe relative path\n"
    );

    let wrong_extension = project.run_with_stdin(
        &["--language", "php", "--stdin-filename", "snippet.js", "-"],
        b"",
    );
    assert_eq!(wrong_extension.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(wrong_extension.stderr).expect("error should be UTF-8"),
        "error: --stdin-filename extension must match the selected language\n"
    );
}

#[test]
fn stdin_rejects_multiple_selected_languages() {
    let project = TestProject::new();
    let output =
        project.run_with_stdin(&["--language", "javascript", "--language", "php", "-"], b"");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: stdin requires exactly one language\n"
    );
}

#[test]
fn stdin_rejects_absolute_and_current_directory_virtual_filenames() {
    let project = TestProject::new();

    for filename in ["/snippet.tsx", "snippets/./view.tsx"] {
        let output = project.run_with_stdin(
            &[
                "--language",
                "typescript",
                "--stdin-filename",
                filename,
                "-",
            ],
            b"",
        );

        assert_eq!(output.status.code(), Some(2), "filename: {filename}");
        assert!(output.stdout.is_empty(), "filename: {filename}");
        assert_eq!(
            String::from_utf8(output.stderr).expect("error should be UTF-8"),
            "error: --stdin-filename must be a safe relative path\n",
            "filename: {filename}"
        );
    }
}

#[test]
fn stdin_filename_is_invalid_with_a_filesystem_input() {
    let project = TestProject::new();
    project.write("source.ts", "const answer: number = 42;\n");

    let output = project.run(&["--stdin-filename", "snippet.ts", "source.ts"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: --stdin-filename is valid only with -\n"
    );
}

#[test]
fn stdin_rejects_a_backslash_rooted_virtual_filename() {
    let project = TestProject::new();
    let output = project.run_with_stdin(
        &[
            "--language",
            "typescript",
            "--stdin-filename",
            r"\snippet.tsx",
            "-",
        ],
        b"",
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: --stdin-filename must be a safe relative path\n"
    );
}

#[test]
fn stdin_rejects_a_drive_rooted_virtual_filename() {
    let project = TestProject::new();
    let output = project.run_with_stdin(
        &[
            "--language",
            "typescript",
            "--stdin-filename",
            r"C:\snippet.tsx",
            "-",
        ],
        b"",
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("error should be UTF-8"),
        "error: --stdin-filename must be a safe relative path\n"
    );
}

#[test]
fn invalid_utf8_stdin_emits_an_incomplete_file_report() {
    let project = TestProject::new();
    let output = project.run_with_stdin(&["--language", "php", "--format", "json", "-"], &[0xff]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["files"][0]["path"], "stdin.php");
    assert_eq!(report["files"][0]["status"], "io_error");
    assert!(report["files"][0]["signals"].is_null());
    assert_eq!(report["summary"]["errors"], 1);
}

#[test]
fn stdin_parse_error_emits_an_incomplete_virtual_file_report() {
    let project = TestProject::new();
    let output = project.run_with_stdin(
        &["--language", "javascript", "--format", "json", "-"],
        b"function broken( {\n",
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["files"][0]["path"], "stdin.js");
    assert_eq!(report["files"][0]["status"], "parse_error");
    assert!(report["files"][0]["signals"].is_null());
    assert_eq!(report["files"][0]["functions"], serde_json::json!([]));
    assert_eq!(report["summary"]["errors"], 1);
}

#[test]
fn discovery_honors_ignores_and_deduplicates_explicit_overrides() {
    let project = TestProject::new();
    project.write(".gitignore", "ignored.js\n");
    project.write(".ignore", "ignored.php\n");
    project.write("ignored.js", "const ignored = true;\n");
    project.write("ignored.php", "<?php\n");
    project.write(".hidden/hidden.js", "const hidden = true;\n");
    project.write("node_modules/package.js", "const dependency = true;\n");
    project.write("vendor/package.php", "<?php\n");
    project.write("visible.js", "const visible = true;\n");
    project.write("visible.php", "<?php\n");

    let output = project.run(&[
        "--format",
        "json",
        ".",
        ".hidden/hidden.js",
        "ignored.js",
        "visible.js",
    ]);

    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    let paths = report["files"]
        .as_array()
        .expect("files should be an array")
        .iter()
        .map(|file| file["path"].as_str().expect("path should be text"))
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            ".hidden/hidden.js",
            "ignored.js",
            "visible.js",
            "visible.php"
        ]
    );
}

#[test]
fn text_summary_names_every_schema_summary_field() {
    let project = TestProject::new();
    project.write("empty.js", "const answer = 42;\n");

    let output = project.run(&["empty.js"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).expect("report should be UTF-8"),
        concat!(
            "Summary: files=1 functions=0 violations=0 errors=0 max_score=0 ",
            "max_control_depth=0 max_function_line_span=0 max_functions_per_file=0 ",
            "conditions=0 max_condition_operators=0 max_condition_predicates=0 ",
            "max_boolean_depth=0\n"
        )
    );
}

#[test]
fn exact_mixed_text_has_one_stable_function_line_and_full_summary() {
    let project = TestProject::new();
    project.write("a.php", "<?php\n");
    project.write(
        "z.js",
        "function js(a, b, c) {\n  if (a && (b || !c)) {}\n}\n",
    );

    let output = project.run(&["z.js", "a.php"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).expect("report should be UTF-8"),
        concat!(
            "PASS z.js:1:1 js score=2 lines=3 control-depth=1 conditions=1 ",
            "condition-operators=3 condition-predicates=3 boolean-depth=3\n",
            "Summary: files=2 functions=1 violations=0 errors=0 max_score=2 ",
            "max_control_depth=1 max_function_line_span=3 max_functions_per_file=1 ",
            "conditions=1 max_condition_operators=3 max_condition_predicates=3 ",
            "max_boolean_depth=3\n"
        )
    );
}

#[test]
fn exact_fail_text_exits_one_without_changing_signal_fields() {
    let project = TestProject::new();
    project.write("a.php", "<?php\n");
    project.write(
        "z.js",
        "function js(a, b, c) {\n  if (a && (b || !c)) {}\n}\n",
    );

    let output = project.run(&["--max-complexity", "1", "z.js", "a.php"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).expect("report should be UTF-8"),
        concat!(
            "FAIL z.js:1:1 js score=2 lines=3 control-depth=1 conditions=1 ",
            "condition-operators=3 condition-predicates=3 boolean-depth=3\n",
            "Summary: files=2 functions=1 violations=1 errors=0 max_score=2 ",
            "max_control_depth=1 max_function_line_span=3 max_functions_per_file=1 ",
            "conditions=1 max_condition_operators=3 max_condition_predicates=3 ",
            "max_boolean_depth=3\n"
        )
    );
}

#[test]
fn text_diagnostic_precedes_the_incomplete_summary() {
    let project = TestProject::new();
    project.write("broken.js", "function broken( {\n");

    let output = project.run(&["broken.js"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("report should be UTF-8");
    let diagnostic = stdout
        .find("ERROR broken.js:")
        .expect("diagnostic should be reported");
    let summary = stdout
        .find("Summary: files=1 functions=0 violations=0 errors=1")
        .expect("incomplete summary should be reported");
    assert!(diagnostic < summary);
    assert!(stdout.ends_with('\n'));
}

#[test]
fn text_diagnostics_escape_filename_control_characters_without_changing_json_paths() {
    let project = TestProject::new();
    let filename = "bad\n\r\t\u{1b}[31m\u{202e}.js";
    project.write(filename, "function broken( {\n");

    let text = project.run(&[filename]);

    assert_eq!(text.status.code(), Some(2));
    let text = String::from_utf8(text.stdout).expect("text report should be UTF-8");
    assert!(
        text.contains("ERROR bad\\n\\r\\t\\u{1b}[31m\\u{202e}.js:"),
        "{text}"
    );
    assert!(!text.contains("bad\n"));
    assert!(!text.contains('\r'));
    assert!(!text.contains('\t'));
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains('\u{202e}'));

    let json = project.run(&["--format", "json", filename]);

    assert_eq!(json.status.code(), Some(2));
    let report: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("JSON report should be valid JSON");
    assert_eq!(report["files"][0]["path"], filename);
}

#[test]
fn text_function_ids_escape_filename_control_characters() {
    let project = TestProject::new();
    let filename = "safe\n\u{1b}[31m\u{202e}.js";
    project.write(filename, "function safe() {}\n");

    let output = project.run(&[filename]);

    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).expect("text report should be UTF-8");
    assert!(
        text.contains("PASS safe\\n\\u{1b}[31m\\u{202e}.js:1:1 safe"),
        "{text}"
    );
    assert!(!text.contains("safe\n"));
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains('\u{202e}'));
}

#[cfg(unix)]
#[test]
fn atomic_symlink_swap_never_reads_outside_the_working_directory() {
    use std::os::unix::fs::symlink;
    use std::time::Duration;

    let project = TestProject::new();
    let outside = project.root.with_file_name(format!(
        "complexity-cli-outside-{}",
        NEXT_PROJECT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&outside).expect("outside directory should be created");
    let outside_source = outside.join("outside.py");
    fs::write(&outside_source, "def outside_secret():\n    return 1\n")
        .expect("outside source should be written");

    for index in 0..8_000 {
        project.write(&format!("filler/entry-{index:04}.txt"), "walk delay\n");
    }

    let victim = project.root.join("sources/z_victim.py");
    let swap_link = project.root.join("sources/swap-link");
    fs::create_dir_all(victim.parent().expect("victim should have a parent"))
        .expect("source directory should be created");
    for delay in [500, 1_000, 2_000, 4_000, 8_000, 16_000] {
        if victim.exists() || victim.is_symlink() {
            fs::remove_file(&victim).expect("previous victim should be removed");
        }
        if swap_link.exists() || swap_link.is_symlink() {
            fs::remove_file(&swap_link).expect("previous swap link should be removed");
        }
        fs::write(&victim, "def inside_original():\n    return 0\n")
            .expect("inside source should be written");

        let child = Command::new(env!("CARGO_BIN_EXE_complexity"))
            .current_dir(&project.root)
            .args(["--format", "json", "sources/z_victim.py", "filler"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("complexity command should start");
        std::thread::sleep(Duration::from_micros(delay));
        symlink(&outside_source, &swap_link).expect("outside link should be created");
        fs::rename(&swap_link, &victim).expect("outside link should replace the victim");

        let output = child
            .wait_with_output()
            .expect("complexity command should finish");
        if output.stdout.is_empty() {
            assert_eq!(output.status.code(), Some(2));
            continue;
        }
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
        let files = report["files"]
            .as_array()
            .expect("files should be an array");
        let function_names = files
            .iter()
            .flat_map(|file| {
                file["functions"]
                    .as_array()
                    .expect("functions should be an array")
            })
            .map(|function| function["name"].as_str().expect("name should be text"));
        let function_names = function_names.collect::<Vec<_>>();
        assert!(
            !function_names.contains(&"outside_secret"),
            "outside source was analyzed after a {delay} microsecond swap"
        );
    }

    fs::remove_dir_all(outside).expect("outside directory should be removed");
}

#[cfg(unix)]
#[test]
fn directory_discovery_keeps_file_symlinks_without_following_directory_symlinks() {
    use std::os::unix::fs::symlink;

    let project = TestProject::new();
    project.write("target.js", "const answer = 42;\n");
    fs::create_dir_all(project.root.join("src")).expect("source directory should be created");
    symlink("../target.js", project.root.join("src/link.js"))
        .expect("file symlink should be created");
    symlink("..", project.root.join("src/parent")).expect("directory symlink should be created");

    let output = project.run(&["--format", "json", "src"]);

    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert_eq!(report["summary"]["files"], 1);
    assert_eq!(report["files"][0]["path"], "target.js");
}

#[cfg(unix)]
#[test]
fn discovered_file_symlinks_cannot_bypass_skipped_or_ignored_targets() {
    use std::os::unix::fs::symlink;

    let project = TestProject::new();
    project.write(".gitignore", "*.js\n!target/keep/keep.js\n");
    project.write(".ignore", "ignored.php\n");
    project.write("gitignored.js", "const ignored = true;\n");
    project.write("ignored.php", "<?php\n");
    project.write("target/.gitignore", "target/*.js\n!keep/keep.js\n");
    project.write("target/keep/keep.js", "const kept = true;\n");
    project.write(".hidden/target.js", "const hidden = true;\n");
    project.write("node_modules/target.js", "const node = true;\n");
    project.write("vendor/target.js", "const vendor = true;\n");
    project.write("src/visible.php", "<?php\n");
    symlink(
        "../.hidden/target.js",
        project.root.join("src/hidden-link.source"),
    )
    .expect("hidden file symlink should be created");
    symlink(
        "../node_modules/target.js",
        project.root.join("src/node-link.source"),
    )
    .expect("node_modules file symlink should be created");
    symlink(
        "../vendor/target.js",
        project.root.join("src/vendor-link.source"),
    )
    .expect("vendor file symlink should be created");
    symlink(
        "../gitignored.js",
        project.root.join("src/gitignored-link.source"),
    )
    .expect("gitignored file symlink should be created");
    symlink(
        "../ignored.php",
        project.root.join("src/ignored-link.source"),
    )
    .expect("ignored file symlink should be created");
    symlink(
        "../target/keep/keep.js",
        project.root.join("src/kept-link.source"),
    )
    .expect("whitelisted file symlink should be created");

    let output = project.run(&["--format", "json", "src"]);

    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert_eq!(report["summary"]["files"], 2);
    assert_eq!(report["files"][0]["path"], "src/visible.php");
    assert_eq!(report["files"][1]["path"], "target/keep/keep.js");
}

#[cfg(unix)]
#[test]
fn canonical_symlink_targets_keep_walk_builder_ignore_precedence() {
    use std::os::unix::fs::symlink;

    let project = TestProject::new();
    project.write(".ignore", "!target/keep.js\n");
    project.write("target/.gitignore", "keep.js\n");
    project.write("target/keep.js", "const kept = true;\n");
    fs::create_dir_all(project.root.join("src")).expect("source directory should be created");
    symlink(
        "../target/keep.js",
        project.root.join("src/kept-link.source"),
    )
    .expect("file symlink should be created");

    let direct = project.run(&["--format", "json", "target"]);
    let symlinked = project.run(&["--format", "json", "src"]);

    assert_eq!(direct.status.code(), Some(0));
    assert_eq!(symlinked.status.code(), Some(0));
    let direct_report: serde_json::Value =
        serde_json::from_slice(&direct.stdout).expect("direct report should be valid JSON");
    let symlinked_report: serde_json::Value =
        serde_json::from_slice(&symlinked.stdout).expect("symlink report should be valid JSON");
    assert_eq!(direct_report["summary"]["files"], 1);
    assert_eq!(direct_report["files"][0]["path"], "target/keep.js");
    assert_eq!(symlinked_report["summary"]["files"], 1);
    assert_eq!(symlinked_report["files"][0]["path"], "target/keep.js");
}

#[test]
fn directory_discovery_does_not_use_git_local_excludes() {
    let project = TestProject::new();
    project.write(".git/info/exclude", "kept.js\n");
    project.write("kept.js", "const kept = true;\n");

    let output = project.run(&["--format", "json", "."]);

    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert_eq!(report["summary"]["files"], 1);
    assert_eq!(report["files"][0]["path"], "kept.js");
}
