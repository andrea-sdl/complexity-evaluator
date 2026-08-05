use tree_sitter::{Node, Parser, Tree};

use crate::model::{
    CognitiveLoadFinding, ConditionRecord, Contribution, Diagnostic, FileResult, FileSignals,
    FileStatus, FunctionResult, FunctionSignals, Position, SourceRange,
};

const MAX_ANALYSIS_TREE_DEPTH: usize = 512;

pub(crate) fn analyze_source(path: &str, source: &str, max_complexity: u32) -> FileResult {
    let tree = match parse_source(source) {
        Ok(tree) => tree,
        Err(message) => return failed_file(path, message, Position { line: 1, column: 1 }),
    };
    let root = tree.root_node();
    let positions = LineIndex::new(source);
    if tree_exceeds_supported_depth(root) {
        return failed_file(
            path,
            format!("analysis nesting exceeds the supported limit of {MAX_ANALYSIS_TREE_DEPTH}"),
            positions.position(root.start_byte()),
        );
    }
    if root.has_error() {
        let error = first_syntax_error(root).unwrap_or(root);
        return failed_file(
            path,
            format!("syntax error near {}", error.kind()),
            positions.position(error.start_byte()),
        );
    }

    let functions = function_results(root, source, &positions, path, max_complexity);

    FileResult {
        path: path.to_string(),
        language: "rust".to_string(),
        status: FileStatus::Ok,
        signals: Some(FileSignals {
            function_count: functions.len(),
        }),
        functions,
        diagnostics: Vec::new(),
    }
}

fn tree_exceeds_supported_depth(root: Node<'_>) -> bool {
    let mut nodes = vec![(root, 0)];

    while let Some((node, depth)) = nodes.pop() {
        if depth > MAX_ANALYSIS_TREE_DEPTH {
            return true;
        }

        let mut cursor = node.walk();
        nodes.extend(node.children(&mut cursor).map(|child| (child, depth + 1)));
    }

    false
}

fn parse_source(source: &str) -> Result<Tree, String> {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE.into();
    parser
        .set_language(&language)
        .map_err(|error| format!("cannot load Rust grammar: {error}"))?;
    parser
        .parse(source, None)
        .ok_or_else(|| "parser returned no syntax tree".to_string())
}

fn function_results(
    root: Node<'_>,
    source: &str,
    positions: &LineIndex<'_>,
    path: &str,
    max_complexity: u32,
) -> Vec<FunctionResult> {
    let mut functions = Vec::new();
    collect_functions(
        root,
        source,
        positions,
        path,
        max_complexity,
        &mut functions,
    );
    functions.sort_by(function_order);
    functions
}

fn function_order(left: &FunctionResult, right: &FunctionResult) -> std::cmp::Ordering {
    (
        left.range.start.line,
        left.range.start.column,
        left.range.end.line,
        left.range.end.column,
        &left.kind,
        &left.id,
    )
        .cmp(&(
            right.range.start.line,
            right.range.start.column,
            right.range.end.line,
            right.range.end.column,
            &right.kind,
            &right.id,
        ))
}

fn collect_functions(
    node: Node<'_>,
    source: &str,
    positions: &LineIndex<'_>,
    path: &str,
    max_complexity: u32,
    functions: &mut Vec<FunctionResult>,
) {
    if is_macro(node) {
        return;
    }
    if let Some(function) = function_result(node, source, positions, path, max_complexity) {
        functions.push(function);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_functions(child, source, positions, path, max_complexity, functions);
    }
}

fn function_result(
    node: Node<'_>,
    source: &str,
    positions: &LineIndex<'_>,
    path: &str,
    max_complexity: u32,
) -> Option<FunctionResult> {
    let kind = function_kind(node)?;
    let body = callable_body(node)?;
    let start = positions.position(node.start_byte());
    let end = positions.position(node.end_byte());
    let name = function_name(node, kind, source);
    let mut contributions = Vec::new();
    score_node(body, 0, positions, &mut contributions);
    contributions.sort_by(contribution_order);
    let score = contributions.iter().map(|item| item.increment).sum();

    let id = format!("{path}:{}:{}", start.line, start.column);
    Some(FunctionResult {
        id: id.clone(),
        name,
        kind: kind.to_string(),
        range: SourceRange { start, end },
        score,
        over_limit: score > max_complexity,
        contributions,
        signals: function_signals(body, start, end, positions),
        cognitive_load_findings: cognitive_load_findings(body, path, &id, positions),
    })
}

fn cognitive_load_findings(
    body: Node<'_>,
    path: &str,
    function_id: &str,
    positions: &LineIndex<'_>,
) -> Vec<CognitiveLoadFinding> {
    let mut findings = Vec::new();
    collect_cognitive_load_findings(body, path, function_id, positions, &mut findings);
    findings
}

fn collect_cognitive_load_findings(
    node: Node<'_>,
    path: &str,
    function_id: &str,
    positions: &LineIndex<'_>,
    findings: &mut Vec<CognitiveLoadFinding>,
) {
    if is_analysis_boundary(node) {
        return;
    }
    if node.kind() == "return_expression"
        && let Some(expression) = node.named_child(0)
        && expression.kind() == "if_expression"
        && expression.child_by_field_name("alternative").is_some()
    {
        let has_boolean_operator = expression
            .child_by_field_name("condition")
            .is_some_and(|test| count_boolean_operators(test).0 > 0);
        let branch_has_cast = ["consequence", "alternative"].into_iter().any(|field| {
            expression
                .child_by_field_name(field)
                .is_some_and(branch_has_direct_cast)
        });
        let load = 1 + u32::from(has_boolean_operator) + u32::from(branch_has_cast);
        findings.push(CognitiveLoadFinding {
            rule: "cognitive_load.inline_conditional_return",
            path: path.to_string(),
            function_id: function_id.to_string(),
            location: positions.position(expression.start_byte()),
            load,
        });
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_cognitive_load_findings(child, path, function_id, positions, findings);
    }
}

fn branch_has_direct_cast(branch: Node<'_>) -> bool {
    if branch.kind() == "type_cast_expression" {
        return true;
    }
    let mut cursor = branch.walk();
    branch
        .named_children(&mut cursor)
        .any(|child| child.kind() == "type_cast_expression")
}

fn function_name(node: Node<'_>, kind: &str, source: &str) -> String {
    if kind == "closure" {
        return "<anonymous>".to_string();
    }
    node.child_by_field_name("name")
        .and_then(|name| name.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<anonymous>")
        .to_string()
}

fn function_signals(
    body: Node<'_>,
    start: Position,
    end: Position,
    positions: &LineIndex<'_>,
) -> FunctionSignals {
    let conditions = condition_records(body, positions);
    let max_condition_operators = conditions
        .iter()
        .map(|condition| condition.operator_count)
        .max()
        .unwrap_or(0);
    let max_condition_predicates = conditions
        .iter()
        .map(|condition| condition.predicate_count)
        .max()
        .unwrap_or(0);
    let max_boolean_depth = conditions
        .iter()
        .map(|condition| condition.max_boolean_depth)
        .max()
        .unwrap_or(0);

    FunctionSignals {
        line_span: end.line - start.line + 1,
        max_control_depth: max_control_depth(body),
        condition_count: conditions.len(),
        max_condition_operators,
        max_condition_predicates,
        max_boolean_depth,
        conditions,
    }
}

fn contribution_order(left: &Contribution, right: &Contribution) -> std::cmp::Ordering {
    (
        left.location.line,
        left.location.column,
        &left.rule,
        left.increment,
    )
        .cmp(&(
            right.location.line,
            right.location.column,
            &right.rule,
            right.increment,
        ))
}

fn function_kind(node: Node<'_>) -> Option<&'static str> {
    match node.kind() {
        "function_item" => {
            let parameters = node.child_by_field_name("parameters")?;
            let mut cursor = parameters.walk();
            if parameters
                .named_children(&mut cursor)
                .any(|parameter| parameter.kind() == "self_parameter")
            {
                Some("method")
            } else {
                Some("function")
            }
        }
        "closure_expression" => Some("closure"),
        _ => None,
    }
}

fn callable_body(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "function_item" => node.child_by_field_name("body"),
        "closure_expression" => node.child_by_field_name("body"),
        _ => None,
    }
}

fn is_callable(node: Node<'_>) -> bool {
    function_kind(node).is_some()
}

fn is_macro(node: Node<'_>) -> bool {
    matches!(node.kind(), "macro_definition" | "macro_invocation")
}

fn is_analysis_boundary(node: Node<'_>) -> bool {
    is_callable(node) || is_macro(node)
}

fn score_node(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    if is_analysis_boundary(node) {
        return;
    }
    match node.kind() {
        "if_expression" => return score_if(node, nesting, positions, contributions),
        "loop_expression" | "while_expression" | "for_expression" => {
            return score_loop(node, nesting, positions, contributions);
        }
        "match_expression" => return score_match(node, nesting, positions, contributions),
        "break_expression" | "continue_expression" => {
            let mut cursor = node.walk();
            if node
                .named_children(&mut cursor)
                .any(|child| child.kind() == "label")
            {
                add_flat_contribution("labeled_jump", node, positions, contributions);
            }
        }
        "binary_expression" if boolean_binary_operator(node).is_some() => {
            return score_logical_sequence(node, nesting, positions, contributions);
        }
        _ => {}
    }
    score_children(node, nesting, positions, contributions);
}

fn score_children(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        score_node(child, nesting, positions, contributions);
    }
}

fn score_if(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("if", node, nesting, positions, contributions);
    if let Some(condition) = node.child_by_field_name("condition") {
        score_node(condition, nesting, positions, contributions);
    }
    if let Some(consequence) = node.child_by_field_name("consequence") {
        score_node(consequence, nesting + 1, positions, contributions);
    }
    let Some(alternative) = node.child_by_field_name("alternative") else {
        return;
    };
    let alternative_body = else_clause_body(alternative).unwrap_or(alternative);
    if alternative_body.kind() == "if_expression" {
        score_else_if(alternative_body, nesting, positions, contributions);
    } else {
        add_flat_contribution(
            "else",
            child_with_kind(node, "else").unwrap_or(alternative),
            positions,
            contributions,
        );
        score_node(alternative_body, nesting + 1, positions, contributions);
    }
}

fn score_else_if(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_flat_contribution("else_if", node, positions, contributions);
    if let Some(condition) = node.child_by_field_name("condition") {
        score_node(condition, nesting, positions, contributions);
    }
    if let Some(consequence) = node.child_by_field_name("consequence") {
        score_node(consequence, nesting + 1, positions, contributions);
    }
    let Some(alternative) = node.child_by_field_name("alternative") else {
        return;
    };
    let alternative_body = else_clause_body(alternative).unwrap_or(alternative);
    if alternative_body.kind() == "if_expression" {
        score_else_if(alternative_body, nesting, positions, contributions);
        return;
    }
    add_flat_contribution(
        "else",
        child_with_kind(node, "else").unwrap_or(alternative),
        positions,
        contributions,
    );
    score_node(alternative_body, nesting + 1, positions, contributions);
}

fn score_loop(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("loop", node, nesting, positions, contributions);
    score_with_nested_body(node, nesting, positions, contributions);
}

fn score_match(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("match", node, nesting, positions, contributions);
    if let Some(value) = node.child_by_field_name("value") {
        score_node(value, nesting, positions, contributions);
    }
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let mut cursor = body.walk();
    for arm in body.named_children(&mut cursor) {
        if let Some(guard) = match_arm_guard(arm) {
            score_node(guard, nesting, positions, contributions);
        }
        let Some(arm_value) = arm.child_by_field_name("value") else {
            continue;
        };
        score_node(arm_value, nesting + 1, positions, contributions);
    }
}

fn score_with_nested_body(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    let body_id = node.child_by_field_name("body").map(|body| body.id());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let child_nesting = if Some(child.id()) == body_id {
            nesting + 1
        } else {
            nesting
        };
        score_node(child, child_nesting, positions, contributions);
    }
}

fn score_logical_sequence(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    let mut operators = Vec::new();
    collect_boolean_operators(node, &mut operators);
    let mut previous = None;
    for (operator, point) in operators {
        if previous != Some(operator) {
            add_flat_contribution("logical_sequence", point, positions, contributions);
        }
        previous = Some(operator);
    }
    score_logical_operands(node, nesting, positions, contributions);
}

fn collect_boolean_operators<'tree>(
    node: Node<'tree>,
    operators: &mut Vec<(&'static str, Node<'tree>)>,
) {
    if let Some(expression) = parenthesized_inner_expression(node) {
        collect_boolean_operators(expression, operators);
        return;
    }
    let Some((operator, point)) = boolean_binary_operator(node) else {
        return;
    };
    if let Some(left) = node.child_by_field_name("left") {
        collect_boolean_operators(left, operators);
    }
    operators.push((operator, point));
    if let Some(right) = node.child_by_field_name("right") {
        collect_boolean_operators(right, operators);
    }
}

fn score_logical_operands(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    if let Some(expression) = parenthesized_inner_expression(node) {
        score_logical_operands(expression, nesting, positions, contributions);
        return;
    }
    if boolean_binary_operator(node).is_none() {
        score_node(node, nesting, positions, contributions);
        return;
    }
    if let Some(left) = node.child_by_field_name("left") {
        score_logical_operands(left, nesting, positions, contributions);
    }
    if let Some(right) = node.child_by_field_name("right") {
        score_logical_operands(right, nesting, positions, contributions);
    }
}

fn condition_records(body: Node<'_>, positions: &LineIndex<'_>) -> Vec<ConditionRecord> {
    let mut records = Vec::new();
    collect_condition_records(body, positions, &mut records);
    records.sort_by(|left, right| {
        (left.location.line, left.location.column, &left.kind).cmp(&(
            right.location.line,
            right.location.column,
            &right.kind,
        ))
    });
    records
}

fn collect_condition_records(
    node: Node<'_>,
    positions: &LineIndex<'_>,
    records: &mut Vec<ConditionRecord>,
) {
    if is_analysis_boundary(node) {
        return;
    }
    if let Some((kind, expression)) = condition_for_node(node) {
        add_condition_record(kind, expression, positions, records);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_condition_records(child, positions, records);
    }
}

fn condition_for_node(node: Node<'_>) -> Option<(&'static str, Node<'_>)> {
    match node.kind() {
        "if_expression" => Some(("if", node.child_by_field_name("condition")?)),
        "while_expression" => Some(("while", node.child_by_field_name("condition")?)),
        "match_arm" => Some(("match_guard", match_arm_guard(node)?)),
        _ => None,
    }
}

fn add_condition_record(
    kind: &str,
    expression: Node<'_>,
    positions: &LineIndex<'_>,
    records: &mut Vec<ConditionRecord>,
) {
    let (operator_count, binary_operator_count) = count_boolean_operators(expression);
    records.push(ConditionRecord {
        kind: kind.to_string(),
        location: positions.position(expression.start_byte()),
        operator_count,
        predicate_count: binary_operator_count + 1,
        max_boolean_depth: boolean_depth(expression, None),
    });
}

fn count_boolean_operators(node: Node<'_>) -> (usize, usize) {
    if is_analysis_boundary(node) {
        return (0, 0);
    }
    let is_binary = boolean_binary_operator(node).is_some();
    let is_not = boolean_not_argument(node).is_some();
    let mut operators = usize::from(is_binary) + usize::from(is_not);
    let mut binary_operators = usize::from(is_binary);
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let (child_operators, child_binary_operators) = count_boolean_operators(child);
        operators += child_operators;
        binary_operators += child_binary_operators;
    }
    (operators, binary_operators)
}

fn boolean_depth(node: Node<'_>, parent_operator: Option<&str>) -> usize {
    if is_analysis_boundary(node) {
        return 0;
    }
    if let Some(expression) = parenthesized_inner_expression(node) {
        return boolean_depth(expression, parent_operator);
    }
    if let Some(argument) = boolean_not_argument(node) {
        return 1 + boolean_depth(argument, None);
    }
    if let Some((operator, _)) = boolean_binary_operator(node) {
        let layer = usize::from(parent_operator != Some(operator));
        let left_depth = node
            .child_by_field_name("left")
            .map_or(0, |left| boolean_depth(left, Some(operator)));
        let right_depth = node
            .child_by_field_name("right")
            .map_or(0, |right| boolean_depth(right, Some(operator)));
        return layer + left_depth.max(right_depth);
    }
    let mut maximum = 0;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        maximum = maximum.max(boolean_depth(child, None));
    }
    maximum
}

fn boolean_binary_operator(node: Node<'_>) -> Option<(&'static str, Node<'_>)> {
    if node.kind() != "binary_expression" {
        return None;
    }
    let point = node.child_by_field_name("operator")?;
    match point.kind() {
        "&&" => Some(("&&", point)),
        "||" => Some(("||", point)),
        _ => None,
    }
}

fn boolean_not_argument(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "unary_expression" || child_with_kind(node, "!").is_none() {
        return None;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| !child.is_extra())
}

fn max_control_depth(body: Node<'_>) -> usize {
    let mut maximum = 0;
    collect_control_depth(body, 0, &mut maximum);
    maximum
}

fn collect_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    if is_analysis_boundary(node) {
        return;
    }
    match node.kind() {
        "if_expression" => return collect_if_control_depth(node, active_depth, maximum),
        "loop_expression" | "while_expression" | "for_expression" => {
            let region_depth = active_depth + 1;
            *maximum = (*maximum).max(region_depth);
            return collect_with_nested_body(node, active_depth, region_depth, maximum);
        }
        "match_expression" => return collect_match_control_depth(node, active_depth, maximum),
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_control_depth(child, active_depth, maximum);
    }
}

fn collect_match_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    let region_depth = active_depth + 1;
    *maximum = (*maximum).max(region_depth);
    if let Some(value) = node.child_by_field_name("value") {
        collect_control_depth(value, active_depth, maximum);
    }
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };

    let mut cursor = body.walk();
    for arm in body.named_children(&mut cursor) {
        if let Some(guard) = match_arm_guard(arm) {
            collect_control_depth(guard, active_depth, maximum);
        }
        let Some(arm_value) = arm.child_by_field_name("value") else {
            continue;
        };
        collect_control_depth(arm_value, region_depth, maximum);
    }
}

fn collect_if_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    let region_depth = active_depth + 1;
    *maximum = (*maximum).max(region_depth);
    if let Some(condition) = node.child_by_field_name("condition") {
        collect_control_depth(condition, active_depth, maximum);
    }
    if let Some(consequence) = node.child_by_field_name("consequence") {
        collect_control_depth(consequence, region_depth, maximum);
    }
    let Some(alternative) = node.child_by_field_name("alternative") else {
        return;
    };
    let alternative_body = else_clause_body(alternative).unwrap_or(alternative);
    if alternative_body.kind() == "if_expression" {
        collect_if_control_depth(alternative_body, active_depth, maximum);
        return;
    }
    collect_control_depth(alternative_body, region_depth, maximum);
}

fn collect_with_nested_body(
    node: Node<'_>,
    active_depth: usize,
    region_depth: usize,
    maximum: &mut usize,
) {
    let body_id = node.child_by_field_name("body").map(|body| body.id());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let child_depth = if Some(child.id()) == body_id {
            region_depth
        } else {
            active_depth
        };
        collect_control_depth(child, child_depth, maximum);
    }
}

fn parenthesized_inner_expression(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "parenthesized_expression" {
        return None;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| !child.is_extra())
}

fn else_clause_body(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "else_clause" {
        return None;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| !child.is_extra())
}

fn match_arm_guard(node: Node<'_>) -> Option<Node<'_>> {
    let pattern = node.child_by_field_name("pattern")?;
    pattern.child_by_field_name("condition")
}

fn child_with_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn add_contribution(
    rule: &str,
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    contributions.push(Contribution {
        rule: rule.to_string(),
        location: positions.position(node.start_byte()),
        base_increment: 1,
        nesting_increment: nesting,
        increment: 1 + nesting,
    });
}

fn add_flat_contribution(
    rule: &str,
    node: Node<'_>,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    contributions.push(Contribution {
        rule: rule.to_string(),
        location: positions.position(node.start_byte()),
        base_increment: 1,
        nesting_increment: 0,
        increment: 1,
    });
}

fn first_syntax_error(node: Node<'_>) -> Option<Node<'_>> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.has_error()
            && let Some(error) = first_syntax_error(child)
        {
            return Some(error);
        }
    }
    None
}

fn failed_file(path: &str, message: String, location: Position) -> FileResult {
    FileResult {
        path: path.to_string(),
        language: "rust".to_string(),
        status: FileStatus::ParseError,
        signals: None,
        functions: Vec::new(),
        diagnostics: vec![Diagnostic { location, message }],
    }
}

struct LineIndex<'source> {
    source: &'source str,
    line_starts: Vec<usize>,
}

impl<'source> LineIndex<'source> {
    fn new(source: &'source str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self {
            source,
            line_starts,
        }
    }

    fn position(&self, byte_offset: usize) -> Position {
        let line_index = self
            .line_starts
            .partition_point(|start| *start <= byte_offset)
            - 1;
        let line_start = self.line_starts[line_index];
        Position {
            line: line_index + 1,
            column: self.source[line_start..byte_offset].chars().count() + 1,
        }
    }
}
