//! The data names a query reads, and the query reading a renamed data.
//!
//! A table name is an input unless a `WITH` table of an enclosing query, or an earlier one of
//! the same `WITH`, has that name. Renaming changes inputs and the column qualifiers naming them;
//! columns and `WITH` tables keep their names. The names a query binds itself are listed so a
//! caller can refuse a new name they would capture.

use std::collections::{BTreeSet, HashMap};
use std::ops::ControlFlow;

use sqlparser::ast::{
    Expr, Ident, ObjectName, ObjectNamePart, Query, SelectItem, SelectItemQualifiedWildcardKind, SetExpr, TableFactor,
    VisitMut, Visitor, VisitorMut,
};
use sqlparser::keywords::ALL_KEYWORDS;

use super::ast::for_each_select;

/// The lowercased data names `query` reads.
pub(super) fn inputs(query: &Query) -> BTreeSet<String> {
    let mut renamer = Renamer::new(None);
    let _ = query.clone().visit(&mut renamer);
    renamer.inputs
}

/// The lowercased names `query` gives its own relations: its `WITH` tables and its relation
/// aliases, at any depth.
pub(super) fn bound_names(query: &Query) -> BTreeSet<String> {
    let mut bound = Bound(BTreeSet::new());
    // Named in full: `Visit` in scope would make `visit` on a query ambiguous with `VisitMut`.
    let _ = sqlparser::ast::Visit::visit(query, &mut bound);
    bound.0
}

struct Bound(BTreeSet<String>);

impl Visitor for Bound {
    type Break = ();

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<()> {
        let ctes = query.with.iter().flat_map(|with| &with.cte_tables);
        self.0.extend(ctes.map(|cte| cte.alias.name.value.to_ascii_lowercase()));
        ControlFlow::Continue(())
    }

    fn pre_visit_table_factor(&mut self, factor: &TableFactor) -> ControlFlow<()> {
        if let TableFactor::Table { alias: Some(alias), .. }
        | TableFactor::Derived { alias: Some(alias), .. }
        | TableFactor::Pivot { alias: Some(alias), .. }
        | TableFactor::Unpivot { alias: Some(alias), .. } = factor
        {
            self.0.insert(alias.name.value.to_ascii_lowercase());
        }
        ControlFlow::Continue(())
    }
}

/// `query` reading data `new` wherever it read data `old` (any case).
pub(super) fn renamed(query: &Query, old: &str, new: &str) -> Query {
    let mut query = query.clone();
    let mut renamer = Renamer::new(Some((old.to_ascii_lowercase(), quoted_ident(new))));
    let _ = query.visit(&mut renamer);
    query
}

/// `name` as an identifier, quoted when it is a keyword or not a plain word.
fn quoted_ident(name: &str) -> Ident {
    let plain = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let keyword = ALL_KEYWORDS.binary_search(&name.to_ascii_uppercase().as_str()).is_ok();
    if plain && !keyword { Ident::new(name) } else { Ident::with_quote('"', name) }
}

struct Renamer {
    /// The rename: the lowercased old name and the new identifier.
    rename: Option<(String, Ident)>,
    /// `WITH` names visible in each enclosing query, innermost last.
    scopes: Vec<BTreeSet<String>>,
    /// `WITH` names visible to a `WITH` table's query, keyed by its address.
    pending: HashMap<usize, BTreeSet<String>>,
    inputs: BTreeSet<String>,
}

impl Renamer {
    fn new(rename: Option<(String, Ident)>) -> Self {
        Renamer { rename, scopes: vec![], pending: HashMap::new(), inputs: BTreeSet::new() }
    }

    fn visible(&self, name: &str) -> bool {
        self.scopes.last().is_some_and(|scope| scope.contains(name))
    }

    fn is_old(&self, ident: &Ident) -> bool {
        self.rename.as_ref().is_some_and(|(old, _)| ident.value.to_ascii_lowercase() == *old)
    }
}

impl VisitorMut for Renamer {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<()> {
        let key = std::ptr::from_ref::<Query>(query) as usize;
        let base = self.pending.remove(&key).or_else(|| self.scopes.last().cloned()).unwrap_or_default();
        let mut names = base.clone();
        for cte in query.with.iter().flat_map(|with| &with.cte_tables) {
            self.pending.insert(std::ptr::from_ref::<Query>(&cte.query) as usize, names.clone());
            names.insert(cte.alias.name.value.to_ascii_lowercase());
        }
        let this = &*self;
        let mut qualified = false;
        let _: Result<(), ()> = for_each_select(&mut query.body, &mut |select| {
            let Some((_, new)) = &this.rename else { return Ok(()) };
            let mut factors = select.from.iter().flat_map(|table| {
                std::iter::once(&table.relation).chain(table.joins.iter().map(|join| &join.relation))
            });
            let reads_old = factors.any(|factor| match factor {
                TableFactor::Table { name, alias: None, .. } => match name.0.as_slice() {
                    [ObjectNamePart::Identifier(ident)] => {
                        this.is_old(ident) && !names.contains(&ident.value.to_ascii_lowercase())
                    }
                    _ => false,
                },
                _ => false,
            });
            if !reads_old {
                return Ok(());
            }
            // Per select: another branch of a set operation may read another table under the old name.
            let mut qualifiers = Qualifiers { new, depth: 0, renamer: this };
            let _ = select.visit(&mut qualifiers);
            qualified = true;
            for item in &mut select.projection {
                if let SelectItem::QualifiedWildcard(SelectItemQualifiedWildcardKind::ObjectName(name), _) = item
                    && let [ObjectNamePart::Identifier(ident)] = name.0.as_slice()
                    && this.is_old(ident)
                {
                    *name = ObjectName::from(vec![new.clone()]);
                }
            }
            Ok(())
        });
        // A single select's `ORDER BY` sits on its query and reads the select's relations.
        if qualified
            && matches!(*query.body, SetExpr::Select(_))
            && let (Some(order_by), Some((_, new))) = (&mut query.order_by, &this.rename)
        {
            let _ = order_by.visit(&mut Qualifiers { new, depth: 0, renamer: this });
        }
        self.scopes.push(names);
        ControlFlow::Continue(())
    }

    fn post_visit_query(&mut self, _query: &mut Query) -> ControlFlow<()> {
        self.scopes.pop();
        ControlFlow::Continue(())
    }

    fn pre_visit_relation(&mut self, relation: &mut ObjectName) -> ControlFlow<()> {
        let [ObjectNamePart::Identifier(ident)] = relation.0.as_slice() else { return ControlFlow::Continue(()) };
        let lower = ident.value.to_ascii_lowercase();
        if self.visible(&lower) {
            return ControlFlow::Continue(());
        }
        if self.is_old(ident)
            && let Some((_, new)) = &self.rename
        {
            *relation = ObjectName::from(vec![new.clone()]);
            self.inputs.insert(new.value.to_ascii_lowercase());
        } else {
            self.inputs.insert(lower);
        }
        ControlFlow::Continue(())
    }
}

/// Renames the column qualifiers naming the old data in one select, outside its subqueries
/// (each subquery is its own query, which the [`Renamer`] decides for).
struct Qualifiers<'r> {
    new: &'r Ident,
    /// Subqueries entered: qualifiers are renamed at depth 0 only.
    depth: usize,
    renamer: &'r Renamer,
}

impl VisitorMut for Qualifiers<'_> {
    type Break = ();

    fn pre_visit_query(&mut self, _query: &mut Query) -> ControlFlow<()> {
        self.depth += 1;
        ControlFlow::Continue(())
    }

    fn post_visit_query(&mut self, _query: &mut Query) -> ControlFlow<()> {
        self.depth -= 1;
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<()> {
        if self.depth == 0
            && let Expr::CompoundIdentifier(parts) = expr
            && let [qualifier, _] = parts.as_mut_slice()
            && self.renamer.is_old(qualifier)
        {
            *qualifier = self.new.clone();
        }
        ControlFlow::Continue(())
    }
}

#[cfg(test)]
#[path = "rename_tests.rs"]
mod rename_tests;
