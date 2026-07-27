use tree_sitter::{Node, Parser, Tree};

use crate::model::{
    ConditionRecord, Contribution, Diagnostic, FileResult, FileSignals, FileStatus, FunctionResult,
    FunctionSignals, Position, SourceRange,
};

pub(crate) fn analyze_source(path: &str, source: &str, max_complexity: u32) -> FileResult {
    let mut parser = Parser::new();
    let language = tree_sitter_php::LANGUAGE_PHP.into();
    if let Err(error) = parser.set_language(&language) {
        return failed_file(
            path,
            format!("cannot load PHP grammar: {error}"),
            Position { line: 1, column: 1 },
        );
    }

    let Some(mut tree) = parser.parse(source, None) else {
        return failed_file(
            path,
            "parser returned no syntax tree".to_string(),
            Position { line: 1, column: 1 },
        );
    };

    if let Some(reparsed_tree) = reparse_reserved_class_constants(&mut parser, &tree, source) {
        tree = reparsed_tree;
    }

    let root = tree.root_node();
    let positions = LineIndex::new(source);
    if root.has_error() {
        let error = first_syntax_error(root).unwrap_or(root);
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
    functions.sort_by(|left, right| {
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
    });

    let function_count = functions.len();
    FileResult {
        path: path.to_string(),
        language: "php".to_string(),
        status: FileStatus::Ok,
        signals: Some(FileSignals { function_count }),
        functions,
        diagnostics: Vec::new(),
    }
}

fn collect_functions(
    node: Node<'_>,
    source: &str,
    positions: &LineIndex<'_>,
    path: &str,
    max_complexity: u32,
    functions: &mut Vec<FunctionResult>,
) {
    if let Some(kind) = function_kind(node)
        && let Some(body) = node.child_by_field_name("body")
    {
        let start = positions.position(node.start_byte());
        let end = positions.position(node.end_byte());
        let name = node
            .child_by_field_name("name")
            .and_then(|name| name.utf8_text(source.as_bytes()).ok())
            .unwrap_or("<anonymous>")
            .to_string();
        let mut contributions = Vec::new();
        score_node(body, 0, positions, &mut contributions);
        contributions.sort_by(|left, right| {
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
        });
        let score = contributions
            .iter()
            .map(|contribution| contribution.increment)
            .sum();
        let mut signals = FunctionSignals::empty();
        signals.line_span = end.line - start.line + 1;
        signals.max_control_depth = max_control_depth(body);
        signals.conditions = condition_records(body, positions);
        signals.condition_count = signals.conditions.len();
        signals.max_condition_operators = signals
            .conditions
            .iter()
            .map(|condition| condition.operator_count)
            .max()
            .unwrap_or(0);
        signals.max_condition_predicates = signals
            .conditions
            .iter()
            .map(|condition| condition.predicate_count)
            .max()
            .unwrap_or(0);
        signals.max_boolean_depth = signals
            .conditions
            .iter()
            .map(|condition| condition.max_boolean_depth)
            .max()
            .unwrap_or(0);

        functions.push(FunctionResult {
            id: format!("{path}:{}:{}", start.line, start.column),
            name,
            kind: kind.to_string(),
            range: SourceRange { start, end },
            score,
            over_limit: score > max_complexity,
            contributions,
            signals,
        });
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_functions(child, source, positions, path, max_complexity, functions);
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
    if function_kind(node).is_some() {
        return;
    }

    if node.kind() == "if_statement" {
        collect_if_condition_records(node, "if", positions, records);
        return;
    }

    let condition_kind = match node.kind() {
        "while_statement" => Some(("while", true)),
        "do_statement" => Some(("do_while", true)),
        "for_statement" => Some(("for", false)),
        _ => None,
    };
    if let Some((kind, strip_required_parentheses)) = condition_kind
        && let Some(condition) = node.child_by_field_name("condition")
    {
        add_condition_record(
            kind,
            condition,
            strip_required_parentheses,
            positions,
            records,
        );
    }

    if node.kind() == "conditional_expression"
        && let Some(condition) = node.child_by_field_name("condition")
    {
        add_condition_record("ternary", condition, false, positions, records);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_condition_records(child, positions, records);
    }
}

fn collect_if_condition_records(
    node: Node<'_>,
    kind: &str,
    positions: &LineIndex<'_>,
    records: &mut Vec<ConditionRecord>,
) {
    if let Some(condition) = node.child_by_field_name("condition") {
        add_condition_record(kind, condition, true, positions, records);
        collect_condition_records(condition, positions, records);
    }
    if let Some(body) = node.child_by_field_name("body") {
        collect_condition_records(body, positions, records);
    }

    let mut alternatives = node.walk();
    for alternative in node.children_by_field_name("alternative", &mut alternatives) {
        if alternative.kind() == "else_if_clause" {
            if let Some(condition) = alternative.child_by_field_name("condition") {
                add_condition_record("elseif", condition, true, positions, records);
                collect_condition_records(condition, positions, records);
            }
            if let Some(body) = alternative.child_by_field_name("body") {
                collect_condition_records(body, positions, records);
            }
            continue;
        }

        let Some(body) = alternative.child_by_field_name("body") else {
            continue;
        };
        if body.kind() == "if_statement" {
            collect_if_condition_records(body, "else_if", positions, records);
        } else {
            collect_condition_records(body, positions, records);
        }
    }
}

fn add_condition_record(
    kind: &str,
    condition: Node<'_>,
    strip_required_parentheses: bool,
    positions: &LineIndex<'_>,
    records: &mut Vec<ConditionRecord>,
) {
    let expression = if strip_required_parentheses {
        parenthesized_inner_expression(condition).unwrap_or(condition)
    } else {
        condition
    };
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
    if function_kind(node).is_some() || node.kind() == "conditional_expression" {
        return (0, 0);
    }

    let is_binary_operator = boolean_binary_operator(node).is_some();
    let is_not = boolean_not_argument(node).is_some();
    let mut operator_count = usize::from(is_binary_operator) + usize::from(is_not);
    let mut binary_operator_count = usize::from(is_binary_operator);
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let (child_operators, child_binary_operators) = count_boolean_operators(child);
        operator_count += child_operators;
        binary_operator_count += child_binary_operators;
    }
    (operator_count, binary_operator_count)
}

fn boolean_depth(node: Node<'_>, parent_operator: Option<&str>) -> usize {
    if function_kind(node).is_some() {
        return 0;
    }
    if let Some(expression) = parenthesized_inner_expression(node) {
        return boolean_depth(expression, parent_operator);
    }
    if node.kind() == "conditional_expression" {
        return 0;
    }

    if let Some(argument) = boolean_not_argument(node) {
        return 1 + boolean_depth(argument, None);
    }

    if let Some(operator) = boolean_binary_operator(node) {
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

fn boolean_binary_operator(node: Node<'_>) -> Option<&str> {
    if node.kind() != "binary_expression" {
        return None;
    }
    let operator = node.child_by_field_name("operator")?;
    match operator.kind() {
        "&&" | "||" | "??" | "and" | "or" | "xor" => Some(operator.kind()),
        _ => None,
    }
}

fn boolean_not_argument(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "unary_op_expression" {
        return None;
    }
    if node.child_by_field_name("operator")?.kind() != "!" {
        return None;
    }
    node.child_by_field_name("argument")
}

fn max_control_depth(body: Node<'_>) -> usize {
    let mut maximum = 0;
    collect_control_depth(body, 0, &mut maximum);
    maximum
}

fn collect_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    if function_kind(node).is_some() {
        return;
    }

    match node.kind() {
        "if_statement" => {
            collect_if_control_depth(node, active_depth, maximum);
            return;
        }
        "for_statement" | "foreach_statement" | "while_statement" | "do_statement" => {
            let region_depth = active_depth + 1;
            *maximum = (*maximum).max(region_depth);
            let mut body_cursor = node.walk();
            let body_ids = node
                .children_by_field_name("body", &mut body_cursor)
                .map(|body| body.id())
                .collect::<Vec<_>>();
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                let child_depth = if body_ids.contains(&child.id()) {
                    region_depth
                } else {
                    active_depth
                };
                collect_control_depth(child, child_depth, maximum);
            }
            return;
        }
        "switch_statement" => {
            let region_depth = active_depth + 1;
            *maximum = (*maximum).max(region_depth);
            if let Some(condition) = node.child_by_field_name("condition") {
                collect_control_depth(condition, active_depth, maximum);
            }
            if let Some(body) = node.child_by_field_name("body") {
                collect_control_depth(body, region_depth, maximum);
            }
            return;
        }
        "catch_clause" => {
            let region_depth = active_depth + 1;
            *maximum = (*maximum).max(region_depth);
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
            return;
        }
        "conditional_expression" => {
            let region_depth = active_depth + 1;
            *maximum = (*maximum).max(region_depth);
            if let Some(condition) = node.child_by_field_name("condition") {
                collect_control_depth(condition, active_depth, maximum);
            }
            if let Some(body) = node.child_by_field_name("body") {
                collect_control_depth(body, region_depth, maximum);
            }
            if let Some(alternative) = node.child_by_field_name("alternative") {
                collect_control_depth(alternative, region_depth, maximum);
            }
            return;
        }
        "match_expression" => {
            let region_depth = active_depth + 1;
            *maximum = (*maximum).max(region_depth);
            if let Some(condition) = node.child_by_field_name("condition") {
                collect_control_depth(condition, active_depth, maximum);
            }
            if let Some(body) = node.child_by_field_name("body") {
                let mut branch_cursor = body.walk();
                for branch in body.named_children(&mut branch_cursor) {
                    let result_id = branch
                        .child_by_field_name("return_expression")
                        .map(|result| result.id());
                    let mut child_cursor = branch.walk();
                    for child in branch.named_children(&mut child_cursor) {
                        let child_depth = if Some(child.id()) == result_id {
                            region_depth
                        } else {
                            active_depth
                        };
                        collect_control_depth(child, child_depth, maximum);
                    }
                }
            }
            return;
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_control_depth(child, active_depth, maximum);
    }
}

fn collect_if_control_depth(node: Node<'_>, active_depth: usize, maximum: &mut usize) {
    let region_depth = active_depth + 1;
    *maximum = (*maximum).max(region_depth);

    if let Some(condition) = node.child_by_field_name("condition") {
        collect_control_depth(condition, active_depth, maximum);
    }
    if let Some(body) = node.child_by_field_name("body") {
        collect_control_depth(body, region_depth, maximum);
    }

    let mut alternatives = node.walk();
    for alternative in node.children_by_field_name("alternative", &mut alternatives) {
        if alternative.kind() == "else_if_clause" {
            if let Some(condition) = alternative.child_by_field_name("condition") {
                collect_control_depth(condition, active_depth, maximum);
            }
            if let Some(body) = alternative.child_by_field_name("body") {
                collect_control_depth(body, region_depth, maximum);
            }
            continue;
        }

        if let Some(body) = alternative.child_by_field_name("body") {
            collect_control_depth(body, region_depth, maximum);
        }
    }
}

fn score_node(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    if function_kind(node).is_some() {
        return;
    }

    match node.kind() {
        "if_statement" => {
            score_if(node, nesting, false, positions, contributions);
            return;
        }
        "for_statement" | "foreach_statement" | "while_statement" | "do_statement" => {
            score_loop(node, nesting, positions, contributions);
            return;
        }
        "switch_statement" => {
            score_switch(node, nesting, positions, contributions);
            return;
        }
        "catch_clause" => {
            score_catch(node, nesting, positions, contributions);
            return;
        }
        "conditional_expression" => {
            score_ternary(node, nesting, positions, contributions);
            return;
        }
        "match_expression" => {
            score_match(node, nesting, positions, contributions);
            return;
        }
        "break_statement" | "continue_statement" => {
            score_numbered_jump(node, nesting, positions, contributions);
            return;
        }
        "goto_statement" => {
            add_flat_contribution("goto", node, positions, contributions);
            return;
        }
        _ => {}
    }

    if node.kind() == "binary_expression" && flow_operator(node).is_some() {
        score_flow_sequence(node, nesting, positions, contributions);
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        score_node(child, nesting, positions, contributions);
    }
}

fn score_match(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("match", node, nesting, positions, contributions);
    if let Some(condition) = node.child_by_field_name("condition") {
        score_node(condition, nesting, positions, contributions);
    }
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };

    let mut branch_cursor = body.walk();
    for branch in body.named_children(&mut branch_cursor) {
        let result_id = branch
            .child_by_field_name("return_expression")
            .map(|result| result.id());
        let mut child_cursor = branch.walk();
        for child in branch.named_children(&mut child_cursor) {
            let child_nesting = if Some(child.id()) == result_id {
                nesting + 1
            } else {
                nesting
            };
            score_node(child, child_nesting, positions, contributions);
        }
    }
}

fn score_numbered_jump(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    let mut argument_cursor = node.walk();
    let has_argument = node
        .named_children(&mut argument_cursor)
        .any(|child| !child.is_extra());
    if has_argument {
        add_flat_contribution("numbered_jump", node, positions, contributions);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        score_node(child, nesting, positions, contributions);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FlowOperator {
    And,
    Or,
    Pipe,
}

impl FlowOperator {
    fn rule(self) -> &'static str {
        match self {
            Self::And | Self::Or => "logical_sequence",
            Self::Pipe => "pipe",
        }
    }
}

fn score_flow_sequence(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    let root_operator = flow_operator(node).expect("called for a flow operator").0;
    let mut operators = Vec::new();
    collect_flow_operators(node, &mut operators);

    match root_operator {
        FlowOperator::And | FlowOperator::Or => {
            let mut previous = None;
            for (operator, point) in &operators {
                if previous != Some(*operator) {
                    add_flat_contribution(operator.rule(), *point, positions, contributions);
                }
                previous = Some(*operator);
            }
        }
        FlowOperator::Pipe => {
            let mut previous = None;
            for (index, (operator, point)) in operators.iter().enumerate() {
                if index == 0 || previous == Some(*operator) {
                    add_contribution(operator.rule(), *point, nesting, positions, contributions);
                }
                previous = Some(*operator);
            }
        }
    }

    score_flow_operands(node, nesting, positions, contributions);
}

fn collect_flow_operators<'tree>(
    node: Node<'tree>,
    operators: &mut Vec<(FlowOperator, Node<'tree>)>,
) {
    if let Some(expression) = parenthesized_inner_expression(node) {
        collect_flow_operators(expression, operators);
        return;
    }

    let Some((operator, point)) = flow_operator(node) else {
        return;
    };
    if let Some(left) = node.child_by_field_name("left") {
        collect_flow_operators(left, operators);
    }
    operators.push((operator, point));
    if let Some(right) = node.child_by_field_name("right") {
        collect_flow_operators(right, operators);
    }
}

fn flow_operator(node: Node<'_>) -> Option<(FlowOperator, Node<'_>)> {
    if node.kind() != "binary_expression" {
        return None;
    }
    let point = node.child_by_field_name("operator")?;
    let operator = match point.kind() {
        "&&" => FlowOperator::And,
        "||" => FlowOperator::Or,
        "|>" => FlowOperator::Pipe,
        _ => return None,
    };
    Some((operator, point))
}

fn score_flow_operands(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    if let Some(expression) = parenthesized_inner_expression(node) {
        score_flow_operands(expression, nesting, positions, contributions);
        return;
    }

    if flow_operator(node).is_none() {
        score_node(node, nesting, positions, contributions);
        return;
    }

    if let Some(left) = node.child_by_field_name("left") {
        score_flow_operands(left, nesting, positions, contributions);
    }
    if let Some(right) = node.child_by_field_name("right") {
        score_flow_operands(right, nesting, positions, contributions);
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

fn score_ternary(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    let point = child_with_kind(node, "?").unwrap_or(node);
    add_contribution("ternary", point, nesting, positions, contributions);

    if let Some(condition) = node.child_by_field_name("condition") {
        score_node(condition, nesting, positions, contributions);
    }
    if let Some(body) = node.child_by_field_name("body") {
        score_node(body, nesting + 1, positions, contributions);
    }
    if let Some(alternative) = node.child_by_field_name("alternative") {
        score_node(alternative, nesting + 1, positions, contributions);
    }
}

fn child_with_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn score_catch(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("catch", node, nesting, positions, contributions);
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

fn score_switch(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("switch", node, nesting, positions, contributions);

    if let Some(condition) = node.child_by_field_name("condition") {
        score_node(condition, nesting + 1, positions, contributions);
    }
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };

    let mut cursor = body.walk();
    for branch in body.named_children(&mut cursor) {
        score_switch_branch(branch, nesting, positions, contributions);
    }
}

fn score_switch_branch(
    branch: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    let mut cursor = branch.walk();
    for child in branch.named_children(&mut cursor) {
        score_node(child, nesting + 1, positions, contributions);
    }
}

fn score_loop(
    node: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    add_contribution("loop", node, nesting, positions, contributions);

    let mut body_cursor = node.walk();
    let body_ids = node
        .children_by_field_name("body", &mut body_cursor)
        .map(|body| body.id())
        .collect::<Vec<_>>();

    let mut child_cursor = node.walk();
    for child in node.named_children(&mut child_cursor) {
        let child_nesting = if body_ids.contains(&child.id()) {
            nesting + 1
        } else {
            nesting
        };
        score_node(child, child_nesting, positions, contributions);
    }
}

fn score_if(
    node: Node<'_>,
    nesting: u32,
    flat_else_if: bool,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    if flat_else_if {
        add_flat_contribution("else_if", node, positions, contributions);
    } else {
        add_contribution("if", node, nesting, positions, contributions);
    }

    if let Some(condition) = node.child_by_field_name("condition") {
        score_node(condition, nesting, positions, contributions);
    }
    if let Some(body) = node.child_by_field_name("body") {
        score_node(body, nesting + 1, positions, contributions);
    }

    let mut alternatives = node.walk();
    for alternative in node.children_by_field_name("alternative", &mut alternatives) {
        score_if_alternative(alternative, nesting, positions, contributions);
    }
}

fn score_if_alternative(
    alternative: Node<'_>,
    nesting: u32,
    positions: &LineIndex<'_>,
    contributions: &mut Vec<Contribution>,
) {
    if alternative.kind() == "else_if_clause" {
        add_flat_contribution("elseif", alternative, positions, contributions);
        if let Some(condition) = alternative.child_by_field_name("condition") {
            score_node(condition, nesting, positions, contributions);
        }
        if let Some(body) = alternative.child_by_field_name("body") {
            score_node(body, nesting + 1, positions, contributions);
        }
        return;
    }

    let Some(body) = alternative.child_by_field_name("body") else {
        return;
    };
    if body.kind() == "if_statement" {
        score_if(body, nesting + 1, true, positions, contributions);
        return;
    }

    add_flat_contribution("else", alternative, positions, contributions);
    score_node(body, nesting + 1, positions, contributions);
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

fn function_kind(node: Node<'_>) -> Option<&'static str> {
    match node.kind() {
        "function_definition" => Some("function"),
        "method_declaration" => Some("method"),
        "anonymous_function" => Some("closure"),
        "arrow_function" => Some("arrow"),
        _ => None,
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

fn reparse_reserved_class_constants(
    parser: &mut Parser,
    tree: &Tree,
    source: &str,
) -> Option<Tree> {
    if !tree.root_node().has_error() {
        return None;
    }

    let masked_source = mask_reserved_class_constant_names(tree.root_node(), source)?;
    let reparsed_tree = parser.parse(masked_source.as_bytes(), None)?;
    if reparsed_tree.root_node().has_error() {
        return None;
    }

    Some(reparsed_tree)
}

fn mask_reserved_class_constant_names(root: Node<'_>, source: &str) -> Option<String> {
    let mut name_starts = Vec::new();
    collect_reserved_class_constant_names(root, source.as_bytes(), &mut name_starts);
    name_starts.sort_unstable();
    name_starts.dedup();

    if name_starts.is_empty() {
        return None;
    }

    let mut masked = source.as_bytes().to_vec();
    for start in name_starts {
        masked[start] = b'_';
    }

    String::from_utf8(masked).ok()
}

fn collect_reserved_class_constant_names(
    node: Node<'_>,
    source: &[u8],
    name_starts: &mut Vec<usize>,
) {
    if node.is_error()
        && is_at_class_member_scope(node)
        && let Some(start) = reserved_class_constant_name_start(node, source)
    {
        name_starts.push(start);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.has_error() {
            collect_reserved_class_constant_names(child, source, name_starts);
        }
    }
}

fn is_at_class_member_scope(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "declaration_list" | "enum_declaration_list" => return true,
            "compound_statement" => return false,
            _ => {}
        }
        node = parent;
    }

    false
}

fn reserved_class_constant_name_start(node: Node<'_>, source: &[u8]) -> Option<usize> {
    const RESERVED_NAMES: &[&[u8]] = &[
        b"array",
        b"bool",
        b"callable",
        b"false",
        b"float",
        b"int",
        b"iterable",
        b"mixed",
        b"namespace",
        b"null",
        b"object",
        b"string",
        b"true",
        b"void",
    ];

    let mut saw_const = false;
    let mut candidate = None;
    let mut comment_ranges = Vec::new();
    collect_comment_ranges(node, &mut comment_ranges);
    comment_ranges.sort_unstable();
    let mut next_comment = 0;
    let mut index = node.start_byte();

    while index < node.end_byte() {
        if next_comment < comment_ranges.len() && index == comment_ranges[next_comment].0 {
            index = comment_ranges[next_comment].1;
            next_comment += 1;
            continue;
        }

        let byte = source[index];
        if byte == b'=' {
            let (start, end) = candidate?;
            let name = &source[start..end];
            let is_known_reserved_name = RESERVED_NAMES
                .iter()
                .any(|reserved| name.eq_ignore_ascii_case(reserved));
            if saw_const && is_known_reserved_name {
                return Some(start);
            }
            return None;
        }

        if saw_const && matches!(byte, b';' | b'{' | b'}') {
            return None;
        }

        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = index;
            index += 1;
            while index < node.end_byte()
                && (source[index].is_ascii_alphanumeric() || source[index] == b'_')
            {
                index += 1;
            }

            let identifier = &source[start..index];
            if identifier.eq_ignore_ascii_case(b"const") {
                saw_const = true;
                candidate = None;
            } else if saw_const {
                candidate = Some((start, index));
            }
            continue;
        }

        index += 1;
    }

    None
}

fn collect_comment_ranges(node: Node<'_>, ranges: &mut Vec<(usize, usize)>) {
    if node.kind() == "comment" {
        ranges.push((node.start_byte(), node.end_byte()));
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_comment_ranges(child, ranges);
    }
}

fn failed_file(path: &str, message: String, location: Position) -> FileResult {
    FileResult {
        path: path.to_string(),
        language: "php".to_string(),
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
