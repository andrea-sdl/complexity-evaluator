use std::{
    collections::HashSet,
    env,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use oxc::{
    allocator::Allocator,
    ast::ast::{
        ArrowFunctionExpression, Expression, FormalParameters, Function, FunctionBody,
        MethodDefinition, MethodDefinitionKind, ObjectProperty, PropertyKey, PropertyKind,
        VariableDeclarator,
    },
    ast_visit::{Visit, walk},
    diagnostics::Diagnostics,
    parser::Parser,
    span::{GetSpan, SourceType, Span},
    syntax::scope::ScopeFlags,
};

use crate::model::{
    ConditionRecord, Contribution, Diagnostic, FileResult, FileSignals, FileStatus, FunctionResult,
    FunctionSignals, Position, SourceRange,
};

const MAX_RISKY_OPENING_DELIMITERS: usize = 2_048;
const MAX_RISKY_QUESTION_MARKS: usize = 2_048;
const PROBE_CHILD_ENV: &str = "COMPLEXITY_JS_PROBE_CHILD";

pub(crate) fn analyze_source(path: &str, source: &str, max_complexity: u32) -> FileResult {
    let source_type = source_type_for(path);
    let language = language_for(path);
    if needs_safety_probe(source)
        && env::var_os(PROBE_CHILD_ENV).is_none()
        && let Err(message) = run_safety_probe(path, source, max_complexity)
    {
        return probe_error_result(path, language, message);
    }
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();

    if parsed.panicked || !parsed.diagnostics.is_empty() {
        return parse_error_result(path, source, language, parsed.diagnostics);
    }

    let mut collector = FunctionCollector::new(source, path, max_complexity);
    collector.visit_program(&parsed.program);
    let functions = collector.finish();
    let function_count = functions.len();
    FileResult {
        path: path.to_string(),
        language,
        status: FileStatus::Ok,
        signals: Some(FileSignals { function_count }),
        functions,
        diagnostics: Vec::new(),
    }
}

fn needs_safety_probe(source: &str) -> bool {
    let mut opening_delimiters = 0usize;
    let mut question_marks = 0usize;

    for byte in source.bytes() {
        opening_delimiters += usize::from(matches!(byte, b'(' | b'[' | b'{'));
        question_marks += usize::from(byte == b'?');
    }

    opening_delimiters > MAX_RISKY_OPENING_DELIMITERS || question_marks > MAX_RISKY_QUESTION_MARKS
}

fn run_safety_probe(path: &str, source: &str, max_complexity: u32) -> Result<(), String> {
    let executable = env::current_exe()
        .map_err(|error| format!("cannot locate the analysis executable: {error}"))?;
    let language = safety_probe_language(path);
    let max_complexity = max_complexity.to_string();
    let mut child = Command::new(executable)
        .args(["--format", "json"])
        .args(["--language", language])
        .args(["--max-complexity", &max_complexity])
        .args(["--stdin-filename", path, "-"])
        .env(PROBE_CHILD_ENV, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot start the analysis probe: {error}"))?;

    let write_result = child
        .stdin
        .as_mut()
        .ok_or_else(|| "analysis probe has no standard input".to_string())
        .and_then(|stdin| {
            stdin
                .write_all(source.as_bytes())
                .map_err(|error| format!("cannot write source to the analysis probe: {error}"))
        });
    drop(child.stdin.take());
    if let Err(message) = write_result {
        let _ = child.wait();
        return Err(message);
    }

    let status = child
        .wait()
        .map_err(|error| format!("cannot wait for the analysis probe: {error}"))?;
    match status.code() {
        Some(0..=2) => Ok(()),
        Some(code) => Err(format!(
            "analysis probe exited unexpectedly with status {code}"
        )),
        None => Err("analysis probe did not complete normally".to_string()),
    }
}

fn safety_probe_language(path: &str) -> &str {
    match language_for(path).as_str() {
        "typescript" | "tsx" => "typescript",
        _ => "javascript",
    }
}

fn probe_error_result(path: &str, language: String, message: String) -> FileResult {
    FileResult {
        path: path.to_string(),
        language,
        status: FileStatus::ParseError,
        signals: None,
        functions: Vec::new(),
        diagnostics: vec![Diagnostic {
            location: Position { line: 1, column: 1 },
            message,
        }],
    }
}

fn parse_error_result(
    path: &str,
    source: &str,
    language: String,
    parser_diagnostics: Diagnostics,
) -> FileResult {
    let positions = SourcePositions::new(source);
    let mut diagnostics = parser_diagnostics
        .into_iter()
        .map(|diagnostic| {
            let offset = diagnostic
                .labels
                .iter()
                .find(|label| label.primary())
                .or_else(|| diagnostic.labels.first())
                .map_or(0, |label| label.offset());
            Diagnostic {
                location: positions.position(offset),
                message: diagnostic.to_string(),
            }
        })
        .collect::<Vec<_>>();
    if diagnostics.is_empty() {
        diagnostics.push(Diagnostic {
            location: Position { line: 1, column: 1 },
            message: "parser stopped before completing the file".to_string(),
        });
    }
    diagnostics.sort_by(|left, right| {
        (left.location.line, left.location.column, &left.message).cmp(&(
            right.location.line,
            right.location.column,
            &right.message,
        ))
    });
    FileResult {
        path: path.to_string(),
        language,
        status: FileStatus::ParseError,
        signals: None,
        functions: Vec::new(),
        diagnostics,
    }
}

struct FunctionCollector<'source> {
    source: &'source str,
    positions: SourcePositions<'source>,
    path: &'source str,
    max_complexity: u32,
    skipped_functions: Vec<Span>,
    skipped_arrows: Vec<Span>,
    functions: Vec<FunctionResult>,
}

impl<'source> FunctionCollector<'source> {
    fn new(source: &'source str, path: &'source str, max_complexity: u32) -> Self {
        Self {
            source,
            positions: SourcePositions::new(source),
            path,
            max_complexity,
            skipped_functions: Vec::new(),
            skipped_arrows: Vec::new(),
            functions: Vec::new(),
        }
    }

    fn finish(mut self) -> Vec<FunctionResult> {
        let functions = &mut self.functions;
        functions.sort_by(|left, right| {
            (left.range.start.line, left.range.start.column)
                .cmp(&(right.range.start.line, right.range.start.column))
                .then_with(|| {
                    (left.range.end.line, left.range.end.column)
                        .cmp(&(right.range.end.line, right.range.end.column))
                })
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.id.cmp(&right.id))
        });
        self.functions
    }

    fn skip_function(&mut self, span: Span) {
        self.skipped_functions.push(span);
    }

    fn skip_arrow(&mut self, span: Span) {
        self.skipped_arrows.push(span);
    }

    fn take_skipped_function(&mut self, span: Span) -> bool {
        take_span(&mut self.skipped_functions, span)
    }

    fn take_skipped_arrow(&mut self, span: Span) -> bool {
        take_span(&mut self.skipped_arrows, span)
    }

    fn push_function(&mut self, function: &Function<'_>, name: String, kind: &str, span: Span) {
        let Some(body) = function.body.as_deref() else {
            return;
        };
        let context = ScoringContext {
            source: self.source,
            positions: &self.positions,
            path: self.path,
            max_complexity: self.max_complexity,
        };
        self.functions.push(score_callable(
            body,
            &function.params,
            span,
            name,
            kind,
            &context,
        ));
    }

    fn push_arrow(&mut self, arrow: &ArrowFunctionExpression<'_>, name: String) {
        let context = ScoringContext {
            source: self.source,
            positions: &self.positions,
            path: self.path,
            max_complexity: self.max_complexity,
        };
        self.functions.push(score_callable(
            &arrow.body,
            &arrow.params,
            arrow.span,
            name,
            "arrow",
            &context,
        ));
    }
}

impl<'ast, 'source> Visit<'ast> for FunctionCollector<'source> {
    fn visit_function(&mut self, function: &Function<'ast>, flags: ScopeFlags) {
        if !self.take_skipped_function(function.span) {
            let name = function.id.as_ref().map_or_else(
                || "<anonymous>".to_string(),
                |identifier| identifier.name.to_string(),
            );
            self.push_function(function, name, "function", function.span);
        }
        walk::walk_function(self, function, flags);
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'ast>) {
        if !self.take_skipped_arrow(arrow.span) {
            self.push_arrow(arrow, "<anonymous>".to_string());
        }
        walk::walk_arrow_function_expression(self, arrow);
    }

    fn visit_logical_expression(&mut self, expression: &oxc::ast::ast::LogicalExpression<'ast>) {
        visit_logical_children_without_recursion(self, expression);
    }

    fn visit_unary_expression(&mut self, expression: &oxc::ast::ast::UnaryExpression<'ast>) {
        visit_unary_child_without_recursion(self, expression);
    }

    fn visit_parenthesized_expression(
        &mut self,
        expression: &oxc::ast::ast::ParenthesizedExpression<'ast>,
    ) {
        visit_parenthesized_child_without_recursion(self, expression);
    }

    fn visit_conditional_expression(
        &mut self,
        expression: &oxc::ast::ast::ConditionalExpression<'ast>,
    ) {
        visit_conditional_children_without_recursion(self, expression);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'ast>) {
        let name = declarator.id.get_identifier_name().map_or_else(
            || "<anonymous>".to_string(),
            |identifier| identifier.to_string(),
        );
        match declarator.init.as_ref() {
            Some(Expression::FunctionExpression(function)) => {
                self.skip_function(function.span);
                let name = function
                    .id
                    .as_ref()
                    .map_or_else(|| name, |identifier| identifier.name.to_string());
                self.push_function(function, name, "function", function.span);
            }
            Some(Expression::ArrowFunctionExpression(arrow)) => {
                self.skip_arrow(arrow.span);
                self.push_arrow(arrow, name);
            }
            _ => {}
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_method_definition(&mut self, definition: &MethodDefinition<'ast>) {
        self.skip_function(definition.value.span);
        self.push_function(
            &definition.value,
            property_key_name(&definition.key),
            method_kind(definition.kind),
            definition.span,
        );
        walk::walk_method_definition(self, definition);
    }

    fn visit_object_property(&mut self, property: &ObjectProperty<'ast>) {
        let name = property_key_name(&property.key);
        match &property.value {
            Expression::FunctionExpression(function) => {
                self.skip_function(function.span);
                let kind = object_property_kind(property);
                let name = function
                    .id
                    .as_ref()
                    .map_or(name, |identifier| identifier.name.to_string());
                self.push_function(function, name, kind, property.span);
            }
            Expression::ArrowFunctionExpression(arrow) => {
                self.skip_arrow(arrow.span);
                self.push_arrow(arrow, name);
            }
            _ => {}
        }
        walk::walk_object_property(self, property);
    }
}

fn take_span(spans: &mut Vec<Span>, span: Span) -> bool {
    let Some(index) = spans.iter().position(|candidate| *candidate == span) else {
        return false;
    };
    spans.swap_remove(index);
    true
}

fn property_key_name(key: &PropertyKey<'_>) -> String {
    match key {
        PropertyKey::StaticIdentifier(identifier) => identifier.name.to_string(),
        PropertyKey::PrivateIdentifier(identifier) => format!("#{}", identifier.name),
        _ => "<anonymous>".to_string(),
    }
}

fn method_kind(kind: MethodDefinitionKind) -> &'static str {
    match kind {
        MethodDefinitionKind::Constructor => "constructor",
        MethodDefinitionKind::Method => "method",
        MethodDefinitionKind::Get => "getter",
        MethodDefinitionKind::Set => "setter",
    }
}

fn object_property_kind(property: &ObjectProperty<'_>) -> &'static str {
    match property.kind {
        PropertyKind::Get => "getter",
        PropertyKind::Set => "setter",
        PropertyKind::Init if property.method => "method",
        PropertyKind::Init => "function",
    }
}

fn score_callable(
    body: &FunctionBody<'_>,
    parameters: &FormalParameters<'_>,
    span: Span,
    name: String,
    kind: &str,
    context: &ScoringContext<'_>,
) -> FunctionResult {
    let mut scorer = Scorer::new(context.source, context.positions);
    scorer.visit_formal_parameters(parameters);
    scorer.visit_function_body(body);
    let mut contributions = scorer.contributions;

    contributions.sort_by(|left, right| {
        (left.location.line, left.location.column)
            .cmp(&(right.location.line, right.location.column))
            .then_with(|| left.rule.cmp(&right.rule))
    });

    let score = contributions
        .iter()
        .map(|contribution| contribution.increment)
        .sum();
    let start = context.positions.position(span.start);
    let end = context.positions.position(span.end);
    let mut signals = FunctionSignals::empty();
    signals.line_span = end.line - start.line + 1;
    let mut signal_collector = SignalCollector::new(context.positions);
    signal_collector.visit_formal_parameters(parameters);
    signal_collector.visit_function_body(body);
    signals.max_control_depth = signal_collector.max_control_depth;
    signal_collector.finish(&mut signals);
    FunctionResult {
        id: format!("{}:{}:{}", context.path, start.line, start.column),
        name,
        kind: kind.to_string(),
        range: SourceRange { start, end },
        score,
        over_limit: score > context.max_complexity,
        contributions,
        signals,
    }
}

struct SignalCollector<'source> {
    positions: &'source SourcePositions<'source>,
    control_depth: usize,
    max_control_depth: usize,
    conditions: Vec<ConditionRecord>,
}

impl<'source> SignalCollector<'source> {
    fn new(positions: &'source SourcePositions<'source>) -> Self {
        Self {
            positions,
            control_depth: 0,
            max_control_depth: 0,
            conditions: Vec::new(),
        }
    }

    fn record_condition(&mut self, kind: &str, expression: &Expression<'_>) {
        let shape = boolean_shape(expression);
        self.conditions.push(ConditionRecord {
            kind: kind.to_string(),
            location: self.positions.position(expression.span().start),
            operator_count: shape.operator_count,
            predicate_count: shape.predicate_count,
            max_boolean_depth: shape.max_depth,
        });
    }

    fn finish(mut self, signals: &mut FunctionSignals) {
        self.conditions.sort_by(|left, right| {
            (left.location.line, left.location.column, &left.kind).cmp(&(
                right.location.line,
                right.location.column,
                &right.kind,
            ))
        });
        signals.condition_count = self.conditions.len();
        signals.max_condition_operators = self
            .conditions
            .iter()
            .map(|condition| condition.operator_count)
            .max()
            .unwrap_or(0);
        signals.max_condition_predicates = self
            .conditions
            .iter()
            .map(|condition| condition.predicate_count)
            .max()
            .unwrap_or(0);
        signals.max_boolean_depth = self
            .conditions
            .iter()
            .map(|condition| condition.max_boolean_depth)
            .max()
            .unwrap_or(0);
        signals.conditions = self.conditions;
    }

    fn visit_if_branch<'ast>(&mut self, statement: &oxc::ast::ast::IfStatement<'ast>, kind: &str) {
        self.record_condition(kind, &statement.test);
        self.visit_expression(&statement.test);
        self.visit_control_statement(&statement.consequent);
        let Some(alternate) = &statement.alternate else {
            return;
        };
        if let oxc::ast::ast::Statement::IfStatement(else_if) = alternate {
            self.visit_if_branch(else_if, "else_if");
        } else {
            self.visit_control_statement(alternate);
        }
    }

    fn visit_control_statement<'ast>(&mut self, statement: &oxc::ast::ast::Statement<'ast>) {
        self.enter_control();
        self.visit_statement(statement);
        self.leave_control();
    }

    fn enter_control(&mut self) {
        self.control_depth += 1;
        self.max_control_depth = self.max_control_depth.max(self.control_depth);
    }

    fn leave_control(&mut self) {
        self.control_depth -= 1;
    }
}

struct BooleanShape {
    operator_count: usize,
    predicate_count: usize,
    max_depth: usize,
}

fn boolean_shape(expression: &Expression<'_>) -> BooleanShape {
    let mut collector = BooleanShapeCollector::new();
    collector.visit_expression(expression);
    collector.finish()
}

struct BooleanShapeCollector {
    operator_count: usize,
    binary_operator_count: usize,
    active_depth: usize,
    max_depth: usize,
    parent_operator: Option<oxc::syntax::operator::LogicalOperator>,
}

enum BooleanShapeItem<'ast> {
    Expression(
        &'ast Expression<'ast>,
        Option<oxc::syntax::operator::LogicalOperator>,
        usize,
    ),
    Logical(
        &'ast oxc::ast::ast::LogicalExpression<'ast>,
        Option<oxc::syntax::operator::LogicalOperator>,
        usize,
    ),
}

impl BooleanShapeCollector {
    fn new() -> Self {
        Self {
            operator_count: 0,
            binary_operator_count: 0,
            active_depth: 0,
            max_depth: 0,
            parent_operator: None,
        }
    }

    fn enter_operator(&mut self) {
        self.active_depth += 1;
        self.max_depth = self.max_depth.max(self.active_depth);
    }

    fn finish(self) -> BooleanShape {
        BooleanShape {
            operator_count: self.operator_count,
            predicate_count: self.binary_operator_count + 1,
            max_depth: self.max_depth,
        }
    }

    fn visit_logical_stack<'ast>(
        &mut self,
        expression: &'ast oxc::ast::ast::LogicalExpression<'ast>,
    ) {
        let mut items = vec![BooleanShapeItem::Logical(
            expression,
            self.parent_operator,
            self.active_depth,
        )];
        while let Some(item) = items.pop() {
            match item {
                BooleanShapeItem::Expression(expression, parent_operator, active_depth) => {
                    self.visit_boolean_expression(
                        expression,
                        parent_operator,
                        active_depth,
                        &mut items,
                    );
                }
                BooleanShapeItem::Logical(expression, parent_operator, active_depth) => {
                    self.visit_boolean_logical(
                        expression,
                        parent_operator,
                        active_depth,
                        &mut items,
                    );
                }
            }
        }
    }

    fn visit_boolean_expression<'ast>(
        &mut self,
        expression: &'ast Expression<'ast>,
        parent_operator: Option<oxc::syntax::operator::LogicalOperator>,
        active_depth: usize,
        items: &mut Vec<BooleanShapeItem<'ast>>,
    ) {
        if let Expression::LogicalExpression(expression) = expression {
            items.push(BooleanShapeItem::Logical(
                expression,
                parent_operator,
                active_depth,
            ));
            return;
        }
        if let Expression::ParenthesizedExpression(expression) = expression {
            items.push(BooleanShapeItem::Expression(
                &expression.expression,
                parent_operator,
                active_depth,
            ));
            return;
        }

        let previous_operator = std::mem::replace(&mut self.parent_operator, parent_operator);
        let previous_depth = std::mem::replace(&mut self.active_depth, active_depth);
        self.visit_expression(expression);
        self.parent_operator = previous_operator;
        self.active_depth = previous_depth;
    }

    fn visit_boolean_logical<'ast>(
        &mut self,
        expression: &'ast oxc::ast::ast::LogicalExpression<'ast>,
        parent_operator: Option<oxc::syntax::operator::LogicalOperator>,
        active_depth: usize,
        items: &mut Vec<BooleanShapeItem<'ast>>,
    ) {
        self.operator_count += 1;
        self.binary_operator_count += 1;
        let adds_depth = parent_operator != Some(expression.operator);
        let child_depth = active_depth + usize::from(adds_depth);
        self.max_depth = self.max_depth.max(child_depth);
        items.push(BooleanShapeItem::Expression(
            &expression.right,
            Some(expression.operator),
            child_depth,
        ));
        items.push(BooleanShapeItem::Expression(
            &expression.left,
            Some(expression.operator),
            child_depth,
        ));
    }
}

impl<'ast> Visit<'ast> for BooleanShapeCollector {
    fn visit_expression(&mut self, expression: &Expression<'ast>) {
        let previous_operator = self.parent_operator;
        if !matches!(
            expression.get_inner_expression(),
            Expression::LogicalExpression(_)
        ) {
            self.parent_operator = None;
        }
        walk::walk_expression(self, expression);
        self.parent_operator = previous_operator;
    }

    fn visit_logical_expression(&mut self, expression: &oxc::ast::ast::LogicalExpression<'ast>) {
        self.visit_logical_stack(expression);
    }

    fn visit_parenthesized_expression(
        &mut self,
        expression: &oxc::ast::ast::ParenthesizedExpression<'ast>,
    ) {
        visit_parenthesized_child_without_recursion(self, expression);
    }

    fn visit_unary_expression(&mut self, expression: &oxc::ast::ast::UnaryExpression<'ast>) {
        if !expression.operator.is_not() {
            walk::walk_unary_expression(self, expression);
            return;
        }
        let previous_operator = self.parent_operator.take();
        let previous_depth = self.active_depth;
        let mut expression = expression;
        loop {
            self.operator_count += 1;
            self.enter_operator();
            let argument = &expression.argument;
            let argument = unparenthesized_expression(argument);
            let Expression::UnaryExpression(next) = argument else {
                self.visit_expression(argument);
                break;
            };
            if !next.operator.is_not() {
                self.visit_expression(argument);
                break;
            }
            expression = next;
        }
        self.parent_operator = previous_operator;
        self.active_depth = previous_depth;
    }

    fn visit_conditional_expression(&mut self, _: &oxc::ast::ast::ConditionalExpression<'ast>) {}

    fn visit_function(&mut self, _: &Function<'ast>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'ast>) {}
}

impl<'ast, 'source> Visit<'ast> for SignalCollector<'source> {
    fn visit_function(&mut self, _: &Function<'ast>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'ast>) {}

    fn visit_logical_expression(&mut self, expression: &oxc::ast::ast::LogicalExpression<'ast>) {
        visit_logical_children_without_recursion(self, expression);
    }

    fn visit_unary_expression(&mut self, expression: &oxc::ast::ast::UnaryExpression<'ast>) {
        visit_unary_child_without_recursion(self, expression);
    }

    fn visit_parenthesized_expression(
        &mut self,
        expression: &oxc::ast::ast::ParenthesizedExpression<'ast>,
    ) {
        visit_parenthesized_child_without_recursion(self, expression);
    }

    fn visit_if_statement(&mut self, statement: &oxc::ast::ast::IfStatement<'ast>) {
        self.visit_if_branch(statement, "if");
    }

    fn visit_do_while_statement(&mut self, statement: &oxc::ast::ast::DoWhileStatement<'ast>) {
        self.visit_control_statement(&statement.body);
        self.record_condition("do_while", &statement.test);
        self.visit_expression(&statement.test);
    }

    fn visit_while_statement(&mut self, statement: &oxc::ast::ast::WhileStatement<'ast>) {
        self.record_condition("while", &statement.test);
        self.visit_expression(&statement.test);
        self.visit_control_statement(&statement.body);
    }

    fn visit_for_statement(&mut self, statement: &oxc::ast::ast::ForStatement<'ast>) {
        if let Some(init) = &statement.init {
            self.visit_for_statement_init(init);
        }
        if let Some(test) = &statement.test {
            self.record_condition("for", test);
            self.visit_expression(test);
        }
        if let Some(update) = &statement.update {
            self.visit_expression(update);
        }
        self.visit_control_statement(&statement.body);
    }

    fn visit_for_in_statement(&mut self, statement: &oxc::ast::ast::ForInStatement<'ast>) {
        self.visit_for_statement_left(&statement.left);
        self.visit_expression(&statement.right);
        self.visit_control_statement(&statement.body);
    }

    fn visit_for_of_statement(&mut self, statement: &oxc::ast::ast::ForOfStatement<'ast>) {
        self.visit_for_statement_left(&statement.left);
        self.visit_expression(&statement.right);
        self.visit_control_statement(&statement.body);
    }

    fn visit_switch_statement(&mut self, statement: &oxc::ast::ast::SwitchStatement<'ast>) {
        self.visit_expression(&statement.discriminant);
        self.enter_control();
        for case in &statement.cases {
            self.visit_switch_case(case);
        }
        self.leave_control();
    }

    fn visit_try_statement(&mut self, statement: &oxc::ast::ast::TryStatement<'ast>) {
        self.visit_block_statement(&statement.block);
        if let Some(handler) = &statement.handler {
            self.enter_control();
            self.visit_block_statement(&handler.body);
            self.leave_control();
        }
        if let Some(finalizer) = &statement.finalizer {
            self.visit_block_statement(finalizer);
        }
    }

    fn visit_conditional_expression(
        &mut self,
        expression: &oxc::ast::ast::ConditionalExpression<'ast>,
    ) {
        enum Item<'ast> {
            Expression(&'ast Expression<'ast>),
            Conditional(&'ast oxc::ast::ast::ConditionalExpression<'ast>),
            EnterControl,
            LeaveControl,
        }

        let mut items = vec![Item::Conditional(expression)];
        while let Some(item) = items.pop() {
            match item {
                Item::Expression(expression) => match unparenthesized_expression(expression) {
                    Expression::ConditionalExpression(expression) => {
                        items.push(Item::Conditional(expression));
                    }
                    expression => self.visit_expression(expression),
                },
                Item::Conditional(expression) => {
                    self.record_condition("ternary", &expression.test);
                    self.visit_expression(&expression.test);
                    items.push(Item::LeaveControl);
                    items.push(Item::Expression(&expression.alternate));
                    items.push(Item::EnterControl);
                    items.push(Item::LeaveControl);
                    items.push(Item::Expression(&expression.consequent));
                    items.push(Item::EnterControl);
                }
                Item::EnterControl => self.enter_control(),
                Item::LeaveControl => self.leave_control(),
            }
        }
    }
}

struct ScoringContext<'source> {
    source: &'source str,
    positions: &'source SourcePositions<'source>,
    path: &'source str,
    max_complexity: u32,
}

struct Scorer<'source> {
    source: &'source str,
    positions: &'source SourcePositions<'source>,
    nesting: u32,
    contributions: Vec<Contribution>,
    logical_depth: u32,
    suppressed_logical_roots: HashSet<Span>,
}

impl<'source> Scorer<'source> {
    fn new(source: &'source str, positions: &'source SourcePositions<'source>) -> Self {
        Self {
            source,
            positions,
            nesting: 0,
            contributions: Vec::new(),
            logical_depth: 0,
            suppressed_logical_roots: HashSet::new(),
        }
    }

    fn add(&mut self, rule: &str, offset: u32, nesting: u32) {
        self.contributions.push(Contribution {
            rule: rule.to_string(),
            location: self.positions.position(offset),
            base_increment: 1,
            nesting_increment: nesting,
            increment: 1 + nesting,
        });
    }

    fn token_after(&self, start: u32, end: u32, token: &str) -> u32 {
        let start = usize::try_from(start).expect("Oxc span fits usize");
        let end = usize::try_from(end).expect("Oxc span fits usize");
        let mut offset = start;
        while offset < end {
            let remainder = self.source[offset..end].trim_start();
            offset = end - remainder.len();
            if remainder.starts_with(token) {
                return u32::try_from(offset).expect("source offset fits u32");
            }
            offset += match remainder {
                "" => panic!("expected Oxc syntax token before node end"),
                remainder if remainder.starts_with("//") => {
                    remainder.find('\n').unwrap_or(remainder.len())
                }
                remainder if remainder.starts_with("/*") => {
                    remainder
                        .find("*/")
                        .expect("Oxc comments are terminated before the following token")
                        + 2
                }
                _ => panic!("expected Oxc syntax token after child span"),
            };
        }
        panic!("expected Oxc syntax token before node end");
    }

    fn visit_nested_statement<'ast>(&mut self, statement: &oxc::ast::ast::Statement<'ast>) {
        self.nesting += 1;
        self.visit_statement(statement);
        self.nesting -= 1;
    }

    fn visit_else_if<'ast>(
        &mut self,
        statement: &oxc::ast::ast::IfStatement<'ast>,
        else_start: u32,
    ) {
        self.add("else_if", else_start, 0);
        self.visit_expression(&statement.test);
        self.visit_nested_statement(&statement.consequent);
        let Some(alternate) = &statement.alternate else {
            return;
        };
        let token = self.token_after(
            statement.consequent.span().end,
            alternate.span().start,
            "else",
        );
        if let oxc::ast::ast::Statement::IfStatement(next) = alternate {
            self.visit_else_if(next, token);
            return;
        }
        self.add("else", token, 0);
        self.visit_nested_statement(alternate);
    }

    fn visit_conditional_chain<'ast>(
        &mut self,
        expression: &'ast oxc::ast::ast::ConditionalExpression<'ast>,
    ) {
        enum Item<'ast> {
            Expression(&'ast Expression<'ast>),
            Conditional(&'ast oxc::ast::ast::ConditionalExpression<'ast>),
            LeaveNesting,
        }

        let mut items = vec![Item::Conditional(expression)];
        while let Some(item) = items.pop() {
            match item {
                Item::Expression(expression) => match unparenthesized_expression(expression) {
                    Expression::ConditionalExpression(expression) => {
                        items.push(Item::Conditional(expression));
                    }
                    expression => self.visit_expression(expression),
                },
                Item::Conditional(expression) => {
                    let token = self.token_after(
                        expression.test.span().end,
                        expression.consequent.span().start,
                        "?",
                    );
                    self.add("ternary", token, self.nesting);
                    self.visit_expression(&expression.test);
                    self.nesting += 1;
                    items.push(Item::LeaveNesting);
                    items.push(Item::Expression(&expression.alternate));
                    items.push(Item::Expression(&expression.consequent));
                }
                Item::LeaveNesting => self.nesting -= 1,
            }
        }
    }
}

impl<'ast, 'source> Visit<'ast> for Scorer<'source> {
    fn visit_function(&mut self, _: &Function<'ast>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'ast>) {}

    fn visit_expression(&mut self, expression: &Expression<'ast>) {
        let stays_in_logical_chain = matches!(
            expression,
            Expression::LogicalExpression(_) | Expression::ParenthesizedExpression(_)
        );
        let logical_depth = self.logical_depth;
        if !stays_in_logical_chain {
            self.logical_depth = 0;
        }
        walk::walk_expression(self, expression);
        self.logical_depth = logical_depth;
    }

    fn visit_if_statement(&mut self, statement: &oxc::ast::ast::IfStatement<'ast>) {
        self.add("if", statement.span.start, self.nesting);
        self.visit_expression(&statement.test);
        self.visit_nested_statement(&statement.consequent);
        let Some(alternate) = &statement.alternate else {
            return;
        };
        let token = self.token_after(
            statement.consequent.span().end,
            alternate.span().start,
            "else",
        );
        if let oxc::ast::ast::Statement::IfStatement(else_if) = alternate {
            self.visit_else_if(else_if, token);
            return;
        }
        self.add("else", token, 0);
        self.visit_nested_statement(alternate);
    }

    fn visit_do_while_statement(&mut self, statement: &oxc::ast::ast::DoWhileStatement<'ast>) {
        self.add("loop", statement.span.start, self.nesting);
        self.visit_nested_statement(&statement.body);
        self.visit_expression(&statement.test);
    }

    fn visit_while_statement(&mut self, statement: &oxc::ast::ast::WhileStatement<'ast>) {
        self.add("loop", statement.span.start, self.nesting);
        self.visit_expression(&statement.test);
        self.visit_nested_statement(&statement.body);
    }

    fn visit_for_statement(&mut self, statement: &oxc::ast::ast::ForStatement<'ast>) {
        self.add("loop", statement.span.start, self.nesting);
        if let Some(init) = &statement.init {
            self.visit_for_statement_init(init);
        }
        if let Some(test) = &statement.test {
            self.visit_expression(test);
        }
        if let Some(update) = &statement.update {
            self.visit_expression(update);
        }
        self.visit_nested_statement(&statement.body);
    }

    fn visit_for_in_statement(&mut self, statement: &oxc::ast::ast::ForInStatement<'ast>) {
        self.add("loop", statement.span.start, self.nesting);
        self.visit_for_statement_left(&statement.left);
        self.visit_expression(&statement.right);
        self.visit_nested_statement(&statement.body);
    }

    fn visit_for_of_statement(&mut self, statement: &oxc::ast::ast::ForOfStatement<'ast>) {
        self.add("loop", statement.span.start, self.nesting);
        self.visit_for_statement_left(&statement.left);
        self.visit_expression(&statement.right);
        self.visit_nested_statement(&statement.body);
    }

    fn visit_switch_statement(&mut self, statement: &oxc::ast::ast::SwitchStatement<'ast>) {
        self.add("switch", statement.span.start, self.nesting);
        self.visit_expression(&statement.discriminant);
        self.nesting += 1;
        for case in &statement.cases {
            self.visit_switch_case(case);
        }
        self.nesting -= 1;
    }

    fn visit_try_statement(&mut self, statement: &oxc::ast::ast::TryStatement<'ast>) {
        self.visit_block_statement(&statement.block);
        if let Some(handler) = &statement.handler {
            self.add("catch", handler.span.start, self.nesting);
            self.nesting += 1;
            self.visit_block_statement(&handler.body);
            self.nesting -= 1;
        }
        if let Some(finalizer) = &statement.finalizer {
            self.visit_block_statement(finalizer);
        }
    }

    fn visit_break_statement(&mut self, statement: &oxc::ast::ast::BreakStatement<'ast>) {
        if statement.label.is_some() {
            self.add("labeled_jump", statement.span.start, 0);
        }
    }

    fn visit_continue_statement(&mut self, statement: &oxc::ast::ast::ContinueStatement<'ast>) {
        if statement.label.is_some() {
            self.add("labeled_jump", statement.span.start, 0);
        }
    }

    fn visit_conditional_expression(
        &mut self,
        expression: &oxc::ast::ast::ConditionalExpression<'ast>,
    ) {
        self.visit_conditional_chain(expression);
    }

    fn visit_logical_expression(&mut self, expression: &oxc::ast::ast::LogicalExpression<'ast>) {
        let mut operators = Vec::new();
        let suppresses_chain = self.suppressed_logical_roots.contains(&expression.span);
        if self.logical_depth == 0 && !suppresses_chain {
            collect_logical_operators(expression, &mut operators);
        }
        let mut previous_was_and = false;
        for operator in operators {
            let is_and = operator.operator.is_and();
            if is_and && !previous_was_and {
                let token =
                    self.token_after(operator.left.span().end, operator.right.span().start, "&&");
                self.add("logical_and", token, 0);
            }
            previous_was_and = is_and;
        }
        self.logical_depth += 1;
        visit_logical_children_without_recursion(self, expression);
        self.logical_depth -= 1;
    }

    fn visit_unary_expression(&mut self, expression: &oxc::ast::ast::UnaryExpression<'ast>) {
        visit_unary_child_without_recursion(self, expression);
    }

    fn visit_parenthesized_expression(
        &mut self,
        expression: &oxc::ast::ast::ParenthesizedExpression<'ast>,
    ) {
        visit_parenthesized_child_without_recursion(self, expression);
    }

    fn visit_jsx_expression_container(
        &mut self,
        container: &oxc::ast::ast::JSXExpressionContainer<'ast>,
    ) {
        let Some(expression) = container.expression.as_expression() else {
            return;
        };
        let root = homogeneous_jsx_logical_root(expression);
        let inserted = root.is_some_and(|span| self.suppressed_logical_roots.insert(span));
        self.visit_expression(expression);
        if inserted {
            self.suppressed_logical_roots
                .remove(&root.expect("inserted logical root is present"));
        }
    }
}

fn homogeneous_jsx_logical_root(expression: &Expression<'_>) -> Option<Span> {
    let expression = unparenthesized_expression(expression);
    let Expression::LogicalExpression(logical) = expression else {
        return None;
    };
    is_homogeneous_jsx_logical_chain(expression, logical.operator).then_some(logical.span)
}

fn is_homogeneous_jsx_logical_chain(
    expression: &Expression<'_>,
    operator: oxc::syntax::operator::LogicalOperator,
) -> bool {
    let mut expressions = vec![expression];
    while let Some(expression) = expressions.pop() {
        let Expression::LogicalExpression(expression) = unparenthesized_expression(expression)
        else {
            continue;
        };
        if expression.operator != operator {
            return false;
        }
        expressions.push(&expression.right);
        expressions.push(&expression.left);
    }
    true
}

fn unparenthesized_expression<'ast>(expression: &'ast Expression<'ast>) -> &'ast Expression<'ast> {
    let mut expression = expression;
    while let Expression::ParenthesizedExpression(parenthesized) = expression {
        expression = &parenthesized.expression;
    }
    expression
}

fn collect_logical_operators<'ast>(
    expression: &'ast oxc::ast::ast::LogicalExpression<'ast>,
    operators: &mut Vec<&'ast oxc::ast::ast::LogicalExpression<'ast>>,
) {
    enum Item<'ast> {
        Expression(&'ast Expression<'ast>),
        Operator(&'ast oxc::ast::ast::LogicalExpression<'ast>),
    }

    let mut items = vec![
        Item::Expression(&expression.right),
        Item::Operator(expression),
        Item::Expression(&expression.left),
    ];
    while let Some(item) = items.pop() {
        match item {
            Item::Expression(expression) => match expression {
                Expression::LogicalExpression(expression) => {
                    items.push(Item::Expression(&expression.right));
                    items.push(Item::Operator(expression));
                    items.push(Item::Expression(&expression.left));
                }
                Expression::ParenthesizedExpression(expression) => {
                    items.push(Item::Expression(&expression.expression));
                }
                _ => {}
            },
            Item::Operator(expression) => operators.push(expression),
        }
    }
}

fn visit_logical_children_without_recursion<'ast, Visitor: Visit<'ast>>(
    visitor: &mut Visitor,
    expression: &'ast oxc::ast::ast::LogicalExpression<'ast>,
) {
    let mut expressions = vec![&expression.right, &expression.left];
    while let Some(expression) = expressions.pop() {
        match expression {
            Expression::LogicalExpression(expression) => {
                expressions.push(&expression.right);
                expressions.push(&expression.left);
            }
            Expression::ParenthesizedExpression(expression) => {
                expressions.push(&expression.expression);
            }
            _ => visitor.visit_expression(expression),
        }
    }
}

fn visit_unary_child_without_recursion<'ast, Visitor: Visit<'ast>>(
    visitor: &mut Visitor,
    expression: &'ast oxc::ast::ast::UnaryExpression<'ast>,
) {
    let mut child = &expression.argument;
    loop {
        match child {
            Expression::UnaryExpression(unary) => child = &unary.argument,
            Expression::ParenthesizedExpression(parenthesized) => child = &parenthesized.expression,
            _ => {
                visitor.visit_expression(child);
                return;
            }
        }
    }
}

fn visit_parenthesized_child_without_recursion<'ast, Visitor: Visit<'ast>>(
    visitor: &mut Visitor,
    expression: &'ast oxc::ast::ast::ParenthesizedExpression<'ast>,
) {
    let mut child = &expression.expression;
    while let Expression::ParenthesizedExpression(parenthesized) = child {
        child = &parenthesized.expression;
    }
    visitor.visit_expression(child);
}

fn visit_conditional_children_without_recursion<'ast, Visitor: Visit<'ast>>(
    visitor: &mut Visitor,
    expression: &'ast oxc::ast::ast::ConditionalExpression<'ast>,
) {
    let mut expressions = vec![
        &expression.alternate,
        &expression.consequent,
        &expression.test,
    ];
    while let Some(expression) = expressions.pop() {
        match unparenthesized_expression(expression) {
            Expression::ConditionalExpression(expression) => {
                expressions.push(&expression.alternate);
                expressions.push(&expression.consequent);
                expressions.push(&expression.test);
            }
            expression => visitor.visit_expression(expression),
        }
    }
}

const COLUMN_CHECKPOINT_INTERVAL: usize = 128;

struct ColumnCheckpoint {
    line_index: usize,
    offset: u32,
    column: u32,
}

struct SourcePositions<'source> {
    source: &'source str,
    line_starts: Vec<usize>,
    column_checkpoints: Vec<ColumnCheckpoint>,
}

impl<'source> SourcePositions<'source> {
    fn new(source: &'source str) -> Self {
        let mut line_starts = vec![0];
        let mut column_checkpoints = Vec::new();
        let mut line_index = 0;
        let mut column = 1_u32;
        let mut characters_since_checkpoint = 0;

        for (offset, character) in source.char_indices() {
            if character == '\n' {
                let line_start = offset + 1;
                line_starts.push(line_start);
                line_index += 1;
                column = 1;
                characters_since_checkpoint = 0;
                continue;
            }

            column += 1;
            characters_since_checkpoint += 1;
            if characters_since_checkpoint == COLUMN_CHECKPOINT_INTERVAL {
                column_checkpoints.push(ColumnCheckpoint {
                    line_index,
                    offset: u32::try_from(offset + character.len_utf8())
                        .expect("source offset fits u32"),
                    column,
                });
                characters_since_checkpoint = 0;
            }
        }
        Self {
            source,
            line_starts,
            column_checkpoints,
        }
    }

    fn position(&self, offset: u32) -> Position {
        let offset = usize::try_from(offset).expect("Oxc span fits usize");
        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let checkpoint_index = self.column_checkpoints.partition_point(|checkpoint| {
            usize::try_from(checkpoint.offset).expect("u32 fits usize") <= offset
        });
        let checkpoint = checkpoint_index
            .checked_sub(1)
            .and_then(|index| self.column_checkpoints.get(index))
            .filter(|checkpoint| checkpoint.line_index == line_index);
        let (checkpoint_offset, checkpoint_column) =
            checkpoint.map_or((self.line_starts[line_index], 1), |checkpoint| {
                (
                    usize::try_from(checkpoint.offset).expect("u32 fits usize"),
                    usize::try_from(checkpoint.column).expect("u32 fits usize"),
                )
            });
        Position {
            line: line_index + 1,
            column: checkpoint_column + self.source[checkpoint_offset..offset].chars().count(),
        }
    }
}

fn language_for(path: &str) -> String {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("jsx") => "jsx",
        Some("tsx") => "tsx",
        Some("ts" | "mts" | "cts") => "typescript",
        _ => "javascript",
    }
    .to_string()
}

fn source_type_for(path: &str) -> SourceType {
    let source_type = SourceType::from_path(Path::new(path)).unwrap_or_default();
    if source_type.is_javascript() {
        source_type.with_jsx(true)
    } else {
        source_type
    }
}
