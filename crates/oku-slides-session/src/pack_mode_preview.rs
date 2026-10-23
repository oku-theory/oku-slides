//! Les slides d'aperçu d'un pack de widgets, en mode pack.
//!
//! Le deck de travail d'un pack de widgets a une slide hôte par widget
//! d'élément du pack (tous ses fichiers sauf `_commun`), dans l'ordre des
//! fichiers : id `widget-<nom>`, sorte `essentiel/content`, un seul bloc de ce
//! widget, amorcé à sa valeur d'exemple. Elles ne montrent que le style du
//! thème édité sur chaque widget : toute op sur l'une d'elles ou ses éléments
//! est refusée (`PackModePreview`), et l'écriture ou le retrait d'un fichier
//! de widget les réaligne (`realign_widget_previews`).

use oku_slides_core::{Deck, Element, Op, PackFiles, PackKind, ParamValue, Slide, WidgetFile, kinds, qualify};
use oku_slides_i18n::Localizer;
use snafu::ensure;

use crate::error::PackModePreviewSnafu;
use crate::pack::PackManifest;
use crate::pack_mode::manifest_example_params;
use crate::{Mode, Result, Session};

/// Préfixe des ids des slides d'aperçu.
const PREVIEW_PREFIX: &str = "widget-";

/// Le slot de `essentiel/content` qui tient le bloc d'aperçu.
const BLOCKS: &str = "blocks";

/// L'id de la slide d'aperçu du widget `name`.
pub(crate) fn preview_slide_id(name: &WidgetFile) -> String {
    format!("{PREVIEW_PREFIX}{}", name.as_str())
}

/// La slide d'aperçu du widget `name` du pack `manifest`, son exemple dans la langue de `l`.
fn preview_slide(manifest: &PackManifest, name: &WidgetFile, l: &Localizer) -> Slide {
    let id = preview_slide_id(name);
    let mut element = Element::new(format!("{id}-block"), &qualify(&manifest.namespace, name.as_str()));
    element.params = manifest_example_params(manifest, name, &element.kind, l);
    let mut slide = Slide::new(id, kinds::essentiel::CONTENT);
    slide.params.set(BLOCKS, ParamValue::Slot(vec![element]));
    slide
}

/// Les slides d'aperçu d'un pack de widgets : une par widget de `files`, version du pack `manifest`.
///
/// Aucune pour un pack de layouts.
pub(crate) fn preview_slides(manifest: &PackManifest, files: &PackFiles, l: &Localizer) -> Vec<Slide> {
    if files.kind() != PackKind::Widgets {
        return Vec::new();
    }
    files.widgets().keys().filter(|n| !n.is_common()).map(|n| preview_slide(manifest, n, l)).collect()
}

/// Le deck de travail suit les widgets de `files`, version du pack `manifest`.
///
/// La slide d'un widget retiré part ; celle d'un nouveau widget s'ajoute, à
/// sa valeur d'exemple ; les autres restent telles quelles (une valeur
/// d'exemple réglée ne les change pas). Elles suivent l'ordre des fichiers,
/// après les autres slides. Rien pour un pack de layouts.
pub(crate) fn realign_widget_previews(deck: &mut Deck, manifest: &PackManifest, files: &PackFiles, l: &Localizer) {
    if files.kind() != PackKind::Widgets {
        return;
    }
    let (mut kept, others): (Vec<Slide>, Vec<Slide>) =
        std::mem::take(&mut deck.slides).into_iter().partition(|s| s.id.starts_with(PREVIEW_PREFIX));
    deck.slides = others;
    for name in files.widgets().keys().filter(|n| !n.is_common()) {
        let id = preview_slide_id(name);
        let slide = match kept.iter().position(|s| s.id == id) {
            Some(i) => kept.swap_remove(i),
            None => preview_slide(manifest, name, l),
        };
        deck.slides.push(slide);
    }
    oku_slides_core::canonicalize(deck);
}

/// Les slides que vise `op`.
fn op_slides(op: &Op) -> Vec<&str> {
    match op {
        Op::SetParam { target, .. }
        | Op::SetKind { target, .. }
        | Op::SetChartOption { target, .. }
        | Op::SetTableOption { target, .. }
        | Op::SetData { target, .. }
        | Op::EditList { target, .. } => vec![&target.slide],
        Op::SetStyle { slide, .. }
        | Op::ReplaceStyle { slide, .. }
        | Op::SetFrame { slide, .. }
        | Op::SetCell { slide, .. }
        | Op::SetGrid { slide, .. }
        | Op::ReplaceElement { slide, .. }
        | Op::RemoveElement { slide, .. }
        | Op::RemoveSlide { slide }
        | Op::MoveSlide { slide, .. }
        | Op::SetNotes { slide, .. } => vec![slide],
        Op::MoveElement { slide, to, .. } => vec![slide, &to.owner.slide],
        Op::AddElement { slot, .. } => vec![&slot.owner.slide],
        Op::AddSlide { slide, .. } => vec![&slide.id],
        Op::SetDeckParam { .. }
        | Op::SetTheme { .. }
        | Op::AddPack { .. }
        | Op::RemovePack { .. }
        | Op::SetToken { .. }
        | Op::UnsetToken { .. }
        | Op::SetTokens { .. }
        | Op::SetVar { .. } => Vec::new(),
    }
}

impl Session {
    /// Refuse, en édition d'un pack de widgets, une transaction qui touche une slide d'aperçu.
    pub(crate) fn ensure_no_preview_ops(&self, ops: &[Op]) -> Result<()> {
        let Mode::Pack { edit, .. } = &self.mode else { return Ok(()) };
        let previews = edit.kind == PackKind::Widgets;
        let refused = previews && ops.iter().flat_map(op_slides).any(|s| s.starts_with(PREVIEW_PREFIX));
        ensure!(!refused, PackModePreviewSnafu);
        Ok(())
    }
}

#[cfg(test)]
#[path = "pack_mode_preview_tests.rs"]
mod pack_mode_preview_tests;
