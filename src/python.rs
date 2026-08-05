use tree_sitter::{Node, Parser, Tree};

use crate::model::{
    CognitiveLoadFinding, ConditionRecord, Contribution, Diagnostic, FileResult, FileSignals,
    FileStatus, FunctionResult, FunctionSignals, Position, SourceRange,
};

const MAX_ANALYSIS_TREE_DEPTH: usize = 512;

pub(crate) fn analyze_source(path: &str, source: &str, max_complexity: u32) -> FileResult {
    let tree = match parse_python_source(source) {
        Ok(tree) => tree,
        Err(message) => {
            return failed_file(path, message, Position { line: 1, column: 1 });
        }
    };

    let root = tree.root_node();
    if exceeds_analysis_depth(root) {
        return failed_file(
            path,
            format!("analysis nesting exceeds the supported limit of {MAX_ANALYSIS_TREE_DEPTH}"),
            Position { line: 1, column: 1 },
        );
    }
    let positions = LineIndex::new(source);
    if let Some(error) = syntax_error(root) {
        return failed_file(
            path,
            format!("syntax error near {}", error.kind()),
            positions.position(error.start_byte()),
        );
    }

    let mut functions = Vec::new();
    collect_functions(
        root,
        source,
        &positions,
        path,
        max_complexity,
        &mut functions,
    );
    functions.sort_by(function_order);

    FileResult {
        path: path.to_string(),
        language: "python".to_string(),
        status: FileStatus::Ok,
        signals: Some(FileSignals {
            function_count: functions.len(),
        }),
        functions,
        diagnostics: Vec::new(),
    }
}

fn exceeds_analysis_depth(root: Node<'_>) -> bool {
    let mut nodes = vec![(root, 0usize)];
    while let Some((node, depth)) = nodes.pop() {
        if depth > MAX_ANALYSIS_TREE_DEPTH {
            return true;
        }

        let mut cursor = node.walk();
        nodes.extend(node.children(&mut cursor).map(|child| (child, depth + 1)));
    }

    false
}

fn parse_python_source(source: &str) -> Result<Tree, String> {
    let mut parser = Parser::new();
    let language = tree_sitter_python::LANGUAGE.into();
    parser
        .set_language(&language)
        .map_err(|error| format!("cannot load Python grammar: {error}"))?;
    parser
        .parse(source, None)
        .ok_or_else(|| "parser returned no syntax tree".to_string())
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
    let start = positions.position(node.start_byte());
    let end = positions.position(node.end_byte());
    let name = node
        .child_by_field_name("name")
        .and_then(|name| name.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<anonymous>")
        .to_string();
    let body = node.child_by_field_name("body");
    let mut contributions = Vec::new();
    if let Some(body) = body {
        score_node(body, 0, positions, &mut contributions);
    }
    contributions.sort_by(contribution_order);
    let score = contributions
        .iter()
        .map(|contribution| contribution.increment)
        .sum();

    let id = format!("{path}:{}:{}", start.line, start.column);
    Some(FunctionResult {
        id: id.clone(),
        name,
        kind: kind.to_string(),
        range: SourceRange { start, end },
        score,
        over_limit: score > max_complexity,
        contributions,
        signals: function_signals(start, end, body, positions),
        cognitive_load_findings: body.map_or_else(Vec::new, |body| {
            cognitive_load_findings(body, path, &id, positions)
        }),
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
    if is_callable_boundary(node) {
        return;
    }
    if node.kind() == "return_statement"
        && let Some(expression) = node.named_child(0)
        && expression.kind() == "conditional_expression"
    {
        let condition = expression.named_child(1);
        let has_boolean_operator =
            condition.is_some_and(|test| count_boolean_operators(test).0 > 0);
        findings.push(CognitiveLoadFinding {
            rule: "cognitive_load.inline_conditional_return",
            path: path.to_string(),
            function_id: function_id.to_string(),
            location: positions.position(expression.start_byte()),
            load: 1 + u32::from(has_boolean_operator),
        });
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_cognitive_load_findings(child, path, function_id, positions, findings);
    }
}

fn function_signals(
    start: Position,
    end: Position,
    body: Option<Node<'_>>,
    positions: &LineIndex<'_>,
) -> FunctionSignals {
    let mut signals = FunctionSignals::empty();
    signals.line_span = end.line - start.line + 1;
    if let Some(body) = body {
        signals.max_control_depth = max_control_depth(body);
        signals.conditions = condition_records(body, positions);
    }
    signals.condition_count = signals.conditions.len();
    for condition in &signals.conditions {
        signals.max_condition_operators = signals
            .max_condition_operators
            .max(condition.operator_count);
        signals.max_condition_predicates = signals
            .max_condition_predicates
            .max(condition.predicate_count);
        signals.max_boolean_depth = signals.max_boolean_depth.max(condition.max_boolean_depth);
    }
    signals
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
        "lambda" => Some("lambda"),
        "function_definition" if is_class_method(node) => Some("method"),
        "function_definition" => Some("function"),
        _ => None,
    }
}

fn is_callable_boundary(node: Node<'_>) -> bool {
    if function_kind(node).is_some() {
        return true;
    }
    if node.kind() != "decorated_definition" {
        return false;
    }
    node.child_by_field_name("definition")
        .is_some_and(|definition| function_kind(definition).is_some())
}

fn is_class_method(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let parent = if parent.kind() == "decorated_definition" {
        parent.parent()
    } else {
        Some(parent)
    };
    matches!(parent, Some(parent) if parent.kind() == "block" && matches!(parent.parent(), Some(class) if class.kind() == "class_definition"))
}

fn score_node(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    if is_callable_boundary(node) {
        return;
    }
    match node.kind() {
        "if_statement" => return score_if(node, nesting, positions, contributions),
        "for_statement" | "while_statement" => {
            return score_loop(node, nesting, positions, contributions);
        }
        "try_statement" => return score_try(node, nesting, positions, contributions),
        "except_clause" => return score_except(node, nesting, positions, contributions),
        "conditional_expression" => return score_ternary(node, nesting, positions, contributions),
        "boolean_operator" => {
            return score_boolean_sequence(node, nesting, positions, contributions);
        }
        _ => {}
    }
    score_children(node, nesting, positions, contributions);
}

fn score_if(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("if", node, nesting, positions, contributions);
    score_field(node, "condition", nesting, positions, contributions);
    score_field(node, "consequence", nesting + 1, positions, contributions);
    let mut cursor = node.walk();
    for alternative in node.children_by_field_name("alternative", &mut cursor) {
        match alternative.kind() {
            "elif_clause" => {
                add_flat_contribution("elif", alternative, positions, contributions);
                score_field(alternative, "condition", nesting, positions, contributions);
                score_field(
                    alternative,
                    "consequence",
                    nesting + 1,
                    positions,
                    contributions,
                );
            }
            "else_clause" => {
                add_flat_contribution("else", alternative, positions, contributions);
                score_field(alternative, "body", nesting + 1, positions, contributions);
            }
            _ => score_node(alternative, nesting, positions, contributions),
        }
    }
}

fn score_loop(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("loop", node, nesting, positions, contributions);
    let body_id = node.child_by_field_name("body").map(|body| body.id());
    let alternative_id = node
        .child_by_field_name("alternative")
        .map(|body| body.id());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if Some(child.id()) == body_id {
            score_node(child, nesting + 1, positions, contributions);
        } else if Some(child.id()) == alternative_id {
            add_flat_contribution("else", child, positions, contributions);
            score_field(child, "body", nesting + 1, positions, contributions);
        } else {
            score_node(child, nesting, positions, contributions);
        }
    }
}

fn score_try(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    let body_id = node.child_by_field_name("body").map(|body| body.id());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if Some(child.id()) == body_id {
            score_node(child, nesting, positions, contributions);
        } else if child.kind() == "else_clause" {
            add_flat_contribution("else", child, positions, contributions);
            score_field(child, "body", nesting + 1, positions, contributions);
        } else {
            score_node(child, nesting, positions, contributions);
        }
    }
}

fn score_except(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("except", node, nesting, positions, contributions);
    let body = first_block(node);
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let child_nesting = if Some(child.id()) == body.map(|body| body.id()) {
            nesting + 1
        } else {
            nesting
        };
        score_node(child, child_nesting, positions, contributions);
    }
}

fn score_ternary(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    let children = named_children(node);
    if children.len() != 3 {
        return score_children(node, nesting, positions, contributions);
    }
    let (consequence, condition, alternative) = (children[0], children[1], children[2]);
    let point =
        token_after(node, consequence.end_byte(), condition.start_byte(), "if").unwrap_or(node);
    add_contribution("ternary", point, nesting, positions, contributions);
    score_node(condition, nesting, positions, contributions);
    score_node(consequence, nesting + 1, positions, contributions);
    score_node(alternative, nesting + 1, positions, contributions);
}

fn score_boolean_sequence(
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
    score_boolean_operands(node, nesting, positions, contributions);
}

fn collect_boolean_operators<'tree>(
    node: Node<'tree>,
    operators: &mut Vec<(&'static str, Node<'tree>)>,
) {
    if let Some(inner) = parenthesized_inner_expression(node) {
        collect_boolean_operators(inner, operators);
        return;
    }
    let Some((operator, point)) = boolean_operator(node) else {
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

fn score_boolean_operands(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    if let Some(inner) = parenthesized_inner_expression(node) {
        return score_boolean_operands(inner, nesting, positions, contributions);
    }
    if boolean_operator(node).is_none() {
        return score_node(node, nesting, positions, contributions);
    }
    if let Some(left) = node.child_by_field_name("left") {
        score_boolean_operands(left, nesting, positions, contributions);
    }
    if let Some(right) = node.child_by_field_name("right") {
        score_boolean_operands(right, nesting, positions, contributions);
    }
}

fn score_field(
    node: Node<'_>,
    field: &str,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    if let Some(child) = node.child_by_field_name(field) {
        score_node(child, nesting, positions, contributions);
    }
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

fn max_control_depth(body: Node<'_>) -> usize {
    let mut maximum = 0;
    collect_control_depth(body, 0, &mut maximum);
    maximum
}

fn collect_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    if is_callable_boundary(node) {
        return;
    }
    match node.kind() {
        "if_statement" => return collect_if_control_depth(node, active_depth, maximum),
        "for_statement" | "while_statement" => {
            return collect_loop_control_depth(node, active_depth, maximum);
        }
        "try_statement" => return collect_try_control_depth(node, active_depth, maximum),
        "except_clause" => return collect_except_control_depth(node, active_depth, maximum),
        "conditional_expression" => {
            return collect_ternary_control_depth(node, active_depth, maximum);
        }
        _ => {}
    }
    collect_child_control_depth(node, active_depth, maximum);
}

fn collect_loop_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    let region_depth = active_depth + 1;
    *maximum = (*maximum).max(region_depth);
    collect_body_and_else_depth(node, active_depth, region_depth, maximum);
}

fn collect_except_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    let region_depth = active_depth + 1;
    *maximum = (*maximum).max(region_depth);
    let body = first_block(node).map(|body| body.id());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let depth = if Some(child.id()) == body {
            region_depth
        } else {
            active_depth
        };
        collect_control_depth(child, depth, maximum);
    }
}

fn collect_ternary_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    let children = named_children(node);
    if children.len() != 3 {
        return collect_child_control_depth(node, active_depth, maximum);
    }
    let region_depth = active_depth + 1;
    *maximum = (*maximum).max(region_depth);
    collect_control_depth(children[1], active_depth, maximum);
    collect_control_depth(children[0], region_depth, maximum);
    collect_control_depth(children[2], region_depth, maximum);
}

fn collect_child_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_control_depth(child, active_depth, maximum);
    }
}

fn collect_if_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    let region_depth = active_depth + 1;
    *maximum = (*maximum).max(region_depth);
    collect_field_depth(node, "condition", active_depth, maximum);
    collect_field_depth(node, "consequence", region_depth, maximum);
    let mut cursor = node.walk();
    for alternative in node.children_by_field_name("alternative", &mut cursor) {
        match alternative.kind() {
            "elif_clause" => {
                collect_field_depth(alternative, "condition", active_depth, maximum);
                collect_field_depth(alternative, "consequence", region_depth, maximum);
            }
            "else_clause" => collect_field_depth(alternative, "body", region_depth, maximum),
            _ => collect_control_depth(alternative, active_depth, maximum),
        }
    }
}

fn collect_body_and_else_depth(
    node: Node<'_>,
    active_depth: usize,
    region_depth: usize,
    maximum: &mut usize,
) {
    let body_id = node.child_by_field_name("body").map(|body| body.id());
    let alternative_id = node
        .child_by_field_name("alternative")
        .map(|alternative| alternative.id());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let depth = if Some(child.id()) == body_id || Some(child.id()) == alternative_id {
            region_depth
        } else {
            active_depth
        };
        collect_control_depth(child, depth, maximum);
    }
}

fn collect_try_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "else_clause" {
            let region_depth = active_depth + 1;
            *maximum = (*maximum).max(region_depth);
            collect_field_depth(child, "body", region_depth, maximum);
        } else {
            collect_control_depth(child, active_depth, maximum);
        }
    }
}

fn collect_field_depth(node: Node<'_>, field: &str, depth: usize, maximum: &mut usize) {
    if let Some(child) = node.child_by_field_name(field) {
        collect_control_depth(child, depth, maximum);
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
    if is_callable_boundary(node) {
        return;
    }
    if node.kind() == "if_statement" {
        collect_if_condition_records(node, positions, records);
        return;
    }
    if let Some((kind, condition)) = condition_for_node(node) {
        add_condition_record(kind, condition, positions, records);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_condition_records(child, positions, records);
    }
}

fn condition_for_node(node: Node<'_>) -> Option<(&'static str, Node<'_>)> {
    match node.kind() {
        "while_statement" => Some(("while", node.child_by_field_name("condition")?)),
        "conditional_expression" => Some(("ternary", named_children(node).get(1).copied()?)),
        "case_clause" => {
            let guard = node.child_by_field_name("guard")?;
            Some(("case_guard", first_named_child(guard)?))
        }
        _ => None,
    }
}

fn collect_if_condition_records(
    node: Node<'_>,
    positions: &LineIndex<'_>,
    records: &mut Vec<ConditionRecord>,
) {
    add_field_condition(node, "if", "condition", positions, records);
    collect_field_conditions(node, "condition", positions, records);
    collect_field_conditions(node, "consequence", positions, records);

    let mut cursor = node.walk();
    for alternative in node.children_by_field_name("alternative", &mut cursor) {
        if alternative.kind() == "elif_clause" {
            add_field_condition(alternative, "elif", "condition", positions, records);
            collect_field_conditions(alternative, "condition", positions, records);
            collect_field_conditions(alternative, "consequence", positions, records);
        } else {
            collect_condition_records(alternative, positions, records);
        }
    }
}

fn add_field_condition(
    node: Node<'_>,
    kind: &str,
    field: &str,
    positions: &LineIndex<'_>,
    records: &mut Vec<ConditionRecord>,
) {
    if let Some(condition) = node.child_by_field_name(field) {
        add_condition_record(kind, condition, positions, records);
    }
}

fn collect_field_conditions(
    node: Node<'_>,
    field: &str,
    positions: &LineIndex<'_>,
    records: &mut Vec<ConditionRecord>,
) {
    if let Some(child) = node.child_by_field_name(field) {
        collect_condition_records(child, positions, records);
    }
}

fn add_condition_record(
    kind: &str,
    condition: Node<'_>,
    positions: &LineIndex<'_>,
    records: &mut Vec<ConditionRecord>,
) {
    let (operator_count, binary_operator_count) = count_boolean_operators(condition);
    records.push(ConditionRecord {
        kind: kind.to_string(),
        location: positions.position(condition.start_byte()),
        operator_count,
        predicate_count: binary_operator_count + 1,
        max_boolean_depth: boolean_depth(condition, None),
    });
}

fn count_boolean_operators(node: Node<'_>) -> (usize, usize) {
    if is_boolean_boundary(node) {
        return (0, 0);
    }
    let is_binary = boolean_operator(node).is_some();
    let is_not = not_argument(node).is_some();
    let mut operators = usize::from(is_binary) + usize::from(is_not);
    let mut binaries = usize::from(is_binary);
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let (child_operators, child_binaries) = count_boolean_operators(child);
        operators += child_operators;
        binaries += child_binaries;
    }
    (operators, binaries)
}

fn boolean_depth(node: Node<'_>, parent_operator: Option<&str>) -> usize {
    if is_boolean_boundary(node) {
        return 0;
    }
    if let Some(inner) = parenthesized_inner_expression(node) {
        return boolean_depth(inner, parent_operator);
    }
    if let Some(argument) = not_argument(node) {
        return 1 + boolean_depth(argument, None);
    }
    if let Some((operator, _)) = boolean_operator(node) {
        let layer = usize::from(parent_operator != Some(operator));
        let left = node
            .child_by_field_name("left")
            .map_or(0, |child| boolean_depth(child, Some(operator)));
        let right = node
            .child_by_field_name("right")
            .map_or(0, |child| boolean_depth(child, Some(operator)));
        return layer + left.max(right);
    }
    let mut maximum = 0;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        maximum = maximum.max(boolean_depth(child, None));
    }
    maximum
}

fn is_boolean_boundary(node: Node<'_>) -> bool {
    function_kind(node).is_some() || node.kind() == "conditional_expression"
}

fn boolean_operator(node: Node<'_>) -> Option<(&'static str, Node<'_>)> {
    if node.kind() != "boolean_operator" {
        return None;
    }
    let point = node.child_by_field_name("operator")?;
    match point.kind() {
        "and" => Some(("and", point)),
        "or" => Some(("or", point)),
        _ => None,
    }
}

fn not_argument(node: Node<'_>) -> Option<Node<'_>> {
    (node.kind() == "not_operator")
        .then(|| node.child_by_field_name("argument"))
        .flatten()
}

fn parenthesized_inner_expression(node: Node<'_>) -> Option<Node<'_>> {
    (node.kind() == "parenthesized_expression")
        .then(|| first_named_child(node))
        .flatten()
}

fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn first_named_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).next()
}

fn first_block(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == "block")
}

fn token_after<'tree>(
    node: Node<'tree>,
    start: usize,
    end: usize,
    token: &str,
) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|child| {
        child.start_byte() >= start && child.end_byte() <= end && child.kind() == token
    })
}

fn syntax_error(node: Node<'_>) -> Option<Node<'_>> {
    let error = first_syntax_error(node);
    if node.has_error() || error.is_some() {
        Some(error.unwrap_or(node))
    } else {
        None
    }
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
        language: "python".to_string(),
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
