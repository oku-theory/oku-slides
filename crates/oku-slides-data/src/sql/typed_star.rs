//! `*` over a join, named as `DuckDB` names a stored result (`product`, `product_1`…), and a
//! select alias read in `WHERE`.

use std::collections::{BTreeMap, BTreeSet};

use sqlparser::ast::{
    ExcludeSelectItem, Expr, Ident, JoinConstraint, JoinOperator, ObjectNamePart, Select, SelectItem,
    SelectItemQualifiedWildcardKind, TableFactor, WildcardAdditionalOptions,
};

use super::ast::each_mut;
use super::typed::{Walk, quoted};
use super::whitelist::{refuse, relation_ident};
use crate::DataError;

/// One relation of a `FROM`, as `*` reads it.
struct Relation {
    qualifier: Ident,
    /// Lowercased names `*` skips: `USING` keys and `NATURAL` columns already shown on the left.
    shared: BTreeSet<String>,
    columns: Vec<String>,
}

/// Expands every `*` and `t.*` of `select` when it reads more than one relation.
pub(super) fn expand(walk: &mut Walk<'_>, select: &mut Select) -> Result<(), DataError> {
    let joined = select.from.len() > 1 || select.from.iter().any(|table| !table.joins.is_empty());
    let starred = select
        .projection
        .iter()
        .any(|item| matches!(item, SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..)));
    if !joined || !starred {
        return Ok(());
    }
    let relations = relations(walk, select)?;
    let mut taken: BTreeSet<String> = select
        .projection
        .iter()
        .filter_map(|item| match item {
            SelectItem::ExprWithAlias { alias, .. } => Some(alias.value.to_ascii_lowercase()),
            SelectItem::UnnamedExpr(Expr::Identifier(name)) => Some(name.value.to_ascii_lowercase()),
            SelectItem::UnnamedExpr(Expr::CompoundIdentifier(parts)) => {
                parts.last().map(|name| name.value.to_ascii_lowercase())
            }
            _ => None,
        })
        .collect();
    let mut projection = Vec::new();
    for item in std::mem::take(&mut select.projection) {
        match item {
            SelectItem::Wildcard(options) => {
                let shown = relations.iter().flat_map(|r| {
                    r.columns.iter().filter(|c| !r.shared.contains(&c.to_ascii_lowercase())).map(move |c| (r, c))
                });
                projection.extend(items(shown, &options, &mut taken));
            }
            SelectItem::QualifiedWildcard(SelectItemQualifiedWildcardKind::ObjectName(name), options) => {
                let Some(ObjectNamePart::Identifier(qualifier)) = name.0.last() else {
                    return Err(refuse(format!("{name}.*")));
                };
                let relation = relations
                    .iter()
                    .find(|r| r.qualifier.value.eq_ignore_ascii_case(&qualifier.value))
                    .ok_or_else(|| refuse(format!("{name}.*")))?;
                let shown = relation.columns.iter().map(|c| (relation, c));
                projection.extend(items(shown, &options, &mut taken));
            }
            other => projection.push(other),
        }
    }
    select.projection = projection;
    Ok(())
}

/// The relations of `select`'s `FROM`, in order, with their columns.
fn relations(walk: &mut Walk<'_>, select: &Select) -> Result<Vec<Relation>, DataError> {
    let mut relations: Vec<Relation> = Vec::new();
    for table in &select.from {
        let factors = std::iter::once((&table.relation, None))
            .chain(table.joins.iter().map(|join| (&join.relation, Some(&join.join_operator))));
        for (factor, operator) in factors {
            let qualifier = qualifier(factor)?;
            let columns: Vec<String> = walk.factor_columns(factor)?.into_iter().map(|(name, _)| name).collect();
            let shared = match operator.and_then(constraint) {
                Some(JoinConstraint::Using(names)) => names
                    .iter()
                    .filter_map(|name| name.0.last()?.as_ident().map(|i| i.value.to_ascii_lowercase()))
                    .collect(),
                Some(JoinConstraint::Natural) => {
                    let left: BTreeSet<String> =
                        relations.iter().flat_map(|r| r.columns.iter().map(|c| c.to_ascii_lowercase())).collect();
                    columns.iter().map(|c| c.to_ascii_lowercase()).filter(|c| left.contains(c)).collect()
                }
                _ => BTreeSet::new(),
            };
            relations.push(Relation { qualifier, shared, columns });
        }
    }
    Ok(relations)
}

fn qualifier(factor: &TableFactor) -> Result<Ident, DataError> {
    relation_ident(factor).cloned().ok_or_else(|| refuse(format!("{factor}.*")))
}

fn constraint(operator: &JoinOperator) -> Option<&JoinConstraint> {
    match operator {
        JoinOperator::Join(c)
        | JoinOperator::Inner(c)
        | JoinOperator::Left(c)
        | JoinOperator::LeftOuter(c)
        | JoinOperator::Right(c)
        | JoinOperator::RightOuter(c)
        | JoinOperator::FullOuter(c)
        | JoinOperator::CrossJoin(c) => Some(c),
        _ => None,
    }
}

/// The items of one `*`: `EXCLUDE` drops names, `REPLACE` swaps expressions, duplicates get `_1`….
fn items<'r>(
    shown: impl Iterator<Item = (&'r Relation, &'r String)>,
    options: &WildcardAdditionalOptions,
    taken: &mut BTreeSet<String>,
) -> Vec<SelectItem> {
    let excluded: BTreeSet<String> = match &options.opt_exclude {
        Some(ExcludeSelectItem::Single(name)) => BTreeSet::from([name.value.to_ascii_lowercase()]),
        Some(ExcludeSelectItem::Multiple(names)) => names.iter().map(|name| name.value.to_ascii_lowercase()).collect(),
        None => BTreeSet::new(),
    };
    let replaced: BTreeMap<String, Expr> = options
        .opt_replace
        .iter()
        .flat_map(|replace| &replace.items)
        .map(|element| (element.column_name.value.to_ascii_lowercase(), element.expr.clone()))
        .collect();
    let mut items = Vec::new();
    for (relation, column) in shown {
        let lower = column.to_ascii_lowercase();
        if excluded.contains(&lower) {
            continue;
        }
        let expr = replaced
            .get(&lower)
            .cloned()
            .unwrap_or_else(|| Expr::CompoundIdentifier(vec![relation.qualifier.clone(), quoted(column)]));
        let mut name = column.clone();
        let mut n = 0;
        while !taken.insert(name.to_ascii_lowercase()) {
            n += 1;
            name = format!("{column}_{n}");
        }
        items.push(SelectItem::ExprWithAlias { expr, alias: quoted(&name) });
    }
    items
}

/// Replaces a select alias read in `WHERE` by its expression, as `DuckDB` resolves it when no
/// column of the `FROM` has that name.
pub(super) fn where_aliases(walk: &mut Walk<'_>, select: &mut Select) -> Result<(), DataError> {
    let aliases: BTreeMap<String, Expr> = select
        .projection
        .iter()
        .filter_map(|item| match item {
            SelectItem::ExprWithAlias { expr, alias } => Some((alias.value.to_ascii_lowercase(), expr.clone())),
            _ => None,
        })
        .collect();
    let Some(selection) = &mut select.selection else { return Ok(()) };
    let mut named = false;
    super::ast::each(selection, &mut |expr| {
        named |= matches!(expr, Expr::Identifier(name) if aliases.contains_key(&name.value.to_ascii_lowercase()));
    });
    if !named {
        return Ok(());
    }
    let columns: BTreeSet<String> =
        walk.relation_columns(&select.from)?.into_iter().map(|(name, _)| name.to_ascii_lowercase()).collect();
    each_mut(selection, &mut |expr| {
        if let Expr::Identifier(name) = expr
            && let Some(aliased) = aliases.get(&name.value.to_ascii_lowercase())
            && !columns.contains(&name.value.to_ascii_lowercase())
        {
            *expr = Expr::Nested(Box::new(aliased.clone()));
            return false;
        }
        true
    });
    Ok(())
}
