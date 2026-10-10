//! L'op `move_element` : un élément change de slot, de slide ou de place.
//!
//! Le déplacement garde l'id de l'élément et de ses descendants. Entre deux
//! slides, [`apply_all_moving`](super::apply_all_moving) le rend en [`Moved`] :
//! le replica apparie alors l'élément à son nœud, même si son id figure dans
//! d'autres slides, et les éditions concurrentes d'un pair restent.

use snafu::OptionExt;

use crate::error::{OpError, SlideNotFoundSnafu};
use crate::model::{Deck, Slide};
use crate::widget::{SlotMode, WidgetRegistry};

use super::{Op, SlotRef, check_add, insert_at, not_found, remove_from};

/// Un élément passé d'une slide à une autre par `move_element`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    pub element: String,
    pub from: String,
    pub to: String,
}

/// Applique `move_element` : retrait, contrôles d'`add_element`, insertion.
///
/// Sur des copies des slides : en cas d'erreur, le deck est intact. Un
/// propriétaire cible introuvable après le retrait (l'élément déplacé dans
/// son propre sous-arbre) donne `ElementNotFound`. `index` compte dans le
/// slot cible sans l'élément.
pub(super) fn move_element(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    slide: String,
    element: String,
    to: SlotRef,
    index: Option<usize>,
) -> Result<(), OpError> {
    let si = slide_index(deck, &slide)?;
    let ti = if to.owner.slide == slide { None } else { Some(slide_index(deck, &to.owner.slide)?) };
    // `si` et `ti` viennent de `slide_index` : dans les bornes de `deck.slides`.
    let mut source = deck.slides[si].clone();
    let mut target: Option<Slide> = ti.map(|i| deck.slides[i].clone());
    let removed = source.params.slots_mut().find_map(|(_, list)| remove_from(list, &element));
    let mut moved = removed.ok_or_else(|| not_found(&slide, &element))?;
    let dest = target.as_mut().unwrap_or(&mut source);
    let (at, mode) = check_add(registry, dest, &to, index, &mut moved)?;
    if mode == SlotMode::Flow {
        moved.frame = None;
        moved.cell = None;
    }
    insert_at(dest, &to, at, moved)?;
    deck.slides[si] = source;
    if let (Some(i), Some(t)) = (ti, target) {
        deck.slides[i] = t;
    }
    Ok(())
}

/// La position de la slide `id` dans le deck.
fn slide_index(deck: &Deck, id: &str) -> Result<usize, OpError> {
    Ok(deck.slide_index(id).context(SlideNotFoundSnafu { slide: id })?)
}

/// Ce qu'une op fait aux déplacements entre slides d'une transaction.
pub(super) enum Effect {
    /// `move_element` d'une slide à une autre.
    Moves(Moved),
    /// Retrait d'un élément (`remove_element`) : `(slide, élément)`.
    Removes(String, String),
    /// Retrait d'une slide et de ses éléments.
    RemovesSlide(String),
    None,
}

impl Effect {
    /// L'effet de `op`, lu avant de l'appliquer.
    pub(super) fn of(op: &Op) -> Self {
        match op {
            Op::MoveElement { slide, element, to, .. } if *slide != to.owner.slide => {
                Effect::Moves(Moved { element: element.clone(), from: slide.clone(), to: to.owner.slide.clone() })
            }
            Op::RemoveElement { slide, element } => Effect::Removes(slide.clone(), element.clone()),
            Op::RemoveSlide { slide } => Effect::RemovesSlide(slide.clone()),
            _ => Effect::None,
        }
    }

    /// Verse l'effet d'une op réussie dans `moves`.
    ///
    /// `(e, A→B)` puis `(e, B→C)` donnent `(e, A→C)` ; un retour à `A` n'est
    /// plus un déplacement. Un élément retiré, ou dont la slide l'est, perd
    /// son entrée.
    pub(super) fn record(self, moves: &mut Vec<Moved>) {
        match self {
            Effect::Moves(m) => match moves.iter().position(|p| p.element == m.element && p.to == m.from) {
                Some(i) if moves.get(i).is_some_and(|p| p.from == m.to) => {
                    moves.remove(i);
                }
                Some(i) => {
                    if let Some(p) = moves.get_mut(i) {
                        p.to = m.to;
                    }
                }
                None => moves.push(m),
            },
            Effect::Removes(slide, element) => moves.retain(|p| !(p.element == element && p.to == slide)),
            Effect::RemovesSlide(slide) => moves.retain(|p| p.to != slide),
            Effect::None => {}
        }
    }
}
