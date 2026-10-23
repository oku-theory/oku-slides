//! Enregistrement du mode pack : manifeste édité, version de la séance.
//!
//! On édite la version courante du pack. Une version par session
//! d'édition : le premier enregistrement crée une version (`parent` = la
//! courante au début de la session), la rend courante, l'embarque dans le
//! deck mis de côté, qui l'épingle sous son espace de noms (une seule étape
//! d'annulation) ; les suivants la remplacent (`Library::replace_version`,
//! même parente, l'intermédiaire quitte la bibliothèque et les sources des
//! deux documents si l'enregistrement a rejoint son étape d'annulation ;
//! sinon il y reste, annuler y revenant), jusqu'à la sortie du mode pack.
//! « Enregistrer comme copie » : un nouvel id, sans parente, un nouvel espace
//! de noms tiré de son nom (`namespace_slug`), et la session continue sur la
//! copie. Un pack intégré suit la même règle, sous son id.
//! Les exemples des layouts ne passent pas par ici : chaque op sur une slide
//! de layout les écrit aussitôt (`layout_examples`) ; un enregistrement
//! reprend ceux de la dernière version et requiert les espaces de noms
//! qu'emploient ses exemples.

use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{Deck, Moved, Namespace, PackFiles, PackVersion};
use oku_slides_i18n::Localize;

use crate::deck_closure::{closure_with_files, examples_requires};
use crate::error::{NotEditingPackSnafu, PackJsonSnafu, PackManifestSnafu, PackVersionMissingSnafu, SourcesSnafu};
use crate::example_text::localized_examples;
use crate::library_packs::PackLibrary;
use crate::namespace_slug::{library_namespace, rename_namespace};
use crate::pack::{PackManifest, PackScope, now_created};
use crate::pack_library::{restore_session_version, write_session_version};
use crate::pack_mode::{embed_packs, pin_deck, pinned};
use crate::{Mode, PackMode, Result, Session};

/// Préfixe de la clé d'étape d'annulation des enregistrements du pack.
const ADOPT_MERGE: &str = "pack-adopt";

/// Clé d'étape d'annulation d'une session d'édition : son pack et la parente
/// de sa version. Ses enregistrements n'en font qu'une ; une autre session
/// (autre parente) ou une copie (autre id) en fait une autre.
pub(crate) fn adopt_merge_key(id: Uuid, parent: Option<&PackVersion>) -> String {
    format!("{ADOPT_MERGE}:{id}:{}", parent.map_or("", PackVersion::as_str))
}

/// L'étape d'annulation du deck de travail qui adopte une version de la séance.
pub(crate) enum AdoptStep<'a> {
    /// Aucune : un enregistrement.
    Silent,
    /// Une écriture de widget, jointe à l'étape de séance du sommet.
    Session(&'a str),
    /// Une op de l'utilisateur sur une slide de layout : son étape, jointe au sommet de même clé ;
    /// les éléments déplacés gardent leur nœud.
    Op(&'a str, Option<&'a str>, &'a [Moved]),
}

impl Session {
    /// Le manifeste que montre `deck`, le deck de travail du pack en cours d'édition.
    ///
    /// `kept` : la dernière version enregistrée, dont la description, les
    /// valeurs d'exemple, la portée, l'espace de noms et les besoins restent ;
    /// sans elle, la portée de l'édition (`deck_only`). Un nouveau pack de la
    /// bibliothèque (`fresh` : une copie, ou sans `kept`) prend un espace de
    /// noms libre tiré de son nom ; un nouveau pack du deck seul garde celui
    /// de `kept`, sinon `deck`.
    fn edited_manifest(
        &self,
        deck: &Deck,
        (id, parent, fresh): (Uuid, Option<PackVersion>, bool),
        deck_only: bool,
        kept: Option<PackManifest>,
    ) -> Result<PackManifest> {
        let fallback = if deck_only { PackScope::Deck } else { PackScope::Library };
        let scope = kept.as_ref().map_or(fallback, |m| m.scope);
        let name = deck
            .params
            .title
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| self.localizer.tr("pack-default-name"));
        let namespace = match kept.as_ref().map(|m| m.namespace.clone()) {
            Some(ns) if !fresh || !scope.is_library() => ns,
            None if !scope.is_library() => Namespace::deck(),
            _ => library_namespace(&self.library, &name)?,
        };
        let source = kept.as_ref().map(|m| m.id);
        // Ses jetons de composant suivent la version d'où il part.
        let (description, mut examples, styles, requires, profiles, tokens) =
            kept.map(|m| (m.description, m.examples, m.styles, m.requires, m.profiles, m.tokens)).unwrap_or_default();
        // La copie d'un pack intégré ne l'est plus : ses textes d'exemple s'écrivent en clair, dans la langue courante.
        if let Some(source) = source.filter(|_| fresh) {
            for values in examples.values_mut() {
                *values = localized_examples(source, values, &self.localizer);
            }
        }
        let mut manifest = PackManifest {
            id,
            name,
            namespace,
            requires,
            description,
            parent,
            created: now_created(),
            scope,
            profiles,
            examples,
            styles,
            tokens,
        };
        // Ses besoins suivent ce qu'emploient ses exemples (`base/text`…).
        manifest.requires = examples_requires(&manifest.namespace, &manifest);
        Ok(manifest)
    }

    /// Enregistre le pack édité dans la version de cette session d'édition.
    ///
    /// Créée au premier enregistrement, remplacée ensuite, elle devient
    /// courante ; le deck mis de côté l'embarque et y passe. Les exemples d'un
    /// pack de layouts sont ceux de la dernière version (chaque op les écrit
    /// déjà, `layout_examples`). `as_new` : une
    /// copie (nouvel id, sans parente), sur laquelle la session continue,
    /// sans historique d'annulation (un annuler ne ramène pas à l'original).
    /// Le brouillon du thème est enregistré aussi (`save_theme_edit`) ; le
    /// deck mis de côté passe à son thème dans l'étape du pack.
    /// Un rendu raté ensuite n'est pas une erreur : la version reste, un avis
    /// donne la cause (choix « diagnostics + succès », comme `set_widget`).
    /// Rend la version enregistrée.
    pub fn save_pack(&mut self, as_new: bool) -> Result<PackVersion> {
        let Mode::Pack { edit, .. } = &self.mode else {
            return NotEditingPackSnafu.fail()?;
        };
        let edit = edit.clone();
        // Le deck modifié (titre de la copie, version) remplace la vue d'un
        // coup, sans étape d'annulation : ce n'est pas une édition.
        let mut deck = self.doc.sources.deck().clone();
        if as_new && deck.params.title.as_deref() == Some(edit.name.as_str()) {
            let name = self.localizer.tr_with("pack-copy-name", &[("name", edit.name.as_str().into())]);
            deck.params.title = Some(name);
        }
        // Description, valeurs d'exemple, sorte et widgets : ceux de la dernière version enregistrée.
        let base = self.doc.sources.packs.get(&edit.base);
        let base = base.context(PackVersionMissingSnafu { id: edit.pack, version: edit.base.clone() })?;
        let kept = PackManifest::from_files(base).ok();
        let (kind, widgets) = (base.kind(), base.widgets().clone());
        let (id, parent) = if as_new { (Uuid::new_v4(), None) } else { (edit.pack, edit.parent.clone()) };
        let old = kept.as_ref().map(|m| m.namespace.clone());
        let manifest = self.edited_manifest(&deck, (id, parent, as_new), edit.deck_only, kept)?;
        // Une copie sous un autre espace de noms : le deck de travail suit ses widgets.
        if let Some(old) = old.filter(|old| *old != manifest.namespace) {
            for slide in &mut deck.slides {
                rename_namespace(slide, &old, &manifest.namespace);
            }
        }
        let files = manifest.to_files(kind, widgets).context(PackJsonSnafu { id })?;
        let version = self.adopt_session_version(deck, files, &edit, as_new, AdoptStep::Silent)?;
        // Le brouillon du thème, s'il change le thème du deck, rejoint l'étape du pack.
        if let Some((ns, pin)) = self.save_theme_edit()? {
            self.stash_adopt_theme(ns, pin)?;
        }
        if as_new {
            // Une copie est un autre pack : son document repart sans historique,
            // un annuler ne ramène jamais à l'original (l'édition suit la copie).
            self.doc.sources.clear_undo();
        }
        self.doc.dirty = false;
        // La bibliothèque a une nouvelle version courante, le deck mis de côté l'épingle.
        self.refresh_pack_updates();
        // La version est adoptée : un rendu raté ne l'annule pas, son avis le dit.
        match self.rerender() {
            Err(e) if e.render_error().is_some() => {
                tracing::warn!(error = %e.one_line(), "pack enregistré, rendu en échec");
                self.set_notice(e.localize(&self.localizer));
                Ok(version)
            }
            other => other.map(|()| version),
        }
    }

    /// `files` devient la version de la séance d'édition `edit`.
    ///
    /// Créée la première fois (ou toujours, pour une copie `as_new`),
    /// remplacée ensuite, rendue courante dans la bibliothèque (jamais écrite
    /// pour un pack du deck seul) ; `deck` (le document du pack)
    /// y passe en une étape `step` (voir `AdoptStep`), et le deck mis de côté
    /// en une étape d'annulation.
    /// Sans rendu : l'appelant rend ensuite le document du pack.
    pub(crate) fn adopt_session_version(
        &mut self,
        mut deck: Deck,
        files: PackFiles,
        edit: &PackMode,
        as_new: bool,
        step: AdoptStep<'_>,
    ) -> Result<PackVersion> {
        let manifest = PackManifest::from_files(&files).context(PackManifestSnafu { id: edit.pack })?;
        let (id, parent) = (manifest.id, manifest.parent.clone());
        // Une version par séance : la première est créée, les suivantes la remplacent.
        let replaced = edit.session_version.as_ref().filter(|_| !as_new);
        let version = files.version();
        let deck_only = !manifest.scope.is_library();
        let merge = adopt_merge_key(id, parent.as_ref());
        // Le document du pack passe à la nouvelle version : une étape pour
        // une écriture de widget (clé de séance, ou groupe du tour), rien pour
        // un enregistrement. Ses versions restent dans ses sources : une
        // étape peut y ramener. Une copie remplace le pack d'origine.
        let edited = pinned(&manifest, version.clone(), files.clone());
        deck.widgets.retain(|_, p| p.id != edit.pack);
        deck.layouts.retain(|_, p| p.id != edit.pack);
        let mut packs = closure_with_files(&self.library, &deck, edited.clone(), &self.doc.sources.packs)?;
        self.add_host_packs(&deck, &mut packs, files.kind())?;
        // Le deck mis de côté embarque la version (et ses besoins) et l'épingle.
        let mut held = self.doc.sources.packs.clone();
        held.extend(packs.iter().map(|(_, pin, f)| (pin.version.clone(), f.clone())));
        let stashed = match &self.mode {
            Mode::Pack { stash, .. } => {
                let user = stash.sources.deck().clone();
                let packs = closure_with_files(&self.library, &user, edited, &held)?;
                Some((user, packs))
            }
            _ => None,
        };
        // Les deux épinglages se font sur des copies d'abord : un refus
        // (`NamespaceTaken`) n'écrit rien dans la bibliothèque (t-1093).
        pin_deck(&mut deck, &packs)?;
        let stashed =
            stashed.map(|(mut user, packs)| pin_deck(&mut user, &packs).map(|()| (user, packs))).transpose()?;
        write_session_version(&self.library, id, replaced, &files)?;
        embed_packs(&mut self.doc.sources, packs);
        match step {
            AdoptStep::Session(label) => self.doc.sources.apply(deck, label, Some(&merge)),
            AdoptStep::Op(label, key, moves) => self.doc.sources.apply_moving(deck, label, key, moves),
            AdoptStep::Silent => self.doc.sources.apply_silent(deck),
        }
        .context(SourcesSnafu)?;
        let label = self.localizer.tr_with("pack-adopt-label", &[("name", manifest.name.as_str().into())]);
        if let (Mode::Pack { stash, edit, .. }, Some((user, packs))) = (&mut self.mode, stashed) {
            embed_packs(&mut stash.sources, packs);
            let steps = stash.sources.undo_steps();
            stash.sources.apply(user, &label, Some(&merge)).context(SourcesSnafu)?;
            stash.dirty = true;
            if let Some(old) = replaced.filter(|old| **old != version && Some(*old) != parent.as_ref()) {
                if stash.sources.undo_steps() == steps {
                    // Fusionnée dans l'étape qui l'avait introduite : plus
                    // atteignable depuis le deck mis de côté.
                    stash.sources.packs.remove(old);
                } else if let Some(files) = stash.sources.packs.get(old) {
                    // Nouvelle étape : annuler y revient ; la bibliothèque la retrouve.
                    restore_session_version(&self.library, id, files);
                }
            }
            *edit = PackMode {
                pack: id,
                name: manifest.name,
                kind: files.kind(),
                builtin: PackLibrary::is_builtin(id),
                base: version.clone(),
                parent,
                session_version: Some(version.clone()),
                deck_only,
            };
        }
        self.sync_widgets()?;
        Ok(version)
    }
}

#[cfg(test)]
#[path = "pack_mode_save_tests.rs"]
mod pack_mode_save_tests;
