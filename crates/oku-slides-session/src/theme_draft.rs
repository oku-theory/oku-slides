//! Le brouillon du thème du mode pack : sa fermeture, son calque, son enregistrement.
//!
//! Le brouillon se rend sur la chaîne linéarisée des parents de sa base, ou
//! de la base elle-même pour un enfant d'un intégré ([`ThemeEdit::child`]) :
//! la base et ses parents se lisent dans les versions embarquées (deck mis de
//! côté, puis deck de travail), sinon dans la bibliothèque.
//!
//! À l'enregistrement du pack, un brouillon qui diffère de son point de
//! départ (jetons ou nom) devient une version courante de la bibliothèque :
//! d'un enfant de l'intégré édité (nouvel id, espace de noms tiré de son nom,
//! `extends: [<intégré>]`, les seuls jetons édités, nommé « (variante) » si
//! le nom n'a pas changé), ou du même thème de l'utilisateur (même espace de
//! noms, mêmes parents). Sinon, une base autre que le thème du deck est
//! adoptée telle quelle. Le deck mis de côté embarque la fermeture du thème
//! retenu, ses parents pris aux mêmes sources, et y passe, dans l'étape
//! d'annulation du pack.

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, SourceBundle, ThemePin};

use crate::deck_theme::{manifest_of, pin_closure};
use crate::error::{NotEditingPackSnafu, SourcesSnafu, ThemeJsonSnafu, ThemeVersionMissingSnafu};
use crate::namespace_slug::library_namespace;
use crate::pack::now_created;
use crate::pack_mode_save::adopt_merge_key;
use crate::theme::ThemeManifest;
use crate::theme_closure::{Closure, chain_tokens, closure_with, embedded_or_library, pin_of};
use crate::theme_edit::ThemeEdit;
use crate::{Mode, Result, Session};

impl Session {
    /// Les sources où se lisent les thèmes du brouillon : deck mis de côté, puis deck de travail.
    fn draft_sources(&self) -> Vec<&SourceBundle> {
        match &self.mode {
            Mode::Pack { stash, .. } => vec![&stash.sources, &self.doc.sources],
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => vec![&self.doc.sources],
        }
    }

    /// La fermeture du thème `ns`, dont la version `root` (épinglage et
    /// fichiers) est donnée ; ses parents viennent des versions embarquées,
    /// sinon de la bibliothèque.
    fn closure_from(&self, ns: &Namespace, root: (ThemePin, PackFiles)) -> Result<Closure> {
        let sources = self.draft_sources();
        closure_with(ns, |n| match n == ns {
            true => Some(root.clone()),
            false => embedded_or_library(&self.themes, &sources, n),
        })
    }

    /// La fermeture de la base de `edit`, et son espace de noms.
    fn draft_closure(&self, edit: &ThemeEdit) -> Result<(Namespace, Closure)> {
        let base = &edit.base;
        let missing = || ThemeVersionMissingSnafu { id: base.id, version: base.version.clone() };
        let files = self.theme_files(base).context(missing())?;
        let ns = manifest_of(base, &files)?.namespace;
        let closure = self.closure_from(&ns, (base.clone(), files))?;
        Ok((ns, closure))
    }

    /// Les jetons de chaque thème sous le brouillon `edit`, dans l'ordre de la chaîne.
    ///
    /// La chaîne linéarisée de sa base sans elle (ses parents), ou avec elle pour un enfant.
    pub(crate) fn draft_chain(&self, edit: &ThemeEdit) -> Result<Vec<Map<String, Value>>> {
        let (ns, closure) = self.draft_closure(edit)?;
        chain_tokens(&closure, &ns, edit.child())
    }

    /// Les noms des thèmes qu'étendra le brouillon `edit` enregistré : l'intégré
    /// pour un enfant, sinon les parents de sa base.
    pub(crate) fn draft_extends_names(&self, edit: &ThemeEdit) -> Vec<String> {
        let Some(base) = self.theme_version(&edit.base) else { return Vec::new() };
        let parents = if edit.child() { vec![base.namespace] } else { base.extends };
        let sources = self.draft_sources();
        parents
            .iter()
            .map(|ns| {
                let found = embedded_or_library(&self.themes, &sources, ns)
                    .and_then(|(_, files)| ThemeManifest::from_files(&files).ok());
                found.map(|m| m.name).unwrap_or_else(|| {
                    tracing::warn!(theme = %ns, "parent du thème introuvable : son espace de noms en tient lieu");
                    ns.as_str().to_owned()
                })
            })
            .collect()
    }

    /// Le manifeste qu'enregistre le brouillon `edit` de jetons `tokens`, s'il
    /// diffère de son point de départ `base`.
    fn saved_manifest(
        &self,
        edit: &ThemeEdit,
        base: ThemeManifest,
        tokens: Map<String, Value>,
    ) -> Result<Option<ThemeManifest>> {
        let origin = if edit.child() { Map::new() } else { base.tokens.clone() };
        if tokens == origin && edit.name == base.name {
            return Ok(None);
        }
        let manifest = match edit.child() {
            true => {
                let name = match edit.name == base.name {
                    true => self.localizer.tr_with("theme-copy-name", &[("name", base.name.as_str().into())]),
                    false => edit.name.clone(),
                };
                let namespace = library_namespace(&self.library, &name)?;
                let (id, extends) = (Uuid::new_v4(), vec![base.namespace]);
                let created = now_created();
                ThemeManifest {
                    id,
                    name,
                    namespace,
                    extends,
                    description: base.description,
                    parent: None,
                    created,
                    tokens,
                }
            }
            false => ThemeManifest {
                id: edit.base.id,
                name: edit.name.clone(),
                parent: Some(edit.base.version.clone()),
                created: now_created(),
                tokens,
                ..base
            },
        };
        self.library.check_theme(manifest.id, &manifest.namespace)?;
        Ok(Some(manifest))
    }

    /// Enregistre le brouillon du thème ; le thème que le deck mis de côté
    /// doit prendre (`None` : il garde le sien).
    ///
    /// Voir le module : un enfant d'un intégré, ou une nouvelle version d'un
    /// thème de l'utilisateur ; sa fermeture est vérifiée avant l'écriture.
    pub(crate) fn save_theme_edit(&mut self) -> Result<Option<(Namespace, ThemePin)>> {
        let Mode::Pack { theme, .. } = &self.mode else {
            return NotEditingPackSnafu.fail()?;
        };
        let edit = theme.clone();
        let base = self
            .theme_version(&edit.base)
            .context(ThemeVersionMissingSnafu { id: edit.base.id, version: edit.base.version.clone() })?;
        let tokens = self.edited_tokens();
        let base_ns = base.namespace.clone();
        let (pin, ns) = match self.saved_manifest(&edit, base, tokens)? {
            Some(manifest) => {
                let id = manifest.id;
                let files = manifest.to_files().context(ThemeJsonSnafu { id })?;
                let version = files.version();
                let pin = pin_of(&manifest, version.clone());
                // Ses parents doivent se trouver avant toute écriture.
                self.closure_from(&manifest.namespace, (pin.clone(), files.clone()))?;
                self.themes.add_version(id, &files)?;
                self.themes.set_current(id, &version)?;
                self.theme_list = self.themes.themes();
                (pin, manifest.namespace)
            }
            None if self.deck_theme_pin() != Some(&edit.base) => (edit.base.clone(), base_ns),
            None => return Ok(None),
        };
        let files =
            self.theme_files(&pin).context(ThemeVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        let name = ThemeManifest::from_files(&files).context(ThemeJsonSnafu { id: pin.id })?.name;
        if let Mode::Pack { theme, switches, .. } = &mut self.mode {
            let saved = ThemeEdit::of(pin.clone(), name);
            // L'historique du deck de travail reste : les changements de base
            // qui menaient à l'ancienne base mènent à la version enregistrée.
            for s in switches.iter_mut() {
                for edit in [&mut s.from, &mut s.to] {
                    if edit.base == theme.base {
                        *edit = saved.clone();
                    }
                }
            }
            *theme = saved;
        }
        Ok(Some((ns, pin)))
    }

    /// Le deck mis de côté passe au thème `ns` dans la version `pin`, et à ses
    /// parents (versions embarquées, sinon courantes de la bibliothèque), tous
    /// embarqués, dans l'étape d'annulation de l'enregistrement du pack qui vient d'avoir lieu.
    pub(crate) fn stash_adopt_theme(&mut self, ns: Namespace, pin: ThemePin) -> Result<()> {
        let files =
            self.theme_files(&pin).context(ThemeVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        let closure = self.closure_from(&ns, (pin, files))?;
        let Mode::Pack { stash, edit, .. } = &mut self.mode else {
            return NotEditingPackSnafu.fail()?;
        };
        let label = self.localizer.tr_with("pack-adopt-label", &[("name", edit.name.as_str().into())]);
        let merge_key = adopt_merge_key(edit.pack, edit.parent.as_ref());
        let mut user = stash.sources.deck().clone();
        pin_closure(&mut stash.sources, &mut user, ns, closure);
        stash.sources.apply(user, &label, Some(&merge_key)).context(SourcesSnafu)?;
        stash.dirty = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "theme_draft_tests.rs"]
mod theme_draft_tests;
