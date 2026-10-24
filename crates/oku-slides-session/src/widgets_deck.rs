//! Widgets modifiés en mode deck : un pack épinglé par le deck gagne une version.
//!
//! La cible : le pack épinglé sous l'espace de noms demandé, sinon le pack
//! propre du deck de la catégorie du widget écrit, créé à la première
//! écriture (sans parente) : `deck` (widgets) pour un élément, `deck-layouts`
//! (layouts) pour une slide, tous deux du deck seul. La catégorie vient de la
//! sonde du fichier écrit ; un fichier qui ne se sonde pas (ou `_commun`) va au
//! pack nommé, sinon à `deck`, ou à `deck-layouts` si le deck n'épingle que lui
//! (le pack d'un import hi-fi). Un pack nommé d'une autre sorte que la catégorie du widget
//! est refusé (`SchemaFault::PackCategory`, `widgets_edit.rs`). Jamais un autre pack par défaut : une
//! écriture sans espace de noms ne touche pas les packs intégrés du deck.
//!
//! Séance : la première modification crée une version du pack cible
//! (`parent` = la version embarquée), l'ajoute à la bibliothèque, la rend
//! courante et l'épingle (`AddPack`, qui l'embarque) ; les
//! suivantes la remplacent (nouvel `AddPack`), en une seule étape d'annulation (clé
//! `pack-adopt:<id>:<parent>`). La version remplacée quitte les sources
//! du deck et la bibliothèque seulement si l'écriture a rejoint son étape
//! (`undo_steps` inchangé) ; sinon annuler y revient, elle reste. Annuler
//! rend la version d'avant ; la bibliothèque garde la version créée. Les
//! écritures passent par `draft_write` et `save_draft` sous le verrou des
//! brouillons : un brouillon figé entre-temps (un autre deck l'a adopté)
//! reçoit une fille. Le brouillon de la séance n'est adopté qu'à sa fin. La
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
//! Un pack du deck seul (`PackScope::Deck` : `deck`, `deck-layouts`, le pack
//! d'un import hi-fi) suit la même séance, sans jamais écrire la bibliothèque
//! ni poser d'avis : ses versions ne vivent que dans les sources du deck.

use std::collections::BTreeMap;

use serde_json::Map;
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{
    Category, Namespace, Op, PackFiles, PackKind, PackPin, PackScope, PackVersion, WidgetFile, now_created,
};
use oku_slides_render::PackWidgets;

use crate::error::{NamespaceReservedSnafu, NotPinnedSnafu, PackJsonSnafu, PackVersionMissingSnafu, RegistrySnafu};
use crate::library::{DraftWrite, draft_lock};
use crate::pack::PackManifest;
use crate::pack_mode_save::adopt_merge_key;
use crate::pack_receive::is_deck_only;
use crate::widgets_edit::WidgetTarget;
use crate::{Result, Session};

/// L'espace de noms du pack propre d'un deck de sorte `kind` : `deck` ou `deck-layouts`.
fn own_namespace(kind: PackKind) -> Namespace {
    match kind {
        PackKind::Widgets => Namespace::deck(),
        PackKind::Layouts => Namespace::deck_layouts(),
    }
}

/// La sorte de pack dont les widgets ont la catégorie `category`.
fn kind_of_category(category: Category) -> PackKind {
    match category {
        Category::Element => PackKind::Widgets,
        Category::Slide => PackKind::Layouts,
    }
}

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
    ///
    /// La version de la séance cesse d'être un brouillon : le deck l'adopte.
    pub(crate) fn end_widget_session(&mut self) {
        if self.widget_session.take().is_some() {
            self.adopt_pins();
        }
    }

    /// La version épinglée `pin` est-elle le brouillon de la séance de widgets en cours ?
    pub(crate) fn is_widget_draft(&self, pin: &PackPin) -> bool {
        self.widget_session.as_ref().is_some_and(|s| s.id == pin.id && s.version == pin.version)
    }

    /// Le pack du deck de l'utilisateur et sa sorte, épinglé sous `ns`, s'il y en a un.
    fn user_pin(&self, ns: &Namespace) -> Option<(PackKind, &PackPin)> {
        self.user_deck().all_pins().find(|(_, n, _)| *n == ns).map(|(kind, _, pin)| (kind, pin))
    }

    /// Les fichiers embarqués du pack `pin`.
    fn user_pack_files(&self, pin: &PackPin) -> Result<PackFiles> {
        let files = self.user_blobs().packs.get(&pin.version).cloned();
        Ok(files.context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?)
    }

    /// La catégorie du widget `name` de source `source`, par la sonde ; `None` s'il ne se sonde pas.
    ///
    /// Sondé seul, avec le `_commun` du pack qui le recevrait : d'abord dans la
    /// sorte du pack `pack` (sans : widgets), puis dans l'autre.
    pub(crate) fn written_category(
        &self,
        pack: Option<&Namespace>,
        name: &WidgetFile,
        source: &str,
    ) -> Result<Option<Category>> {
        let named = pack.and_then(|ns| self.user_pin(ns));
        let first = named.map_or(PackKind::Widgets, |(kind, _)| kind);
        let other = if first == PackKind::Widgets { PackKind::Layouts } else { PackKind::Widgets };
        for kind in [first, other] {
            let ns = pack.filter(|_| first == kind).cloned().unwrap_or_else(|| own_namespace(kind));
            let mut widgets = BTreeMap::from([(name.clone(), source.to_owned())]);
            if let Some((_, pin)) = self.user_pin(&ns).filter(|(k, _)| *k == kind) {
                let common = self.user_pack_files(pin)?.widget(&WidgetFile::common()).map(str::to_owned);
                widgets.extend(common.map(|c| (WidgetFile::common(), c)));
            }
            let id = PackFiles::new(kind, Map::new(), widgets.clone()).version();
            let trial = PackWidgets::new(id.hex(), ns, kind, widgets).context(RegistrySnafu)?;
            let probe = self.worker.probe_pack(&trial).context(RegistrySnafu)?;
            if probe.files.get(name).is_some_and(oku_slides_render::FileProbe::is_valid) {
                return Ok(Some(kind.category()));
            }
        }
        Ok(None)
    }

    /// Le pack du deck de l'utilisateur sous `pack`, sinon son pack propre pour `category`, et ses fichiers.
    ///
    /// Sans `pack` : `deck-layouts` pour une slide, `deck` pour un élément ; pour
    /// une catégorie inconnue, `deck`, ou `deck-layouts` si le deck n'épingle que
    /// lui. Sans pack propre épinglé : un pack du deck seul
    /// neuf, vide, au nom du deck, que la première écriture épingle. Un `pack`
    /// que le deck n'épingle pas est refusé (`NotPinned`).
    pub(crate) fn deck_widget_target(
        &self,
        pack: Option<&Namespace>,
        category: Option<Category>,
    ) -> Result<WidgetTarget> {
        let only_layouts =
            self.user_pin(&Namespace::deck()).is_none() && self.user_pin(&Namespace::deck_layouts()).is_some();
        let unknown = if only_layouts { PackKind::Layouts } else { PackKind::Widgets };
        let wanted = category.map_or(unknown, kind_of_category);
        let namespace = pack.cloned().unwrap_or_else(|| own_namespace(wanted));
        let Some((kind, pin)) = self.user_pin(&namespace) else {
            if pack.is_some() {
                return NotPinnedSnafu { namespace: namespace.as_str() }.fail()?;
            }
            return self.new_deck_pack(wanted);
        };
        let files = self.user_pack_files(pin)?;
        // `deck` et `deck-layouts` n'appartiennent qu'aux packs du deck seul : un pack de bibliothèque reçu sous ce nom n'est pas écrit.
        let own = namespace.is_deck() || namespace.is_deck_layouts();
        ensure!(!own || is_deck_only(&files), NamespaceReservedSnafu { namespace: namespace.as_str() });
        Ok(WidgetTarget { kind, namespace, id: pin.id, version: pin.version.clone(), files })
    }

    /// Le pack propre du deck de sorte `kind`, pas encore créé : vide, du deck seul, au nom du deck.
    fn new_deck_pack(&self, kind: PackKind) -> Result<WidgetTarget> {
        let id = Uuid::new_v4();
        let namespace = own_namespace(kind);
        let manifest = PackManifest {
            id,
            name: self.document_name(&self.doc),
            namespace: namespace.clone(),
            requires: Vec::new(),
            description: String::new(),
            parent: None,
            created: now_created(),
            scope: PackScope::Deck,
            profiles: Vec::new(),
            examples: Default::default(),
            styles: Default::default(),
            tokens: Vec::new(),
        };
        let files = manifest.to_files(kind, BTreeMap::new()).context(PackJsonSnafu { id })?;
        Ok(WidgetTarget { kind, namespace, id, version: files.version(), files })
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
        // La séance continue tant que le deck est sur sa version.
        let session = self.widget_session.take().filter(|s| s.id == t.id && s.version == t.version);
        let committed = self.adopt_deck_widgets(t, &mut manifest, widgets, session.as_ref());
        match committed {
            Ok(next) => {
                self.widget_session = Some(next);
                // Après la séance posée : son brouillon n'est pas adopté.
                self.refresh_pack_updates();
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
        // Le brouillon de la séance, figé depuis (un autre deck l'a adopté) : la version suivante est sa fille.
        let frozen = session.filter(|s| {
            let lock = draft_lock();
            matches!(self.library.draft_write(&lock, t.id, Some(&s.version)), DraftWrite::Child(_))
        });
        let session = session.filter(|_| frozen.is_none());
        let parent = match (session, frozen) {
            (_, Some(f)) => Some(f.version.clone()),
            (Some(s), None) => s.parent.clone(),
            (None, None) => (!created).then(|| t.version.clone()),
        };
        manifest.parent.clone_from(&parent);
        let files = manifest.to_files(t.kind, widgets).context(PackJsonSnafu { id: t.id })?;
        let version = files.version();
        let embedded = self.doc.sources.packs.contains_key(&version);
        self.doc.sources.add_pack(files.clone());
        let label = self.localizer.tr_with("widgets-edit-label", &[("name", manifest.name.as_str().into())]);
        let merge = adopt_merge_key(t.id, parent.as_ref());
        let pin = PackPin { id: t.id, version: version.clone(), requires: manifest.requires.clone() };
        let steps = self.doc.sources.undo_steps();
        let op = Op::AddPack { kind: t.kind, namespace: t.namespace.clone(), pin };
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
        Ok(WidgetSession { id: t.id, parent, version })
    }

    /// Écrit `files` dans la bibliothèque, en place de la version `replaced` éventuelle.
    ///
    /// Un échec pose l'avis « non enregistrée ».
    fn write_deck_widgets(&mut self, id: Uuid, name: &str, replaced: Option<&WidgetSession>, files: &PackFiles) {
        // Décider puis écrire sous le verrou ; relâché avant que le deck compare ses épinglages.
        let lock = draft_lock();
        let write = match replaced.map(|s| self.library.draft_write(&lock, id, Some(&s.version))) {
            Some(replace @ DraftWrite::Replace(_)) => replace,
            // Figé entre-temps : la version du deck s'ajoute en sœur, deck et bibliothèque gardent la même version.
            _ => DraftWrite::New,
        };
        let written = self.library.save_draft(&lock, id, &write, files).map(|_| ());
        drop(lock);
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
