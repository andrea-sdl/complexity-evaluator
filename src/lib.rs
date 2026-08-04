mod javascript;
pub(crate) mod model;
mod php;
mod python;
mod rust;

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};

use cap_std::{
    ambient_authority,
    fs::{Dir, DirEntry},
};
use ignore::{WalkBuilder, gitignore::GitignoreBuilder};
use model::{FileResult, FileStatus, Report, RunStatus, Summary, Tool};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const USAGE: &str = "Usage: complexity [--language javascript|typescript|php|rust|python]... \
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

struct OptionParser {
    options: Options,
    format_seen: bool,
    max_complexity_seen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LanguageFilter {
    JavaScript,
    TypeScript,
    Php,
    Rust,
    Python,
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
    if let Some(command) = sole_command(&arguments) {
        return Ok(command);
    }

    let mut parser = OptionParser::new();
    let mut arguments = arguments.into_iter();

    while let Some(argument) = arguments.next() {
        let argument = argument
            .to_str()
            .ok_or_else(|| "selected path is not valid UTF-8".to_string())?;
        parser.add_argument(argument, &mut arguments)?;
    }

    Ok(Command::Run(parser.finish()?))
}

impl OptionParser {
    fn new() -> Self {
        Self {
            options: Options {
                format: OutputFormat::Text,
                max_complexity: DEFAULT_MAX_COMPLEXITY,
                languages: BTreeSet::new(),
                stdin_filename: None,
                paths: Vec::new(),
            },
            format_seen: false,
            max_complexity_seen: false,
        }
    }

    fn add_argument(
        &mut self,
        argument: &str,
        arguments: &mut impl Iterator<Item = OsString>,
    ) -> Result<(), String> {
        match argument {
            "--language" => {
                self.options.languages.insert(parse_language(arguments)?);
            }
            "--format" => self.set_format(arguments)?,
            "--max-complexity" => self.set_max_complexity(arguments)?,
            "--stdin-filename" => self.set_stdin_filename(arguments)?,
            "-" => self.options.paths.push(PathBuf::from("-")),
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}"));
            }
            value => self.options.paths.push(PathBuf::from(value)),
        }

        Ok(())
    }

    fn set_format(&mut self, arguments: &mut impl Iterator<Item = OsString>) -> Result<(), String> {
        if self.format_seen {
            return Err("--format cannot be repeated".to_string());
        }

        self.format_seen = true;
        self.options.format = parse_format(arguments)?;
        Ok(())
    }

    fn set_max_complexity(
        &mut self,
        arguments: &mut impl Iterator<Item = OsString>,
    ) -> Result<(), String> {
        if self.max_complexity_seen {
            return Err("--max-complexity cannot be repeated".to_string());
        }

        self.max_complexity_seen = true;
        self.options.max_complexity = parse_max_complexity(arguments)?;
        Ok(())
    }

    fn set_stdin_filename(
        &mut self,
        arguments: &mut impl Iterator<Item = OsString>,
    ) -> Result<(), String> {
        if self.options.stdin_filename.is_some() {
            return Err("--stdin-filename cannot be repeated".to_string());
        }

        self.options.stdin_filename = Some(option_value(arguments, "--stdin-filename", "a path")?);
        Ok(())
    }

    fn finish(self) -> Result<Options, String> {
        validate_input_options(
            &self.options.paths,
            &self.options.languages,
            self.options.stdin_filename.is_some(),
        )?;
        Ok(self.options)
    }
}

fn sole_command(arguments: &[OsString]) -> Option<Command> {
    if arguments.len() != 1 {
        return None;
    }

    match arguments[0].to_str()? {
        "--help" => Some(Command::Help),
        "--version" => Some(Command::Version),
        _ => None,
    }
}

fn parse_language(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<LanguageFilter, String> {
    match option_value(
        arguments,
        "--language",
        "javascript, typescript, php, rust, or python",
    )?
    .as_str()
    {
        "javascript" => Ok(LanguageFilter::JavaScript),
        "typescript" => Ok(LanguageFilter::TypeScript),
        "php" => Ok(LanguageFilter::Php),
        "rust" => Ok(LanguageFilter::Rust),
        "python" => Ok(LanguageFilter::Python),
        _ => Err("--language requires javascript, typescript, php, rust, or python".to_string()),
    }
}

fn parse_format(arguments: &mut impl Iterator<Item = OsString>) -> Result<OutputFormat, String> {
    match option_value(arguments, "--format", "text or json")?.as_str() {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err("--format requires text or json".to_string()),
    }
}

fn parse_max_complexity(arguments: &mut impl Iterator<Item = OsString>) -> Result<u32, String> {
    option_value(arguments, "--max-complexity", "a non-negative integer")?
        .parse()
        .map_err(|_| "--max-complexity requires a non-negative integer".to_string())
}

fn option_value(
    arguments: &mut impl Iterator<Item = OsString>,
    option: &str,
    required_value: &str,
) -> Result<String, String> {
    let value = arguments
        .next()
        .ok_or_else(|| format!("{option} requires {required_value}"))?;
    value
        .into_string()
        .map_err(|_| format!("{option} value is not valid UTF-8"))
}

fn validate_input_options(
    paths: &[PathBuf],
    languages: &BTreeSet<LanguageFilter>,
    has_stdin_filename: bool,
) -> Result<(), String> {
    if paths.is_empty() {
        return Err("at least one path is required".to_string());
    }

    let reads_stdin = paths.iter().any(|path| path == Path::new("-"));
    if reads_stdin {
        return validate_stdin_input(paths, languages);
    }
    if has_stdin_filename {
        return Err("--stdin-filename is valid only with -".to_string());
    }

    Ok(())
}

fn validate_stdin_input(
    paths: &[PathBuf],
    languages: &BTreeSet<LanguageFilter>,
) -> Result<(), String> {
    if paths.len() != 1 {
        return Err("- must be the sole input".to_string());
    }
    if languages.len() != 1 {
        return Err("stdin requires exactly one language".to_string());
    }

    Ok(())
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
    let root = match Dir::open_ambient_dir(&cwd, ambient_authority()) {
        Ok(root) => root,
        Err(error) => return usage_error(format!("cannot open working directory: {error}")),
    };
    let selected_files = match select_files(&options.paths, &options.languages, &cwd) {
        Ok(files) => files,
        Err(message) => return usage_error(message),
    };
    let mut parent_directories = BTreeMap::new();
    let files = selected_files
        .into_iter()
        .map(|(relative, language)| {
            analyze_selected_file(
                &root,
                &mut parent_directories,
                relative,
                language,
                options.max_complexity,
            )
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
        LanguageFilter::Rust => "stdin.rs",
        LanguageFilter::Python => "stdin.py",
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
) -> Result<Vec<(String, String)>, String> {
    let mut files = BTreeMap::new();
    for supplied in paths {
        select_input(supplied, languages, cwd, &mut files)?;
    }
    if files.is_empty() {
        return Err("no selected files found".to_string());
    }
    Ok(files.into_iter().collect())
}

fn select_input(
    supplied: &Path,
    languages: &BTreeSet<LanguageFilter>,
    cwd: &Path,
    files: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let resolved = resolve_input(supplied, cwd)?;
    if resolved.is_directory_symlink {
        return Ok(());
    }
    if resolved.canonical.is_file() {
        return add_explicit_file(&resolved.canonical, supplied, languages, cwd, files);
    }
    if !resolved.canonical.is_dir() {
        return Err(format!("unsupported input path: {}", supplied.display()));
    }
    if is_skipped_path(&resolved.canonical, cwd)? {
        return Ok(());
    }

    discover_directory(&resolved.canonical, languages, cwd, files)
}

struct ResolvedInput {
    canonical: PathBuf,
    is_directory_symlink: bool,
}

fn resolve_input(supplied: &Path, cwd: &Path) -> Result<ResolvedInput, String> {
    let candidate = if supplied.is_absolute() {
        supplied.to_path_buf()
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

    Ok(ResolvedInput {
        is_directory_symlink: metadata.file_type().is_symlink() && canonical.is_dir(),
        canonical,
    })
}

fn add_explicit_file(
    canonical: &Path,
    supplied: &Path,
    languages: &BTreeSet<LanguageFilter>,
    cwd: &Path,
    files: &mut BTreeMap<String, String>,
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
    files: &mut BTreeMap<String, String>,
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
        add_discovered_file(entry, languages, cwd, files)?;
    }
    Ok(())
}

fn add_discovered_file(
    entry: ignore::DirEntry,
    languages: &BTreeSet<LanguageFilter>,
    cwd: &Path,
    files: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let Some(resolved) = resolve_discovered_file(entry, cwd)? else {
        return Ok(());
    };
    if resolved.is_symlink && is_ignored_path(&resolved.canonical, cwd)? {
        return Ok(());
    }
    let Some((family, language)) = language_for_path(&resolved.canonical) else {
        return Ok(());
    };
    if !language_is_selected(family, languages) {
        return Ok(());
    }

    insert_file(&resolved.canonical, cwd, language, files)
}

struct ResolvedDiscoveredFile {
    canonical: PathBuf,
    is_symlink: bool,
}

fn resolve_discovered_file(
    entry: ignore::DirEntry,
    cwd: &Path,
) -> Result<Option<ResolvedDiscoveredFile>, String> {
    let Some(file_type) = entry.file_type() else {
        return Ok(None);
    };
    let is_symlink = file_type.is_symlink();
    if !file_type.is_file() && !is_symlink {
        return Ok(None);
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
        return Ok(None);
    }
    if is_skipped_path(&canonical, cwd)? {
        return Ok(None);
    }

    Ok(Some(ResolvedDiscoveredFile {
        canonical,
        is_symlink,
    }))
}

fn insert_file(
    canonical: &Path,
    cwd: &Path,
    language: &str,
    files: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let relative = portable_path(
        canonical
            .strip_prefix(cwd)
            .expect("containment was checked"),
    )?;
    files
        .entry(relative)
        .or_insert_with(|| language.to_string());
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
        "rs" => Some((LanguageFilter::Rust, "rust")),
        "py" => Some((LanguageFilter::Python, "python")),
        _ => None,
    }
}

fn language_is_selected(language: LanguageFilter, selected: &BTreeSet<LanguageFilter>) -> bool {
    selected.is_empty() || selected.contains(&language)
}

fn analyze_selected_file(
    root: &Dir,
    parent_directories: &mut BTreeMap<PathBuf, ParentDirectory>,
    relative: String,
    language: String,
    max_complexity: u32,
) -> FileResult {
    let bytes = match read_selected_file(root, parent_directories, Path::new(&relative)) {
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

fn read_selected_file(
    root: &Dir,
    parent_directories: &mut BTreeMap<PathBuf, ParentDirectory>,
    relative: &Path,
) -> std::io::Result<Vec<u8>> {
    let parent = relative
        .parent()
        .expect("selected file should have a parent");
    let filename = relative
        .file_name()
        .expect("selected file should have a name");
    if parent.as_os_str().is_empty() {
        return root.read(filename);
    }

    let directory = match parent_directories.entry(parent.to_path_buf()) {
        std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(ParentDirectory::open(root, parent)?)
        }
    };
    directory.read(filename)
}

struct ParentDirectory {
    entries: BTreeMap<OsString, DirEntry>,
}

impl ParentDirectory {
    fn open(root: &Dir, relative: &Path) -> std::io::Result<Self> {
        let directory = root.open_dir(relative)?;
        let mut entries = BTreeMap::new();
        for entry in directory.entries()? {
            let entry = entry?;
            entries.insert(entry.file_name(), entry);
        }
        Ok(Self { entries })
    }

    fn read(&self, filename: &std::ffi::OsStr) -> std::io::Result<Vec<u8>> {
        let Some(entry) = self.entries.get(filename) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "selected file changed before analysis",
            ));
        };
        let mut file = entry.open()?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

fn analyze_source(relative: &str, source: &str, language: &str, max_complexity: u32) -> FileResult {
    match language {
        "javascript" | "jsx" | "typescript" | "tsx" => {
            javascript::analyze_source(relative, source, max_complexity)
        }
        "php" => php::analyze_source(relative, source, max_complexity),
        "python" => python::analyze_source(relative, source, max_complexity),
        "rust" => rust::analyze_source(relative, source, max_complexity),
        _ => failed_file(
            relative.to_string(),
            language.to_string(),
            "unsupported internal language label".to_string(),
        ),
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
    let mut summary = empty_summary(files.len());

    for file in &files {
        update_summary(&mut summary, file);
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

fn empty_summary(file_count: usize) -> Summary {
    Summary {
        files: file_count,
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
    }
}

fn update_summary(summary: &mut Summary, file: &FileResult) {
    if file.status != FileStatus::Ok {
        summary.errors += 1;
        return;
    }

    if let Some(signals) = &file.signals {
        summary.max_functions_per_file = summary.max_functions_per_file.max(signals.function_count);
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
        append_file_text(&mut output, file);
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

fn append_file_text(output: &mut String, file: &FileResult) {
    for function in &file.functions {
        let status = if function.over_limit { "FAIL" } else { "PASS" };
        let id = text_safe(&function.id);
        let name = text_safe(&function.name);
        output.push_str(&format!(
            "{status} {} {} score={} lines={} control-depth={} conditions={} \
condition-operators={} condition-predicates={} boolean-depth={}\n",
            id,
            name,
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
        let path = text_safe(&file.path);
        let message = text_safe(&diagnostic.message);
        output.push_str(&format!(
            "ERROR {}:{}:{} {}\n",
            path, diagnostic.location.line, diagnostic.location.column, message
        ));
    }
}

fn text_safe(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        let is_bidirectional_control = matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        );
        if character.is_control() || is_bidirectional_control {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
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
        stderr: format!("error: {}\n", text_safe(&message)),
        exit_code: 2,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsStr,
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn parent_directory_snapshot_rejects_missing_and_replaced_names() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should follow the Unix epoch")
            .as_nanos();
        let root_path = std::env::temp_dir().join(format!("complexity-parent-cache-{unique}"));
        let nested = root_path.join("nested");
        fs::create_dir_all(&nested).expect("nested test directory should be created");
        let root = Dir::open_ambient_dir(&root_path, ambient_authority())
            .expect("test root capability should open");
        let parent =
            ParentDirectory::open(&root, Path::new("nested")).expect("parent snapshot should open");
        fs::write(nested.join("late.js"), "function late() {}\n")
            .expect("late file should be written");

        let error = parent
            .read(OsStr::new("late.js"))
            .expect_err("a name missing from the snapshot must fail closed");

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let victim = nested.join("victim.js");
            let outside = root_path.with_extension("outside.js");
            fs::write(&victim, "function inside() {}\n").expect("selected file should be written");
            fs::write(&outside, "function outside_secret() {}\n")
                .expect("outside file should be written");
            let parent = ParentDirectory::open(&root, Path::new("nested"))
                .expect("parent snapshot should include the selected file");
            fs::remove_file(&victim).expect("selected file should be removed");
            symlink(&outside, &victim).expect("outside symlink should replace the selected file");

            parent
                .read(OsStr::new("victim.js"))
                .expect_err("a cached name replaced by an outside symlink must fail closed");
            fs::remove_file(outside).expect("outside test file should be removed");
        }

        fs::remove_dir_all(root_path).expect("test directory should be removed");
    }
}
