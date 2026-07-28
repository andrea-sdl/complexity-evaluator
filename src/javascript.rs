use std::path::Path;

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

pub(crate) fn analyze_source(path: &str, source: &str, max_complexity: u32) -> FileResult {
    let source_type = source_type_for(path);
    let language = language_for(path);
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

    fn leave_operator(&mut self) {
        self.active_depth -= 1;
    }

    fn finish(self) -> BooleanShape {
        BooleanShape {
            operator_count: self.operator_count,
            predicate_count: self.binary_operator_count + 1,
            max_depth: self.max_depth,
        }
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
        self.operator_count += 1;
        self.binary_operator_count += 1;
        let adds_depth = self.parent_operator != Some(expression.operator);
        if adds_depth {
            self.enter_operator();
        }
        let previous_operator = self.parent_operator.replace(expression.operator);
        self.visit_expression(&expression.left);
        self.visit_expression(&expression.right);
        self.parent_operator = previous_operator;
        if adds_depth {
            self.leave_operator();
        }
    }

    fn visit_unary_expression(&mut self, expression: &oxc::ast::ast::UnaryExpression<'ast>) {
        if !expression.operator.is_not() {
            walk::walk_unary_expression(self, expression);
            return;
        }
        self.operator_count += 1;
        self.enter_operator();
        let previous_operator = self.parent_operator.take();
        self.visit_expression(&expression.argument);
        self.parent_operator = previous_operator;
        self.leave_operator();
    }

    fn visit_conditional_expression(&mut self, _: &oxc::ast::ast::ConditionalExpression<'ast>) {}

    fn visit_function(&mut self, _: &Function<'ast>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'ast>) {}
}

impl<'ast, 'source> Visit<'ast> for SignalCollector<'source> {
    fn visit_function(&mut self, _: &Function<'ast>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'ast>) {}

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
        self.record_condition("ternary", &expression.test);
        self.visit_expression(&expression.test);
        self.enter_control();
        self.visit_expression(&expression.consequent);
        self.leave_control();
        self.enter_control();
        self.visit_expression(&expression.alternate);
        self.leave_control();
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
    suppressed_logical_spans: Vec<Span>,
}

impl<'source> Scorer<'source> {
    fn new(source: &'source str, positions: &'source SourcePositions<'source>) -> Self {
        Self {
            source,
            positions,
            nesting: 0,
            contributions: Vec::new(),
            logical_depth: 0,
            suppressed_logical_spans: Vec::new(),
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
        let token = self.token_after(
            expression.test.span().end,
            expression.consequent.span().start,
            "?",
        );
        self.add("ternary", token, self.nesting);
        self.visit_expression(&expression.test);
        self.nesting += 1;
        self.visit_expression(&expression.consequent);
        self.visit_expression(&expression.alternate);
        self.nesting -= 1;
    }

    fn visit_logical_expression(&mut self, expression: &oxc::ast::ast::LogicalExpression<'ast>) {
        let mut operators = Vec::new();
        if self.logical_depth == 0 {
            collect_logical_operators(expression, &mut operators);
        }
        let mut previous_was_and = false;
        for operator in operators {
            let is_and = operator.operator.is_and();
            if is_and
                && !previous_was_and
                && !self.suppressed_logical_spans.contains(&operator.span)
            {
                let token =
                    self.token_after(operator.left.span().end, operator.right.span().start, "&&");
                self.add("logical_and", token, 0);
            }
            previous_was_and = is_and;
        }
        self.logical_depth += 1;
        self.visit_expression(&expression.left);
        self.visit_expression(&expression.right);
        self.logical_depth -= 1;
    }

    fn visit_jsx_expression_container(
        &mut self,
        container: &oxc::ast::ast::JSXExpressionContainer<'ast>,
    ) {
        let Some(expression) = container.expression.as_expression() else {
            return;
        };
        let mut suppressed = Vec::new();
        let suppresses_chain = is_homogeneous_jsx_logical_chain(expression, &mut suppressed);
        if suppresses_chain {
            self.suppressed_logical_spans
                .extend(suppressed.iter().copied());
        }
        self.visit_expression(expression);
        if suppresses_chain {
            for span in suppressed {
                let index = self
                    .suppressed_logical_spans
                    .iter()
                    .rposition(|candidate| *candidate == span)
                    .expect("suppressed span was added");
                self.suppressed_logical_spans.remove(index);
            }
        }
    }
}

fn is_homogeneous_jsx_logical_chain(expression: &Expression<'_>, spans: &mut Vec<Span>) -> bool {
    let expression = unparenthesized_expression(expression);
    let Expression::LogicalExpression(logical) = expression else {
        return false;
    };
    collect_homogeneous_jsx_logical_spans(expression, logical.operator, spans)
}

fn collect_homogeneous_jsx_logical_spans(
    expression: &Expression<'_>,
    operator: oxc::syntax::operator::LogicalOperator,
    spans: &mut Vec<Span>,
) -> bool {
    match unparenthesized_expression(expression) {
        Expression::LogicalExpression(expression) => {
            if expression.operator != operator {
                return false;
            }
            spans.push(expression.span);
            collect_homogeneous_jsx_logical_spans(&expression.left, operator, spans)
                && collect_homogeneous_jsx_logical_spans(&expression.right, operator, spans)
        }
        _ => true,
    }
}

fn unparenthesized_expression<'ast>(expression: &'ast Expression<'ast>) -> &'ast Expression<'ast> {
    match expression {
        Expression::ParenthesizedExpression(expression) => {
            unparenthesized_expression(&expression.expression)
        }
        _ => expression,
    }
}

fn collect_logical_operators<'ast>(
    expression: &'ast oxc::ast::ast::LogicalExpression<'ast>,
    operators: &mut Vec<&'ast oxc::ast::ast::LogicalExpression<'ast>>,
) {
    collect_logical_operators_in_expression(&expression.left, operators);
    operators.push(expression);
    collect_logical_operators_in_expression(&expression.right, operators);
}

fn collect_logical_operators_in_expression<'ast>(
    expression: &'ast Expression<'ast>,
    operators: &mut Vec<&'ast oxc::ast::ast::LogicalExpression<'ast>>,
) {
    match expression {
        Expression::LogicalExpression(expression) => {
            collect_logical_operators(expression, operators);
        }
        Expression::ParenthesizedExpression(expression) => {
            collect_logical_operators_in_expression(&expression.expression, operators);
        }
        _ => {}
    }
}

struct SourcePositions<'source> {
    source: &'source str,
    line_starts: Vec<usize>,
}

impl<'source> SourcePositions<'source> {
    fn new(source: &'source str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            source
                .char_indices()
                .filter_map(|(offset, character)| (character == '\n').then_some(offset + 1)),
        );
        Self {
            source,
            line_starts,
        }
    }

    fn position(&self, offset: u32) -> Position {
        let offset = usize::try_from(offset).expect("Oxc span fits usize");
        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let line_start = self.line_starts[line_index];
        Position {
            line: line_index + 1,
            column: self.source[line_start..offset].chars().count() + 1,
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
