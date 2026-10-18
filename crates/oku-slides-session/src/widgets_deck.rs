//! Widgets modifiés en mode deck : un pack épinglé par le deck gagne une version.
//!
//! La cible : le pack épinglé sous l'espace de noms demandé, sinon le pack
//! propre du deck (`deck`, du deck seul), créé à la première écriture (sans
//! parente). Jamais un autre pack par défaut : une écriture sans espace de
//! noms ne touche pas le pack intégré du deck.
//!
//! Séance : la première modification crée une version du pack cible
//! (`parent` = la version embarquée), l'ajoute à la bibliothèque, la rend
//! courante et l'épingle (`AddPack`, qui l'embarque) ; les
//! suivantes la remplacent (`Library::replace_version`, nouvel
//! `AddPack`), en une seule étape d'annulation (clé
//! `templates-adopt:<id>:<parent>`). La version remplacée quitte les sources
//! du deck et la bibliothèque seulement si l'écriture a rejoint son étape
//! (`undo_steps` inchangé) ; sinon annuler y revient, elle reste. Annuler
//! rend la version d'avant ; la bibliothèque garde la version créée. La
//! séance finit à la première opération qui n'est pas une modification de
//! widget, et aux bornes d'un tour de l'agent. Un enregistrement la garde
//! (l'interface enregistre après chaque écriture), sauf s'il fusionne un
//! fichier changé sur disque.
//!
//! Le deck est rendu avant toute écriture dans la bibliothèque : un widget
//! dont le corps échoue au rendu ne change rien (`set_widget` en rend les
//! diagnostics). Une écriture qui échoue ensuite pose l'avis
//! `session-widgets-not-saved` : le deck garde la version, pas la bibliothèque.
//!
//! Un pack du deck seul (`PackScope::Deck` : le pack de l'import) suit la
//! même séance, sans jamais écrire la bibliothèque ni poser d'avis : ses
//! versions ne vivent que dans les sources du deck. Un deck de retrofit sur
//! son pack cible y passe à la première écriture : la version de la séance
//! est alors celle de son pack enfant (`import::child_pack`), de parente la
//! version cible.

use std::collections::BTreeMap;

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Namespace, Op, PackFiles, PackPin, PackScope, PackVersion, WidgetFile, now_created};

use crate::error::{NamespaceReservedSnafu, NotPinnedSnafu, PackJsonSnafu, PackVersionMissingSnafu};
use crate::pack::PackManifest;
use crate::pack_notice::is_deck_only;
use crate::templates_mode::adopt_merge_key;
use crate::widgets_edit::WidgetTarget;
use crate::{Result, Session};

/// La séance de modification des widgets en mode deck.
#[derive(Debug, Clone)]
pub(crate) struct WidgetSession {
    id: Uuid,
    /// La version embarquée au début de la séance ; `None` pour un pack créé par elle.
    parent: Option<PackVersion>,
    /// La version de la séance, celle du deck.
    version: PackVersion,
}

impl Session {
    /// Termine la séance de widgets du mode deck, s'il y en a une.
    pub(crate) fn end_widget_session(&mut self) {
        self.widget_session = None;
    }

    /// Le pack du deck de l'utilisateur sous `pack` (sans : `deck`) et ses fichiers.
    ///
    /// Sans `pack` ni pack `deck` épinglé : un pack du deck seul neuf, vide,
    /// au nom du deck, que la première écriture épingle. Un `pack` que le deck
    /// n'épingle pas est refusé (`UnknownNamespace`).
    pub(crate) fn deck_widget_target(&self, pack: Option<&Namespace>) -> Result<WidgetTarget> {
        let namespace = pack.cloned().unwrap_or_else(Namespace::deck);
        let Some(pin) = self.deck_pins().get(&namespace) else {
            if pack.is_some() {
                return NotPinnedSnafu { namespace: namespace.as_str() }.fail()?;
            }
            return self.new_deck_pack();
        };
        let files = self
            .user_blobs()
            .packs
            .get(&pin.version)
            .cloned()
            .context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        // `deck` n'appartient qu'au pack du deck seul : un pack de bibliothèque reçu sous ce nom n'est pas écrit.
        ensure!(!namespace.is_deck() || is_deck_only(&files), NamespaceReservedSnafu { namespace: namespace.as_str() });
        Ok(WidgetTarget { namespace, id: pin.id, version: pin.version.clone(), files })
    }

    /// Le pack propre du deck, pas encore créé : vide, du deck seul, au nom du deck.
    fn new_deck_pack(&self) -> Result<WidgetTarget> {
        let id = Uuid::new_v4();
        let manifest = PackManifest {
            id,
            name: self.document_name(&self.doc),
            namespace: Namespace::deck(),
            requires: Vec::new(),
            description: String::new(),
            parent: None,
            created: now_created(),
            scope: PackScope::Deck,
            templates: Vec::new(),
            examples: Default::default(),
        };
        let files = manifest.to_files(BTreeMap::new()).context(PackJsonSnafu { id })?;
        Ok(WidgetTarget { namespace: Namespace::deck(), id, version: files.version(), files })
    }

    /// `manifest` et `widgets` deviennent la version de la séance ; le deck y passe.
    ///
    /// La version est embarquée et le deck rendu d'abord : la bibliothèque
    /// n'est écrite qu'après. Un échec (rendu compris) ne change rien.
    pub(crate) fn commit_deck_widgets(
        &mut self,
        t: &WidgetTarget,
        mut manifest: PackManifest,
        widgets: BTreeMap<WidgetFile, String>,
    ) -> Result<()> {
        // Un deck de retrofit sur son pack cible : l'écriture crée son pack enfant, du deck seul.
        let child = self.retrofit_child_target(t, &mut manifest);
        let t = child.as_ref().unwrap_or(t);
        // La séance continue tant que le deck est sur sa version.
        let session = self.widget_session.take().filter(|s| s.id == t.id && s.version == t.version);
        let committed = self.adopt_deck_widgets(t, &mut manifest, widgets, session.as_ref());
        match committed {
            Ok(next) => {
                self.widget_session = Some(next);
                Ok(())
            }
            Err(e) => {
                self.widget_session = session;
                Err(e)
            }
        }
    }

    /// Embarque la version, y fait passer le deck, puis l'écrit dans la bibliothèque.
    ///
    /// Sauf pour un pack du deck seul, jamais écrit.
    fn adopt_deck_widgets(
        &mut self,
        t: &WidgetTarget,
        manifest: &mut PackManifest,
        widgets: BTreeMap<WidgetFile, String>,
        session: Option<&WidgetSession>,
    ) -> Result<WidgetSession> {
        // Un pack que la séance crée (pas encore embarqué) n'a pas de parente.
        let created = !self.doc.sources.packs.contains_key(&t.version);
        let parent = session.map_or_else(|| (!created).then(|| t.version.clone()), |s| s.parent.clone());
        manifest.parent.clone_from(&parent);
        let files = manifest.to_files(widgets).context(PackJsonSnafu { id: t.id })?;
        let version = files.version();
        let embedded = self.doc.sources.packs.contains_key(&version);
        self.doc.sources.add_pack(files.clone());
        let label = self.localizer.tr_with("widgets-edit-label", &[("name", manifest.name.as_str().into())]);
        let merge = adopt_merge_key(t.id, parent.as_ref());
        let pin = PackPin { id: t.id, version: version.clone(), requires: manifest.requires.clone() };
        let steps = self.doc.sources.undo_steps();
        let op = Op::AddPack { namespace: t.namespace.clone(), pin };
        if let Err(e) = self.apply(&label, vec![op], Some(&merge)) {
            if !embedded {
                self.doc.sources.packs.remove(&version);
            }
            return Err(e);
        }
        // Fusionnée dans l'étape de l'écriture d'avant : celle-ci n'est plus
        // atteignable, sa version quitte les sources. Sinon (nouvelle étape),
        // annuler y revient : elle reste, dans les sources et la bibliothèque.
        let merged = self.doc.sources.undo_steps() == steps;
        let replaced = session.filter(|_| merged);
        if let Some(s) = replaced
            && s.version != version
            && Some(&s.version) != parent.as_ref()
        {
            self.doc.sources.packs.remove(&s.version);
        }
        // Le deck embarque la version : un échec d'écriture n'y perd rien. Un
        // pack du deck seul ne quitte jamais le deck.
        if manifest.scope.is_library() {
            self.write_deck_widgets(t.id, &manifest.name, replaced, &files);
        }
        // Après l'écriture : la bibliothèque connaît la version, courante (pas de bandeau).
        self.pack_notice = self.pack_notice_for_deck();
        Ok(WidgetSession { id: t.id, parent, version })
    }

    /// Écrit `files` dans la bibliothèque, en place de la version `replaced` éventuelle.
    ///
    /// Un échec pose l'avis « non enregistrée ».
    fn write_deck_widgets(&mut self, id: Uuid, name: &str, replaced: Option<&WidgetSession>, files: &PackFiles) {
        let written = match replaced {
            Some(s) => self.library.replace_version(id, &s.version, files).map(|_| ()),
            None => self.library.add_version(id, files).and_then(|_| self.library.set_current(id, &files.version())),
        };
        if let Err(e) = written {
            tracing::error!(error = %e.one_line(), "version de widgets non écrite dans la bibliothèque");
            let notice = self.localizer.tr_with("session-widgets-not-saved", &[("pack", name.into())]);
            self.set_notice(notice);
        }
    }
}

#[cfg(test)]
#[path = "widgets_deck_tests.rs"]
mod widgets_deck_tests;
