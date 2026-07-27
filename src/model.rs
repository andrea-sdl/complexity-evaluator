use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u8,
    pub tool: Tool,
    pub profile: &'static str,
    pub max_complexity: u32,
    pub status: RunStatus,
    pub files: Vec<FileResult>,
    pub summary: Summary,
}

#[derive(Debug, Serialize)]
pub struct Tool {
    pub name: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Complete,
    Incomplete,
}

#[derive(Debug, Serialize)]
pub struct FileResult {
    pub path: String,
    pub language: String,
    pub status: FileStatus,
    pub signals: Option<FileSignals>,
    pub functions: Vec<FunctionResult>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Ok,
    ParseError,
    IoError,
}

#[derive(Debug, Serialize)]
pub struct FileSignals {
    pub function_count: usize,
}

#[derive(Debug, Serialize)]
pub struct FunctionResult {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub range: SourceRange,
    pub score: u32,
    pub over_limit: bool,
    pub contributions: Vec<Contribution>,
    pub signals: FunctionSignals,
}

#[derive(Debug, Serialize)]
pub struct Contribution {
    pub rule: String,
    pub location: Position,
    pub base_increment: u32,
    pub nesting_increment: u32,
    pub increment: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SourceRange {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub location: Position,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct FunctionSignals {
    pub line_span: usize,
    pub max_control_depth: usize,
    pub condition_count: usize,
    pub max_condition_operators: usize,
    pub max_condition_predicates: usize,
    pub max_boolean_depth: usize,
    pub conditions: Vec<ConditionRecord>,
}

impl FunctionSignals {
    pub fn empty() -> Self {
        Self {
            line_span: 0,
            max_control_depth: 0,
            condition_count: 0,
            max_condition_operators: 0,
            max_condition_predicates: 0,
            max_boolean_depth: 0,
            conditions: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ConditionRecord {
    pub kind: String,
    pub location: Position,
    pub operator_count: usize,
    pub predicate_count: usize,
    pub max_boolean_depth: usize,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub files: usize,
    pub functions: usize,
    pub violations: usize,
    pub errors: usize,
    pub max_score: u32,
    pub max_control_depth: usize,
    pub max_function_line_span: usize,
    pub max_functions_per_file: usize,
    pub conditions: usize,
    pub max_condition_operators: usize,
    pub max_condition_predicates: usize,
    pub max_boolean_depth: usize,
}
