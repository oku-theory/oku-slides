//! Fusion d'un autre PDF du même deck dans la session, à la demande ou avant
//! d'enregistrer par-dessus un fichier changé sur disque.

use std::path::{Path, PathBuf};

use oku_slides_core::{MergeReport, SourceBundle};
use oku_slides_i18n::Localizer;
use snafu::{ResultExt, ensure};

use crate::error::{
    EditingPackSnafu, InPackTabSnafu, NoPathSnafu, NotOkuSlidesSnafu, ReadSnafu, TargetExistsSnafu, merge_error,
};
use oku_slides_platform::storage::StorageErrorKind;

use crate::deck_file::{free_path, safe_stem};
use crate::files::{EXTENSION, IssuesAt, log_issues, parse_document, read_document};
use crate::{Mode, Result, Session};

/// Issue d'un enregistrement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    /// The file had not changed (or `overwrite`): written as is.
    Written,
    /// Le fichier avait changé (même deck) : fusionné, puis écrit.
    Merged(MergeReport),
    /// Le fichier a été remplacé par un autre deck : rien n'est écrit.
    DiskReplaced,
}

/// Ce que le disque impose avant d'écrire un chemin.
enum OnDisk {
    Unchanged,
    SameDeck(Box<SourceBundle>),
    Replaced,
}

/// Le nom de fichier de `path`, pour l'avis de fusion.
fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// L'avis montré après une fusion : le fichier, le nombre de modifications
/// importées et leurs auteurs nommés (s'il y en a).
pub(crate) fn merge_notice(l: &Localizer, file: &str, report: &MergeReport) -> String {
    let count = report.changes;
    if report.authors.is_empty() {
        l.tr_with("notice-merged-anonymous", &[("file", file.into()), ("count", count.into())])
    } else {
        let authors = report.authors.join(", ");
        l.tr_with("notice-merged", &[("file", file.into()), ("count", count.into()), ("authors", authors.into())])
    }
}

impl Session {
    /// Merges `path`, another `.oku-slides.pdf` copy of this deck, into the open deck.
    ///
    /// The merge cannot be undone (the imported changes are those of other
    /// peers); the document becomes modified and the merge notice goes out
    /// with the next snapshot. Rejected while a pack is being edited.
    ///
    /// # Errors
    ///
    /// Fails when read-only or in pack mode, in a theme tab (`WrongDocument`), if the file cannot
    /// be read, is not an Oku Slides document (`NotOkuSlides`) or does not merge.
    pub fn merge_file(&mut self, path: &Path) -> Result<MergeReport> {
        self.ensure_not_help()?;
        if matches!(self.mode, Mode::Theme { .. }) {
            return crate::document_kind::refused_in_theme_tab();
        }
        ensure!(matches!(self.mode, Mode::Deck), EditingPackSnafu);
        let opened = read_document(&self.storage, path, self.source_limits, &self.registry)?;
        ensure!(opened.pdf, NotOkuSlidesSnafu);
        self.absorb(opened.sources, &file_name(path), false)
    }

    /// Fusionne `theirs` dans le document ; `file` nomme sa provenance dans l'avis.
    ///
    /// Le document est marqué modifié (si la fusion apporte quelque chose) et
    /// l'avis ajouté à celui qui attend dès la fusion faite, avant le rendu :
    /// un rendu raté ne laisse pas un contenu fusionné passer pour enregistré.
    /// `on_save` : une fusion qui n'importe rien ne pose pas d'avis.
    fn absorb(&mut self, theirs: SourceBundle, file: &str, on_save: bool) -> Result<MergeReport> {
        // Le deck reçoit un autre état : la séance des widgets finit (`Save` compris).
        self.end_widget_session();
        let count = |s: &SourceBundle| (s.fonts.len(), s.assets.len(), s.packs.len(), s.themes.len());
        let before = count(&self.doc.sources);
        let (report, issues) = self.doc.sources.merge(theirs).map_err(merge_error)?;
        // Parties illisibles journalisées comme avant ; les constats de
        // validation ne sont pas des pertes.
        log_issues(&issues, &self.localizer, IssuesAt::Merge);
        // Les versions d'un fichier fusionné entrent aussi en silence, sans bandeau.
        self.receive_packs();
        // Deux `set_theme` concurrents laissent l'union des épinglages : la fermeture est refaite.
        match crate::deck_theme::repair_theme_closure(&self.themes, &mut self.doc.sources) {
            Ok(repaired) => self.doc.dirty |= repaired,
            Err(e) => tracing::warn!(error = %e.one_line(), "épinglages des thèmes non réparés après fusion"),
        }
        // La fusion n'ajoute que des polices, des assets, des packs et des thèmes (à nom égal, on garde les nôtres).
        if report.changes > 0 || count(&self.doc.sources) != before {
            self.doc.dirty = true;
        }
        // À l'enregistrement, une fusion vide n'a rien à dire (« Enregistré » suffit).
        if report.changes > 0 || !on_save {
            self.set_notice(merge_notice(&self.localizer, file, &report));
        }
        // Les épinglages ont pu changer (packs, thème) : la pastille du panneau des packs suit.
        self.refresh_pack_updates();
        // La fusion peut apporter un autre pack de modèles : ses widgets
        // servent aussi aux polices refaites ci-dessous.
        self.sync_widgets()?;
        // Comparé au rendu en place, pas à l'avant-fusion : une reconstruction
        // ratée (polices déjà fusionnées) est retentée à la fusion suivante.
        if self.doc.sources.fonts.len() != self.render_deck_fonts {
            self.refresh_fonts()?;
        }
        self.rerender()?;
        Ok(report)
    }

    /// Le fichier `path` a-t-il changé depuis l'ouverture ou le dernier
    /// enregistrement ? Seul le fichier du document est vérifié : un autre
    /// chemin est un « Enregistrer sous… », que le dialogue a déjà confirmé.
    fn on_disk(&self, path: &Path) -> OnDisk {
        let Some(saved) = self.doc.saved_hash.as_deref() else { return OnDisk::Unchanged };
        if self.doc.path.as_deref() != Some(path) {
            return OnDisk::Unchanged;
        }
        let bytes = match self.storage.read(path) {
            Ok(bytes) => bytes,
            // Supprimé : on l'écrit à nouveau.
            Err(e) if e.kind() == StorageErrorKind::NotFound => return OnDisk::Unchanged,
            // Présent mais illisible (droits, dossier, trop gros…) : on demande.
            Err(_) => return OnDisk::Replaced,
        };
        let hash = oku_slides_core::asset_id(&bytes);
        if hash == saved {
            return OnDisk::Unchanged;
        }
        // Changé : décodé seulement maintenant, pour savoir de quel deck il s'agit.
        match parse_document(&bytes, hash, self.source_limits, &self.registry) {
            Ok(o) if o.pdf && o.sources.replica().deck_uid() == self.doc.sources.replica().deck_uid() => {
                OnDisk::SameDeck(Box::new(o.sources))
            }
            // Un autre deck, un deck JSON, un PDF sans sources ou illisible.
            _ => OnDisk::Replaced,
        }
    }

    /// Saves without overwriting a file changed on disk.
    ///
    /// Same deck: merge then write; another deck (or no longer an Oku Slides
    /// document): nothing is written and the next snapshot reports it — unless `overwrite`.
    ///
    /// # Errors
    ///
    /// Fails if the session is not editable, if no path is available, or if the merge or the
    /// write fails.
    pub fn save_checked(&mut self, path: Option<&Path>, overwrite: bool) -> Result<Saved> {
        // L'historique montre une version passée : elle ne s'écrit jamais.
        self.ensure_editable()?;
        let path = self.save_path(path)?;
        let on_disk = if overwrite { OnDisk::Unchanged } else { self.on_disk(&path) };
        match on_disk {
            OnDisk::Unchanged => {
                self.write_to(path)?;
                Ok(Saved::Written)
            }
            OnDisk::SameDeck(theirs) => {
                let report = self.absorb(*theirs, &file_name(&path), true)?;
                self.write_to(path)?;
                Ok(Saved::Merged(report))
            }
            OnDisk::Replaced => {
                self.doc.disk_replaced = true;
                Ok(Saved::DiskReplaced)
            }
        }
    }

    /// Rejects a `path` where a file already exists, the deck's own included.
    ///
    /// Not being able to tell also rejects. Neither moving nor copying for the
    /// agent replaces a file.
    ///
    /// # Errors
    ///
    /// Fails if a file exists at `path` (`TargetExists`) or if that cannot be determined (`Read`).
    pub fn ensure_free(&self, path: &Path) -> Result<()> {
        let path = deck_path(path);
        let taken = self.storage.exists(&path).context(ReadSnafu { path: &path })?;
        ensure!(!taken, TargetExistsSnafu { path });
        Ok(())
    }

    /// Moves the document's file to a free `path`, then deletes the old one.
    ///
    /// See [`Session::ensure_free`].
    ///
    /// Checked here, under the session lock, at write time: a file that appeared
    /// during the confirmation is not replaced.
    ///
    /// # Errors
    ///
    /// Fails if the session is not editable, if `path` is taken, or if the write fails.
    pub fn move_to(&mut self, path: &Path) -> Result<()> {
        self.ensure_editable()?;
        self.ensure_free(path)?;
        let old = self.doc.path.clone();
        self.save(Some(path))?;
        if let Some(old) = old
            && self.doc.path.as_deref() != Some(old.as_path())
        {
            // Déjà parti : rien à supprimer ; sinon il reste, et c'est dit.
            if let Err(e) = self.storage.delete(&old)
                && e.kind() != StorageErrorKind::NotFound
            {
                tracing::warn!(path = %old.display(), error = %e, "ancien fichier gardé après déplacement");
            }
        }
        Ok(())
    }

    /// [`Session::save_copy`] to a free `path` (see [`Session::ensure_free`]), checked at write time.
    ///
    /// # Errors
    ///
    /// Fails if the session is not editable, if `path` is taken, in a pack tab (`InPackTab`), or if
    /// the write fails.
    pub fn save_copy_new(&mut self, path: &Path) -> Result<()> {
        self.ensure_editable()?;
        self.ensure_free(path)?;
        self.save_copy(path)
    }

    /// Writes a copy to `path`; the document keeps its file and its unsaved changes.
    ///
    /// An existing file is replaced (the system dialog had it confirmed).
    ///
    /// # Errors
    ///
    /// Fails in a pack tab (`InPackTab`: its preview is never saved), if the
    /// session is not editable, or if the path or the write fails.
    pub fn save_copy(&mut self, path: &Path) -> Result<()> {
        ensure!(!self.is_pack_tab(), InPackTabSnafu);
        self.ensure_editable()?;
        let path = self.save_path(Some(path))?;
        self.write_file(&path).map(|_| ())
    }

    /// Saves without checking the disk (CLI, "Overwrite" choice).
    ///
    /// # Errors
    ///
    /// Fails like [`Self::save_checked`]: session not editable, failed path or write.
    pub fn save(&mut self, path: Option<&Path>) -> Result<()> {
        self.save_checked(path, true).map(|_| ())
    }

    /// Le chemin où enregistrer : `path`, sinon celui du document, sinon une
    /// place libre du dossier des decks sous le titre ; toujours avec
    /// l'extension `.oku-slides.pdf`.
    fn save_path(&self, path: Option<&Path>) -> Result<PathBuf> {
        let path = match (path, &self.doc.path, &self.decks_dir) {
            (Some(p), _, _) => p.to_owned(),
            (None, Some(p), _) => p.clone(),
            (None, None, Some(dir)) => {
                let stem = safe_stem(&self.document_name(&self.doc), &self.localizer.tr("deck-untitled"));
                free_path(dir, &stem, |p| self.storage.exists(p).unwrap_or(true))
            }
            (None, None, None) => NoPathSnafu.fail()?,
        };
        Ok(deck_path(&path))
    }
}

/// `path` avec l'extension `.oku-slides.pdf` : celui qu'un enregistrement écrit vraiment.
#[must_use]
pub fn deck_path(path: &Path) -> PathBuf {
    let mut path = path.to_owned();
    let name = file_name(&path);
    if !name.ends_with(EXTENSION) {
        let stem = name.strip_suffix(".pdf").unwrap_or(&name);
        path.set_file_name(format!("{stem}{EXTENSION}"));
    }
    path
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod merge_tests;
