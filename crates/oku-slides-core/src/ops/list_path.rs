//! Chemin d'une liste : `param`, ou `param/item/colonne` pour la liste imbriquée
//! que porte la colonne `list` d'un item (un seul niveau, t-495). Le champ d'un
//! item imbriqué s'adresse `param/item/colonne/sous-item/sous-colonne`.

use super::list::{ListOp, ListSpec, position};
use crate::error::{OpError, ParamTypeSnafu, Shape};
use crate::{ListItem, ParamSchema, ParamType, ParamValue, Params};

/// Une liste adressée par son chemin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListPath<'a> {
    /// Le chemin entier, tel qu'écrit ; nom de la liste dans les erreurs.
    pub path: &'a str,
    /// Le paramètre `list` du widget.
    pub param: &'a str,
    /// Pour une liste imbriquée : l'id de l'item du paramètre et sa colonne `list`.
    pub nested: Option<(&'a str, &'a str)>,
}

impl<'a> ListPath<'a> {
    /// Lit `param` ou `param/item/colonne` ; `None` pour toute autre forme (segment vide compris).
    #[must_use]
    pub fn parse(path: &'a str) -> Option<Self> {
        let parts: Vec<&'a str> = path.split('/').collect();
        if parts.iter().any(|p| p.is_empty()) {
            return None;
        }
        match parts.as_slice() {
            [param] => Some(Self { path, param, nested: None }),
            [param, item, column] => Some(Self { path, param, nested: Some((item, column)) }),
            _ => None,
        }
    }

    /// Le schéma de la liste parmi les paramètres `params` d'un widget : le paramètre, ou
    /// la colonne `list` de ses items ; `None` si elle n'est pas déclarée en liste.
    #[must_use]
    pub fn schema<'s>(&self, params: &'s [ParamSchema]) -> Option<&'s ParamSchema> {
        let p = params.iter().find(|p| p.name == self.param)?;
        let ParamType::List { item, .. } = &p.ty else { return None };
        match self.nested {
            None => Some(p),
            Some((_, column)) => item.iter().find(|c| c.name == column && matches!(c.ty, ParamType::List { .. })),
        }
    }

    /// Les items de la liste dans `params` ; une liste absente est vide. `None` si l'item
    /// qui porte la liste imbriquée n'existe pas.
    #[must_use]
    pub fn items<'p>(&self, params: &'p Params) -> Option<&'p [ListItem]> {
        let top = params.list(self.param).unwrap_or_default();
        match self.nested {
            None => Some(top),
            Some((item, column)) => Some(top.iter().find(|i| i.id == item)?.values.list(column).unwrap_or_default()),
        }
    }

    /// Applies `change` to the list at the path.
    ///
    /// `top` is the parameter's list, of schema `spec`. On error, `top` is intact.
    /// A nested list left empty is removed from its item (absent = empty).
    ///
    /// The nested items of the whole list stay under [`NESTED_ITEMS_MAX`](super::list::NESTED_ITEMS_MAX).
    ///
    /// # Errors
    ///
    /// Item of the path not found, column that is not a list, or
    /// change refused by [`ListOp::apply`], nested bounds included.
    pub fn apply(&self, top: &mut Vec<ListItem>, spec: &ListSpec<'_>, change: ListOp) -> Result<(), OpError> {
        let Some((item, column)) = self.nested else {
            // Un item inséré amorce ses listes imbriquées : le total se contrôle sur une copie.
            let mut next = top.clone();
            change.apply(&mut next, spec)?;
            spec.nested_total_fits(&next)?;
            *top = next;
            return Ok(());
        };
        let i = position(spec, top, item)?;
        let c = spec.column(column)?;
        let sub = ListSpec::of(self.path, &c.ty).ok_or_else(|| {
            OpError::from(ParamTypeSnafu { param: self.path, expected: Shape::List.required() }.build())
        })?;
        let mut items = top[i].values.list(column).map(<[_]>::to_vec).unwrap_or_default();
        let before = items.len();
        change.apply(&mut items, &sub)?;
        // Total des items imbriqués de la liste, celle-ci changée.
        if items.len() > before && spec.nested_total(top) + items.len() - before > super::list::NESTED_ITEMS_MAX {
            return Err(spec.refuse_total());
        }
        let values = &mut top[i].values;
        if items.is_empty() {
            values.remove(column);
        } else {
            values.set(column, ParamValue::List(items));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "list_path_tests.rs"]
mod list_path_tests;
