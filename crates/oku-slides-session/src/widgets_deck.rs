//! Widgets modifiés en mode deck : le pack du deck gagne une version.
//!
//! Séance : la première modification crée une version du pack du deck
//! (`parent` = la version embarquée), l'ajoute à la bibliothèque, la rend
//! courante et y fait passer le deck (`SetTemplates`, qui l'embarque) ; les
//! suivantes la remplacent (`Library::replace_version`, nouveau
//! `SetTemplates`), en une seule étape d'annulation (clé
//! `templates-adopt:<id>:<parent>`). La version remplacée quitte les sources
//! du deck et la bibliothèque seulement si l'écriture a rejoint son étape
//! (`undo_steps` inchangé) ; sinon annuler y revient, elle reste. Annuler
//! rend la version d'avant ; la bibliothèque garde la version créée. La
//! séance finit à la première opération qui n'est pas une modification de
//! widget, et aux bornes d'un tour de l'agent.
//!
//! Le deck est rendu avant toute écriture dans la bibliothèque : un widget
//! dont le corps échoue au rendu ne change rien (`set_widget` en rend les
//! diagnostics).

use std::collections::BTreeMap;

use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{Op, PackFiles, PackRef, PackVersion, SourceBundle, WidgetFile};

use crate::deck_pack::adopt_default_pack;
use crate::error::{PackJsonSnafu, PackVersionMissingSnafu, SourcesSnafu, UnknownPackSnafu};
use crate::pack::{DEFAULT_PACK, PackManifest};
use crate::templates_mode::{adopt_merge_key, styled_like};
use crate::widgets_edit::WidgetTarget;
use crate::{Result, Session};

/// La séance de modification des widgets en mode deck.
#[derive(Debug, Clone)]
pub(crate) struct WidgetSession {
    id: Uuid,
    /// La version embarquée au début de la séance.
    parent: PackVersion,
    /// La version de la séance, celle du deck.
    version: PackVersion,
}

impl Session {
    /// Termine la séance de widgets du mode deck, s'il y en a une.
    pub(crate) fn end_widget_session(&mut self) {
        self.widget_session = None;
    }

    /// Le pack du deck de l'utilisateur et ses fichiers.
    ///
    /// Un deck sans pack lit celui que `adopt_default_pack` lui donnerait,
    /// sans rien changer : la première écriture l'y fait passer.
    pub(crate) fn deck_pack_files(&self) -> Result<(PackRef, PackFiles)> {
        let embedded = |r: PackRef, sources: &SourceBundle| -> Result<(PackRef, PackFiles)> {
            let files = sources
                .packs
                .get(&r.version)
                .cloned()
                .context(PackVersionMissingSnafu { id: r.id, version: r.version.clone() })?;
            Ok((r, files))
        };
        if let Some(r) = self.deck_pack() {
            return embedded(r.clone(), self.user_sources());
        }
        let deck = styled_like(self.user_deck(), Vec::new());
        let mut sources = SourceBundle::new(deck, self.registry.clone()).context(SourcesSnafu)?;
        adopt_default_pack(&self.library, &mut sources)?;
        let r = sources.deck().templates.clone().context(UnknownPackSnafu { id: DEFAULT_PACK })?;
        embedded(r, &sources)
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
                Ok(())
            }
            Err(e) => {
                self.widget_session = session;
                Err(e)
            }
        }
    }

    /// Embarque la version, y fait passer le deck, puis l'écrit dans la bibliothèque.
    fn adopt_deck_widgets(
        &mut self,
        t: &WidgetTarget,
        manifest: &mut PackManifest,
        widgets: BTreeMap<WidgetFile, String>,
        session: Option<&WidgetSession>,
    ) -> Result<WidgetSession> {
        let parent = session.map_or_else(|| t.version.clone(), |s| s.parent.clone());
        manifest.parent = Some(parent.clone());
        let files = manifest.to_files(widgets).context(PackJsonSnafu { id: t.id })?;
        let version = files.version();
        let embedded = self.doc.sources.packs.contains_key(&version);
        self.doc.sources.add_pack(files.clone());
        let label = self.localizer.tr_with("widgets-edit-label", &[("name", manifest.name.as_str().into())]);
        let merge = adopt_merge_key(t.id, Some(&parent));
        let templates = Some(PackRef { id: t.id, version: version.clone() });
        let steps = self.doc.sources.undo_steps();
        if let Err(e) = self.apply(&label, vec![Op::SetTemplates { templates }], Some(&merge)) {
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
            && s.version != parent
        {
            self.doc.sources.packs.remove(&s.version);
        }
        self.pack_notice = self.pack_notice_for_deck();
        // Le deck embarque la version : un échec d'écriture n'y perd rien.
        let written = match replaced {
            Some(s) => self.library.replace_version(t.id, &s.version, &files).map(|_| ()),
            None => self.library.add_version(t.id, &files).and_then(|_| self.library.set_current(t.id, &version)),
        };
        if let Err(e) = written {
            tracing::error!(error = %e.one_line(), "version de widgets non écrite dans la bibliothèque");
        }
        Ok(WidgetSession { id: t.id, parent, version })
    }
}

#[cfg(test)]
#[path = "widgets_deck_tests.rs"]
mod widgets_deck_tests;
