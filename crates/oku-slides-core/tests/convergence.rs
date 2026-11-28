//! Propriété : des pairs qui éditent en parallèle puis échangent tout convergent.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;
#[path = "convergence/slots.rs"]
mod slots;

use oku_slides_core::kinds::essentiel::CONTENT;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use oku_slides_core::kinds::base;
use oku_slides_core::{
    Deck, DeckReplica, Element, Moved, Op, PackVersion, ParamValue, Params, Slide, Target, ThemePin, WidgetRegistry,
    apply_all, validate,
};
use oku_slides_data::{DataCell, DataId, NewColumn, NewRow, NewTable, Table, TableOp};
use proptest::prelude::*;
use serde_json::{Value, json};
use speculoos::prelude::*;

use slots::{
    DEFAULT_FRAME, FREE_SLOT, bold_of, busy_slot, ids_of, move_across_slots, move_to_slide, nest, remove_owner,
    two_slots, with_pile,
};

#[derive(Debug, Clone)]
enum Edit {
    Title(usize, String),
    Notes(usize, String),
    Move(usize, usize),
    Delete(usize),
    Add(usize),
    Theme(Option<&'static str>),
    Bold(usize),
    /// Déplace un élément d'un slot à deux éléments ([`busy_slot`]) : de `a` vers `b`.
    MoveElement(usize, usize, usize),
    /// Retire l'élément `a` d'un slot à deux éléments ([`busy_slot`]).
    RemoveElement(usize, usize),
    /// Fait passer l'élément `a` d'un slot de la `i`-ième slide à deux slots
    /// ([`two_slots`]) à la fin de l'autre slot : `MoveAcrossSlots(i, a)`.
    MoveAcrossSlots(usize, usize),
    /// Sur `s-constat`, déplace l'élément `a` (toutes profondeurs, modulo) dans
    /// `children` de la pile `b` (modulo), à la position `at` : `Nest(a, b, at)`.
    Nest(usize, usize, usize),
    /// Retire la pile `i` (modulo) de `s-constat`, avec ses enfants.
    RemoveOwner(usize),
    /// `set_cell` on data `d`: row, column, value.
    SetCell(usize, usize, i32),
    InsertRow(usize),
    RemoveRow(usize),
    InsertColumn(usize),
    RemoveColumn(usize),
    /// Pastes a table of the same shape with a few cells changed: `Replace`.
    Replace(usize),
    /// Fait passer l'élément `a` (modulo, toutes profondeurs, slides `content`)
    /// en position `at` du premier slot d'une autre slide `content`, la `b`-ième
    /// (modulo), par l'op `move_element` : `MoveToSlide(a, b, at)`.
    MoveToSlide(usize, usize, usize),
    /// Retire l'élément `i` (modulo) d'un slot à deux éléments ([`busy_slot`]),
    /// puis annule aussitôt.
    RemoveElementThenUndo(usize),
    /// Retire la slide `i` (modulo), puis annule aussitôt.
    DeleteThenUndo(usize),
    /// Annule la dernière édition locale du pair.
    Undo,
    /// Writes the knowledge file at path `i` (modulo): creates it, or replaces its text.
    KnowledgeWrite(usize, String),
    /// Replaces `body` with `body <s>` in the knowledge file at path `i` (modulo).
    KnowledgeReplace(usize, String),
    /// Moves the knowledge file at path `a` to path `b` (modulo).
    KnowledgeMove(usize, usize),
    /// Deletes the knowledge file at path `i` (modulo).
    KnowledgeDelete(usize),
}

/// The paths knowledge edits draw from; `/c/d.md` is nested.
const KNOWLEDGE_PATHS: [&str; 3] = ["/a.md", "/b.md", "/c/d.md"];

/// Pose le thème de test `<c>` (ou aucun) : espace de noms `<c>`, version `<c>` répété 64 fois.
fn set_theme(deck: &mut Deck, c: Option<&str>) {
    deck.theme = c.and_then(oku_slides_core::Namespace::parse);
    deck.themes = c
        .into_iter()
        .map(|c| {
            let version = PackVersion::from_hex(&c.repeat(64)).unwrap_or_else(|| panic!("hex invalide : {c}"));
            let ns = oku_slides_core::Namespace::parse(c).unwrap_or_else(|| panic!("espace de noms invalide : {c}"));
            (ns, ThemePin { id: uuid::Uuid::from_u128(u128::from(c.as_bytes()[0])), version, extends: Vec::new() })
        })
        .collect();
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        (any::<usize>(), "[a-zé ]{0,10}").prop_map(|(i, s)| Edit::Title(i, s)),
        (any::<usize>(), "[a-z ]{0,10}").prop_map(|(i, s)| Edit::Notes(i, s)),
        (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Edit::Move(a, b)),
        any::<usize>().prop_map(Edit::Delete),
        any::<usize>().prop_map(Edit::Add),
        prop::option::of(prop_oneof![Just("a"), Just("b")]).prop_map(Edit::Theme),
        any::<usize>().prop_map(Edit::Bold),
        (any::<usize>(), any::<usize>(), any::<usize>()).prop_map(|(i, a, b)| Edit::MoveElement(i, a, b)),
        (any::<usize>(), any::<usize>()).prop_map(|(i, a)| Edit::RemoveElement(i, a)),
        (any::<usize>(), any::<usize>()).prop_map(|(i, a)| Edit::MoveAcrossSlots(i, a)),
        (any::<usize>(), any::<usize>(), any::<usize>()).prop_map(|(a, b, at)| Edit::Nest(a, b, at)),
        any::<usize>().prop_map(Edit::RemoveOwner),
        (any::<usize>(), any::<usize>(), -50i32..50).prop_map(|(r, c, v)| Edit::SetCell(r, c, v)),
        any::<usize>().prop_map(Edit::InsertRow),
        any::<usize>().prop_map(Edit::RemoveRow),
        any::<usize>().prop_map(Edit::InsertColumn),
        any::<usize>().prop_map(Edit::RemoveColumn),
        any::<usize>().prop_map(Edit::Replace),
        (any::<usize>(), any::<usize>(), any::<usize>()).prop_map(|(a, b, at)| Edit::MoveToSlide(a, b, at)),
        any::<usize>().prop_map(Edit::RemoveElementThenUndo),
        any::<usize>().prop_map(Edit::DeleteThenUndo),
        Just(Edit::Undo),
        (any::<usize>(), "[a-z]{0,6}").prop_map(|(i, s)| Edit::KnowledgeWrite(i, s)),
        (any::<usize>(), "[a-z]{0,6}").prop_map(|(i, s)| Edit::KnowledgeReplace(i, s)),
        (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Edit::KnowledgeMove(a, b)),
        any::<usize>().prop_map(Edit::KnowledgeDelete),
    ]
}

/// Le registre de base, construit une fois.
fn base() -> &'static WidgetRegistry {
    static BASE: OnceLock<WidgetRegistry> = OnceLock::new();
    BASE.get_or_init(common::registry)
}

/// The data `d` that table `t` shows.
fn table_of(deck: &Deck) -> Option<&Table> {
    deck.data.get("d").and_then(|d| d.table())
}

/// The id of data `d`.
fn data_id() -> DataId {
    ok(DataId::try_from("d".to_owned()))
}

/// The row and column never removed by the proptest, so a merge cannot empty the data.
const KEPT_ROW: &str = "fr";
const KEPT_COLUMN: &str = "pays";

/// A row or column id no peer has drawn yet, in this process.
fn fresh_id(prefix: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("{prefix}{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// The cell `v` as column `column` of `table` takes it: a number, or its text.
fn typed_cell(table: &Table, column: &str, v: i32) -> DataCell {
    let number = DataCell::Number(f64::from(v));
    match table.columns.iter().find(|c| c.id == column) {
        Some(c) if !c.ty.fits(&number) => DataCell::Text(v.to_string()),
        _ => number,
    }
}

/// An edit of data `d`, at positions drawn within its current bounds; a refused op leaves the
/// deck unchanged.
fn edit_table(deck: &mut Deck, e: &Edit) {
    let Some(table) = table_of(deck) else { return };
    let rows: Vec<String> = table.rows.iter().map(str::to_owned).collect();
    let columns: Vec<String> = table.columns.iter().map(|c| c.id.clone()).collect();
    let (height, width) = (rows.len(), columns.len());
    if width == 0 {
        return;
    }
    let after = |ids: &[String], at: usize| (at % (ids.len() + 1)).checked_sub(1).map(|i| ids[i].clone());
    let change = match *e {
        Edit::SetCell(r, c, v) if height > 0 => {
            let column = columns[c % width].clone();
            TableOp::SetCell { row: rows[r % height].clone(), value: typed_cell(table, &column, v), column }
        }
        Edit::InsertRow(at) => TableOp::InsertRow {
            id: Some(fresh_id("r")),
            after: after(&rows, at),
            cells: std::collections::BTreeMap::new(),
        },
        Edit::RemoveRow(at) if height > 0 && rows[at % height] != KEPT_ROW => {
            TableOp::RemoveRow { row: rows[at % height].clone() }
        }
        // At most 5 columns per peer.
        Edit::InsertColumn(at) if width < 5 => TableOp::InsertColumn {
            id: Some(fresh_id("c")),
            after: after(&columns, at),
            name: "N".into(),
            cells: std::collections::BTreeMap::new(),
        },
        Edit::RemoveColumn(at) if columns[at % width] != KEPT_COLUMN => {
            TableOp::RemoveColumn { column: columns[at % width].clone() }
        }
        // A paste of the same shape and ids: one cell in three changes.
        Edit::Replace(seed) => {
            let Ok(dataset) = table.to_dataset(None, 0..height, usize::MAX) else { return };
            let mut i = 0;
            let rows = rows
                .iter()
                .zip(dataset.source.iter().skip(1))
                .map(|(id, cells)| NewRow {
                    id: Some(id.clone()),
                    cells: cells
                        .iter()
                        .zip(&table.columns)
                        .map(|(cell, column)| {
                            i += 1;
                            #[expect(
                                clippy::cast_possible_truncation,
                                clippy::cast_possible_wrap,
                                reason = "seed % 97 < 97"
                            )]
                            let value = (seed % 97) as i32;
                            if (i + seed) % 3 == 0 { typed_cell(table, &column.id, value) } else { cell.clone() }
                        })
                        .collect(),
                })
                .collect();
            let columns = table
                .columns
                .iter()
                .map(|c| NewColumn { id: Some(c.id.clone()), name: c.name.clone(), ty: Some(c.ty) })
                .collect();
            let table =
                NewTable { generation: Some(fresh_id("g")), columns, rows, base: None, overlay: imbl::OrdMap::new() };
            TableOp::Replace { table }
        }
        _ => return,
    };
    let _refused = apply_all(deck, vec![Op::EditData { data: data_id(), change }], base());
}

/// Data `d` has unique row and column ids and reads whole; its rows.
fn assert_table_ok(table: &Table) -> Vec<Vec<DataCell>> {
    let rows: Vec<&str> = table.rows.iter().collect();
    assert_eq!(rows.iter().collect::<HashSet<_>>().len(), rows.len(), "duplicate row ids: {rows:?}");
    let columns: Vec<&str> = table.columns.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(columns.iter().collect::<HashSet<_>>().len(), columns.len(), "duplicate columns: {columns:?}");
    let dataset = table.to_dataset(None, 0..rows.len(), usize::MAX).unwrap_or_else(|e| panic!("unreadable: {e}"));
    dataset.source
}

/// Bascule le gras du premier run du premier `text` de `params` (slots
/// parcourus en profondeur) ; l'id du texte et son nouveau gras, si un run a
/// été trouvé.
fn toggle_bold(params: &mut Params) -> Option<(String, bool)> {
    for (_, list) in params.slots_mut() {
        for el in list.iter_mut() {
            if el.kind == base::TEXT
                && let Some(ParamValue::Rich(body)) = el.params.0.get_mut("body")
                && let Some(run) = body.first_mut().and_then(|p| p.runs.first_mut())
            {
                run.bold = !run.bold;
                return Some((el.id.clone(), run.bold));
            }
            if let Some(found) = toggle_bold(&mut el.params) {
                return Some(found);
            }
        }
    }
    None
}

/// Ce qu'une édition locale annonce au replica et à l'oracle du gras.
#[derive(Default)]
struct Applied {
    /// Les déplacements entre slides de `move_element`.
    moves: Vec<Moved>,
    /// Le texte basculé par `Edit::Bold`, et son nouveau gras.
    bold: Option<(String, bool)>,
    /// A knowledge op was accepted.
    knowledge: bool,
}

/// The knowledge op of `edit`, if it is one; `added` counts the files and slides created.
fn knowledge_op(deck: &Deck, edit: &Edit, peer: usize, added: &mut usize) -> Option<Op> {
    let path = |i: usize| KNOWLEDGE_PATHS[i % KNOWLEDGE_PATHS.len()].to_owned();
    Some(match edit {
        Edit::KnowledgeWrite(i, s) => {
            let exists = deck.knowledge.values().any(|f| f.path == path(*i));
            let id = (!exists).then(|| {
                *added += 1;
                format!("{peer:016x}{added:016x}")
            });
            Op::KnowledgeWrite { path: path(*i), text: format!("---\ntype: Note\n---\nbody {s}\n"), id }
        }
        Edit::KnowledgeReplace(i, s) => {
            Op::KnowledgeEdit { path: path(*i), old: "body".into(), new: format!("body {s}") }
        }
        Edit::KnowledgeMove(a, b) => Op::KnowledgeMove { from: path(*a), to: path(*b) },
        Edit::KnowledgeDelete(i) => Op::KnowledgeDelete { path: path(*i) },
        _ => return None,
    })
}

/// Applies `edit` to `deck`; `added` counts the slides added by this peer.
fn apply(deck: &mut Deck, edit: &Edit, peer: usize, added: &mut usize) -> Applied {
    let len = deck.slides.len();
    if let Some(op) = knowledge_op(deck, edit, peer, added) {
        // A refused op (missing file, taken path…) leaves the deck unchanged.
        let mut next = deck.clone();
        let knowledge = apply_all(&mut next, vec![op], base()).is_ok();
        if knowledge {
            *deck = next;
        }
        return Applied { knowledge, ..Applied::default() };
    }
    match edit {
        Edit::SetCell(..)
        | Edit::InsertRow(_)
        | Edit::RemoveRow(_)
        | Edit::InsertColumn(_)
        | Edit::RemoveColumn(_)
        | Edit::Replace(_) => edit_table(deck, edit),
        Edit::MoveToSlide(a, b, at) => {
            return Applied { moves: move_to_slide(deck, *a, *b, *at), ..Applied::default() };
        }
        // L'annulation passe par le réplica (voir `peers_editing_in_parallel_converge`).
        // Knowledge edits are handled above by `knowledge_op`.
        Edit::Undo
        | Edit::RemoveElementThenUndo(_)
        | Edit::DeleteThenUndo(_)
        | Edit::KnowledgeWrite(..)
        | Edit::KnowledgeReplace(..)
        | Edit::KnowledgeMove(..)
        | Edit::KnowledgeDelete(_) => {}
        Edit::Add(i) => {
            *added += 1;
            let slide = Slide::new(format!("p{peer}-{added}"), CONTENT);
            deck.slides.insert(if len == 0 { 0 } else { i % (len + 1) }, slide);
        }
        _ if len == 0 => {}
        Edit::Title(i, title) => {
            let contents: Vec<usize> = (0..len).filter(|&k| deck.slides[k].kind == CONTENT).collect();
            if let Some(&k) = contents.get(i % contents.len().max(1)) {
                let params = &mut deck.slides[k].params;
                match title.trim() {
                    "" => params.remove("title"),
                    trimmed => params.set("title", ParamValue::Text(trimmed.to_owned())),
                };
            }
        }
        Edit::Notes(i, notes) => deck.slides[i % len].notes.push_str(notes),
        Edit::Move(a, b) => {
            let slide = deck.slides.remove(a % len);
            deck.slides.insert(b % len, slide);
        }
        Edit::Delete(i) => {
            deck.slides.remove(i % len);
        }
        Edit::Theme(t) => set_theme(deck, *t),
        Edit::Bold(i) => return Applied { bold: toggle_bold(&mut deck.slides[i % len].params), ..Applied::default() },
        Edit::MoveElement(i, a, b) => {
            if let Some(list) = busy_slot(deck, *i) {
                let el = list.remove(a % list.len());
                list.insert(b % (list.len() + 1), el);
            }
        }
        Edit::RemoveElement(i, a) => {
            if let Some(list) = busy_slot(deck, *i) {
                list.remove(a % list.len());
            }
        }
        Edit::MoveAcrossSlots(i, a) => {
            if let Some(slide) = two_slots(deck, *i) {
                move_across_slots(&mut slide.params, *a);
            }
        }
        Edit::Nest(a, b, at) => {
            if let Some(slide) = deck.slide_mut("s-constat") {
                nest(slide, *a, *b, *at);
            }
        }
        Edit::RemoveOwner(i) => {
            if let Some(slide) = deck.slide_mut("s-constat") {
                remove_owner(slide, *i);
            }
        }
    }
    Applied::default()
}

/// Le deck `d` après le retrait que `e` annule aussitôt, `None` si `e` n'en
/// est pas un ou ne retire rien.
fn removed_then_undone(d: &Deck, e: &Edit) -> Option<Deck> {
    let mut next = d.clone();
    match *e {
        Edit::RemoveElementThenUndo(i) => {
            let list = busy_slot(&mut next, i)?;
            list.remove(i % list.len());
        }
        Edit::DeleteThenUndo(i) if !next.slides.is_empty() => {
            let len = next.slides.len();
            next.slides.remove(i % len);
        }
        _ => return None,
    }
    Some(next)
}

/// Every ordered pair of distinct peers among `k`, shuffled by `seed`.
fn merge_order(k: usize, seed: u64) -> Vec<(usize, usize)> {
    let mut order: Vec<(usize, usize)> =
        (0..k).flat_map(|a| (0..k).map(move |b| (a, b))).filter(|(a, b)| a != b).collect();
    let mut x = seed | 1;
    for i in (1..order.len()).rev() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        #[expect(clippy::cast_possible_truncation, reason = "remainder < i + 1, which fits in a usize")]
        let j = (x % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

/// Peers apply `edits` in parallel, then merge in an order drawn from `seed`: they converge.
///
/// Returns how many knowledge ops were accepted locally.
fn peers_converge(edits: &[Vec<Edit>], seed: u64) -> Result<usize, TestCaseError> {
    let roadmap = Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap_or_else(fail);
    let deck = with_pile(with_pile(with_free_element(with_table(roadmap, base())), "pile-1"), "pile-2");
    let snap = DeckReplica::new(&deck, common::shared()).unwrap_or_else(fail).snapshot().unwrap_or_else(fail);
    let mut peers: Vec<DeckReplica> = edits
        .iter()
        .map(|_| DeckReplica::from_snapshot(&snap, 1 << 24, common::shared()).unwrap_or_else(fail).0)
        .collect();
    // Oracles : ids jamais créés, textes mis en gras, ids retirés par une
    // édition locale (avec « slide ou pile » : un tel retrait cache aussi ce
    // qu'un pair y a mis).
    let mut known: HashSet<String> = ids_of(&deck).into_keys().collect();
    let mut bolds: HashMap<String, bool> = HashMap::new();
    let mut removed: HashMap<String, bool> = HashMap::new();
    // Knowledge ops accepted locally.
    let mut accepted = 0;
    for (p, (replica, list)) in peers.iter_mut().zip(edits).enumerate() {
        known.extend((1..=list.len()).map(|k| format!("p{p}-{k}")));
        let (mut d, mut n) = (replica.deck().0, 0);
        // Le deck relu avant la dernière édition locale qui a écrit, tant
        // qu'aucune annulation ne l'a consommée.
        let mut undoable: Option<Deck> = None;
        // Les textes que ce pair a mis en gras ; une annulation peut défaire le gras.
        let mut mine: Vec<String> = Vec::new();
        for e in list {
            let ids = ids_of(&d);
            if let Some(next) = removed_then_undone(&d, e) {
                // Annulation exacte d'un retrait, en local.
                let before = replica.deck().0;
                let ops = replica.len_ops();
                replica.apply(&next, "edit", None).unwrap_or_else(fail);
                prop_assert!(replica.len_ops() > ops, "le retrait n'a rien écrit : {e:?}");
                prop_assert!(replica.undo().unwrap_or_else(fail), "rien à annuler après {e:?}");
                d = replica.deck().0;
                prop_assert_eq!(&d, &before, "annulation inexacte de {:?}", e);
                undoable = None;
            } else if matches!(e, Edit::Undo) {
                // Annule la dernière édition locale, quelle qu'elle soit (t-652 : y compris
                // la suppression d'une slide ou d'un élément qu'un pair déplace).
                let undone = replica.undo().unwrap_or_else(fail);
                d = replica.deck().0;
                if let Some(before) = undoable.take() {
                    prop_assert!(undone, "rien à annuler après une édition écrite");
                    prop_assert_eq!(&d, &before, "annulation inexacte");
                }
                for id in mine.drain(..) {
                    bolds.insert(id, false);
                }
            } else if !matches!(e, Edit::RemoveElementThenUndo(_) | Edit::DeleteThenUndo(_)) {
                let before = replica.deck().0;
                let applied = apply(&mut d, e, p, &mut n);
                accepted += usize::from(applied.knowledge);
                let ops = replica.len_ops();
                replica
                    .apply_moving(&d, "edit", None, &applied.moves, oku_slides_core::KnowledgeSync::Changed)
                    .unwrap_or_else(fail);
                if replica.len_ops() != ops {
                    undoable = Some(before);
                }
                if let Some((id, bold)) = applied.bold {
                    // Gras attendu seulement si tous les pairs qui touchent ce texte le posent.
                    let kept = bolds.get(&id).copied().unwrap_or(true) && bold;
                    bolds.insert(id.clone(), kept);
                    mine.push(id);
                }
            }
            let after = ids_of(&d);
            removed.extend(ids.into_iter().filter(|(id, _)| !after.contains_key(id)));
        }
    }
    // Échanges dans un ordre pseudo-aléatoire, jusqu'à ce que chacun ait tout.
    let order = merge_order(peers.len(), seed);
    for _ in 0..2 {
        for &(a, b) in &order {
            let src = DeckReplica::from_snapshot(&peers[b].snapshot().unwrap_or_else(fail), 1 << 24, common::shared())
                .unwrap_or_else(fail)
                .0;
            peers[a].merge(&src).unwrap_or_else(fail);
        }
    }
    let (first, issues) = peers[0].deck();
    // Two peers creating the same knowledge path is a repair the read reports, not a loss.
    let issues: Vec<_> = issues.into_iter().filter(|i| i.message.id() != "issue-knowledge-path-collision").collect();
    prop_assert!(issues.is_empty(), "lecture partielle : {issues:?}");
    for p in &peers[1..] {
        prop_assert_eq!(&p.deck().0, &first);
    }
    let duplicates = ["issue-duplicate-slide-id", "issue-duplicate-element-id", "issue-duplicate-item-id"];
    prop_assert!(validate(&first, &common::registry(), None).iter().all(|i| !duplicates.contains(&i.message.id())));
    if let Some(table) = table_of(&first) {
        assert_table_ok(table);
    }
    // Aucun id renommé (t-652) : la stratégie ne réajoute jamais un id retiré.
    let renamed: Vec<String> = ids_of(&first).into_keys().filter(|id| !known.contains(id)).collect();
    prop_assert!(renamed.is_empty(), "ids jamais créés : {renamed:?}");
    // Survie du gras : un texte mis en gras que personne ne retire le reste.
    // Un retrait de slide ou de pile peut le cacher : il doit alors être
    // absent ou gras.
    let hidden = removed.values().any(|&owner| owner);
    for (id, _) in bolds.iter().filter(|&(id, &bold)| bold && !removed.contains_key(id)) {
        match bold_of(&first, id) {
            Some(bold) => prop_assert!(bold, "gras de {} perdu", id),
            None => prop_assert!(hidden, "{} perdu", id),
        }
    }
    // Relire puis réécrire : le premier `apply` peut écrire une réparation
    // faite à la lecture (t-652, slot réparé), le second n'écrit rien (F1).
    for p in &mut peers {
        p.apply(&first, "relire", None).unwrap_or_else(fail);
        let before = p.len_ops();
        p.apply(&first, "relire", None).unwrap_or_else(fail);
        prop_assert_eq!(p.len_ops(), before);
    }
    Ok(accepted)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn peers_editing_in_parallel_converge(
        edits in prop::collection::vec(prop::collection::vec(edit(), 0..10), 2..4),
        seed in any::<u64>(),
    ) {
        peers_converge(&edits, seed)?;
    }
}

#[test]
fn peers_editing_knowledge_in_parallel_converge() {
    // A fixed case: random runs may draw few knowledge ops that apply.
    let a = vec![
        Edit::KnowledgeWrite(0, "a".into()),
        Edit::KnowledgeWrite(1, "b".into()),
        Edit::KnowledgeReplace(0, "x".into()),
        Edit::KnowledgeMove(1, 2),
    ];
    let b = vec![
        Edit::KnowledgeWrite(0, "c".into()),
        Edit::KnowledgeReplace(0, "y".into()),
        Edit::KnowledgeDelete(0),
        Edit::KnowledgeWrite(3, "d".into()),
    ];
    let accepted = ok(peers_converge(&[a, b], 7));
    assert_that!(accepted).is_greater_than_or_equal_to(6);
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

/// Un changement de la liste `etapes` de l'élément `f` de `s-constat`.
fn list_edit(change: &Value) -> Op {
    op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "change": change }))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &common::list_registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    d
}

/// Panics with the error: a step that must succeed failed.
fn fail<E: std::fmt::Debug, T>(e: E) -> T {
    panic!("{e:?}")
}

/// La valeur d'un résultat attendu réussi.
fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

/// `roadmap` et, dans les blocs de `s-constat`, l'élément `f` (`liste`) aux
/// items `ids` : `date` 2019, 2020…
fn deck_with_list(ids: &[&str]) -> Deck {
    let deck = ok(Deck::from_json(include_str!("../../../fixtures/roadmap.json")));
    let items: Vec<Value> =
        ids.iter().enumerate().map(|(i, id)| json!({ "id": id, "date": (2019 + i).to_string() })).collect();
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "f", "kind": "test-el/liste" } }));
    let set = op(json!({ "op": "set_param", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "value": items }));
    with_ops(&deck, vec![add, set])
}

/// Deux pairs partis du même replica de `deck`.
fn list_peers(deck: &Deck) -> (DeckReplica, DeckReplica) {
    let registry = Arc::new(common::list_registry());
    let snap = ok(ok(DeckReplica::new(deck, registry.clone())).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone())).0;
    (open(), open())
}

/// A et B appliquent chacun leurs ops sur `deck`, puis échangent tout ; le
/// deck commun, identique sur les deux pairs.
fn exchange(deck: &Deck, ops_a: Vec<Op>, ops_b: Vec<Op>) -> Deck {
    let (mut a, mut b) = list_peers(deck);
    ok(a.apply(&with_ops(deck, ops_a), "A", None));
    ok(b.apply(&with_ops(deck, ops_b), "B", None));
    ok(a.merge(&b));
    ok(b.merge(&a));
    let (merged, issues) = a.deck();
    assert!(issues.is_empty(), "lecture partielle : {issues:?}");
    assert_eq!(b.deck().0, merged);
    merged
}

/// Les items de `etapes` : (id, date).
fn items(deck: &Deck) -> Vec<(String, Option<String>)> {
    let f = deck.slide("s-constat").and_then(|s| s.locate("f")).unwrap_or_else(|| panic!("élément f absent")).element;
    f.params
        .list("etapes")
        .unwrap_or_default()
        .iter()
        .map(|i| (i.id.clone(), i.values.text("date").map(str::to_owned)))
        .collect()
}

fn item(id: &str, date: &str) -> (String, Option<String>) {
    (id.to_owned(), Some(date.to_owned()))
}

#[test]
fn concurrent_list_inserts_are_both_kept() {
    let deck = deck_with_list(&["d0"]);
    let merged = exchange(
        &deck,
        vec![list_edit(&json!({ "op": "insert", "id": "i-a", "values": { "date": "2030" } }))],
        vec![list_edit(&json!({ "op": "insert", "id": "i-b", "values": { "date": "2031" } }))],
    );
    let got = items(&merged);
    assert_eq!(got.len(), 3, "{got:?}");
    assert_eq!(got[0], item("d0", "2019"));
    assert!(got.contains(&item("i-a", "2030")), "{got:?}");
    assert!(got.contains(&item("i-b", "2031")), "{got:?}");
}

#[test]
fn concurrent_move_and_set_field_merge() {
    let deck = deck_with_list(&["d0", "i-1", "i-2"]);
    let merged = exchange(
        &deck,
        vec![list_edit(&json!({ "op": "move", "item": "d0", "to": 2 }))],
        vec![list_edit(&json!({ "op": "set_field", "item": "d0", "field": "date", "value": "2018" }))],
    );
    assert_eq!(items(&merged), vec![item("i-1", "2020"), item("i-2", "2021"), item("d0", "2018")]);
}

#[test]
fn concurrent_typing_in_one_plain_cell_merges() {
    let deck = deck_with_list(&["d0", "i-1"]);
    let merged = exchange(
        &deck,
        vec![list_edit(&json!({ "op": "set_field", "item": "d0", "field": "date", "value": "début 2019" }))],
        vec![list_edit(&json!({ "op": "set_field", "item": "d0", "field": "date", "value": "2019 fin" }))],
    );
    assert_eq!(items(&merged), vec![item("d0", "début 2019 fin"), item("i-1", "2020")]);
}

/// `deck` et, dans les blocs de `s-constat`, le tableau `t` : 3 lignes × 2
/// colonnes, aux identifiants explicites.
fn with_table(deck: Deck, registry: &WidgetRegistry) -> Deck {
    let table = json!({ "generation": "g0", "columns": [{ "id": "pays", "name": "Pays" }, { "id": "ventes", "name": "Ventes" }],
        "rows": [{ "id": "fr", "cells": ["FR", 10] }, { "id": "de", "cells": ["DE", 20] }] });
    let add_data = op(json!({ "op": "add_data", "id": "d", "name": "ventes", "table": table }));
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "t", "kind": "base/table", "params": { "data": { "data": "d" } } } }));
    let mut d = deck;
    ok(apply_all(&mut d, vec![add_data, add], registry));
    d
}

/// `deck` et, dans le slot libre de `s-constat`, le texte cadré `n` : les deux
/// slots de la slide sont peuplés.
fn with_free_element(deck: Deck) -> Deck {
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": FREE_SLOT },
        "element": { "id": "n", "kind": "base/text", "frame": { "x": 100, "y": 100, "w": 400, "h": 200 },
            "params": { "body": [{ "runs": [{ "t": "Note libre" }] }] } } }));
    let mut d = deck;
    ok(apply_all(&mut d, vec![add], base()));
    d
}

/// `roadmap` avec le tableau `t`.
fn table_deck() -> Deck {
    with_table(ok(Deck::from_json(include_str!("../../../fixtures/roadmap.json"))), &common::list_registry())
}

/// Le deck au tableau `t`, et deux pairs issus du même snapshot.
fn table_peers() -> (Deck, DeckReplica, DeckReplica) {
    let deck = table_deck();
    let (a, b) = list_peers(&deck);
    (deck, a, b)
}

/// An `edit_data` of data `d`.
fn data_op(change: TableOp) -> Op {
    Op::EditData { data: data_id(), change }
}

fn set_cell(row: &str, column: &str, value: Value) -> Op {
    data_op(TableOp::SetCell { row: row.into(), column: column.into(), value: cell(value) })
}

fn cell(v: Value) -> DataCell {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("invalid cell: {e}"))
}

fn cells(v: Value) -> std::collections::BTreeMap<String, DataCell> {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("invalid cells: {e}"))
}

fn insert_row(id: &str, after: Option<&str>, values: Value) -> Op {
    data_op(TableOp::InsertRow { id: Some(id.into()), after: after.map(str::to_owned), cells: cells(values) })
}

fn insert_column(id: &str, after: &str, name: &str, values: Value) -> Op {
    let (id, after, name) = (Some(id.into()), Some(after.into()), name.into());
    data_op(TableOp::InsertColumn { id, after, name, cells: cells(values) })
}

/// The merged rows of data `d`, header first, checked (unique ids, readable).
fn merged_source(deck: &Deck) -> Vec<Vec<DataCell>> {
    assert_table_ok(table_of(deck).unwrap_or_else(|| panic!("data d missing")))
}

fn source(v: Value) -> Vec<Vec<DataCell>> {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("invalid source: {e}"))
}

/// The row and column ids of data `d`.
fn table_ids(deck: &Deck) -> (Vec<String>, Vec<String>) {
    let table = table_of(deck).unwrap_or_else(|| panic!("data d missing"));
    (table.rows.iter().map(str::to_owned).collect(), table.columns.iter().map(|c| c.id.clone()).collect())
}

#[test]
fn concurrent_edits_of_two_cells_in_one_row_are_both_kept() {
    let merged = exchange(
        &table_deck(),
        vec![set_cell("fr", "pays", json!("France"))],
        vec![set_cell("fr", "ventes", json!(11))],
    );
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["France", 11], ["DE", 20]])));
}

#[test]
fn concurrent_edits_of_two_cells_in_one_column_are_both_kept() {
    let merged =
        exchange(&table_deck(), vec![set_cell("fr", "ventes", json!(11))], vec![set_cell("de", "ventes", json!(21))]);
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["FR", 11], ["DE", 21]])));
}

#[test]
fn concurrent_row_inserts_are_both_kept() {
    let merged = exchange(
        &table_deck(),
        vec![insert_row("it", Some("de"), json!({ "pays": "IT", "ventes": 30 }))],
        vec![insert_row("es", None, json!({ "pays": "ES", "ventes": 40 }))],
    );
    assert_eq!(
        merged_source(&merged),
        source(json!([["Pays", "Ventes"], ["ES", 40], ["FR", 10], ["DE", 20], ["IT", 30]]))
    );
    let (ids, _) = table_ids(&merged);
    assert_eq!(ids.len(), 4);
    assert_eq!(ids[3], "it");
}

#[test]
fn concurrent_column_inserts_are_both_kept() {
    let merged = exchange(
        &table_deck(),
        vec![insert_column("stock", "ventes", "Stock", json!({ "fr": 1, "de": 2 }))],
        vec![insert_column("code", "pays", "Code", json!({ "fr": "f", "de": "d" }))],
    );
    assert_eq!(
        merged_source(&merged),
        source(json!([["Pays", "Code", "Ventes", "Stock"], ["FR", "f", 10, 1], ["DE", "d", 20, 2]]))
    );
    assert_eq!(table_ids(&merged).1, vec!["pays", "code", "ventes", "stock"]);
}

#[test]
fn concurrent_row_remove_and_other_row_edit_merge() {
    let merged = exchange(
        &table_deck(),
        vec![data_op(TableOp::RemoveRow { row: "fr".into() })],
        vec![set_cell("de", "ventes", json!(21))],
    );
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["DE", 21]])));
    assert_eq!(table_ids(&merged).0, vec!["de"]);
}

#[test]
fn concurrent_column_insert_and_cell_edit_merge() {
    let merged = exchange(
        &table_deck(),
        vec![insert_column("stock", "ventes", "Stock", json!({ "fr": 1, "de": 2 }))],
        vec![set_cell("fr", "ventes", json!(11))],
    );
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes", "Stock"], ["FR", 11, 1], ["DE", 20, 2]])));
}

#[test]
fn concurrent_row_insert_and_column_insert_leave_new_row_aligned() {
    let merged = exchange(
        &table_deck(),
        vec![insert_row("it", Some("de"), json!({ "pays": "IT", "ventes": 30 }))],
        vec![insert_column("stock", "ventes", "Stock", json!({ "fr": 1, "de": 2 }))],
    );
    assert_eq!(
        merged_source(&merged),
        source(json!([["Pays", "Ventes", "Stock"], ["FR", 10, 1], ["DE", 20, 2], ["IT", 30, null]]))
    );
}

/// A et B retirent la même chose (`change`), échangent, annulent chacun,
/// échangent encore ; puis chacun réécrit le deck relu et ils échangent une
/// dernière fois. Le deck commun après la double annulation.
fn removed_by_both_then_undone_by_both(change: TableOp) -> Deck {
    let (deck, mut a, mut b) = table_peers();
    let removed = with_ops(&deck, vec![data_op(change)]);
    ok(a.apply(&removed, "A", None));
    ok(b.apply(&removed, "B", None));
    ok(a.merge(&b));
    ok(b.merge(&a));
    assert!(ok(a.undo()));
    assert!(ok(b.undo()));
    ok(a.merge(&b));
    ok(b.merge(&a));
    let (merged, issues) = a.deck();
    assert!(issues.is_empty(), "lecture partielle : {issues:?}");
    assert_eq!(b.deck().0, merged);
    // La prochaine écriture retire le double caché, des deux côtés : l'état converge.
    ok(a.apply(&merged, "A", None));
    ok(b.apply(&merged, "B", None));
    ok(a.merge(&b));
    ok(b.merge(&a));
    assert_eq!(a.deck().0, merged);
    assert_eq!(b.deck().0, merged);
    merged
}

/// Deck data repairs the restored copy rather than dropping it: ids stay unique, and the copy
/// has no cells (they are keyed by the original id).
#[test]
fn same_row_removed_by_both_then_undone_by_both_has_no_duplicate_id() {
    let merged = removed_by_both_then_undone_by_both(TableOp::RemoveRow { row: "fr".into() });
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["FR", 10], [null, null], ["DE", 20]])));
    assert_eq!(table_ids(&merged).0, vec!["fr", "fr_2", "de"]);
}

#[test]
fn same_column_removed_by_both_then_undone_by_both_has_no_duplicate_column() {
    let merged = removed_by_both_then_undone_by_both(TableOp::RemoveColumn { column: "ventes".into() });
    let expected = json!([["Pays", "Ventes", "Ventes"], ["FR", 10, null], ["DE", 20, null]]);
    assert_eq!(merged_source(&merged), source(expected));
    assert_eq!(table_ids(&merged).1, vec!["pays", "ventes", "ventes_2"]);
}

/// Les éléments `id` de `s-constat`, avec leur slot.
fn occurrences<'a>(deck: &'a Deck, id: &str) -> Vec<(&'a str, &'a Element)> {
    let slide = deck.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    slide.walk().filter(|l| l.element.id == id).map(|l| (l.slot, l.element)).collect()
}

/// Le premier run du `body` de `e` est-il gras ?
fn first_run_bold(e: &Element) -> bool {
    match e.params.get("body") {
        Some(ParamValue::Rich(body)) => body.first().and_then(|p| p.runs.first()).is_some_and(|r| r.bold),
        _ => false,
    }
}

/// Le déplacement entre slots est un `mov` de l'arbre : le gras posé par B
/// est gardé (t-694).
#[test]
fn concurrent_move_across_slots_and_edit_converge() {
    let deck = table_deck();
    let (mut a, mut b) = list_peers(&deck);
    // A : `b-points` quitte `blocks` pour `elements`, cadré (retrait puis ajout).
    let Some((_, moved)) = occurrences(&deck, "b-points").pop() else { panic!("b-points absent") };
    let moved = Element { frame: Some(DEFAULT_FRAME), ..moved.clone() };
    let remove = op(json!({ "op": "remove_element", "slide": "s-constat", "element": "b-points" }));
    let add = Op::AddElement {
        slot: oku_slides_core::SlotRef {
            owner: Target { slide: "s-constat".into(), element: None },
            param: FREE_SLOT.into(),
        },
        index: None,
        defaults: false,
        element: moved,
    };
    ok(a.apply(&with_ops(&deck, vec![remove, add]), "A", None));
    // B : met `b-points` en gras, à sa place d'origine.
    let mut bold = deck.clone();
    let slide = bold.slides.iter_mut().find(|s| s.id == "s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    assert!(toggle_bold(&mut slide.params).is_some());
    assert!(occurrences(&bold, "b-points").iter().all(|(_, e)| first_run_bold(e)), "B n'a pas basculé b-points");
    ok(b.apply(&bold, "B", None));
    ok(a.merge(&b));
    ok(b.merge(&a));
    let (merged, issues) = a.deck();
    assert!(issues.is_empty(), "lecture partielle : {issues:?}");
    assert_eq!(b.deck().0, merged);
    let found = occurrences(&merged, "b-points");
    assert_eq!(found.len(), 1, "b-points : {found:?}");
    assert_eq!(found[0].0, FREE_SLOT);
    assert!(first_run_bold(found[0].1), "gras de B perdu : {found:?}");
    assert_eq!(found[0].1.frame, Some(DEFAULT_FRAME));
    let duplicates = ["issue-duplicate-slide-id", "issue-duplicate-element-id"];
    let issues = validate(&merged, &common::list_registry(), None);
    assert!(issues.iter().all(|i| !duplicates.contains(&i.message.id())), "{issues:?}");
}
