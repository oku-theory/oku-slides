//! The select rules of the rewrite, applied to a select before its expressions are rewritten.

use sqlparser::ast::{
    BinaryOperator, ExcludeSelectItem, Expr, GroupByExpr, Ident, Join, JoinConstraint, JoinOperator, OrderByExpr,
    OrderByOptions, Select, SelectItem, SelectItemQualifiedWildcardKind, SetExpr, TableFactor, TableWithJoins, Values,
    WildcardAdditionalOptions, WindowType,
};

use super::ast::{alias, call, children_mut, column_of, derived, each_mut, function_name, ident, number, query_of};
use super::functions;
use super::rewrite::Counters;
use super::whitelist::{and_terms, is_equality, join_keyword, refuse, relation_ident, relation_name};
use crate::DataError;

/// Applies the select rules, in the spec's order, to one select.
pub(super) fn apply(select: &mut Select, counters: &mut Counters) -> Result<(), DataError> {
    inline_windows(select);
    if select.qualify.is_some() {
        // The inner select gets the other rules when the visitor reaches it.
        unqualify(select, counters);
        return Ok(());
    }
    uncomma(select)?;
    inner_terms_to_where(select)?;
    outer_using(select)?;
    group_by_position(select);
    if select.from.is_empty() {
        let one = query_of(SetExpr::Values(Values {
            explicit_row: false,
            value_keyword: false,
            rows: vec![vec![number("1")]],
        }));
        select.from = vec![TableWithJoins { relation: derived(one, alias(ident("__one"), &["__x"])), joins: vec![] }];
    }
    null_flags(select, counters)?;
    scalar_subqueries(select, counters);
    Ok(())
}

/// The expressions of a select's own clauses, outside subqueries and the `FROM`.
fn clauses_mut(select: &mut Select) -> Vec<&mut Expr> {
    let mut exprs: Vec<&mut Expr> = Vec::new();
    for item in &mut select.projection {
        match item {
            SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => exprs.push(expr),
            SelectItem::Wildcard(options) | SelectItem::QualifiedWildcard(_, options) => {
                exprs.extend(options.opt_replace.iter_mut().flat_map(|r| r.items.iter_mut().map(|i| &mut i.expr)));
            }
        }
    }
    exprs.extend(select.selection.as_mut());
    if let GroupByExpr::Expressions(group, _) = &mut select.group_by {
        exprs.extend(group.iter_mut());
    }
    exprs.extend(select.having.as_mut());
    exprs.extend(select.qualify.as_mut());
    exprs
}

/// Replaces `OVER name` with the named window's spec.
fn inline_windows(select: &mut Select) {
    let windows: Vec<(String, sqlparser::ast::WindowSpec)> = std::mem::take(&mut select.named_window)
        .into_iter()
        .filter_map(|definition| match definition.1 {
            sqlparser::ast::NamedWindowExpr::WindowSpec(spec) => Some((definition.0.value.to_ascii_lowercase(), spec)),
            sqlparser::ast::NamedWindowExpr::NamedWindow(_) => None,
        })
        .collect();
    if windows.is_empty() {
        return;
    }
    for expr in clauses_mut(select) {
        each_mut(expr, &mut |node| {
            if let Expr::Function(function) = node
                && let Some(WindowType::NamedWindow(name)) = &function.over
                && let Some((_, spec)) = windows.iter().find(|(window, _)| *window == name.value.to_ascii_lowercase())
            {
                function.over = Some(WindowType::WindowSpec(spec.clone()));
            }
            true
        });
    }
}

/// Adds `columns` to the `EXCLUDE` of a wildcard.
fn exclude(options: &mut WildcardAdditionalOptions, columns: &[Ident]) {
    let mut excluded = match options.opt_exclude.take() {
        None => vec![],
        Some(ExcludeSelectItem::Single(column)) => vec![column],
        Some(ExcludeSelectItem::Multiple(columns)) => columns,
    };
    excluded.extend(columns.iter().cloned());
    options.opt_exclude = Some(ExcludeSelectItem::Multiple(excluded));
}

/// Excludes generated `columns` from every unqualified `*` of the select list.
fn exclude_from_wildcards(select: &mut Select, columns: &[Ident]) {
    for item in &mut select.projection {
        if let SelectItem::Wildcard(options) = item {
            exclude(options, columns);
        }
    }
}

/// QUALIFY: the windows become columns `__qN` of a derived table, filtered outside.
fn unqualify(select: &mut Select, counters: &mut Counters) {
    let Some(mut condition) = select.qualify.take() else { return };
    let mut columns = Vec::new();
    each_mut(&mut condition, &mut |node| {
        if matches!(node, Expr::Function(function) if function.over.is_some()) {
            let name = ident(&counters.name("__q"));
            let window = std::mem::replace(node, Expr::Identifier(name.clone()));
            columns.push((window, name));
            return false;
        }
        true
    });
    let mut inner = select.clone();
    inner.distinct = None;
    let names: Vec<Ident> = columns.iter().map(|(_, name)| name.clone()).collect();
    inner.projection.extend(columns.into_iter().map(|(expr, alias)| SelectItem::ExprWithAlias { expr, alias }));
    let mut options = WildcardAdditionalOptions::default();
    exclude(&mut options, &names);
    let mut outer = super::ast::select(
        vec![SelectItem::Wildcard(options)],
        derived(query_of(SetExpr::Select(Box::new(inner))), alias(ident("__q"), &[])),
    );
    outer.distinct = select.distinct.take();
    outer.selection = Some(condition);
    *select = outer;
}

/// Whether a join keeps rows a later filter would remove: RIGHT or FULL.
fn keeps_right(join: &Join) -> bool {
    matches!(join.join_operator, JoinOperator::Right(_) | JoinOperator::RightOuter(_) | JoinOperator::FullOuter(_))
}

/// A comma join becomes `CROSS JOIN`.
fn uncomma(select: &mut Select) -> Result<(), DataError> {
    if select.from.len() < 2 {
        return Ok(());
    }
    let mut tables = std::mem::take(&mut select.from).into_iter();
    let Some(mut first) = tables.next() else { return Ok(()) };
    for table in tables {
        if let Some(join) = table.joins.iter().find(|join| keeps_right(join)) {
            return Err(refuse(format!(", … {}", join_keyword(&join.join_operator))));
        }
        first.joins.push(Join {
            relation: table.relation,
            global: false,
            join_operator: JoinOperator::CrossJoin(JoinConstraint::None),
        });
        first.joins.extend(table.joins);
    }
    select.from = vec![first];
    Ok(())
}

/// Moves the non-equality terms of an inner join's `ON` to `WHERE`.
fn inner_terms_to_where(select: &mut Select) -> Result<(), DataError> {
    let mut moved = Vec::new();
    for table in &mut select.from {
        let mut rest = table.joins.as_mut_slice();
        while let Some((join, later)) = std::mem::take(&mut rest).split_first_mut() {
            rest = later;
            let later_keeps_right = rest.iter().any(keeps_right);
            let (JoinOperator::Join(JoinConstraint::On(condition))
            | JoinOperator::Inner(JoinConstraint::On(condition))) = &mut join.join_operator
            else {
                continue;
            };
            let terms: Vec<Expr> = and_terms(condition).into_iter().cloned().collect();
            let (equal, other): (Vec<Expr>, Vec<Expr>) = terms.into_iter().partition(is_equality);
            let Some(first) = other.first() else { continue };
            if later_keeps_right {
                return Err(refuse(format!("JOIN … ON {first}")));
            }
            join.join_operator = match conjunction(equal) {
                Some(condition) => JoinOperator::Join(JoinConstraint::On(condition)),
                None => JoinOperator::CrossJoin(JoinConstraint::None),
            };
            moved.extend(other);
        }
    }
    if !moved.is_empty() {
        let previous = select.selection.take();
        select.selection = conjunction(previous.into_iter().chain(moved).collect());
    }
    Ok(())
}

/// `a AND b AND …`, each term parenthesized when it is an `OR`.
fn conjunction(terms: Vec<Expr>) -> Option<Expr> {
    terms
        .into_iter()
        .map(|term| match term {
            Expr::BinaryOp { op: BinaryOperator::Or, .. } => Expr::Nested(Box::new(term)),
            other => other,
        })
        .reduce(|left, right| super::ast::binary(left, BinaryOperator::And, right))
}

/// FULL / RIGHT JOIN USING: `ON l.k = r.k`, and the unqualified key reads `COALESCE(l.k, r.k)`.
fn outer_using(select: &mut Select) -> Result<(), DataError> {
    let Some(table) = select.from.first_mut() else { return Ok(()) };
    // Identifiers as written, quotes kept: a quoted alias can hold any text.
    let left_name = relation_ident(&table.relation).cloned();
    let mut keys: Vec<(Ident, Expr)> = Vec::new();
    for (at, join) in table.joins.iter_mut().enumerate() {
        let keyword = join_keyword(&join.join_operator);
        let (JoinOperator::Right(constraint)
        | JoinOperator::RightOuter(constraint)
        | JoinOperator::FullOuter(constraint)) = &mut join.join_operator
        else {
            continue;
        };
        let JoinConstraint::Using(names) = constraint else { continue };
        let (Some(left), Some(right), 0) = (&left_name, relation_ident(&join.relation).cloned(), at) else {
            return Err(refuse(format!("{keyword} … USING")));
        };
        let mut terms = Vec::new();
        for name in names.iter() {
            let key = super::whitelist::single_name(name)?.clone();
            let (l, r) = (column_of(left, &key), column_of(&right, &key));
            terms.push(super::ast::binary(l.clone(), BinaryOperator::Eq, r.clone()));
            keys.push((key, call("coalesce", vec![l, r])));
        }
        let Some(condition) = conjunction(terms) else { continue };
        *constraint = JoinConstraint::On(condition);
    }
    if keys.is_empty() {
        return Ok(());
    }
    if select.projection.iter().any(|item| matches!(item, SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..)))
    {
        return Err(refuse("* … USING"));
    }
    let key_of = |name: &Ident| {
        keys.iter().find(|(key, _)| key.value.eq_ignore_ascii_case(&name.value)).map(|(_, value)| value.clone())
    };
    for item in &mut select.projection {
        if let SelectItem::UnnamedExpr(Expr::Identifier(name)) = item
            && let Some(value) = key_of(name)
        {
            *item = SelectItem::ExprWithAlias { expr: value, alias: name.clone() };
        }
    }
    for expr in clauses_mut(select) {
        each_mut(expr, &mut |node| {
            if let Expr::Identifier(name) = node
                && let Some(value) = key_of(name)
            {
                *node = value;
            }
            true
        });
    }
    Ok(())
}

/// GROUP BY an expression repeated from the select list: its position, else its alias.
fn group_by_position(select: &mut Select) {
    let GroupByExpr::Expressions(group, _) = &mut select.group_by else { return };
    for key in group.iter_mut() {
        if matches!(key, Expr::Identifier(_) | Expr::CompoundIdentifier(_) | Expr::Value(_)) {
            continue;
        }
        let mut wildcard = false;
        for (at, item) in select.projection.iter().enumerate() {
            match item {
                SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..) => wildcard = true,
                SelectItem::UnnamedExpr(expr) if expr == key && !wildcard => {
                    *key = number(&(at + 1).to_string());
                    break;
                }
                SelectItem::ExprWithAlias { expr, alias } if expr == key => {
                    *key = if wildcard { Expr::Identifier(alias.clone()) } else { number(&(at + 1).to_string()) };
                    break;
                }
                _ => {}
            }
        }
    }
}

/// Whether the select groups rows.
fn aggregated(select: &Select) -> bool {
    let grouped = match &select.group_by {
        GroupByExpr::All(_) => true,
        GroupByExpr::Expressions(exprs, _) => !exprs.is_empty(),
    };
    grouped
        || select.having.is_some()
        || select.projection.iter().any(|item| match item {
            SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => has_aggregate(expr),
            _ => false,
        })
}

/// Whether `expr` calls an aggregate outside a window.
fn has_aggregate(expr: &Expr) -> bool {
    let mut found = false;
    super::ast::each(expr, &mut |node| {
        if let Expr::Function(function) = node
            && function.over.is_none()
            && function_name(function).is_some_and(|name| functions::is_aggregate(&name))
        {
            found = true;
        }
    });
    found
}

/// Window `ORDER BY` keys get a NULL flag ordered first, computed in a derived table.
fn null_flags(select: &mut Select, counters: &mut Counters) -> Result<(), DataError> {
    // Flags per relation index: (flag expression, name).
    let relations: Vec<Option<String>> = select
        .from
        .iter()
        .flat_map(|table| std::iter::once(&table.relation).chain(table.joins.iter().map(|join| &join.relation)))
        .map(relation_name)
        .collect();
    let joined = relations.len() > 1;
    let mut flags: Vec<(usize, Expr, Ident)> = Vec::new();
    let mut failure = None;
    let grouped = aggregated(select);
    let mut items: Vec<&mut Expr> = Vec::new();
    for item in &mut select.projection {
        if let SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } = item {
            items.push(expr);
        }
    }
    for expr in items {
        each_mut(expr, &mut |node| {
            let Expr::Function(function) = node else { return true };
            let Some(WindowType::WindowSpec(spec)) = &mut function.over else { return true };
            let mut keys = Vec::new();
            for key in std::mem::take(&mut spec.order_by) {
                if matches!(key.expr, Expr::Value(_)) {
                    keys.push(key);
                    continue;
                }
                if grouped {
                    failure.get_or_insert_with(|| refuse("GROUP BY … OVER (ORDER BY …)"));
                }
                let relation = match (&key.expr, joined) {
                    (_, false) => Some(0),
                    (Expr::CompoundIdentifier(parts), true) => parts.first().and_then(|qualifier| {
                        let lower = qualifier.value.to_ascii_lowercase();
                        relations.iter().position(|name| name.as_deref() == Some(lower.as_str()))
                    }),
                    _ => None,
                };
                let Some(relation) = relation else {
                    failure.get_or_insert_with(|| refuse(format!("OVER (ORDER BY {})", key.expr)));
                    keys.push(key);
                    continue;
                };
                let ascending = key.options.asc.unwrap_or(true);
                let nulls_first = key.options.nulls_first.unwrap_or(false);
                let inner = Box::new(key.expr.clone());
                let flag = if ascending == nulls_first { Expr::IsNotNull(inner) } else { Expr::IsNull(inner) };
                let name =
                    if let Some((_, _, name)) = flags.iter().find(|(at, expr, _)| *at == relation && *expr == flag) {
                        name.clone()
                    } else {
                        let name = ident(&counters.name("__n"));
                        flags.push((relation, flag, name.clone()));
                        name
                    };
                keys.push(OrderByExpr {
                    expr: Expr::Identifier(name),
                    options: OrderByOptions { asc: key.options.asc, nulls_first: None },
                    with_fill: None,
                });
                keys.push(OrderByExpr { options: OrderByOptions { asc: key.options.asc, nulls_first: None }, ..key });
            }
            spec.order_by = keys;
            true
        });
    }
    if let Some(error) = failure {
        return Err(error);
    }
    if flags.is_empty() {
        return Ok(());
    }
    let all: Vec<Ident> = flags.iter().map(|(_, _, name)| name.clone()).collect();
    exclude_from_wildcards(select, &all);
    compute_flags(select, &flags, counters);
    Ok(())
}

/// Wraps each relation with flags in a derived table computing them, under the relation's name.
fn compute_flags(select: &mut Select, flags: &[(usize, Expr, Ident)], counters: &mut Counters) {
    let mut at = 0;
    for table in &mut select.from {
        for factor in std::iter::once(&mut table.relation).chain(table.joins.iter_mut().map(|join| &mut join.relation))
        {
            let mine: Vec<&(usize, Expr, Ident)> = flags.iter().filter(|(relation, ..)| *relation == at).collect();
            if !mine.is_empty() {
                // The relation's identifier as written, quotes kept: a quoted alias can hold any text.
                let name = relation_ident(factor).cloned().unwrap_or_else(|| ident(&counters.name("__w")));
                let mut projection = vec![SelectItem::Wildcard(WildcardAdditionalOptions::default())];
                projection.extend(
                    mine.iter()
                        .map(|(_, expr, name)| SelectItem::ExprWithAlias { expr: expr.clone(), alias: name.clone() }),
                );
                let original = std::mem::replace(
                    factor,
                    TableFactor::Table {
                        name: sqlparser::ast::ObjectName::from(vec![name.clone()]),
                        alias: None,
                        args: None,
                        with_hints: vec![],
                        version: None,
                        with_ordinality: false,
                        partitions: vec![],
                        json_path: None,
                        sample: None,
                        index_hints: vec![],
                    },
                );
                let inner = query_of(SetExpr::Select(Box::new(super::ast::select(projection, original))));
                *factor = derived(inner, alias(name.clone(), &[]));
                let qualified: Vec<Ident> = mine.iter().map(|(_, _, name)| name.clone()).collect();
                for item in &mut select.projection {
                    if let SelectItem::QualifiedWildcard(
                        SelectItemQualifiedWildcardKind::ObjectName(qualifier),
                        options,
                    ) = item
                        && matches!(qualifier.0.as_slice(), [part] if part.as_ident().is_some_and(|q| q.value.eq_ignore_ascii_case(&name.value)))
                    {
                        exclude(options, &qualified);
                    }
                }
            }
            at += 1;
        }
    }
}

/// Uncorrelated scalar subqueries become `CROSS JOIN (…) AS __sK` read as `__sK.__vK`.
fn scalar_subqueries(select: &mut Select, counters: &mut Counters) {
    let grouped = aggregated(select);
    let mut joins = Vec::new();
    let mut values = Vec::new();
    let mut replace = |expr: &mut Expr, wrap_outside_aggregate: bool, joins: &mut Vec<Join>| {
        replace_subqueries(expr, wrap_outside_aggregate, false, &mut |mut query| {
            let source = ident(&counters.name("__s"));
            let value = ident(&counters.name("__v"));
            if let SetExpr::Select(inner) = &mut *query.body
                && let Some(item) = inner.projection.first_mut()
                && let SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } = item
            {
                *item = SelectItem::ExprWithAlias { expr: expr.clone(), alias: value.clone() };
            }
            joins.push(Join {
                relation: derived(query, alias(source.clone(), &[])),
                global: false,
                join_operator: JoinOperator::CrossJoin(JoinConstraint::None),
            });
            values.push(value.clone());
            column_of(&source, &value)
        });
    };
    for item in &mut select.projection {
        if let SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } = item {
            replace(expr, grouped, &mut joins);
        }
    }
    if let Some(expr) = &mut select.selection {
        replace(expr, false, &mut joins);
    }
    if let Some(expr) = &mut select.having {
        replace(expr, true, &mut joins);
    }
    if joins.is_empty() {
        return;
    }
    if let Some(table) = select.from.first_mut() {
        table.joins.extend(joins);
    }
    exclude_from_wildcards(select, &values);
}

/// Replaces each scalar subquery of `expr` by `read(query)`, wrapped in `MAX(…)` outside an
/// aggregate when `wrap` (an aggregated select list, or HAVING).
fn replace_subqueries(
    expr: &mut Expr,
    wrap: bool,
    in_aggregate: bool,
    read: &mut dyn FnMut(sqlparser::ast::Query) -> Expr,
) {
    if let Expr::Subquery(query) = expr {
        let column = read((**query).clone());
        *expr = if wrap && !in_aggregate { call("max", vec![column]) } else { column };
        return;
    }
    let aggregate = in_aggregate
        || matches!(expr, Expr::Function(function) if function.over.is_none()
            && function_name(function).is_some_and(|name| functions::is_aggregate(&name)));
    for child in children_mut(expr) {
        replace_subqueries(child, wrap, aggregate, read);
    }
}
