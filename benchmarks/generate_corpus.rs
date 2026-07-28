use std::{fmt::Write as _, path::Path};

const FUNCTION_COUNT: usize = 2_000;
const LARGE_FILE_COUNT: usize = 8;
const MIXED_FUNCTION_COUNT: usize = 10;

#[cfg(not(test))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .ok_or("usage: generate_corpus <output-directory>")?;
    generate_corpus(Path::new(&output))
}

fn generate_corpus(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    generate_javascript_corpus(&output.join("javascript"))?;
    generate_php_corpus(&output.join("php"))?;
    generate_mixed_corpus(&output.join("mixed"))?;
    Ok(())
}

fn generate_javascript_corpus(output: &Path) -> std::io::Result<()> {
    let many_small = output.join("many-small");
    let few_large = output.join("few-large");
    std::fs::create_dir_all(&many_small)?;
    std::fs::create_dir_all(&few_large)?;

    for index in 0..FUNCTION_COUNT {
        let extension = javascript_extension(index);
        std::fs::write(
            many_small.join(format!("case-{index:04}.{extension}")),
            javascript_file(index, 1),
        )?;
    }

    let functions_per_file = FUNCTION_COUNT / LARGE_FILE_COUNT;
    for file_index in 0..LARGE_FILE_COUNT {
        let first_index = file_index * functions_per_file;
        let extension = javascript_extension(file_index);
        std::fs::write(
            few_large.join(format!("batch-{file_index:02}.{extension}")),
            javascript_file(first_index, functions_per_file),
        )?;
    }
    Ok(())
}

fn generate_php_corpus(output: &Path) -> std::io::Result<()> {
    let many_small = output.join("many-small");
    let few_large = output.join("few-large");
    std::fs::create_dir_all(&many_small)?;
    std::fs::create_dir_all(&few_large)?;

    for index in 0..FUNCTION_COUNT {
        std::fs::write(
            many_small.join(format!("case-{index:04}.php")),
            php_file(index, 1),
        )?;
    }

    let functions_per_file = FUNCTION_COUNT / LARGE_FILE_COUNT;
    for file_index in 0..LARGE_FILE_COUNT {
        let first_index = file_index * functions_per_file;
        std::fs::write(
            few_large.join(format!("batch-{file_index:02}.php")),
            php_file(first_index, functions_per_file),
        )?;
    }
    Ok(())
}

/// Writes one file and ten functions for each supported language.
///
/// The mixed corpus is intentionally small because it measures one combined
/// discovery and analysis run, not language throughput.
fn generate_mixed_corpus(output: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(output)?;
    std::fs::write(
        output.join("javascript.js"),
        javascript_file(0, MIXED_FUNCTION_COUNT),
    )?;
    std::fs::write(
        output.join("typescript.ts"),
        javascript_file(MIXED_FUNCTION_COUNT, MIXED_FUNCTION_COUNT),
    )?;
    std::fs::write(output.join("php.php"), php_file(0, MIXED_FUNCTION_COUNT))?;
    std::fs::write(output.join("rust.rs"), rust_file(0, MIXED_FUNCTION_COUNT))?;
    std::fs::write(
        output.join("python.py"),
        python_file(0, MIXED_FUNCTION_COUNT),
    )?;
    Ok(())
}

fn javascript_extension(index: usize) -> &'static str {
    if index.is_multiple_of(2) { "js" } else { "ts" }
}

fn javascript_file(first_index: usize, function_count: usize) -> String {
    let mut source = String::new();
    for index in first_index..first_index + function_count {
        writeln!(
            source,
            "function benchmark_{index}(left, right) {{\n    if (left) {{\n        if (right) {{\n            return {index};\n        }}\n    }} else {{\n        return 0;\n    }}\n    return 1;\n}}"
        )
        .expect("write to string");
    }
    source
}

fn php_file(first_index: usize, function_count: usize) -> String {
    let mut source = String::from("<?php\n");
    for index in first_index..first_index + function_count {
        writeln!(
            source,
            "function case_{index}(bool $a, bool $b): int {{\n    if ($a) {{\n        if ($b) {{ return 1; }}\n    }} else {{\n        return 0;\n    }}\n    return 2;\n}}"
        )
        .expect("write to string");
    }
    source
}

fn rust_file(first_index: usize, function_count: usize) -> String {
    let mut source = String::new();
    for index in first_index..first_index + function_count {
        writeln!(
            source,
            "fn benchmark_{index}(left: bool, right: bool) -> i32 {{\n    if left {{\n        if right {{\n            return {index};\n        }}\n    }} else {{\n        return 0;\n    }}\n    1\n}}"
        )
        .expect("write to string");
    }
    source
}

fn python_file(first_index: usize, function_count: usize) -> String {
    let mut source = String::new();
    for index in first_index..first_index + function_count {
        writeln!(
            source,
            "def benchmark_{index}(left: bool, right: bool) -> int:\n    if left:\n        if right:\n            return {index}\n    else:\n        return 0\n    return 1"
        )
        .expect("write to string");
    }
    source
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{FUNCTION_COUNT, LARGE_FILE_COUNT, MIXED_FUNCTION_COUNT, generate_corpus};

    #[test]
    fn reproduces_the_legacy_corpus_sizes() {
        let output = test_output("legacy");
        reset(&output);

        generate_corpus(&output).expect("corpus should generate");

        assert_layout(
            &output.join("javascript/many-small"),
            FUNCTION_COUNT,
            FUNCTION_COUNT,
            325_780,
        );
        assert_layout(
            &output.join("javascript/few-large"),
            LARGE_FILE_COUNT,
            FUNCTION_COUNT,
            325_780,
        );
        assert_layout(
            &output.join("php/many-small"),
            FUNCTION_COUNT,
            FUNCTION_COUNT,
            292_890,
        );
        assert_layout(
            &output.join("php/few-large"),
            LARGE_FILE_COUNT,
            FUNCTION_COUNT,
            280_938,
        );

        fs::remove_dir_all(output).expect("test corpus should be removable");
    }

    #[test]
    fn writes_a_small_repeatable_mixed_corpus() {
        let output = test_output("mixed");
        reset(&output);
        let first = output.join("first");
        let second = output.join("second");

        generate_corpus(&first).expect("first corpus should generate");
        generate_corpus(&second).expect("second corpus should generate");

        let files = corpus_files(&first.join("mixed"));
        assert_eq!(
            files
                .iter()
                .map(|(name, source)| (name.as_str(), mixed_function_count(name, source)))
                .collect::<Vec<_>>(),
            [
                ("javascript.js", MIXED_FUNCTION_COUNT),
                ("php.php", MIXED_FUNCTION_COUNT),
                ("python.py", MIXED_FUNCTION_COUNT),
                ("rust.rs", MIXED_FUNCTION_COUNT),
                ("typescript.ts", MIXED_FUNCTION_COUNT),
            ]
        );
        assert_eq!(files, corpus_files(&second.join("mixed")));

        fs::remove_dir_all(output).expect("test corpus should be removable");
    }

    fn mixed_function_count(name: &str, source: &str) -> usize {
        if name.ends_with(".rs") {
            return source
                .lines()
                .filter(|line| line.starts_with("fn "))
                .count();
        }
        if name.ends_with(".py") {
            return source
                .lines()
                .filter(|line| line.starts_with("def "))
                .count();
        }
        source.matches("function ").count()
    }

    fn assert_layout(
        directory: &Path,
        expected_files: usize,
        expected_functions: usize,
        expected_bytes: usize,
    ) {
        let files = corpus_files(directory);
        assert_eq!(files.len(), expected_files);
        assert_eq!(
            files
                .iter()
                .map(|(_, source)| source.matches("function ").count())
                .sum::<usize>(),
            expected_functions
        );
        assert_eq!(
            files.iter().map(|(_, source)| source.len()).sum::<usize>(),
            expected_bytes
        );
    }

    fn corpus_files(directory: &Path) -> Vec<(String, String)> {
        let mut files = fs::read_dir(directory)
            .expect("corpus directory should exist")
            .map(|entry| entry.expect("directory entry should be readable").path())
            .map(|path| {
                let name = path
                    .file_name()
                    .expect("corpus file should have a name")
                    .to_string_lossy()
                    .into_owned();
                let source = fs::read_to_string(path).expect("corpus file should be readable");
                (name, source)
            })
            .collect::<Vec<_>>();
        files.sort();
        files
    }

    fn test_output(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "complexity-benchmark-{name}-{}",
            std::process::id()
        ))
    }

    fn reset(path: &Path) {
        if path.exists() {
            fs::remove_dir_all(path).expect("stale test corpus should be removable");
        }
    }
}
