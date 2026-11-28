//! The whitelist: walks a parsed query with an explicit stack and refuses what phase 2 does not run.
//!
//! Queries, selects, tables and joins are checked here, expressions in `whitelist_expr`. A
//! construct refusal wins over an unknown function, which is reported only once the walk ends.

use std::rc::Rc;

use sqlparser::ast::{
    BinaryOperator, Expr, GroupByExpr, Ident, Join, JoinConstraint, JoinOperator, LimitClause, NamedWindowExpr,
    ObjectName, ObjectNamePart, OrderByKind, Query, Select, SelectFlavor, SelectItem, SelectItemQualifiedWildcardKind,
    SetExpr, SetOperator, SetQuantifier, TableAlias, TableFactor, WildcardAdditionalOptions, WindowSpec,
};

use crate::DataError;
use crate::error::{QueryConstructSnafu, QueryTooDeepSnafu, QueryUnknownFunctionSnafu, quoted};
use crate::limits::MAX_QUERY_DEPTH;

/// What the clauses of one select may reference.
#[derive(Debug, Default)]
pub(super) struct Facts<'a> {
    /// Lowercased names of the select's tables; `None` where qualifiers are not checked.
    pub qualifiers: Option<Vec<String>>,
    /// The select's named windows, lowercased names.
    pub windows: Vec<(String, &'a WindowSpec)>,
}

/// Where an expression stands: what it may reference and contain.
#[derive(Debug, Clone)]
pub(super) struct Ctx<'a> {
    pub facts: Rc<Facts<'a>>,
    /// An uncorrelated scalar subquery is allowed (WHERE, select list, HAVING).
    pub scalar: bool,
    /// A window function is allowed (select list, QUALIFY).
    pub windows: bool,
}

impl<'a> Ctx<'a> {
    /// A context allowing neither scalar subqueries nor windows.
    pub fn plain(facts: &Rc<Facts<'a>>) -> Self {
        Ctx { facts: Rc::clone(facts), scalar: false, windows: false }
    }
}

/// A node still to check.
pub(super) enum Node<'a> {
    Query(&'a Query),
    Body(&'a SetExpr),
    Select(&'a Select),
    Factor(&'a TableFactor),
    Expr(&'a Expr, Ctx<'a>),
}

/// The explicit walk: pending nodes with their depth, and the first unknown function met.
pub(super) struct Walk<'a> {
    stack: Vec<(Node<'a>, usize)>,
    pub unknown: Option<String>,
    pub names: Names,
}

/// Whose identifiers the walk checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Names {
    /// The user's query: the `__` namespace is reserved for the rewrites.
    User,
    /// The rewriter's output, which names its own relations and columns `__…`.
    Engine,
}

/// Checks `query` against the phase 2 whitelist.
pub(super) fn check(query: &Query, names: Names) -> Result<(), DataError> {
    let mut walk = Walk { stack: vec![(Node::Query(query), 1)], unknown: None, names };
    while let Some((node, depth)) = walk.stack.pop() {
        if depth > MAX_QUERY_DEPTH {
            return Err(QueryTooDeepSnafu.build().into());
        }
        match node {
            Node::Query(query) => walk.query(query, depth)?,
            Node::Body(body) => walk.body(body, depth)?,
            Node::Select(select) => walk.select(select, depth)?,
            Node::Factor(factor) => walk.factor(factor, depth)?,
            Node::Expr(expr, ctx) => walk.expr(expr, &ctx, depth)?,
        }
    }
    match walk.unknown {
        Some(function) => Err(QueryUnknownFunctionSnafu { function: quoted(&function) }.build().into()),
        None => Ok(()),
    }
}

/// The refusal naming `construct`.
///
/// The construct is cut as messages quote it: a refused subtree can be most of a 16 KiB query.
pub(super) fn refuse(construct: impl Into<String>) -> DataError {
    QueryConstructSnafu { construct: quoted(&construct.into()) }.build().into()
}

/// The single identifier a name is made of, or the name refused as written.
pub(super) fn single_name(name: &ObjectName) -> Result<&Ident, DataError> {
    match name.0.as_slice() {
        [ObjectNamePart::Identifier(ident)] => Ok(ident),
        _ => Err(refuse(name.to_string())),
    }
}

/// The identifier a relation is referenced by in its select, as written, when it has one.
pub(super) fn relation_ident(factor: &TableFactor) -> Option<&Ident> {
    match factor {
        TableFactor::Table { alias: Some(alias), .. }
        | TableFactor::Derived { alias: Some(alias), .. }
        | TableFactor::Pivot { alias: Some(alias), .. }
        | TableFactor::Unpivot { alias: Some(alias), .. } => Some(&alias.name),
        TableFactor::Table { name, .. } => match name.0.as_slice() {
            [ObjectNamePart::Identifier(ident)] => Some(ident),
            _ => None,
        },
        _ => None,
    }
}

/// The lowercased name a relation is referenced by in its select, when it has one.
pub(super) fn relation_name(factor: &TableFactor) -> Option<String> {
    relation_ident(factor).map(|ident| ident.value.to_ascii_lowercase())
}

/// The lowercased names columns of `select` may be qualified with.
pub(super) fn qualifiers(select: &Select) -> Vec<String> {
    let mut names = Vec::new();
    for table in &select.from {
        for factor in std::iter::once(&table.relation).chain(table.joins.iter().map(|join| &join.relation)) {
            names.extend(relation_name(factor));
            if let TableFactor::Pivot { table, .. } | TableFactor::Unpivot { table, .. } = factor {
                names.extend(relation_name(table));
            }
        }
    }
    names
}

/// The join's keyword, as the refusals name it.
pub(super) fn join_keyword(operator: &JoinOperator) -> &'static str {
    match operator {
        JoinOperator::Left(_) | JoinOperator::LeftOuter(_) => "LEFT JOIN",
        JoinOperator::Right(_) | JoinOperator::RightOuter(_) => "RIGHT JOIN",
        JoinOperator::FullOuter(_) => "FULL JOIN",
        JoinOperator::CrossJoin(_) => "CROSS JOIN",
        JoinOperator::Semi(_) | JoinOperator::LeftSemi(_) | JoinOperator::RightSemi(_) => "SEMI JOIN",
        JoinOperator::Anti(_) | JoinOperator::LeftAnti(_) | JoinOperator::RightAnti(_) => "ANTI JOIN",
        JoinOperator::AsOf { .. } => "ASOF JOIN",
        JoinOperator::CrossApply => "CROSS APPLY",
        JoinOperator::OuterApply => "OUTER APPLY",
        JoinOperator::StraightJoin(_) => "STRAIGHT_JOIN",
        JoinOperator::Join(_) | JoinOperator::Inner(_) => "JOIN",
    }
}

/// The AND-terms of a condition.
pub(super) fn and_terms(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::BinaryOp { left, op: BinaryOperator::And, right } => {
            let mut terms = and_terms(left);
            terms.extend(and_terms(right));
            terms
        }
        Expr::Nested(inner) if matches!(**inner, Expr::BinaryOp { op: BinaryOperator::And, .. }) => and_terms(inner),
        _ => vec![expr],
    }
}

/// Whether `expr` is an equality, the only join condition polars runs.
pub(super) fn is_equality(expr: &Expr) -> bool {
    matches!(expr, Expr::BinaryOp { op: BinaryOperator::Eq, .. })
}

impl<'a> Walk<'a> {
    /// Refuses a user identifier in the `__` namespace the rewrites use.
    pub fn plain(&self, ident: &Ident) -> Result<(), DataError> {
        if self.names == Names::User && ident.value.starts_with("__") {
            Err(refuse(ident.value.clone()))
        } else {
            Ok(())
        }
    }

    fn plain_alias(&self, alias: Option<&TableAlias>) -> Result<(), DataError> {
        let Some(alias) = alias else { return Ok(()) };
        self.plain(&alias.name)?;
        alias.columns.iter().try_for_each(|column| match &column.data_type {
            Some(data_type) => Err(refuse(format!("AS … ({} {data_type})", column.name))),
            None => self.plain(&column.name),
        })
    }

    /// Queues `node` one level below `depth`.
    pub fn push(&mut self, node: Node<'a>, depth: usize) {
        self.stack.push((node, depth + 1));
    }

    /// Queues `expr` in `ctx` one level below `depth`.
    pub fn push_expr(&mut self, expr: &'a Expr, ctx: &Ctx<'a>, depth: usize) {
        self.push(Node::Expr(expr, ctx.clone()), depth);
    }

    fn query(&mut self, query: &'a Query, depth: usize) -> Result<(), DataError> {
        let refused = [
            (!query.locks.is_empty(), "FOR UPDATE"),
            (query.for_clause.is_some(), "FOR"),
            (query.settings.is_some(), "SETTINGS"),
            (query.format_clause.is_some(), "FORMAT"),
            (!query.pipe_operators.is_empty(), "|>"),
        ];
        if let Some((_, construct)) = refused.iter().find(|(found, _)| *found) {
            return Err(refuse(*construct));
        }
        if let Some(with) = &query.with {
            if with.recursive {
                return Err(refuse("WITH RECURSIVE"));
            }
            for cte in &with.cte_tables {
                self.plain_alias(Some(&cte.alias))?;
                if cte.materialized.is_some() {
                    return Err(refuse("MATERIALIZED"));
                }
                if let Some(from) = &cte.from {
                    return Err(refuse(format!("FROM {from}")));
                }
                self.push(Node::Query(&cte.query), depth);
            }
        }
        self.push(Node::Body(&query.body), depth);
        let facts = Rc::new(Facts {
            qualifiers: match &*query.body {
                SetExpr::Select(select) => Some(qualifiers(select)),
                _ => None,
            },
            windows: vec![],
        });
        let ctx = Ctx::plain(&facts);
        if let Some(order_by) = &query.order_by {
            if order_by.interpolate.is_some() {
                return Err(refuse("INTERPOLATE"));
            }
            if let OrderByKind::Expressions(keys) = &order_by.kind {
                for key in keys {
                    if key.with_fill.is_some() {
                        return Err(refuse("WITH FILL"));
                    }
                    self.push_expr(&key.expr, &ctx, depth);
                }
            }
        }
        match &query.limit_clause {
            Some(LimitClause::LimitOffset { limit, offset, limit_by }) => {
                if !limit_by.is_empty() {
                    return Err(refuse("LIMIT … BY"));
                }
                for expr in limit.iter().chain(offset.iter().map(|offset| &offset.value)) {
                    self.push_expr(expr, &ctx, depth);
                }
            }
            Some(LimitClause::OffsetCommaLimit { .. }) => return Err(refuse("LIMIT …, …")),
            None => {}
        }
        if let Some(fetch) = &query.fetch {
            if fetch.with_ties {
                return Err(refuse("WITH TIES"));
            }
            if fetch.percent {
                return Err(refuse("PERCENT"));
            }
            if let Some(expr) = &fetch.quantity {
                self.push_expr(expr, &ctx, depth);
            }
        }
        Ok(())
    }

    fn body(&mut self, body: &'a SetExpr, depth: usize) -> Result<(), DataError> {
        match body {
            SetExpr::Select(select) => self.push(Node::Select(select), depth),
            SetExpr::Query(query) => self.push(Node::Query(query), depth),
            SetExpr::SetOperation { op, set_quantifier, left, right } => {
                match op {
                    SetOperator::Union => {}
                    SetOperator::Except => return Err(refuse("EXCEPT")),
                    SetOperator::Intersect => return Err(refuse("INTERSECT")),
                    SetOperator::Minus => return Err(refuse("MINUS")),
                }
                match set_quantifier {
                    SetQuantifier::All | SetQuantifier::Distinct | SetQuantifier::None => {}
                    SetQuantifier::ByName | SetQuantifier::AllByName | SetQuantifier::DistinctByName => {
                        return Err(refuse(format!("UNION {set_quantifier}")));
                    }
                }
                self.push(Node::Body(left), depth);
                self.push(Node::Body(right), depth);
            }
            SetExpr::Values(values) => {
                if values.explicit_row || values.value_keyword {
                    return Err(refuse(if values.explicit_row { "ROW" } else { "VALUE" }));
                }
                let ctx = Ctx::plain(&Rc::new(Facts::default()));
                for expr in values.rows.iter().flatten() {
                    self.push_expr(expr, &ctx, depth);
                }
            }
            SetExpr::Insert(_) => return Err(refuse("INSERT")),
            SetExpr::Update(_) => return Err(refuse("UPDATE")),
            SetExpr::Delete(_) => return Err(refuse("DELETE")),
            SetExpr::Merge(_) => return Err(refuse("MERGE")),
            SetExpr::Table(_) => return Err(refuse("TABLE")),
        }
        Ok(())
    }

    fn select(&mut self, select: &'a Select, depth: usize) -> Result<(), DataError> {
        let refused = [
            (select.top.is_some(), "TOP"),
            (select.exclude.is_some(), "EXCLUDE"),
            (select.into.is_some(), "INTO"),
            (!select.lateral_views.is_empty(), "LATERAL VIEW"),
            (select.prewhere.is_some(), "PREWHERE"),
            (!select.cluster_by.is_empty(), "CLUSTER BY"),
            (!select.distribute_by.is_empty(), "DISTRIBUTE BY"),
            (!select.sort_by.is_empty(), "SORT BY"),
            (select.connect_by.is_some(), "CONNECT BY"),
            (select.value_table_mode.is_some(), "AS VALUE"),
            (!matches!(select.flavor, SelectFlavor::Standard), "FROM … SELECT"),
        ];
        if let Some((_, construct)) = refused.iter().find(|(found, _)| *found) {
            return Err(refuse(*construct));
        }
        let mut windows = Vec::new();
        for definition in &select.named_window {
            self.plain(&definition.0)?;
            match &definition.1 {
                NamedWindowExpr::WindowSpec(spec) => windows.push((definition.0.value.to_ascii_lowercase(), spec)),
                NamedWindowExpr::NamedWindow(other) => {
                    return Err(refuse(format!("WINDOW {} AS {other}", definition.0)));
                }
            }
        }
        let facts = Rc::new(Facts { qualifiers: Some(qualifiers(select)), windows });
        let plain = Ctx::plain(&facts);
        let listed = Ctx { facts: Rc::clone(&facts), scalar: true, windows: true };
        let filtered = Ctx { facts: Rc::clone(&facts), scalar: true, windows: false };
        let qualify = Ctx { facts: Rc::clone(&facts), scalar: false, windows: true };
        if let Some(sqlparser::ast::Distinct::On(exprs)) = &select.distinct {
            for expr in exprs {
                self.push_expr(expr, &plain, depth);
            }
        }
        for table in &select.from {
            if let TableFactor::Table { alias, .. } = &table.relation {
                positional(alias.as_ref())?;
            }
            self.push(Node::Factor(&table.relation), depth);
            for join in &table.joins {
                self.join(join, &plain, depth)?;
            }
        }
        for item in &select.projection {
            self.item(item, &listed, depth)?;
        }
        if let Some(expr) = &select.selection {
            self.push_expr(expr, &filtered, depth);
        }
        match &select.group_by {
            GroupByExpr::All(modifiers) | GroupByExpr::Expressions(_, modifiers) if !modifiers.is_empty() => {
                return Err(refuse(select.group_by.to_string().trim().to_owned()));
            }
            GroupByExpr::All(_) => {}
            GroupByExpr::Expressions(exprs, _) => exprs.iter().for_each(|expr| self.push_expr(expr, &plain, depth)),
        }
        if let Some(expr) = &select.having {
            self.push_expr(expr, &filtered, depth);
        }
        if let Some(expr) = &select.qualify {
            self.push_expr(expr, &qualify, depth);
        }
        Ok(())
    }

    fn item(&mut self, item: &'a SelectItem, ctx: &Ctx<'a>, depth: usize) -> Result<(), DataError> {
        match item {
            SelectItem::UnnamedExpr(expr) => self.projected(expr, ctx, depth),
            SelectItem::ExprWithAlias { expr, alias } => {
                self.plain(alias)?;
                self.projected(expr, ctx, depth)
            }
            SelectItem::Wildcard(options) => self.wildcard(options, ctx, depth),
            SelectItem::QualifiedWildcard(SelectItemQualifiedWildcardKind::ObjectName(name), options) => {
                check_qualifier(single_name(name)?, &ctx.facts, &format!("{name}.*"))?;
                self.wildcard(options, ctx, depth)
            }
            SelectItem::QualifiedWildcard(SelectItemQualifiedWildcardKind::Expr(expr), _) => {
                Err(refuse(format!("{expr}.*")))
            }
        }
    }

    /// A select-list expression: `COLUMNS('re')` is allowed here, and only here.
    fn projected(&mut self, expr: &'a Expr, ctx: &Ctx<'a>, depth: usize) -> Result<(), DataError> {
        if let Expr::Function(function) = expr
            && function.name.to_string().eq_ignore_ascii_case("columns")
        {
            let args = super::ast::args(function);
            let regex = matches!(
                args.as_slice(),
                [Expr::Value(value)] if matches!(value.value, sqlparser::ast::Value::SingleQuotedString(_))
            );
            let plain = function.over.is_none() && function.filter.is_none();
            return if regex && plain && super::ast::arg_count(function) == 1 {
                Ok(())
            } else {
                Err(refuse("COLUMNS(…)"))
            };
        }
        self.push_expr(expr, ctx, depth);
        Ok(())
    }

    fn wildcard(
        &mut self,
        options: &'a WildcardAdditionalOptions,
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> Result<(), DataError> {
        if options.opt_ilike.is_some() {
            return Err(refuse("* ILIKE"));
        }
        if options.opt_except.is_some() {
            return Err(refuse("* EXCEPT"));
        }
        if options.opt_rename.is_some() {
            return Err(refuse("* RENAME"));
        }
        // The scalar-subquery rewrite reads the select list's expressions, not `* REPLACE`'s.
        let replaced = Ctx { scalar: false, ..ctx.clone() };
        for element in options.opt_replace.iter().flat_map(|replace| &replace.items) {
            self.plain(&element.column_name)?;
            self.push_expr(&element.expr, &replaced, depth);
        }
        Ok(())
    }

    fn join(&mut self, join: &'a Join, ctx: &Ctx<'a>, depth: usize) -> Result<(), DataError> {
        if join.global {
            return Err(refuse("GLOBAL JOIN"));
        }
        self.push(Node::Factor(&join.relation), depth);
        let keyword = join_keyword(&join.join_operator);
        let (constraint, outer) = match &join.join_operator {
            JoinOperator::Join(constraint) | JoinOperator::Inner(constraint) => (constraint, false),
            JoinOperator::Left(constraint)
            | JoinOperator::LeftOuter(constraint)
            | JoinOperator::Right(constraint)
            | JoinOperator::RightOuter(constraint)
            | JoinOperator::FullOuter(constraint) => (constraint, true),
            JoinOperator::CrossJoin(JoinConstraint::None) => return Ok(()),
            _ => return Err(refuse(keyword)),
        };
        match constraint {
            JoinConstraint::On(condition) => {
                if outer && !and_terms(condition).into_iter().all(is_equality) {
                    return Err(refuse(format!("{keyword} … ON {condition}")));
                }
                self.push_expr(condition, ctx, depth);
            }
            JoinConstraint::Using(names) => {
                names.iter().try_for_each(|name| single_name(name).and_then(|ident| self.plain(ident)))?;
            }
            JoinConstraint::Natural if keyword == "JOIN" || keyword == "LEFT JOIN" => {}
            JoinConstraint::Natural => return Err(refuse(format!("NATURAL {keyword}"))),
            JoinConstraint::None => return Err(refuse(format!("{keyword} without ON"))),
        }
        Ok(())
    }

    fn factor(&mut self, factor: &'a TableFactor, depth: usize) -> Result<(), DataError> {
        match factor {
            TableFactor::Table {
                name,
                alias,
                args,
                with_hints,
                version,
                with_ordinality,
                partitions,
                json_path,
                sample,
                index_hints,
            } => {
                if args.is_some() {
                    return Err(refuse(name.to_string()));
                }
                let ident = single_name(name)?;
                if ident.quote_style == Some('\'') {
                    return Err(refuse(name.to_string()));
                }
                self.plain(ident)?;
                if sample.is_some() {
                    return Err(refuse("SAMPLE"));
                }
                let refused = [
                    (!with_hints.is_empty(), "WITH (…)"),
                    (version.is_some(), "FOR SYSTEM_TIME"),
                    (*with_ordinality, "WITH ORDINALITY"),
                    (!partitions.is_empty(), "PARTITION"),
                    (json_path.is_some(), "JSON path"),
                    (!index_hints.is_empty(), "INDEX"),
                ];
                if let Some((_, construct)) = refused.iter().find(|(found, _)| *found) {
                    return Err(refuse(*construct));
                }
                positional(alias.as_ref())?;
                self.plain_alias(alias.as_ref())
            }
            TableFactor::Derived { lateral, subquery, alias } => {
                if *lateral {
                    return Err(refuse("LATERAL"));
                }
                positional(alias.as_ref())?;
                self.plain_alias(alias.as_ref())?;
                self.push(Node::Query(subquery), depth);
                Ok(())
            }
            TableFactor::Pivot { table, aggregate_functions, value_column, value_source, default_on_null, alias } => {
                self.plain_alias(alias.as_ref())?;
                self.pivot(aggregate_functions, value_column, value_source, default_on_null.as_ref(), depth)?;
                self.push(Node::Factor(table), depth);
                Ok(())
            }
            TableFactor::Unpivot { table, value, name, columns, alias, .. } => {
                self.plain_alias(alias.as_ref())?;
                self.plain(name)?;
                let value_ok = matches!(value, Expr::Identifier(ident) if !ident.value.starts_with("__"));
                let columns_ok =
                    columns.iter().all(|column| column.alias.is_none() && matches!(column.expr, Expr::Identifier(_)));
                if !value_ok || !columns_ok {
                    return Err(refuse("UNPIVOT (…)"));
                }
                self.push(Node::Factor(table), depth);
                Ok(())
            }
            TableFactor::TableFunction { expr, .. } => Err(refuse(expr.to_string())),
            TableFactor::Function { name, .. } => Err(refuse(name.to_string())),
            TableFactor::UNNEST { .. } => Err(refuse("UNNEST")),
            TableFactor::NestedJoin { .. } => Err(refuse("(… JOIN …)")),
            TableFactor::JsonTable { .. } => Err(refuse("JSON_TABLE")),
            TableFactor::OpenJsonTable { .. } => Err(refuse("OPENJSON")),
            TableFactor::MatchRecognize { .. } => Err(refuse("MATCH_RECOGNIZE")),
            TableFactor::XmlTable { .. } => Err(refuse("XMLTABLE")),
            TableFactor::SemanticView { .. } => Err(refuse("SEMANTIC_VIEW")),
        }
    }

    fn pivot(
        &mut self,
        aggregates: &'a [sqlparser::ast::ExprWithAlias],
        value_column: &'a [Expr],
        source: &'a sqlparser::ast::PivotValueSource,
        default_on_null: Option<&Expr>,
        depth: usize,
    ) -> Result<(), DataError> {
        use sqlparser::ast::PivotValueSource;
        if default_on_null.is_some() {
            return Err(refuse("DEFAULT ON NULL"));
        }
        let [aggregate] = aggregates else { return Err(refuse("PIVOT (…, …)")) };
        let is_aggregate = matches!(&aggregate.expr, Expr::Function(function)
            if function.over.is_none()
                && super::ast::function_name(function).is_some_and(|name| super::functions::is_aggregate(&name)));
        if !is_aggregate {
            return Err(refuse(format!("PIVOT ({} …)", aggregate.expr)));
        }
        if let Some(alias) = &aggregate.alias {
            self.plain(alias)?;
        }
        let [Expr::Identifier(column)] = value_column else { return Err(refuse("PIVOT (… FOR (…))")) };
        self.plain(column)?;
        let values = match source {
            PivotValueSource::List(values) => values,
            PivotValueSource::Any(_) => return Err(refuse("IN (ANY)")),
            PivotValueSource::Subquery(_) => return Err(refuse("IN (SELECT …)")),
        };
        for value in values {
            if !matches!(&value.expr, Expr::Value(v) if !matches!(v.value, sqlparser::ast::Value::Null)) {
                return Err(refuse(format!("IN ({} …)", value.expr)));
            }
            if let Some(alias) = &value.alias {
                self.plain(alias)?;
            }
        }
        let ctx = Ctx::plain(&Rc::new(Facts::default()));
        self.push_expr(&aggregate.expr, &ctx, depth);
        Ok(())
    }
}

/// Refuses a column qualifier naming no table of the select (a correlated reference).
pub(super) fn check_qualifier(qualifier: &Ident, facts: &Facts<'_>, written: &str) -> Result<(), DataError> {
    match &facts.qualifiers {
        Some(names) if !names.contains(&qualifier.value.to_ascii_lowercase()) => Err(refuse(written)),
        _ => Ok(()),
    }
}

/// Refuses the `DuckDB` `POSITIONAL JOIN`, which the parser reads as a table named by an alias.
fn positional(alias: Option<&TableAlias>) -> Result<(), DataError> {
    match alias {
        Some(alias) if !alias.explicit && alias.name.value.eq_ignore_ascii_case("positional") => {
            Err(refuse("POSITIONAL JOIN"))
        }
        _ => Ok(()),
    }
}
