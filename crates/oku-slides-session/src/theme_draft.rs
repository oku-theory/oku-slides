//! A theme tab's draft: its closure, its layer, the manifest it saves.
//!
//! The draft renders on the linearized chain of its parents
//! ([`ThemeEdit::extends`]): under its namespace for a user theme (a loop
//! that comes back to it is refused), under a synthetic root for a child of
//! a built-in ([`ThemeEdit::child`]), which has none yet. The base and the
//! parents are read from the versions the specimen deck embeds, else from
//! the library.
//!
//! A draft that differs from its starting point (tokens, name or parents)
//! saves as a version of the library (`theme_tab_save`): of a child of the
//! edited built-in (a new id, a namespace from its name, only the edited
//! tokens, named "(variante)" if the name did not change), of a copy, or of
//! the same user theme (same namespace); all extend the draft's list.

use serde_json::{Map, Value};
use snafu::OptionExt;
use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, PackVersion, SourceBundle, ThemePin};

use crate::error::ThemeVersionMissingSnafu;
use crate::namespace_slug::library_namespace;
use crate::pack::now_created;
use crate::theme::ThemeManifest;
use crate::theme_closure::{
    Closure, chain_tokens, closure_of_parents, closure_with, embedded_or_library, parents_order, tokens_in_order,
};
use crate::theme_edit::{ThemeEdit, ThemeParentView, draft_differs};
use crate::{Result, Session};

impl Session {
    /// The sources the draft's themes are read from: the specimen deck's (a theme tab sets no deck aside).
    fn draft_sources(&self) -> Vec<&SourceBundle> {
        vec![&self.doc.sources]
    }

    /// The closure of theme `ns` whose version `root` (pin and files) is given.
    ///
    /// Its parents come from the embedded versions, else from the library.
    pub(crate) fn closure_from(&self, ns: &Namespace, root: &(ThemePin, PackFiles)) -> Result<Closure> {
        let sources = self.draft_sources();
        closure_with(ns, |n| if n == ns { Some(root.clone()) } else { embedded_or_library(&self.themes, &sources, n) })
    }

    /// Les jetons de chaque thème sous le brouillon `edit`, dans l'ordre de la chaîne.
    ///
    /// Un thème de l'utilisateur : la chaîne linéarisée de sa base, ses
    /// parents remplacés par ceux du brouillon, sans elle ; une boucle qui
    /// revient à son espace de noms est `ThemeCycle`. Un enfant : la chaîne de
    /// ses seuls parents ([`closure_of_parents`]). Une liste vide n'en donne aucune.
    pub(crate) fn draft_chain(&self, edit: &ThemeEdit) -> Result<Vec<Map<String, Value>>> {
        if edit.child() {
            let sources = self.draft_sources();
            let closure = closure_of_parents(&edit.extends, |n| embedded_or_library(&self.themes, &sources, n))?;
            return tokens_in_order(&closure, &parents_order(&edit.extends, &closure.0)?);
        }
        let base = &edit.base;
        let missing = || ThemeVersionMissingSnafu { id: base.id, version: base.version.clone() };
        let files = self.theme_files(base).context(missing())?;
        let root = ThemePin { extends: edit.extends.clone(), ..base.clone() };
        let closure = self.closure_from(&edit.namespace, &(root, files))?;
        chain_tokens(&closure, &edit.namespace, false)
    }

    /// Les thèmes qu'étend le brouillon `edit`, nommés : un parent introuvable
    /// prend son espace de noms pour nom.
    pub(crate) fn draft_parents(&self, edit: &ThemeEdit) -> Vec<ThemeParentView> {
        let sources = self.draft_sources();
        edit.extends
            .iter()
            .map(|ns| {
                let found = embedded_or_library(&self.themes, &sources, ns)
                    .and_then(|(_, files)| ThemeManifest::from_files(&files).ok());
                let name = found.map_or_else(
                    || {
                        tracing::warn!(theme = %ns, "parent du thème introuvable : son espace de noms en tient lieu");
                        ns.as_str().to_owned()
                    },
                    |m| m.name,
                );
                ThemeParentView { namespace: ns.to_string(), name }
            })
            .collect()
    }

    /// The manifest the draft `edit` of tokens `tokens` saves, from its starting point `base`.
    ///
    /// `None` when the draft does not differ, unless `as_new`. A child of a
    /// built-in, or a copy (`as_new`), is a new theme: a new id, a namespace
    /// from its name, the draft's parents, no parent version; named
    /// "(variante)" (a child) or "(copie)" (a copy) when the name did not
    /// change. Otherwise the same user theme, parented on `parent`, the
    /// version the tab's writes descend from (never `edit.base`).
    pub(crate) fn saved_manifest(
        &self,
        (edit, parent): (&ThemeEdit, Option<PackVersion>),
        base: ThemeManifest,
        tokens: Map<String, Value>,
        as_new: bool,
    ) -> Result<Option<ThemeManifest>> {
        if !as_new && !draft_differs(edit, &base, &tokens) {
            return Ok(None);
        }
        let manifest = if as_new || edit.child() {
            let name = if edit.name == base.name {
                let key = if as_new { "theme-copy-of" } else { "theme-copy-name" };
                self.localizer.tr_with(key, &[("name", base.name.as_str().into())])
            } else {
                edit.name.clone()
            };
            let namespace = library_namespace(&self.library, &name)?;
            let (id, extends) = (Uuid::new_v4(), edit.extends.clone());
            let created = now_created();
            ThemeManifest { id, name, namespace, extends, description: base.description, parent: None, created, tokens }
        } else {
            ThemeManifest {
                id: edit.base.id,
                name: edit.name.clone(),
                extends: edit.extends.clone(),
                parent,
                created: now_created(),
                tokens,
                ..base
            }
        };
        self.library.check_theme(manifest.id, &manifest.namespace)?;
        Ok(Some(manifest))
    }
}

#[cfg(test)]
#[path = "theme_draft_tests.rs"]
mod theme_draft_tests;
