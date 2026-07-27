mod javascript;
pub(crate) mod model;
mod php;

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};

use ignore::{WalkBuilder, gitignore::GitignoreBuilder};
use model::{FileResult, FileStatus, Report, RunStatus, Summary, Tool};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const USAGE: &str = "Usage: complexity [--language javascript|typescript|php]... \
[--format text|json] [--max-complexity N] [--stdin-filename PATH] <path...|->";

const DEFAULT_MAX_COMPLEXITY: u32 = 15;
const PROFILE: &str = "core-v1";

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u8,
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug)]
struct Options {
    format: OutputFormat,
    max_complexity: u32,
    languages: BTreeSet<LanguageFilter>,
    stdin_filename: Option<String>,
    paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LanguageFilter {
    JavaScript,
    TypeScript,
    Php,
}

enum Command {
    Help,
    Version,
    Run(Options),
}

pub fn run<I>(arguments: I, cwd: &Path) -> CommandOutput
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    let command = match parse_options(arguments.into_iter().map(Into::into).collect()) {
        Ok(command) => command,
        Err(message) => return usage_error(message),
    };

    match command {
        Command::Help => success(format!("{USAGE}\n")),
        Command::Version => success(format!("complexity {VERSION}\n")),
        Command::Run(options) => run_analysis(options, cwd),
    }
}

fn parse_options(arguments: Vec<OsString>) -> Result<Command, String> {
    if arguments.len() == 1 && arguments[0] == "--help" {
        return Ok(Command::Help);
    }
    if arguments.len() == 1 && arguments[0] == "--version" {
        return Ok(Command::Version);
    }

    let mut format = OutputFormat::Text;
    let mut format_seen = false;
    let mut max_complexity = DEFAULT_MAX_COMPLEXITY;
    let mut max_complexity_seen = false;
    let mut languages = BTreeSet::new();
    let mut stdin_filename = None;
    let mut paths = Vec::new();
    let mut index = 0;

    while index < arguments.len() {
        let argument = arguments[index]
            .to_str()
            .ok_or_else(|| "selected path is not valid UTF-8".to_string())?;
        match argument {
            "--language" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    "--language requires javascript, typescript, or php".to_string()
                })?;
                let value = value
                    .to_str()
                    .ok_or_else(|| "--language value is not valid UTF-8".to_string())?;
                let language = match value {
                    "javascript" => LanguageFilter::JavaScript,
                    "typescript" => LanguageFilter::TypeScript,
                    "php" => LanguageFilter::Php,
                    _ => {
                        return Err(
                            "--language requires javascript, typescript, or php".to_string()
                        );
                    }
                };
                languages.insert(language);
            }
            "--format" => {
                if format_seen {
                    return Err("--format cannot be repeated".to_string());
                }
                format_seen = true;
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| "--format requires text or json".to_string())?;
                let value = value
                    .to_str()
                    .ok_or_else(|| "--format value is not valid UTF-8".to_string())?;
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err("--format requires text or json".to_string()),
                };
            }
            "--max-complexity" => {
                if max_complexity_seen {
                    return Err("--max-complexity cannot be repeated".to_string());
                }
                max_complexity_seen = true;
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    "--max-complexity requires a non-negative integer".to_string()
                })?;
                let value = value
                    .to_str()
                    .ok_or_else(|| "--max-complexity value is not valid UTF-8".to_string())?;
                max_complexity = value
                    .parse()
                    .map_err(|_| "--max-complexity requires a non-negative integer".to_string())?;
            }
            "--stdin-filename" => {
                if stdin_filename.is_some() {
                    return Err("--stdin-filename cannot be repeated".to_string());
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| "--stdin-filename requires a path".to_string())?;
                let value = value
                    .to_str()
                    .ok_or_else(|| "--stdin-filename value is not valid UTF-8".to_string())?;
                stdin_filename = Some(value.to_string());
            }
            "-" => paths.push(PathBuf::from("-")),
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}"));
            }
            value => paths.push(PathBuf::from(value)),
        }
        index += 1;
    }

    if paths.is_empty() {
        return Err("at least one path is required".to_string());
    }
    let reads_stdin = paths.iter().any(|path| path == Path::new("-"));
    if reads_stdin && paths.len() != 1 {
        return Err("- must be the sole input".to_string());
    }
    if reads_stdin && languages.len() != 1 {
        return Err("stdin requires exactly one language".to_string());
    }
    if !reads_stdin && stdin_filename.is_some() {
        return Err("--stdin-filename is valid only with -".to_string());
    }

    Ok(Command::Run(Options {
        format,
        max_complexity,
        languages,
        stdin_filename,
        paths,
    }))
}

fn run_analysis(options: Options, cwd: &Path) -> CommandOutput {
    let reads_stdin = options.paths.len() == 1 && options.paths[0] == Path::new("-");
    if reads_stdin {
        return run_stdin(options);
    }

    let cwd = match cwd.canonicalize() {
        Ok(cwd) => cwd,
        Err(error) => return usage_error(format!("cannot resolve working directory: {error}")),
    };
    let selected_files = match select_files(&options.paths, &options.languages, &cwd) {
        Ok(files) => files,
        Err(message) => return usage_error(message),
    };
    let files = selected_files
        .into_iter()
        .map(|(path, relative, language)| {
            analyze_selected_file(&path, relative, language, options.max_complexity)
        })
        .collect();
    finish_report(options.format, options.max_complexity, files)
}

fn run_stdin(options: Options) -> CommandOutput {
    let family = *options
        .languages
        .iter()
        .next()
        .expect("stdin requires one language");
    let filename = match options.stdin_filename {
        Some(filename) => filename,
        None => default_stdin_filename(family).to_string(),
    };
    let (filename, language) = match validate_stdin_filename(&filename, family) {
        Ok(result) => result,
        Err(message) => return usage_error(message),
    };
    let mut bytes = Vec::new();
    let read_result = std::io::stdin().read_to_end(&mut bytes);
    let file = match read_result {
        Ok(_) => match String::from_utf8(bytes) {
            Ok(source) => analyze_source(&filename, &source, &language, options.max_complexity),
            Err(error) => failed_file(
                filename,
                language,
                format!("standard input is not valid UTF-8: {error}"),
            ),
        },
        Err(error) => failed_file(filename, language, error.to_string()),
    };

    finish_report(options.format, options.max_complexity, vec![file])
}

fn finish_report(
    format: OutputFormat,
    max_complexity: u32,
    files: Vec<FileResult>,
) -> CommandOutput {
    let report = build_report(max_complexity, files);
    let exit_code = report_exit_code(&report);
    let stdout = match format {
        OutputFormat::Text => format_text(&report),
        OutputFormat::Json => format_json(&report),
    };

    CommandOutput {
        stdout,
        stderr: String::new(),
        exit_code,
    }
}

fn default_stdin_filename(language: LanguageFilter) -> &'static str {
    match language {
        LanguageFilter::JavaScript => "stdin.js",
        LanguageFilter::TypeScript => "stdin.ts",
        LanguageFilter::Php => "stdin.php",
    }
}

fn validate_stdin_filename(
    filename: &str,
    selected: LanguageFilter,
) -> Result<(String, String), String> {
    let normalized = filename.replace('\\', "/");
    let path = Path::new(&normalized);
    let has_unsafe_component = normalized
        .split('/')
        .any(|component| component == "." || component == "..");
    let bytes = normalized.as_bytes();
    let has_drive_prefix = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if path.is_absolute() || has_drive_prefix || has_unsafe_component {
        return Err("--stdin-filename must be a safe relative path".to_string());
    }
    let (family, language) = language_for_path(path)
        .ok_or_else(|| "--stdin-filename extension must match the selected language".to_string())?;
    if family != selected {
        return Err("--stdin-filename extension must match the selected language".to_string());
    }
    Ok((normalized, language.to_string()))
}

fn select_files(
    paths: &[PathBuf],
    languages: &BTreeSet<LanguageFilter>,
    cwd: &Path,
) -> Result<Vec<(PathBuf, String, String)>, String> {
    let mut files = BTreeMap::new();
    for supplied in paths {
        let candidate = if supplied.is_absolute() {
            supplied.clone()
        } else {
            cwd.join(supplied)
        };
        let metadata = std::fs::symlink_metadata(&candidate)
            .map_err(|error| format!("cannot resolve {}: {error}", supplied.display()))?;
        let canonical = candidate
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", supplied.display()))?;
        if !canonical.starts_with(cwd) {
            return Err(format!(
                "path is outside the working directory: {}",
                supplied.display()
            ));
        }
        if metadata.file_type().is_symlink() && canonical.is_dir() {
            continue;
        }
        if canonical.is_file() {
            add_explicit_file(&canonical, supplied, languages, cwd, &mut files)?;
            continue;
        }
        if canonical.is_dir() {
            if !is_skipped_path(&canonical, cwd)? {
                discover_directory(&canonical, languages, cwd, &mut files)?;
            }
            continue;
        }
        return Err(format!("unsupported input path: {}", supplied.display()));
    }
    if files.is_empty() {
        return Err("no selected files found".to_string());
    }
    Ok(files
        .into_iter()
        .map(|(relative, (absolute, language))| (absolute, relative, language))
        .collect())
}

fn add_explicit_file(
    canonical: &Path,
    supplied: &Path,
    languages: &BTreeSet<LanguageFilter>,
    cwd: &Path,
    files: &mut BTreeMap<String, (PathBuf, String)>,
) -> Result<(), String> {
    let (family, language) = language_for_path(canonical)
        .ok_or_else(|| format!("unsupported explicit file: {}", supplied.display()))?;
    if !language_is_selected(family, languages) {
        return Err(format!(
            "explicit file is excluded by language filters: {}",
            supplied.display()
        ));
    }
    insert_file(canonical, cwd, language, files)
}

fn discover_directory(
    directory: &Path,
    languages: &BTreeSet<LanguageFilter>,
    cwd: &Path,
    files: &mut BTreeMap<String, (PathBuf, String)>,
) -> Result<(), String> {
    let walker = WalkBuilder::new(directory)
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .ignore(true)
        .follow_links(false)
        .filter_entry(|entry| entry.file_name() != "node_modules" && entry.file_name() != "vendor")
        .build();

    for entry in walker {
        let entry = entry.map_err(|error| format!("directory discovery failed: {error}"))?;
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        let is_file_symlink = file_type.is_symlink();
        if !file_type.is_file() && !is_file_symlink {
            continue;
        }
        let canonical = entry
            .path()
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", entry.path().display()))?;
        if !canonical.starts_with(cwd) {
            return Err(format!(
                "discovered path is outside the working directory: {}",
                entry.path().display()
            ));
        }
        if !canonical.is_file() {
            continue;
        }
        if is_skipped_path(&canonical, cwd)? {
            continue;
        }
        if is_file_symlink && is_ignored_path(&canonical, cwd)? {
            continue;
        }
        let Some((family, language)) = language_for_path(&canonical) else {
            continue;
        };
        if !language_is_selected(family, languages) {
            continue;
        }
        insert_file(&canonical, cwd, language, files)?;
    }
    Ok(())
}

fn insert_file(
    canonical: &Path,
    cwd: &Path,
    language: &str,
    files: &mut BTreeMap<String, (PathBuf, String)>,
) -> Result<(), String> {
    let relative = portable_path(
        canonical
            .strip_prefix(cwd)
            .expect("containment was checked"),
    )?;
    files
        .entry(relative)
        .or_insert_with(|| (canonical.to_path_buf(), language.to_string()));
    Ok(())
}

fn is_skipped_path(path: &Path, cwd: &Path) -> Result<bool, String> {
    let relative = path.strip_prefix(cwd).expect("containment was checked");
    for component in relative.components() {
        let name = component
            .as_os_str()
            .to_str()
            .ok_or_else(|| "selected path is not valid UTF-8".to_string())?;
        if name.starts_with('.') || name == "node_modules" || name == "vendor" {
            return Ok(true);
        }
    }
    Ok(false)
}

fn is_ignored_path(path: &Path, cwd: &Path) -> Result<bool, String> {
    let mut ignore_builder = GitignoreBuilder::new(cwd);
    let mut gitignore_builder = GitignoreBuilder::new(cwd);
    add_ignore_files(&mut ignore_builder, &mut gitignore_builder, cwd)?;

    let relative_parent = path
        .strip_prefix(cwd)
        .expect("containment was checked")
        .parent()
        .expect("selected file should have a parent");
    let mut directory = cwd.to_path_buf();
    for component in relative_parent.components() {
        directory.push(component);
        add_ignore_files(&mut ignore_builder, &mut gitignore_builder, &directory)?;
    }

    let ignore_matcher = ignore_builder
        .build()
        .map_err(|error| format!("cannot build .ignore matcher: {error}"))?;
    let ignore_match = ignore_matcher.matched_path_or_any_parents(path, false);
    if !ignore_match.is_none() {
        return Ok(ignore_match.is_ignore());
    }

    let gitignore_matcher = gitignore_builder
        .build()
        .map_err(|error| format!("cannot build .gitignore matcher: {error}"))?;
    Ok(gitignore_matcher
        .matched_path_or_any_parents(path, false)
        .is_ignore())
}

fn add_ignore_files(
    ignore_builder: &mut GitignoreBuilder,
    gitignore_builder: &mut GitignoreBuilder,
    directory: &Path,
) -> Result<(), String> {
    for (filename, builder) in [
        (".ignore", ignore_builder),
        (".gitignore", gitignore_builder),
    ] {
        let ignore_file = directory.join(filename);
        if !ignore_file.is_file() {
            continue;
        }
        if let Some(error) = builder.add(&ignore_file) {
            return Err(format!("cannot read {}: {error}", ignore_file.display()));
        }
    }
    Ok(())
}

fn language_for_path(path: &Path) -> Option<(LanguageFilter, &'static str)> {
    match path.extension()?.to_str()? {
        "js" | "mjs" | "cjs" => Some((LanguageFilter::JavaScript, "javascript")),
        "jsx" => Some((LanguageFilter::JavaScript, "jsx")),
        "ts" | "mts" | "cts" => Some((LanguageFilter::TypeScript, "typescript")),
        "tsx" => Some((LanguageFilter::TypeScript, "tsx")),
        "php" => Some((LanguageFilter::Php, "php")),
        _ => None,
    }
}

fn language_is_selected(language: LanguageFilter, selected: &BTreeSet<LanguageFilter>) -> bool {
    selected.is_empty() || selected.contains(&language)
}

fn analyze_selected_file(
    path: &Path,
    relative: String,
    language: String,
    max_complexity: u32,
) -> FileResult {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return failed_file(relative, language, error.to_string()),
    };
    let source = match String::from_utf8(bytes) {
        Ok(source) => source,
        Err(error) => {
            return failed_file(
                relative,
                language,
                format!("file is not valid UTF-8: {error}"),
            );
        }
    };

    analyze_source(&relative, &source, &language, max_complexity)
}

fn analyze_source(relative: &str, source: &str, language: &str, max_complexity: u32) -> FileResult {
    if language == "php" {
        php::analyze_source(relative, source, max_complexity)
    } else {
        javascript::analyze_source(relative, source, max_complexity)
    }
}

fn failed_file(path: String, language: String, message: String) -> FileResult {
    FileResult {
        path,
        language,
        status: FileStatus::IoError,
        signals: None,
        functions: Vec::new(),
        diagnostics: vec![model::Diagnostic {
            location: model::Position { line: 1, column: 1 },
            message,
        }],
    }
}

fn build_report(max_complexity: u32, files: Vec<FileResult>) -> Report {
    let mut summary = Summary {
        files: files.len(),
        functions: 0,
        violations: 0,
        errors: 0,
        max_score: 0,
        max_control_depth: 0,
        max_function_line_span: 0,
        max_functions_per_file: 0,
        conditions: 0,
        max_condition_operators: 0,
        max_condition_predicates: 0,
        max_boolean_depth: 0,
    };

    for file in &files {
        if file.status != FileStatus::Ok {
            summary.errors += 1;
            continue;
        }

        if let Some(signals) = &file.signals {
            summary.max_functions_per_file =
                summary.max_functions_per_file.max(signals.function_count);
        }
        for function in &file.functions {
            summary.functions += 1;
            if function.over_limit {
                summary.violations += 1;
            }
            summary.max_score = summary.max_score.max(function.score);
            summary.max_control_depth = summary
                .max_control_depth
                .max(function.signals.max_control_depth);
            summary.max_function_line_span = summary
                .max_function_line_span
                .max(function.signals.line_span);
            summary.conditions += function.signals.condition_count;
            summary.max_condition_operators = summary
                .max_condition_operators
                .max(function.signals.max_condition_operators);
            summary.max_condition_predicates = summary
                .max_condition_predicates
                .max(function.signals.max_condition_predicates);
            summary.max_boolean_depth = summary
                .max_boolean_depth
                .max(function.signals.max_boolean_depth);
        }
    }

    Report {
        schema_version: 2,
        tool: Tool {
            name: "complexity",
            version: VERSION,
        },
        profile: PROFILE,
        max_complexity,
        status: if summary.errors == 0 {
            RunStatus::Complete
        } else {
            RunStatus::Incomplete
        },
        summary,
        files,
    }
}

fn report_exit_code(report: &Report) -> u8 {
    if report.status == RunStatus::Incomplete {
        return 2;
    }
    if report.summary.violations > 0 {
        return 1;
    }
    0
}

fn format_json(report: &Report) -> String {
    let mut output = serde_json::to_string(report).expect("report should serialize");
    output.push('\n');
    output
}

fn format_text(report: &Report) -> String {
    let mut output = String::new();
    for file in &report.files {
        for function in &file.functions {
            let status = if function.over_limit { "FAIL" } else { "PASS" };
            output.push_str(&format!(
                "{status} {} {} score={} lines={} control-depth={} conditions={} \
condition-operators={} condition-predicates={} boolean-depth={}\n",
                function.id,
                function.name,
                function.score,
                function.signals.line_span,
                function.signals.max_control_depth,
                function.signals.condition_count,
                function.signals.max_condition_operators,
                function.signals.max_condition_predicates,
                function.signals.max_boolean_depth,
            ));
        }
        for diagnostic in &file.diagnostics {
            output.push_str(&format!(
                "ERROR {}:{}:{} {}\n",
                file.path, diagnostic.location.line, diagnostic.location.column, diagnostic.message
            ));
        }
    }
    output.push_str(&format!(
        "Summary: files={} functions={} violations={} errors={} max_score={} \
max_control_depth={} max_function_line_span={} max_functions_per_file={} conditions={} \
max_condition_operators={} max_condition_predicates={} max_boolean_depth={}\n",
        report.summary.files,
        report.summary.functions,
        report.summary.violations,
        report.summary.errors,
        report.summary.max_score,
        report.summary.max_control_depth,
        report.summary.max_function_line_span,
        report.summary.max_functions_per_file,
        report.summary.conditions,
        report.summary.max_condition_operators,
        report.summary.max_condition_predicates,
        report.summary.max_boolean_depth,
    ));
    output
}

fn portable_path(path: &Path) -> Result<String, String> {
    path.components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .map(str::to_string)
                .ok_or_else(|| "selected path is not valid UTF-8".to_string())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|components| components.join("/"))
}

fn success(stdout: String) -> CommandOutput {
    CommandOutput {
        stdout,
        stderr: String::new(),
        exit_code: 0,
    }
}

fn usage_error(message: String) -> CommandOutput {
    CommandOutput {
        stdout: String::new(),
        stderr: format!("error: {message}\n"),
        exit_code: 2,
    }
}
