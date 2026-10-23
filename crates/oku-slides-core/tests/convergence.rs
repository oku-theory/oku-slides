//! Propriété : des pairs qui éditent en parallèle puis échangent tout convergent.

mod common;

use oku_slides_core::kinds::essentiel::CONTENT;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use oku_slides_chart::{Cell, DataOp, parse_table};
use oku_slides_core::kinds::base;
use oku_slides_core::{
    Deck, DeckReplica, Element, Frame, Moved, Op, PackVersion, ParamType, ParamValue, Params, Slide, Target, ThemePin,
    WidgetRegistry, apply_all, apply_all_moving, validate,
};
use proptest::prelude::*;
use serde_json::{Value, json};

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
    /// `set_cell` du tableau `t` : ligne, colonne, valeur.
    SetCell(usize, usize, i32),
    InsertRow(usize),
    RemoveRow(usize),
    InsertColumn(usize),
    RemoveColumn(usize),
    /// Colle un tableau de même forme dont quelques cellules changent : `Replace`.
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
}

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
    ]
}

/// Le registre de base, construit une fois.
fn base() -> &'static WidgetRegistry {
    static BASE: OnceLock<WidgetRegistry> = OnceLock::new();
    BASE.get_or_init(common::registry)
}

/// Le paramètre `data` du tableau `t` de `s-constat`, s'il existe.
fn table_data(deck: &Deck) -> Option<&Value> {
    deck.slide("s-constat")?.locate("t")?.element.params.value("data")
}

/// La ligne et la colonne jamais retirées par le proptest :
/// sans l'en-tête ni la première colonne, une fusion pourrait vider le tableau.
const KEPT_ROW: &str = "h";
const KEPT_COLUMN: &str = "pays";

/// Une édition du tableau `t`, aux index tirés dans ses bornes courantes ;
/// une op refusée laisse le deck inchangé.
fn edit_table(deck: &mut Deck, e: &Edit) {
    let Some(data) = table_data(deck) else { return };
    let ds = &data["dataset"];
    let source = ds["source"].as_array().cloned().unwrap_or_default();
    let ids: Vec<&str> = ds["ids"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let columns: Vec<&str> =
        ds["columns"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let (rows, width) = (source.len(), columns.len());
    if rows == 0 || width == 0 {
        return;
    }
    let change = match *e {
        Edit::SetCell(r, c, v) => DataOp::SetCell { row: r % rows, col: c % width, value: Cell::Number(f64::from(v)) },
        Edit::InsertRow(at) => DataOp::InsertRow { at: at % (rows + 1), cells: vec![Cell::Null; width], id: None },
        Edit::RemoveRow(at) if ids.get(at % rows) != Some(&KEPT_ROW) => DataOp::RemoveRow { at: at % rows },
        // Au plus 5 colonnes par pair : 4 pairs fusionnés restent sous la borne de 20.
        Edit::InsertColumn(at) if width < 5 => {
            DataOp::InsertColumn { at: at % (width + 1), cells: vec![Cell::Null; rows], id: None }
        }
        Edit::RemoveColumn(at) if columns.get(at % width) != Some(&KEPT_COLUMN) => {
            DataOp::RemoveColumn { at: at % width }
        }
        // Collage de la même forme : une cellule sur trois change.
        Edit::Replace(seed) => {
            let Ok(table) = parse_table(data) else { return };
            let mut source = table.dataset.source;
            for (i, cell) in source.iter_mut().flatten().enumerate() {
                if (i + seed) % 3 == 0 {
                    *cell = Cell::Number((seed % 97) as f64);
                }
            }
            DataOp::Replace { source }
        }
        _ => return,
    };
    let target = Target { slide: "s-constat".into(), element: Some("t".into()) };
    let _refused = apply_all(deck, vec![Op::SetData { target, param: "data".into(), change }], base());
}

/// `parse_table` réussit sur `data`, et `ids`/`columns` sont alignés et uniques.
fn assert_table_ok(data: &Value) -> Vec<Vec<Cell>> {
    let table = parse_table(data).unwrap_or_else(|e| panic!("tableau invalide : {e:?} ({data})"));
    let ds = &table.dataset;
    assert_eq!(ds.ids.len(), ds.source.len(), "ids non alignés : {data}");
    assert_eq!(ds.columns.len(), ds.source.first().map_or(0, Vec::len), "columns non alignées : {data}");
    assert_eq!(ds.ids.iter().collect::<HashSet<_>>().len(), ds.ids.len(), "ids en double : {data}");
    assert_eq!(ds.columns.iter().collect::<HashSet<_>>().len(), ds.columns.len(), "columns en double : {data}");
    table.dataset.source
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

/// Le premier slot à deux éléments au moins (premier niveau) de la `i`-ième
/// slide qui en a un, modulo leur nombre.
fn busy_slot(deck: &mut Deck, i: usize) -> Option<&mut Vec<Element>> {
    let busy = |s: &Slide| s.params.slots().any(|(_, list)| list.len() >= 2);
    let count = deck.slides.iter().filter(|s| busy(s)).count();
    let slide = deck.slides.iter_mut().filter(|s| busy(s)).nth(i % count.max(1))?;
    slide.params.slots_mut().map(|(_, list)| list).find(|list| list.len() >= 2)
}

/// Le slot de flux et le slot libre du layout `content`.
const FLOW_SLOT: &str = "blocks";
const FREE_SLOT: &str = "elements";

/// Le cadre posé sur un élément qui entre dans le slot libre sans en avoir.
const DEFAULT_FRAME: Frame = Frame { x: 0.0, y: 0.0, w: 400.0, h: 200.0 };

/// La `i`-ième slide (modulo leur nombre) dont le layout déclare les slots
/// [`FLOW_SLOT`] et [`FREE_SLOT`] et dont l'un au moins est peuplé.
fn two_slots(deck: &mut Deck, i: usize) -> Option<&mut Slide> {
    let declares = |kind: &str, name: &str| {
        base().get(kind).and_then(|w| w.param(name)).is_some_and(|p| matches!(p.ty, ParamType::Slot { .. }))
    };
    let eligible = |s: &Slide| {
        declares(&s.kind, FLOW_SLOT)
            && declares(&s.kind, FREE_SLOT)
            && [FLOW_SLOT, FREE_SLOT].iter().any(|n| s.params.slot(n).is_some_and(|l| !l.is_empty()))
    };
    let count = deck.slides.iter().filter(|s| eligible(s)).count();
    deck.slides.iter_mut().filter(|s| eligible(s)).nth(i % count.max(1))
}

/// Retire l'élément `a` (modulo) d'un slot peuplé de `params` et l'ajoute à la
/// fin de l'autre slot : source [`FLOW_SLOT`] si `a` est pair, [`FREE_SLOT`]
/// sinon, l'autre si celui-là est vide. Le slot source vidé disparaît ;
/// l'élément reçoit un cadre dans le slot libre, perd cadre et cellule dans le flux.
fn move_across_slots(params: &mut Params, a: usize) {
    let filled = |n: &str| params.slot(n).is_some_and(|l| !l.is_empty());
    let (from, to) = match (a.is_multiple_of(2), filled(FLOW_SLOT), filled(FREE_SLOT)) {
        (true, true, _) | (false, true, false) => (FLOW_SLOT, FREE_SLOT),
        (_, _, true) => (FREE_SLOT, FLOW_SLOT),
        _ => return,
    };
    let Some(list) = params.slot_mut(from) else { return };
    let mut el = list.remove(a % list.len());
    if list.is_empty() {
        params.remove(from);
    }
    if to == FREE_SLOT {
        el.frame.get_or_insert(DEFAULT_FRAME);
    } else {
        (el.frame, el.cell) = (None, None);
    }
    if let Some(list) = params.slot_mut(to) {
        list.push(el);
    }
}

/// Le widget des piles et leur slot.
const PILE: &str = base::STACK;
const PILE_SLOT: &str = "children";

/// `deck` et une pile vide d'id `id` à la fin de `blocks` de `s-constat`.
fn with_pile(deck: Deck, id: &str) -> Deck {
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": FLOW_SLOT },
        "element": { "id": id, "kind": PILE } }));
    let mut d = deck;
    ok(apply_all(&mut d, vec![add], base()));
    d
}

/// Retire l'élément `id` de `params`, à toutes les profondeurs ; un slot vidé
/// disparaît.
fn detach(params: &mut Params, id: &str) -> Option<Element> {
    let mut emptied = None;
    let mut found = None;
    for (name, list) in params.slots_mut() {
        if let Some(at) = list.iter().position(|e| e.id == id) {
            found = Some(list.remove(at));
            if list.is_empty() {
                emptied = Some(name.to_owned());
            }
            break;
        }
        if let Some(el) = list.iter_mut().find_map(|e| detach(&mut e.params, id)) {
            found = Some(el);
            break;
        }
    }
    if let Some(name) = emptied {
        params.remove(&name);
    }
    found
}

/// Ajoute `el` au slot [`PILE_SLOT`] de `owner`, à la position `at` (modulo),
/// à toutes les profondeurs ; `false` si `owner` est absent.
fn attach(params: &mut Params, owner: &str, el: Element, at: usize) -> bool {
    for (_, list) in params.slots_mut() {
        for host in list.iter_mut() {
            if host.id == owner {
                let Some(children) = host.params.slot_mut(PILE_SLOT) else { return false };
                children.insert(at % (children.len() + 1), el);
                return true;
            }
            if contains(host, owner) {
                return attach(&mut host.params, owner, el, at);
            }
        }
    }
    false
}

/// Le sous-arbre de `el` contient-il l'élément `id` ?
fn contains(el: &Element, id: &str) -> bool {
    el.params.slots().any(|(_, list)| list.iter().any(|e| e.id == id || contains(e, id)))
}

/// `Edit::Nest` : l'élément `a` passe dans `children` de la pile `b`, sauf
/// s'il est cette pile ou l'un de ses ancêtres (cycle local).
fn nest(slide: &mut Slide, a: usize, b: usize, at: usize) {
    let (moved, owner) = {
        let all: Vec<&Element> = slide.walk().map(|l| l.element).collect();
        let piles: Vec<&str> = all.iter().filter(|e| e.kind == PILE).map(|e| e.id.as_str()).collect();
        if piles.is_empty() {
            return;
        }
        let (el, owner) = (all[a % all.len()], piles[b % piles.len()]);
        if el.id == owner || contains(el, owner) {
            return;
        }
        (el.id.clone(), owner.to_owned())
    };
    let Some(mut el) = detach(&mut slide.params, &moved) else { return };
    (el.frame, el.cell) = (None, None);
    let attached = attach(&mut slide.params, &owner, el, at);
    assert!(attached, "pile {owner} perdue");
}

/// `Edit::RemoveOwner` : retire la pile `i` (modulo) et ses enfants.
fn remove_owner(slide: &mut Slide, i: usize) {
    let piles: Vec<String> = slide.walk().filter(|l| l.element.kind == PILE).map(|l| l.element.id.clone()).collect();
    if let Some(id) = piles.get(i % piles.len().max(1)) {
        detach(&mut slide.params, id);
    }
}

/// Le premier slot déclaré par le layout `content`.
fn first_content_slot() -> &'static str {
    base()
        .get(CONTENT)
        .and_then(|w| w.params.iter().find(|p| matches!(p.ty, ParamType::Slot { .. })))
        .map(|p| p.name.as_str())
        .unwrap_or_else(|| panic!("le layout content ne déclare aucun slot"))
}

/// `Edit::MoveToSlide` par l'op `move_element` ; ses déplacements entre
/// slides, vides s'il n'y a qu'une slide `content` ou si l'op est refusée.
fn move_to_slide(deck: &mut Deck, a: usize, b: usize, at: usize) -> Vec<Moved> {
    let contents: Vec<&Slide> = deck.slides.iter().filter(|s| s.kind == CONTENT).collect();
    if contents.len() < 2 {
        return Vec::new();
    }
    let all: Vec<(&str, &str)> =
        contents.iter().flat_map(|s| s.walk().map(|l| (s.id.as_str(), l.element.id.as_str()))).collect();
    let Some(&(from, id)) = all.get(a % all.len().max(1)) else { return Vec::new() };
    let others: Vec<&Slide> = contents.iter().copied().filter(|s| s.id != from).collect();
    let to = others[b % others.len()];
    let slot = first_content_slot();
    let len = to.params.slot(slot).map_or(0, <[Element]>::len);
    let mv = op(json!({ "op": "move_element", "slide": from, "element": id,
        "to": { "owner": { "slide": to.id }, "param": slot }, "index": at % (len + 1) }));
    apply_all_moving(deck, vec![mv], base()).unwrap_or_default()
}

/// Les ids des slides et des éléments de `deck`, chacun avec « slide ou pile ».
fn ids_of(deck: &Deck) -> HashMap<String, bool> {
    let mut ids = HashMap::new();
    for s in &deck.slides {
        ids.insert(s.id.clone(), true);
        for l in s.walk() {
            ids.entry(l.element.id.clone()).or_insert(l.element.kind == PILE);
        }
    }
    ids
}

/// Le gras du premier run de l'élément `id`, à sa première occurrence.
fn bold_of(deck: &Deck, id: &str) -> Option<bool> {
    let el = deck.slides.iter().flat_map(Slide::walk).find(|l| l.element.id == id)?.element;
    match el.params.get("body") {
        Some(ParamValue::Rich(body)) => Some(body.first().and_then(|p| p.runs.first()).is_some_and(|r| r.bold)),
        _ => Some(false),
    }
}

/// Ce qu'une édition locale annonce au replica et à l'oracle du gras.
#[derive(Default)]
struct Applied {
    /// Les déplacements entre slides de `move_element`.
    moves: Vec<Moved>,
    /// Le texte basculé par `Edit::Bold`, et son nouveau gras.
    bold: Option<(String, bool)>,
}

/// Applique `e` à `deck`.
fn apply(deck: &mut Deck, e: &Edit, peer: usize, n: &mut usize) -> Applied {
    let len = deck.slides.len();
    match e {
        Edit::SetCell(..)
        | Edit::InsertRow(_)
        | Edit::RemoveRow(_)
        | Edit::InsertColumn(_)
        | Edit::RemoveColumn(_)
        | Edit::Replace(_) => edit_table(deck, e),
        Edit::MoveToSlide(a, b, at) => return Applied { moves: move_to_slide(deck, *a, *b, *at), bold: None },
        // L'annulation passe par le réplica (voir `peers_editing_in_parallel_converge`).
        Edit::Undo | Edit::RemoveElementThenUndo(_) | Edit::DeleteThenUndo(_) => {}
        Edit::Add(i) => {
            *n += 1;
            let s = Slide::new(format!("p{peer}-{n}"), CONTENT);
            deck.slides.insert(if len == 0 { 0 } else { i % (len + 1) }, s);
        }
        _ if len == 0 => {}
        Edit::Title(i, s) => {
            let contents: Vec<usize> = (0..len).filter(|&k| deck.slides[k].kind == CONTENT).collect();
            if let Some(&k) = contents.get(i % contents.len().max(1)) {
                let params = &mut deck.slides[k].params;
                match s.trim() {
                    "" => params.remove("title"),
                    t => params.set("title", ParamValue::Text(t.to_owned())),
                };
            }
        }
        Edit::Notes(i, s) => deck.slides[i % len].notes.push_str(s),
        Edit::Move(a, b) => {
            let s = deck.slides.remove(a % len);
            deck.slides.insert(b % len, s);
        }
        Edit::Delete(i) => {
            deck.slides.remove(i % len);
        }
        Edit::Theme(t) => set_theme(deck, *t),
        Edit::Bold(i) => return Applied { moves: Vec::new(), bold: toggle_bold(&mut deck.slides[i % len].params) },
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn peers_editing_in_parallel_converge(
        edits in prop::collection::vec(prop::collection::vec(edit(), 0..10), 2..4),
        seed in any::<u64>(),
    ) {
        let roadmap = Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap();
        let deck = with_pile(with_pile(with_free_element(with_table(roadmap, base())), "pile-1"), "pile-2");
        let snap = DeckReplica::new(&deck, common::shared()).unwrap().snapshot().unwrap();
        let mut peers: Vec<DeckReplica> =
            edits.iter().map(|_| DeckReplica::from_snapshot(&snap, 1 << 24, common::shared()).unwrap().0).collect();
        // Oracles : ids jamais créés, textes mis en gras, ids retirés par une
        // édition locale (avec « slide ou pile » : un tel retrait cache aussi ce
        // qu'un pair y a mis).
        let mut known: HashSet<String> = ids_of(&deck).into_keys().collect();
        let mut bolds: HashMap<String, bool> = HashMap::new();
        let mut removed: HashMap<String, bool> = HashMap::new();
        for (p, (replica, list)) in peers.iter_mut().zip(&edits).enumerate() {
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
                    replica.apply(&next, "edit", None).unwrap();
                    prop_assert!(replica.len_ops() > ops, "le retrait n'a rien écrit : {e:?}");
                    prop_assert!(replica.undo().unwrap(), "rien à annuler après {e:?}");
                    d = replica.deck().0;
                    prop_assert_eq!(&d, &before, "annulation inexacte de {:?}", e);
                    undoable = None;
                } else if matches!(e, Edit::Undo) {
                    // Annule la dernière édition locale, quelle qu'elle soit (t-652 : y compris
                    // la suppression d'une slide ou d'un élément qu'un pair déplace).
                    let undone = replica.undo().unwrap();
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
                    let ops = replica.len_ops();
                    replica.apply_moving(&d, "edit", None, &applied.moves).unwrap();
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
        let k = peers.len();
        let mut order: Vec<(usize, usize)> = (0..k).flat_map(|a| (0..k).map(move |b| (a, b))).filter(|(a, b)| a != b).collect();
        let mut x = seed | 1;
        for i in (1..order.len()).rev() {
            x ^= x << 13; x ^= x >> 7; x ^= x << 17;
            order.swap(i, (x % (i as u64 + 1)) as usize);
        }
        for _ in 0..2 {
            for &(a, b) in &order {
                let src = DeckReplica::from_snapshot(&peers[b].snapshot().unwrap(), 1 << 24, common::shared()).unwrap().0;
                peers[a].merge(&src).unwrap();
            }
        }
        let (first, issues) = peers[0].deck();
        prop_assert!(issues.is_empty(), "lecture partielle : {issues:?}");
        for p in &peers[1..] {
            prop_assert_eq!(&p.deck().0, &first);
        }
        let duplicates = ["issue-duplicate-slide-id", "issue-duplicate-element-id", "issue-duplicate-item-id"];
        prop_assert!(validate(&first, &common::registry(), None).iter().all(|i| !duplicates.contains(&i.message.id())));
        if let Some(data) = table_data(&first) {
            assert_table_ok(data);
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
            p.apply(&first, "relire", None).unwrap();
            let before = p.len_ops();
            p.apply(&first, "relire", None).unwrap();
            prop_assert_eq!(p.len_ops(), before);
        }
    }
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

/// Un changement de la liste `etapes` de l'élément `f` de `s-constat`.
fn list_edit(change: Value) -> Op {
    op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "change": change }))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &common::list_registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    d
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
        vec![list_edit(json!({ "op": "insert", "id": "i-a", "values": { "date": "2030" } }))],
        vec![list_edit(json!({ "op": "insert", "id": "i-b", "values": { "date": "2031" } }))],
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
        vec![list_edit(json!({ "op": "move", "item": "d0", "to": 2 }))],
        vec![list_edit(json!({ "op": "set_field", "item": "d0", "field": "date", "value": "2018" }))],
    );
    assert_eq!(items(&merged), vec![item("i-1", "2020"), item("i-2", "2021"), item("d0", "2018")]);
}

#[test]
fn concurrent_typing_in_one_plain_cell_merges() {
    let deck = deck_with_list(&["d0", "i-1"]);
    let merged = exchange(
        &deck,
        vec![list_edit(json!({ "op": "set_field", "item": "d0", "field": "date", "value": "début 2019" }))],
        vec![list_edit(json!({ "op": "set_field", "item": "d0", "field": "date", "value": "2019 fin" }))],
    );
    assert_eq!(items(&merged), vec![item("d0", "début 2019 fin"), item("i-1", "2020")]);
}

/// `deck` et, dans les blocs de `s-constat`, le tableau `t` : 3 lignes × 2
/// colonnes, aux identifiants explicites.
fn with_table(deck: Deck, registry: &WidgetRegistry) -> Deck {
    let data = json!({ "dataset": {
        "source": [["Pays", "Ventes"], ["FR", 10], ["DE", 20]],
        "ids": ["h", "fr", "de"],
        "columns": ["pays", "ventes"],
    } });
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "t", "kind": "base/table", "params": { "data": data } } }));
    let mut d = deck;
    ok(apply_all(&mut d, vec![add], registry));
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

/// Un `set_data` du tableau `t`.
fn data_op(change: DataOp) -> Op {
    Op::SetData {
        target: Target { slide: "s-constat".into(), element: Some("t".into()) },
        param: "data".into(),
        change,
    }
}

fn set_cell(row: usize, col: usize, value: Value) -> Op {
    data_op(DataOp::SetCell { row, col, value: cell(value) })
}

fn cell(v: Value) -> Cell {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("cellule invalide : {e}"))
}

fn cells(v: Value) -> Vec<Cell> {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("cellules invalides : {e}"))
}

/// Le `source` du tableau `t` fusionné, vérifié (`parse_table`, identifiants).
fn merged_source(deck: &Deck) -> Vec<Vec<Cell>> {
    assert_table_ok(table_data(deck).unwrap_or_else(|| panic!("tableau t absent")))
}

fn source(v: Value) -> Vec<Vec<Cell>> {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("source invalide : {e}"))
}

/// Les identifiants (`ids`, `columns`) du tableau `t`.
fn table_ids(deck: &Deck) -> (Vec<String>, Vec<String>) {
    let data = table_data(deck).unwrap_or_else(|| panic!("tableau t absent"));
    let table = ok(parse_table(data));
    (table.dataset.ids, table.dataset.columns)
}

#[test]
fn concurrent_edits_of_two_cells_in_one_row_are_both_kept() {
    let merged = exchange(&table_deck(), vec![set_cell(1, 0, json!("France"))], vec![set_cell(1, 1, json!(11))]);
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["France", 11], ["DE", 20]])));
}

#[test]
fn concurrent_edits_of_two_cells_in_one_column_are_both_kept() {
    let merged = exchange(&table_deck(), vec![set_cell(1, 1, json!(11))], vec![set_cell(2, 1, json!(21))]);
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["FR", 11], ["DE", 21]])));
}

#[test]
fn concurrent_row_inserts_are_both_kept() {
    let merged = exchange(
        &table_deck(),
        vec![data_op(DataOp::InsertRow { at: 3, cells: cells(json!(["IT", 30])), id: Some("it".into()) })],
        vec![data_op(DataOp::InsertRow { at: 1, cells: cells(json!(["ES", 40])), id: None })],
    );
    assert_eq!(
        merged_source(&merged),
        source(json!([["Pays", "Ventes"], ["ES", 40], ["FR", 10], ["DE", 20], ["IT", 30]]))
    );
    let (ids, _) = table_ids(&merged);
    assert_eq!(ids.len(), 5);
    assert_eq!(ids[4], "it");
}

#[test]
fn concurrent_column_inserts_are_both_kept() {
    let merged = exchange(
        &table_deck(),
        vec![data_op(DataOp::InsertColumn { at: 2, cells: cells(json!(["Stock", 1, 2])), id: Some("stock".into()) })],
        vec![data_op(DataOp::InsertColumn { at: 1, cells: cells(json!(["Code", "f", "d"])), id: Some("code".into()) })],
    );
    assert_eq!(
        merged_source(&merged),
        source(json!([["Pays", "Code", "Ventes", "Stock"], ["FR", "f", 10, 1], ["DE", "d", 20, 2]]))
    );
    assert_eq!(table_ids(&merged).1, vec!["pays", "code", "ventes", "stock"]);
}

#[test]
fn concurrent_row_remove_and_other_row_edit_merge() {
    let merged = exchange(&table_deck(), vec![data_op(DataOp::RemoveRow { at: 1 })], vec![set_cell(2, 1, json!(21))]);
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["DE", 21]])));
    assert_eq!(table_ids(&merged).0, vec!["h", "de"]);
}

#[test]
fn concurrent_column_insert_and_cell_edit_merge() {
    let merged = exchange(
        &table_deck(),
        vec![data_op(DataOp::InsertColumn { at: 2, cells: cells(json!(["Stock", 1, 2])), id: None })],
        vec![set_cell(1, 1, json!(11))],
    );
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes", "Stock"], ["FR", 11, 1], ["DE", 20, 2]])));
}

#[test]
fn concurrent_row_insert_and_column_insert_leave_new_row_aligned() {
    let merged = exchange(
        &table_deck(),
        vec![data_op(DataOp::InsertRow { at: 3, cells: cells(json!(["IT", 30])), id: Some("it".into()) })],
        vec![data_op(DataOp::InsertColumn { at: 2, cells: cells(json!(["Stock", 1, 2])), id: Some("stock".into()) })],
    );
    assert_eq!(
        merged_source(&merged),
        source(json!([["Pays", "Ventes", "Stock"], ["FR", 10, 1], ["DE", 20, 2], ["IT", 30, null]]))
    );
}

/// A et B retirent la même chose (`change`), échangent, annulent chacun,
/// échangent encore ; puis chacun réécrit le deck relu et ils échangent une
/// dernière fois. Le deck commun après la double annulation.
fn removed_by_both_then_undone_by_both(change: DataOp) -> Deck {
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

#[test]
fn same_row_removed_by_both_then_undone_by_both_has_no_duplicate_id() {
    let merged = removed_by_both_then_undone_by_both(DataOp::RemoveRow { at: 1 });
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["FR", 10], ["DE", 20]])));
    assert_eq!(table_ids(&merged).0, vec!["h", "fr", "de"]);
}

#[test]
fn same_column_removed_by_both_then_undone_by_both_has_no_duplicate_column() {
    let merged = removed_by_both_then_undone_by_both(DataOp::RemoveColumn { at: 1 });
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["FR", 10], ["DE", 20]])));
    assert_eq!(table_ids(&merged).1, vec!["pays", "ventes"]);
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
