# Champs éditables en place par widget — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Chaque paramètre texte d'un widget (carte, texte, slides) s'édite en place, exactement sur son texte rendu, avec placeholders des champs vides et Tab entre les champs de la slide.

**Architecture:** Typst émet une boîte par champ, même vide (`fields-stack` pose les champs vides hors flux, sans changer l'export). Le cœur donne l'ordre de lecture des champs et fusionne des étapes d'annulation par clé. La session expose `SlideView.fields` (ordre de Tab) à côté des boîtes d'objets. L'UI pose un éditeur par champ, recalé à chaque rendu.

**Tech Stack:** Typst 0.15, Rust (serde, snafu, loro, speculoos, insta), TypeScript/React 19 (bun test, happy-dom), CodeMirror 6, Playwright, Fluent (20 langues).

**Spec:** `docs/superpowers/specs/2026-09-28-champs-editables-design.md` (t-271). Contexte : `docs/superpowers/specs/2026-09-27-widgets-typst-design.md`.

## Global Constraints

- Branche et worktree : `271-feature/champs-editables`, sous `.claude/worktrees/271-champs`. Dans le worktree : `git config oku.review deferred`.
- `git add` dans une commande à part, puis `git commit`. Chaque commit cite `(t-271)`. Pas de ligne `Co-Authored-By`.
- **Export identique** : `crates/oku-slides-render/tests/golden.rs` (`render_reference_decks_keeps_every_page_pixel_identical`) passe **sans ré-enregistrement** à chaque tâche Typst. Un écart = un bug du plan, pas un snapshot à mettre à jour.
- Pas de `.unwrap()` ni de `.expect()` hors tests. Erreurs Snafu avec backtrace, messages en minuscules.
- Tout texte destiné à l'utilisateur passe par Fluent, dans les 20 langues (`crates/oku-slides-i18n/src/locales/*/ui.ftl`) ; `every_locale_has_exactly_the_reference_keys` et `ui/src/i18n-keys.test.ts` restent verts.
- Tests nommés `<unité>_<scénario>_<résultat>` (Rust) / `<unité> <scénario> <résultat>` (TS), `*_tests.rs` à côté du code, speculoos.
- Fichiers : cible 500 lignes, limite 1000. `crates/oku-slides-session/src/lib.rs` (904) ne grossit pas : le nouveau code de session va dans `fields.rs`.
- Vocabulaire : **champ** (`field`) = région de texte éditable liée à un paramètre ; **objet** = slide ou élément qui porte des champs (`owner`).
- Vert par tâche : `cargo nextest run -p <crate>` du crate touché ; `cd ui && bun test` pour l'UI. Workspace entier, `just check` et `just e2e` verts à la fin de la tâche 10.

## Décisions de ce plan (écarts à la spec, reportés dans la spec par la tâche 0)

- **Marqueur de champ vide = élément `metadata` sans étiquette** (`(oku-empty-field: true, owner, field, style)`). Hors de `fields-stack`, il est invisible et inerte : un pack qui ne l'utilise pas ne casse pas, ses champs vides ne sont simplement pas atteignables.
- **`layout()` est un bloc** (mesuré : collé en tête d'un item, +13,2 pt). `fields-stack` mesure donc ses items (`measure`) et pose chaque marqueur par `place(top + left, dy: …)` **à côté** du `stack`, jamais dedans (mesuré : 34,48 pt avec ou sans marqueurs).
- **Champ vide mais rendu** (corps `rich` vide d'un texte ou d'une carte, titre `""` d'un chapitre, texte de fin `""`) : `field(…, empty: true)` garde sa boîte mesurée et la marque `empty`. Pas de marqueur, la mise en page ne bouge pas.
- **Pas de `line` dans `FieldBox`** : l'UI calcule déjà la hauteur de ligne depuis `TextStyle` (`editorGeometry`). Une seule formule.
- **Libellés des placeholders côté UI** : clé existante `ui-field` (sélecteur sur le nom du paramètre, complétée de `body`, `author`, `date`, `organization`), puis `label` du schéma, puis le nom. Pas de clés `widget-param-*` en session.
- **Pas d'attente du rendu avant d'ouvrir le champ suivant** : l'éditeur s'ouvre tout de suite ; sa géométrie est recalculée à chaque `SlideView` (il est retrouvé par sa clé), il se recale donc dès que le rendu arrive. Pas de tampon de frappe.
- **Ordre des slots** : celui de `Walk` (ordre des noms de slot), déjà utilisé par les ops et la validation ; pour les widgets de base il coïncide avec l'ordre du schéma (`blocks` avant `elements`).
- **Couverture : `author` et `date` restent hors scène.** Ils sont rendus sur une seule ligne (`auteur · date`) ; `field()` produit un bloc (`layout()`), les envelopper casserait la ligne. Ils restent éditables dans l'inspecteur.
- **Sélection** : les ids `@<paramètre>` disparaissent. Un clic sur un champ de slide ne sélectionne rien ; un clic sur un champ d'élément sélectionne l'élément. La dictée hors édition écrit dans le premier champ de l'élément sélectionné.
- **Clé de champ** : `fieldKey = owner + "\u001f" + field` (séparateur d'unité ASCII, absent des ids générés).

## Carte des fichiers

| Fichier | Rôle |
|---|---|
| `typst/oku-slides/src/widget.typ` | `field(…, empty:)`, marqueur vide, `fields-stack` |
| `typst/oku-slides/src/slides.typ` | carte, titres de slide, couverture, chapitre, fin : `fields-stack` et marqueurs |
| `typst/oku-slides/src/widgets.typ` | `_text`, `_card`, `_chapter`, `_end` : champs vides |
| `typst/oku-slides/lib.typ`, `README.md` | export de `fields-stack`, règle pour les packs |
| `crates/oku-slides-render/src/lib.rs` | `FieldBox { empty, side }`, `FieldSide` |
| `crates/oku-slides-render/tests/fields.rs` | tests des champs émis (nouveau) |
| `crates/oku-slides-core/src/fields.rs` (+ `_tests`) | `TextField`, `Slide::text_fields` : ordre de lecture (nouveau) |
| `crates/oku-slides-core/src/replica/undo.rs` (+ `_tests`) | étapes à clé de fusion |
| `crates/oku-slides-core/src/replica/mod.rs`, `sources.rs` | `apply(…, merge)` |
| `crates/oku-slides-session/src/fields.rs` (+ `_tests`) | `FieldView`, construction de `SlideView.fields` (nouveau) |
| `crates/oku-slides-session/src/views.rs`, `lib.rs`, `deck_widgets.rs` | `Target` réduit, `Request::Apply { merge }` |
| `crates/oku-slides-mcp/src/edit_tools.rs` | `merge: None` |
| `ui/src/api.ts`, `ui/src/deck.ts` | `FieldView`, `ObjectTarget`, `fieldOps` |
| `ui/src/stageFields.ts` (+ `.test.ts`) | navigation, clé, libellé, géométrie des champs vides (nouveau) |
| `ui/src/Stage.tsx`, `ui/src/App.tsx`, `ui/src/agent/ui.ts` | couches, gestes, câblage |
| `crates/oku-slides-i18n/src/locales/*/ui.ftl` | nouvelles clés |
| `ui/e2e/editor.cjs`, `ui/e2e/fields.cjs`, `ui/package.json` | e2e mis à jour, test d'alignement (nouveau) |

---

### Task 0: Spec à jour et sous-tâches

**Files:**
- Modify: `docs/superpowers/specs/2026-09-28-champs-editables-design.md`

- [ ] **Step 1: Reporter les écarts dans la spec**

Dans la spec :
- § Typst / `field()` : remplacer la liste par : « `empty: true`, `h: 0` ; `style` ; `side` posé par `fields-stack`. Le champ vide est un élément `metadata` sans étiquette (`oku-empty-field: true`), inerte hors de `fields-stack`. Un champ rendu mais vide (`field(…, empty: true)`) garde sa boîte mesurée. » Retirer `line`.
- § `fields-stack` : ajouter « `layout()` est un bloc : les marqueurs sont posés par `place(top + left, dy: …)` à côté du `stack`, après mesure des items. »
- § `FieldBox` : retirer `line`.
- § Libellés : remplacer par la règle « `ui-field` (UI), puis `label` du schéma, puis le nom ».
- § Enchaînement, point 4 : remplacer par « L'éditeur visé s'ouvre tout de suite ; sa géométrie suit chaque `SlideView`, il se recale quand le rendu arrive. »
- § Hors périmètre : ajouter « `author` et `date` de la couverture (une seule ligne, inspecteur). »
- § Décisions : ajouter « Sélection : plus d'ids `@<paramètre>` ; la dictée hors édition écrit dans le premier champ de l'élément sélectionné. »

- [ ] **Step 2: Sous-tâches Oku**

```bash
for t in "Typst : champs vides et fields-stack dans la carte" "Typst : champs vides des slides" "Cœur : ordre de lecture des champs" "Cœur : fusion des étapes d'annulation par clé" "Session : Apply avec clé de fusion" "Session : FieldView et SlideView.fields" "UI : types, ops et navigation des champs" "i18n : clés des champs" "UI : couches, gestes et éditeur par champ" "E2E : éditeur et alignement des champs"; do
  oku cli task create --project slider --parent t-271 --name "$t"
done
```

Puis `oku cli task update --project slider --id t-271` avec la description : « Plan : docs/superpowers/plans/2026-09-28-champs-editables.md — tâches 1 à 10. »

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-09-28-champs-editables-design.md
```
```bash
git commit -m "docs(core) : spec des champs éditables alignée sur le plan (t-271)"
```

---

### Task 1: Typst : champs vides et `fields-stack` dans la carte

**Files:**
- Modify: `typst/oku-slides/src/widget.typ`
- Modify: `typst/oku-slides/lib.typ` (export)
- Modify: `typst/oku-slides/src/slides.typ:323-351` (`card`)
- Modify: `typst/oku-slides/src/widgets.typ:12-31` (`_text`, `_card`)
- Modify: `crates/oku-slides-render/src/lib.rs:73-91` (`FieldBox`)
- Create: `crates/oku-slides-render/tests/fields.rs`

**Interfaces:**
- Produces (Typst) : `field(ctx, name, body, style: (:), empty: auto)` ; `fields-stack(spacing: 0pt, ..items)` exporté par `lib.typ`.
- Produces (Rust) : `FieldBox { …, pub empty: bool, pub side: Option<FieldSide> }`, `pub enum FieldSide { Before, After }` (serde `lowercase`), réexporté à la racine de `oku_slides_render`.

- [ ] **Step 1: Écrire les tests qui échouent**

`crates/oku-slides-render/tests/fields.rs` :

```rust
//! Champs émis par le rendu : champs vides compris, sans changer la mise en page.

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_render::{FieldBox, FieldSide, Fonts, Renderer};
use serde_json::json;
use speculoos::prelude::*;

fn roadmap_json() -> serde_json::Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn render(deck: serde_json::Value) -> Vec<FieldBox> {
    let deck = Deck::from_json(&deck.to_string()).unwrap();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.render(&deck, &BTreeMap::new()).unwrap().layout().fields.clone()
}

/// Modifie les params de l'élément `id` de la slide `slide`.
fn with_element(slide: &str, id: &str, edit: impl Fn(&mut serde_json::Value)) -> serde_json::Value {
    let mut d = roadmap_json();
    let s = d["slides"].as_array_mut().unwrap().iter_mut().find(|s| s["id"] == slide).unwrap();
    let e = s["params"]["blocks"].as_array_mut().unwrap().iter_mut().find(|e| e["id"] == id).unwrap();
    edit(&mut e["params"]);
    d
}

fn field<'a>(fields: &'a [FieldBox], owner: &str, name: &str) -> &'a FieldBox {
    fields.iter().find(|f| f.owner == owner && f.field == name).unwrap_or_else(|| panic!("pas de champ {owner}.{name}"))
}

#[test]
fn card_without_kicker_emits_an_empty_kicker_before_its_title() {
    let fields = render(with_element("s-priorites", "c1", |p| {
        p.as_object_mut().unwrap().remove("kicker");
    }));
    let kicker = field(&fields, "c1", "kicker");
    let title = field(&fields, "c1", "title");
    assert_that!(kicker.empty).is_true();
    assert_that!(kicker.side).is_equal_to(Some(FieldSide::Before));
    assert_that!(kicker.h).is_equal_to(0.0);
    assert_that!((kicker.y - title.y).abs()).is_less_than(0.01);
    assert_that!(kicker.style.as_ref().map(|s| s.size)).is_some();
}

#[test]
fn card_filled_fields_are_not_empty() {
    let fields = render(roadmap_json());
    for name in ["kicker", "title", "body"] {
        let f = field(&fields, "c1", name);
        asserting(name).that(&f.empty).is_false();
        asserting(name).that(&f.side).is_none();
    }
}

#[test]
fn card_with_empty_body_marks_its_measured_body_empty() {
    let fields = render(with_element("s-priorites", "c1", |p| {
        p["body"] = json!([]);
    }));
    let body = field(&fields, "c1", "body");
    assert_that!(body.empty).is_true();
    assert_that!(body.side).is_none();
}

#[test]
fn text_element_with_empty_body_marks_its_body_empty() {
    let fields = render(with_element("s-constat", "b-points", |p| {
        p["body"] = json!([]);
    }));
    assert_that!(field(&fields, "b-points", "body").empty).is_true();
}
```

L'invariance de la mise en page se teste par un widget de pack (il n'y a pas de tests Typst isolés dans le dépôt). Ajouter au même fichier :

```rust
#[test]
fn fields_stack_with_empty_markers_keeps_the_stack_height() {
    use oku_slides_render::PackWidgets;
    // Un widget de pack : deux champs, le premier vide, dans fields-stack ; et un
    // témoin identique sans le champ vide. Leurs boîtes d'élément ont la même hauteur.
    let src = r#"#import "/oku-slides/lib.typ": widget, field, fields-stack, params as p
#let duo = widget("duo", params: (a: p.plain(), b: p.plain()),
  (args, ctx) => fields-stack(spacing: 20pt, field(ctx, "a", args.a), field(ctx, "b", args.b)))
#let solo = widget("solo", params: (b: p.plain()),
  (args, ctx) => fields-stack(spacing: 20pt, field(ctx, "b", args.b)))
#let registry = (duo, solo)
"#;
    let pack = PackWidgets::new("duo", src).unwrap();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_pack(Some(pack)).unwrap();
    let deck = json!({
        "version": 2, "params": { "title": "T" },
        "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
            { "id": "d", "kind": "duo", "params": { "b": "Texte" } },
            { "id": "o", "kind": "solo", "params": { "b": "Texte" } }
        ] } }]
    });
    let compiled = r.render(&Deck::from_json(&deck.to_string()).unwrap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let h = |id: &str| layout.boxes.iter().find(|b| b.id == id).map(|b| b.h).unwrap();
    assert_that!((h("d") - h("o")).abs()).is_less_than(0.01);
    let a = layout.fields.iter().find(|f| f.owner == "d" && f.field == "a").unwrap();
    assert_that!(a.empty).is_true();
    assert_that!(a.side).is_equal_to(Some(FieldSide::Before));
}
```

Avant d'écrire ce dernier test, vérifier le nom réel de l'injection de pack dans `Renderer` (`grep -n "pub fn set_pack\|pub fn with_pack\|PackWidgets" crates/oku-slides-render/src/lib.rs`) et l'adapter : c'est l'API existante, pas une nouvelle. Si le deck minimal n'est pas valide en v2 (`Deck::from_json`), partir de `roadmap_json()` et remplacer les `blocks` de `s-constat`.

- [ ] **Step 2: Lancer les tests, vérifier l'échec**

Run: `cargo nextest run -p oku-slides-render --test fields`
Expected: FAIL à la compilation (`FieldSide` absent, champs `empty`/`side` inconnus).

- [ ] **Step 3: `FieldBox`**

Dans `crates/oku-slides-render/src/lib.rs`, après `style` de `FieldBox` :

```rust
    /// Champ sans texte : placeholder à dessiner par l'interface.
    #[serde(default)]
    pub empty: bool,
    /// Champ vide sans place au rendu : dessiné avant ou après son ancre (`y`).
    #[serde(default)]
    pub side: Option<FieldSide>,
}

/// Où se dessine un champ vide sans place au rendu, par rapport à son ancre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldSide {
    /// Au-dessus : l'ancre est le haut de l'item suivant.
    Before,
    /// Au-dessous : l'ancre est le bas de l'item précédent.
    After,
}
```

Ajouter `FieldSide` à la liste des types de la doc du module (ligne 15) et aux réexports explicites.

- [ ] **Step 4: `field()` et `fields-stack`**

Dans `typst/oku-slides/src/widget.typ`, remplacer `field` et ajouter :

```typst
#import "core.typ": _measured, text-probe

/// Vide : `none`, chaîne vide ou contenu vide.
#let _is-empty(body) = body == none or body == "" or body == []

/// Marqueur d'un champ vide sans place : un `metadata` sans étiquette, inerte
/// hors de `fields-stack`.
#let _is-empty-marker(it) = (type(it) == content and it.func() == metadata
  and type(it.value) == dictionary and it.value.at("oku-empty-field", default: false))

/// Champ éditable : émet la boîte de `body` pour le paramètre `name` de
/// l'objet `ctx.id` (<oku-slides-field>), avec sa typographie.
/// `body` absent (`none`, `""`) : un marqueur, que `fields-stack` pose sans
/// lui donner de place. `empty` : le contenu est rendu mais vide (corps
/// riche sans paragraphe) ; `auto` le déduit de `body`.
#let field(ctx, name, body, style: (:), empty: auto) = if ctx.id == none { body } else {
  let meta = (owner: str(ctx.id), field: name)
  if body == none or body == "" {
    metadata(meta + (oku-empty-field: true, style: style))
  } else {
    let empty = if empty == auto { _is-empty(body) } else { empty }
    _measured(meta + (empty: empty), "oku-slides-field", body, style)
  }
}

// Boîte d'un champ vide posé par `fields-stack` : position ici, largeur `aw`.
#let _empty-box(v, aw, side) = context {
  let p = here().position()
  [#metadata((owner: v.owner, field: v.field, page: p.page, x: p.x / 1pt, y: p.y / 1pt,
    w: 0, h: 0, ax: p.x / 1pt, aw: aw / 1pt, style: text-probe(v.style),
    empty: true, side: side))<oku-slides-field>]
}

/// `stack` qui accepte des champs vides (marqueurs de `field`) : ils ne
/// prennent ni place ni espacement ; leur boîte est posée au haut de l'item
/// suivant (`before`), ou au bas du dernier (`after`). Un widget qui empile
/// des champs l'utilise à la place de `stack`.
#let fields-stack(spacing: 0pt, ..items) = layout(sz => context {
  let all = items.pos().filter(x => x != none)
  let full = all.filter(x => not _is-empty-marker(x))
  let hs = full.map(x => measure(x, width: sz.width).height)
  let y = 0pt
  let k = 0
  let marks = ()
  for it in all {
    if _is-empty-marker(it) {
      let side = if k < full.len() or full.len() == 0 { "before" } else { "after" }
      let dy = if side == "after" { y - spacing } else { y }
      marks.push(place(top + left, dy: dy, _empty-box(it.value, sz.width, side)))
    } else {
      y += hs.at(k) + spacing
      k += 1
    }
  }
  marks.join()
  stack(spacing: spacing, ..full)
})
```

Vérifier que `text-probe` est bien exporté par `core.typ` (il est défini ligne 25) ; l'importer comme ci-dessus. Dans `typst/oku-slides/lib.typ`, ajouter `fields-stack` à l'export de `widget.typ` (à côté de `field`).

- [ ] **Step 5: La carte**

Dans `typst/oku-slides/src/slides.typ`, `card` (ligne 341), remplacer le `stack(…)` par :

```typst
        fields-stack(spacing: space-of(t, k.gap),
          wrap("kicker", if kicker != none { styled(t, kicker-c, kicker) },
            text-style(t, kicker-c) + (case: kicker-c.at("case", default: none))),
          wrap("title", if title != none { text(..title-style, title) }, title-style),
          wrap("body", if ink != none { text(fill: ink, body) } else { body }, if ink != none { (fill: ink) } else { (:) }),
        )
```

et importer `fields-stack` depuis `widget.typ` en tête de `slides.typ` (vérifier qu'il n'y a pas d'import circulaire : `widget.typ` n'importe que `core.typ`). Le `wrap` par défaut de `card` (`(name, body, style) => body`) rend `none` pour un champ vide : `fields-stack` filtre les `none`.

Dans `widgets.typ`, `_card` : `wrap: (name, body, style) => field(ctx, name, body, style: style, empty: if name == "body" { args.body == [] } else { auto })`.

`_text` : `field(ctx, "body", style: …, empty: args.body == [], align(…))`.

- [ ] **Step 6: Lancer les tests**

Run: `cargo nextest run -p oku-slides-render`
Expected: PASS, **y compris `golden` sans ré-enregistrement**, `compile_card_emits_one_field_per_filled_text_param` et le snapshot du registre inchangés.

- [ ] **Step 7: Commit**

```bash
git add typst/oku-slides crates/oku-slides-render
```
```bash
git commit -m "feat(render) : champs vides émis sans changer la mise en page, fields-stack (t-271)"
```

---

### Task 2: Typst : champs vides des slides

**Files:**
- Modify: `typst/oku-slides/src/slides.typ` (`_slide-title`, `slide`, `title-slide`, `chapter`, `end-slide`)
- Modify: `typst/oku-slides/src/widgets.typ` (`_chapter`, `_end`)
- Modify: `typst/oku-slides/README.md` (règle pour les packs)
- Test: `crates/oku-slides-render/tests/fields.rs`

**Interfaces:**
- Consumes : `field`, `fields-stack`, `FieldBox.empty/side` (tâche 1).
- Produces : pour chaque slide de base, un `FieldBox` par paramètre `plain` affiché (voir tableau), vide ou non.

| Widget | Champs émis |
|---|---|
| `content` | `kicker`, `title` |
| `title` | `kicker` (mise en page `plain` seulement), `title`, `subtitle` |
| `chapter` | `title`, `subtitle` |
| `toc` | `title` |
| `end` | `text` |

- [ ] **Step 1: Tests qui échouent** (ajouter à `tests/fields.rs`)

```rust
fn with_slide(slide: &str, edit: impl Fn(&mut serde_json::Value)) -> serde_json::Value {
    let mut d = roadmap_json();
    let s = d["slides"].as_array_mut().unwrap().iter_mut().find(|s| s["id"] == slide).unwrap();
    edit(&mut s["params"]);
    d
}

#[test]
fn content_slide_without_kicker_emits_an_empty_kicker() {
    let fields = render(with_slide("s-constat", |p| {
        p.as_object_mut().unwrap().remove("kicker");
    }));
    let kicker = field(&fields, "s-constat", "kicker");
    assert_that!(kicker.empty).is_true();
    assert_that!(kicker.side).is_equal_to(Some(FieldSide::Before));
}

#[test]
fn content_slide_without_title_emits_empty_kicker_and_title_above_the_body() {
    let fields = render(with_slide("s-constat", |p| {
        let o = p.as_object_mut().unwrap();
        o.remove("kicker");
        o.remove("title");
    }));
    let title = field(&fields, "s-constat", "title");
    let body = field(&fields, "b-points", "body");
    assert_that!(title.empty).is_true();
    assert_that!(title.y).is_less_than_or_equal_to(body.y);
    assert_that!(field(&fields, "s-constat", "kicker").empty).is_true();
}

#[test]
fn chapter_without_subtitle_emits_an_empty_subtitle_after_the_title() {
    let fields = render(roadmap_json()); // s-ch2 n'a pas de sous-titre
    let sub = field(&fields, "s-ch2", "subtitle");
    let title = field(&fields, "s-ch2", "title");
    assert_that!(sub.empty).is_true();
    assert_that!(sub.side).is_equal_to(Some(FieldSide::After));
    assert_that!(sub.y).is_greater_than_or_equal_to(title.y + title.h - 0.01);
}

#[test]
fn title_slide_emits_kicker_title_and_subtitle() {
    let fields = render(roadmap_json());
    for name in ["kicker", "title", "subtitle"] {
        asserting(name).that(&fields.iter().any(|f| f.owner == "s-title" && f.field == name)).is_true();
    }
}

#[test]
fn end_slide_with_empty_text_marks_it_empty() {
    let fields = render(with_slide("s-fin", |p| {
        p["text"] = json!("");
    }));
    assert_that!(field(&fields, "s-fin", "text").empty).is_true();
}

#[test]
fn toc_emits_its_title_field() {
    let fields = render(roadmap_json());
    assert_that!(fields.iter().any(|f| f.owner == "s-toc" && f.field == "title")).is_true();
}
```

Run: `cargo nextest run -p oku-slides-render --test fields`
Expected: FAIL sur ces six tests (champs absents).

- [ ] **Step 2: `_slide-title` et `slide`**

`_slide-title(t, title, kicker-body, wrap)` : toujours appeler `wrap` pour le surtitre et le titre, et empiler avec `fields-stack` :

```typst
#let _slide-title(t, title, kicker-body, wrap) = {
  let c = t.component.title
  let items = ()
  items.push(wrap("kicker", if kicker-body != none { styled(t, t.component.kicker, kicker-body) },
    text-style(t, t.component.kicker) + (case: t.component.kicker.at("case", default: none))))
  let r = c.at("rule", default: none)
  if r != none {
    items.push(rect(width: r.width * 1pt, height: r.height * 1pt, fill: color-of(t, r.color)))
  }
  items.push(wrap("title", if title != none {
    set par(leading: leading-of(t, c.at("leading", default: "tight")))
    _heading(2, title, ..text-style(t, c))
  }, text-style(t, c) + (leading: leading-of(t, c.at("leading", default: "tight")))))
  fields-stack(spacing: space-of(t, "sm"), ..items)
}
```

Attention : `if title != none { set par(…); _heading(…) }` doit rester un bloc de code qui rend du contenu (`{ … }` d'un `if` en mode code : vérifier que `set` y est admis ; sinon écrire `if title != none [#set par(…)#_heading(…)]`).

Dans `slide(…)`, la branche sans titre (`title == none`) n'appelle pas `_slide-title` : ajouter les marqueurs en tête du bloc de corps, hors flux :

```typst
      block(width: 100%, height: 100%, {
        if zone != none { place(top + left, zone) }
        if title == none {
          place(top + left, fields-stack(
            wrap("kicker", none, text-style(t, t.component.kicker)),
            wrap("title", none, text-style(t, t.component.title))))
        }
        _layout(t, layout, bodies.pos(), ratio: ratio, grow: grow)
      }),
```

- [ ] **Step 3: `title-slide`**

Mise en page `plain` : remplacer `stack(spacing: space-of(t, "md"), ..(bar, if kick != none { styled(…) }, title-block, sub, body).filter(x => x != none))` par :

```typst
        align(bottom + start, fields-stack(
          spacing: space-of(t, "md"),
          bar,
          wrap("kicker", if kick != none { styled(t, t.component.kicker, kick) },
            text-style(t, t.component.kicker) + (case: t.component.kicker.at("case", default: none)) + under),
          title-block, sub, body,
        )),
```

`sub` devient inconditionnel :

```typst
  let sub = wrap("subtitle", if subtitle != none {
      text(size: size-of(t, k.subtitle-size), fill: col("subtitle-color", "band-subtitle-color"), subtitle)
    }, (size: size-of(t, k.subtitle-size), fill: col("subtitle-color", "band-subtitle-color")) + under)
```

`title-block` : `wrap("title", if title != none { … }, …)` (même corps qu'aujourd'hui dans le `if`).

Mise en page `band` : `stack(spacing: …, ..(bar, title-block, sub, meta-line, body).filter(x => x != none))` → `fields-stack(spacing: space-of(t, "md"), bar, title-block, sub, meta-line, body)`. Le surtitre n'y est pas affiché : pas de champ `kicker` en `band`.

- [ ] **Step 4: `chapter`, `end-slide`, widgets**

`chapter` : `stack(spacing: space-of(t, "lg"), …)` → `fields-stack(spacing: space-of(t, "lg"), …)` ; le sous-titre devient `wrap("subtitle", if subtitle != none { text(…) }, (size: size-of(t, k.subtitle-size)))` sans le `..if`. Le titre garde son rendu (un titre `""` rend un en-tête vide, dont dépend le sommaire).

`widgets.typ` :
- `_chapter` : `wrap: (name, body, style) => field(ctx, name, body, style: style, empty: if name == "title" { args.title == none or args.title == "" } else { auto })`.
- `_end` : même principe pour `text` (`empty: args.text == none or args.text == ""`).
- `_fields(ctx)` reste pour `title`, `toc`, `content`.

- [ ] **Step 5: README**

Dans `typst/oku-slides/README.md`, section des widgets, ajouter un paragraphe « Champs vides » : `field()` d'une valeur absente rend un marqueur sans place ; pour empiler des champs, utiliser `fields-stack` au lieu de `stack`, sinon le marqueur reçoit l'espacement et le champ vide n'a pas d'ancre.

- [ ] **Step 6: Tests**

Run: `cargo nextest run -p oku-slides-render`
Expected: PASS, `golden` compris, sans ré-enregistrement.

- [ ] **Step 7: Commit**

```bash
git add typst/oku-slides crates/oku-slides-render/tests/fields.rs
```
```bash
git commit -m "feat(render) : champs vides des slides, surtitre de couverture éditable (t-271)"
```

---

### Task 3: Cœur : ordre de lecture des champs

**Files:**
- Create: `crates/oku-slides-core/src/fields.rs`, `crates/oku-slides-core/src/fields_tests.rs`
- Modify: `crates/oku-slides-core/src/lib.rs` (module, réexports, doc)

**Interfaces:**
- Consumes : `Slide::walk()` (`locate.rs`), `WidgetRegistry::get`, `WidgetSchema::params`, `ParamType::{Plain, Rich}`.
- Produces :

```rust
pub enum FieldOwner<'a> { Slide, Element(&'a Element) }
pub struct TextField<'a> { pub owner: FieldOwner<'a>, pub owner_id: &'a str, pub param: &'a ParamSchema }
impl Slide { pub fn text_fields<'a>(&'a self, registry: &'a WidgetRegistry) -> Vec<TextField<'a>>; }
```

- [ ] **Step 1: Tests qui échouent**

`fields_tests.rs` :

```rust
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_registry;

fn slide(v: serde_json::Value) -> Slide {
    serde_json::from_value(v).unwrap()
}

fn order(s: &Slide) -> Vec<String> {
    let reg = test_registry::shared();
    s.text_fields(&reg).iter().map(|f| format!("{}.{}", f.owner_id, f.param.name)).collect()
}

#[test]
fn text_fields_of_content_lists_slide_then_elements_in_reading_order() {
    let s = slide(json!({
        "id": "s", "kind": "content",
        "params": { "title": "T", "blocks": [
            { "id": "b1", "kind": "text" },
            { "id": "st", "kind": "stack", "params": { "layout": "columns", "children": [
                { "id": "c1", "kind": "card" }
            ] } }
        ] }
    }));
    assert_that!(order(&s)).is_equal_to(vec![
        "s.kicker".to_owned(), "s.title".into(), "b1.body".into(),
        "c1.kicker".into(), "c1.title".into(), "c1.body".into(),
    ]);
}

#[test]
fn text_fields_skip_widgets_without_text_params() {
    let s = slide(json!({ "id": "s", "kind": "blank", "params": { "elements": [
        { "id": "f", "kind": "shape", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 } }
    ] } }));
    assert_that!(order(&s)).is_empty();
}

#[test]
fn text_fields_of_unknown_widget_are_empty() {
    let s = slide(json!({ "id": "s", "kind": "inconnu" }));
    assert_that!(order(&s)).is_empty();
}
```

Vérifier le nom de la fonction de registre de test (`crate::test_registry::shared()` renvoie un `Arc<WidgetRegistry>` d'après `undo_tests.rs`) et adapter l'emprunt (`&reg` ou `&*reg`).

Run: `cargo nextest run -p oku-slides-core fields`
Expected: FAIL (module absent).

- [ ] **Step 2: Implémentation**

`fields.rs` :

```rust
//! Champs texte d'une slide dans l'ordre de lecture : ceux de la slide, puis
//! ceux de chaque élément, parents avant enfants (ordre de Tab de l'éditeur).

use crate::model::{Element, Slide};
use crate::widget::{ParamSchema, ParamType, WidgetRegistry};

/// Objet qui porte un champ.
#[derive(Debug, Clone, Copy)]
pub enum FieldOwner<'a> {
    Slide,
    Element(&'a Element),
}

/// Un paramètre `plain` ou `rich` d'un objet de la slide.
#[derive(Debug, Clone, Copy)]
pub struct TextField<'a> {
    pub owner: FieldOwner<'a>,
    /// Id de la slide ou de l'élément.
    pub owner_id: &'a str,
    pub param: &'a ParamSchema,
}

fn text_params<'a>(registry: &'a WidgetRegistry, kind: &str) -> impl Iterator<Item = &'a ParamSchema> {
    registry
        .get(kind)
        .into_iter()
        .flat_map(|w| w.params.iter())
        .filter(|p| matches!(p.ty, ParamType::Plain | ParamType::Rich))
}

impl Slide {
    /// Champs texte dans l'ordre de lecture : slide (ordre du schéma), puis
    /// éléments (parcours des slots), chacun dans l'ordre de son schéma.
    pub fn text_fields<'a>(&'a self, registry: &'a WidgetRegistry) -> Vec<TextField<'a>> {
        let slide = text_params(registry, &self.kind)
            .map(|param| TextField { owner: FieldOwner::Slide, owner_id: &self.id, param });
        let elements = self.walk().flat_map(|l| {
            let e = l.element;
            text_params(registry, &e.kind)
                .map(move |param| TextField { owner: FieldOwner::Element(e), owner_id: &e.id, param })
        });
        slide.chain(elements).collect()
    }
}

#[cfg(test)]
#[path = "fields_tests.rs"]
mod fields_tests;
```

Vérifier les chemins réels de `ParamSchema`/`ParamType`/`WidgetRegistry` (`grep -n "pub struct ParamSchema\|pub enum ParamType" crates/oku-slides-core/src/widget.rs`) et que `registry.get` rend `Option<&WidgetSchema>`.

Dans `lib.rs` : `mod fields;`, `pub use fields::{FieldOwner, TextField};`, et une ligne dans la doc du module (table des matières).

- [ ] **Step 3: Tests**

Run: `cargo nextest run -p oku-slides-core`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/oku-slides-core/src/fields.rs crates/oku-slides-core/src/fields_tests.rs crates/oku-slides-core/src/lib.rs
```
```bash
git commit -m "feat(core) : champs texte d'une slide dans l'ordre de lecture (t-271)"
```

---

### Task 4: Cœur : fusion des étapes d'annulation par clé

**Files:**
- Modify: `crates/oku-slides-core/src/replica/undo.rs`
- Modify: `crates/oku-slides-core/src/replica/mod.rs:239-257` (`apply`)
- Modify: `crates/oku-slides-core/src/sources.rs:183-193` (`apply`)
- Test: `crates/oku-slides-core/src/replica/undo_tests.rs`
- Modify: tous les appelants de `DeckReplica::apply` et `SourceBundle::apply` (`grep -rn "\.apply(&\|sources.apply(" crates --include='*.rs'`)

**Interfaces:**
- Produces :
  - `DeckReplica::apply(&mut self, deck: &Deck, label: &str, merge: Option<&str>) -> Result<(), ReplicaError>`
  - `SourceBundle::apply(&mut self, deck: Deck, label: &str, merge: Option<&str>) -> Result<(), SourcesError>`
- Règle : l'application rejoint l'étape du sommet si cette étape porte la même clé `merge` ; l'étape garde le libellé de sa **première** application. Hors clé (`None`), ou clé différente : nouvelle étape. Un groupe ouvert (agent) l'emporte sur la clé. `undo`/`redo` effacent la clé des étapes déplacées.

- [ ] **Step 1: Tests qui échouent** (ajouter à `undo_tests.rs`)

```rust
fn subtitled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].params.set("subtitle", crate::ParamValue::Text(t.into()));
    d
}

#[test]
fn applies_with_the_same_merge_key_are_one_undo_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "Modifier la slide", Some("k1")).unwrap();
    r.apply(&subtitled(&one, "Deux"), "Modifier le texte", Some("k1")).unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Modifier la slide".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn apply_without_key_between_two_keyed_applies_splits_them() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    let two = subtitled(&one, "Agent");
    r.apply(&two, "Agent", None).unwrap();
    r.apply(&titled(&two, "Trois"), "B", Some("k1")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&two);
    assert_that!(r.undo_label()).is_equal_to(Some("Agent".to_owned()));
}

#[test]
fn apply_with_another_key_opens_a_new_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    r.apply(&titled(&deck, "Deux"), "B", Some("k2")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&one);
}

#[test]
fn keyed_apply_after_undo_does_not_join_the_redone_history() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    r.undo().unwrap();
    r.redo().unwrap();
    r.apply(&titled(&deck, "Deux"), "B", Some("k1")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&one);
}
```

Mettre à jour les appels existants du fichier : `r.apply(&x, "L")` → `r.apply(&x, "L", None)`.

Run: `cargo nextest run -p oku-slides-core undo`
Expected: FAIL à la compilation (arité).

- [ ] **Step 2: `undo.rs`**

```rust
/// Une étape d'annulation : `items` items Loro consécutifs, sous un libellé.
#[derive(Debug)]
struct Step {
    label: String,
    items: usize,
    /// Clé de fusion : une application de même clé rejoint l'étape.
    merge: Option<String>,
}
```

`close_step(&mut self, label: &str, merge: Option<&str>)` :

```rust
        match &mut self.group {
            Some((_, true)) => {
                if let Some(top) = self.undo.last_mut() {
                    top.items += items;
                }
            }
            Some((group, open)) => {
                self.undo.push(Step { label: group.clone(), items, merge: None });
                *open = true;
            }
            None => match (merge, self.undo.last_mut()) {
                (Some(k), Some(top)) if top.merge.as_deref() == Some(k) => top.items += items,
                _ => self.undo.push(Step { label: label.to_owned(), items, merge: merge.map(str::to_owned) }),
            },
        }
```

Dans `undo` et `redo`, les étapes poussées sur l'autre pile ont `merge: None`. Mettre à jour la doc du module (fusion par clé, en plus des groupes).

- [ ] **Step 3: `DeckReplica::apply` et `SourceBundle::apply`**

Ajouter le paramètre `merge: Option<&str>`, le passer à `close_step`. Doc de `apply` : « Avec `merge`, rejoint l'étape du sommet de même clé (voir `undo.rs`). » `SourceBundle::apply` le relaie. `restore` (qui appelle l'apply du replica) passe `None`.

- [ ] **Step 4: Appelants**

Tous les appels passent `None`, sauf `Session::commit` (tâche 5). `grep -rn "\.apply(&" crates/oku-slides-core crates/oku-slides-session crates/oku-slides-cli --include='*.rs'` puis corriger, tests compris.

- [ ] **Step 5: Tests**

Run: `cargo nextest run -p oku-slides-core`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-core crates/oku-slides-session crates/oku-slides-cli
```
```bash
git commit -m "feat(core) : étapes d'annulation fusionnées par clé (t-271)"
```

---

### Task 5: Session : `Apply` avec clé de fusion

**Files:**
- Modify: `crates/oku-slides-session/src/lib.rs:153-156, 580` (`Request::Apply`)
- Modify: `crates/oku-slides-session/src/deck_widgets.rs:109, 151` (`apply`, `commit`)
- Modify: `crates/oku-slides-mcp/src/edit_tools.rs:62`
- Modify: `ui/src/api.ts` (`Request`, `apply`)
- Test: `crates/oku-slides-session/src/lib_tests.rs`

**Interfaces:**
- Consumes : `SourceBundle::apply(deck, label, merge)` (tâche 4).
- Produces :
  - `Request::Apply { label: String, ops: Vec<Op>, #[serde(default)] merge: Option<String> }`
  - `Session::apply(&mut self, label: &str, ops: Vec<Op>, merge: Option<&str>) -> Result<()>`
  - TS : `apply: (label: string, ops: Op[], merge?: string) => call<Snapshot>({ cmd: "apply", label, ops, merge: merge ?? null })`

- [ ] **Step 1: Test qui échoue** (`lib_tests.rs`)

```rust
#[test]
fn handle_apply_with_same_merge_key_makes_one_undo_step() {
    let mut s = session_with_fixture();
    let before = s.deck().clone();
    let apply = |s: &mut Session, title: &str| {
        let req = json!({ "cmd": "apply", "label": "Modifier la slide", "ops": [title_op(s, title)], "merge": "k" });
        s.handle(serde_json::from_value(req).unwrap()).unwrap();
    };
    apply(&mut s, "Un");
    apply(&mut s, "Deux");
    s.handle(Request::Undo).unwrap();
    assert_that!(s.deck()).is_equal_to(&before);
}
```

`title_op` rend un `Op` : le sérialiser (`serde_json::to_value(title_op(s, title)).unwrap()`) dans le `json!`.

Run: `cargo nextest run -p oku-slides-session handle_apply_with_same_merge_key`
Expected: FAIL (`merge` ignoré : deux étapes, le deck n'est pas revenu).

- [ ] **Step 2: Implémentation**

`Request::Apply` gagne `#[serde(default)] merge: Option<String>` (doc : « Clé de session de saisie : les `Apply` consécutifs de même clé forment une étape d'annulation. »). Le match : `Request::Apply { label, ops, merge } => self.apply(&label, ops, merge.as_deref())?`. `Session::apply` et `commit` relaient jusqu'à `self.doc.sources.apply(next, label, merge)`. Les autres appels internes de `Session::apply` (`grep -n "self.apply(" crates/oku-slides-session/src`) passent `None`. MCP : `Request::Apply { label, ops, merge: None }`. Test existant ligne 155 : ajouter `merge: None`.

`ui/src/api.ts` : `| { cmd: "apply"; label: string; ops: Op[]; merge: string | null }` et la fonction ci-dessus.

- [ ] **Step 3: Tests**

Run: `cargo nextest run -p oku-slides-session -p oku-slides-mcp && (cd ui && bun run typecheck)`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/oku-slides-session crates/oku-slides-mcp ui/src/api.ts
```
```bash
git commit -m "feat(session) : Apply accepte une clé de fusion d'annulation (t-271)"
```

---

### Task 6: Session : `FieldView` et `SlideView.fields`

**Files:**
- Create: `crates/oku-slides-session/src/fields.rs`, `crates/oku-slides-session/src/fields_tests.rs`
- Modify: `crates/oku-slides-session/src/views.rs:160-250` (`TargetKind`, `Target`, `SlideView`)
- Modify: `crates/oku-slides-session/src/lib.rs:858-898` (`field_text`, `slide`)
- Modify: `crates/oku-slides-session/src/lib_tests.rs:215-265` (tests de vue réécrits)
- Modify: `crates/oku-slides-session/src/deck_widgets_tests.rs:114`

**Interfaces:**
- Consumes : `Slide::text_fields` (tâche 3), `Layout::fields_on`, `FieldBox { empty, side }`, `FieldSide` (tâche 1).
- Produces (JSON vers l'UI) :

```rust
pub struct FieldView<'a> {
    pub owner: &'a str,
    pub owner_kind: OwnerKind,            // "slide" | "element"
    pub widget: &'a str,                  // kind de l'objet
    pub field: &'a str,
    pub kind: FieldKind,                  // "plain" | "rich"
    pub label: Option<&'a str>,           // label du schéma
    pub text: Cow<'a, str>,
    pub empty: bool,
    pub side: Option<FieldSide>,
    pub x: f64, pub y: f64, pub w: f64, pub h: f64,
    pub ax: f64, pub aw: f64,
    pub style: Option<&'a TextStyle>,
}
pub struct Target<'a> { pub id: &'a str, pub kind: &'a str, pub free: bool, pub x: f64, pub y: f64, pub w: f64, pub h: f64 }
pub struct SlideView<'a> { …, pub targets: Vec<Target<'a>>, pub fields: Vec<FieldView<'a>>, … }
```

`TargetKind` disparaît (toutes les boîtes sont des éléments). `OwnerKind` et `FieldKind` : `#[serde(rename_all = "lowercase")]`.

- [ ] **Step 1: Tests qui échouent**

`fields_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::lib_tests_support::session_with_fixture;
use super::*;

fn names(view: &crate::views::SlideView<'_>) -> Vec<String> {
    view.fields.iter().map(|f| format!("{}.{}", f.owner, f.field)).collect()
}

#[test]
fn slide_view_fields_of_cards_follow_reading_order() {
    let s = session_with_fixture();
    let v = s.slide("s-priorites").unwrap();
    assert_that!(names(&v)).is_equal_to(
        ["s-priorites.kicker", "s-priorites.title",
         "c1.kicker", "c1.title", "c1.body", "c2.kicker", "c2.title", "c2.body",
         "c3.kicker", "c3.title", "c3.body"]
            .map(str::to_owned).to_vec(),
    );
}

#[test]
fn slide_view_bound_title_field_shows_the_deck_title() {
    let s = session_with_fixture();
    let v = s.slide("s-title").unwrap();
    let title = v.fields.iter().find(|f| f.field == "title").unwrap();
    assert_that!(title.owner_kind).is_equal_to(OwnerKind::Slide);
    assert_that!(title.text.as_ref()).is_equal_to("Feuille de route {{annee}}");
}

#[test]
fn slide_view_rich_field_carries_its_markdown_text() {
    let s = session_with_fixture();
    let v = s.slide("s-constat").unwrap();
    let body = v.fields.iter().find(|f| f.owner == "b-points").unwrap();
    assert_that!(body.kind).is_equal_to(FieldKind::Rich);
    assert_that!(body.text.as_ref()).is_not_equal_to("");
}

#[test]
fn slide_view_empty_subtitle_is_listed_empty() {
    let s = session_with_fixture();
    let v = s.slide("s-ch2").unwrap();
    let sub = v.fields.iter().find(|f| f.field == "subtitle").unwrap();
    assert_that!(sub.empty).is_true();
    assert_that!(sub.text.as_ref()).is_equal_to("");
}

#[test]
fn slide_view_targets_are_element_boxes_only() {
    let s = session_with_fixture();
    let v = s.slide("s-libre").unwrap();
    let shape = v.targets.iter().find(|t| t.id == "e-fond").unwrap();
    assert_that!(shape.kind).is_equal_to("shape");
    assert_that!(shape.free).is_true();
    assert_that!(v.targets.iter().any(|t| t.id.starts_with('@'))).is_false();
}
```

`session_with_fixture` est aujourd'hui privé à `lib_tests.rs` : le déplacer, avec `data_dir`, `session_in`, `fixture_path`, dans un module `#[cfg(test)] pub(crate) mod lib_tests_support;` (`src/lib_tests_support.rs`), importé par `lib_tests.rs` et `fields_tests.rs`. Supprimer de `lib_tests.rs` les trois tests `slide_view_of_*` (couverts ici).

Run: `cargo nextest run -p oku-slides-session fields`
Expected: FAIL (module absent).

- [ ] **Step 2: `views.rs`**

Remplacer `TargetKind` et `Target` par la nouvelle `Target` (boîte d'élément) ; `Target::element(b: &'a ElementBox, kind: &'a str, free: bool)`. Ajouter à `SlideView` : `/// Champs éditables, dans l'ordre de Tab. pub fields: Vec<FieldView<'a>>,`. Déclarer `FieldView`, `OwnerKind`, `FieldKind` dans `fields.rs` et les réexporter depuis `views.rs` n'est pas nécessaire : `SlideView` les importe de `crate::fields`.

- [ ] **Step 3: `fields.rs`**

```rust
//! Champs éditables d'une slide pour l'interface : boîte rendue, texte tel
//! que saisi, dans l'ordre de Tab (`Slide::text_fields`).

use std::borrow::Cow;

use serde::Serialize;

use oku_slides_core::{Deck, FieldOwner, ParamType, Slide, WidgetRegistry, paragraphs_to_text};
use oku_slides_render::{FieldBox, FieldSide, TextStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnerKind { Slide, Element }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind { Plain, Rich }

/// Un champ éditable en place, posé sur son texte rendu.
#[derive(Debug, Serialize)]
pub struct FieldView<'a> { /* champs de l'interface ci-dessus */ }

/// Texte d'un paramètre `plain` de la slide : la valeur liée (`bind`) l'emporte, comme au rendu.
pub(crate) fn slide_text<'d>(deck: &'d Deck, slide: &'d Slide, registry: &WidgetRegistry, field: &str) -> Option<&'d str> {
    let bind = registry.get(&slide.kind).and_then(|w| w.param(field)).and_then(|p| p.bind.as_deref());
    match bind {
        Some(key) => deck.params.field(key),
        None => slide.params.text(field),
    }
}

/// Champs de `slide` dont le rendu a posé une boîte sur `boxes`, dans l'ordre de Tab.
pub(crate) fn field_views<'a>(
    deck: &'a Deck,
    slide: &'a Slide,
    registry: &'a WidgetRegistry,
    boxes: &'a [FieldBox],
) -> Vec<FieldView<'a>> {
    slide
        .text_fields(registry)
        .into_iter()
        .filter_map(|tf| {
            let b = boxes.iter().find(|b| b.owner == tf.owner_id && b.field == tf.param.name)?;
            let (owner_kind, widget, text) = match tf.owner {
                FieldOwner::Slide => {
                    (OwnerKind::Slide, slide.kind.as_str(), Cow::Borrowed(slide_text(deck, slide, registry, &tf.param.name).unwrap_or_default()))
                }
                FieldOwner::Element(e) => {
                    let text = match tf.param.ty {
                        ParamType::Rich => Cow::Owned(paragraphs_to_text(e.params.rich(&tf.param.name).unwrap_or_default())),
                        _ => Cow::Borrowed(e.params.text(&tf.param.name).unwrap_or_default()),
                    };
                    (OwnerKind::Element, e.kind.as_str(), text)
                }
            };
            Some(FieldView {
                owner: tf.owner_id,
                owner_kind,
                widget,
                field: &tf.param.name,
                kind: if tf.param.ty == ParamType::Rich { FieldKind::Rich } else { FieldKind::Plain },
                label: tf.param.label.as_deref(),
                text,
                empty: b.empty,
                side: b.side,
                x: b.x, y: b.y, w: b.w, h: b.h,
                ax: b.ax.unwrap_or(b.x).min(b.x),
                aw: b.aw.unwrap_or(b.w).max(b.w),
                style: b.style.as_ref(),
            })
        })
        .collect()
}
```

Un `FieldBox` sans champ texte déclaré n'est jamais cherché, donc ignoré ; ajouter après le `collect` une trace :
`for b in boxes.iter().filter(|b| b.owner == slide.id || slide.locate(&b.owner).is_some()) { if !views.iter().any(|v| v.owner == b.owner && v.field == b.field) { tracing::debug!(owner = %b.owner, field = %b.field, "champ rendu sans paramètre texte déclaré"); } }`.

Vérifier où vit `paragraphs_to_text` aujourd'hui (`grep -rn "fn paragraphs_to_text" crates`) et l'importer de là, sans le copier (G2).

- [ ] **Step 4: `Session::slide`**

Dans `lib.rs`, supprimer `field_text` (remplacé par `fields::slide_text`) et réécrire le corps :

```rust
        let targets = layout
            .boxes_on(page)
            .iter()
            .filter_map(|b| {
                let l = slide.locate(&b.id)?;
                Some(Target::element(b, &l.element.kind, l.is_free()))
            })
            .collect();
        let fields = fields::field_views(deck, slide, &self.registry, layout.fields_on(page));
```

puis `SlideView { …, targets, fields, … }`. `SlideSummary.title` utilisait peut-être `field_text` : le faire passer par `fields::slide_text`.

- [ ] **Step 5: Tests**

Run: `cargo nextest run -p oku-slides-session`
Expected: PASS (dont `deck_widgets_tests.rs:114`, qui cherche une cible d'élément par id : inchangé).

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-session
```
```bash
git commit -m "feat(session) : SlideView.fields dans l'ordre de Tab, cibles réduites aux éléments (t-271)"
```

---

### Task 7: UI : types, ops et navigation des champs

**Files:**
- Modify: `ui/src/api.ts:176-205` (`EditTarget` → `ObjectTarget`, `FieldView`, `SlideView.fields`)
- Modify: `ui/src/deck.ts:98-118` (`textChanged`, `textOps` → `fieldChanged`, `fieldOps`)
- Modify: `ui/src/deck.test.ts`
- Create: `ui/src/stageFields.ts`, `ui/src/stageFields.test.ts`

**Interfaces:**
- Consumes : JSON de la tâche 6.
- Produces :

```ts
export interface ObjectTarget { id: string; kind: string; free: boolean; x: number; y: number; w: number; h: number }
export interface FieldView {
  owner: string; owner_kind: "slide" | "element"; widget: string; field: string;
  kind: "plain" | "rich"; label: string | null; text: string; empty: boolean;
  side: "before" | "after" | null;
  x: number; y: number; w: number; h: number; ax: number; aw: number; style: TextStyle | null;
}
// deck.ts
export function fieldChanged(f: Pick<FieldView, "kind" | "text">, text: string): boolean;
export function fieldOps(view: SlideView, f: FieldView, value: string): Op[];
// stageFields.ts
export type FieldKey = string;
export const fieldKey: (f: Pick<FieldView, "owner" | "field">) => FieldKey;
export function findField(view: SlideView, key: FieldKey | null): FieldView | null;
export function nextField(view: SlideView, key: FieldKey, dir: 1 | -1): FieldView | null;
export function firstFieldOf(view: SlideView, owner: string): FieldView | null;
export function fieldLabel(f: Pick<FieldView, "field" | "label">): string;
export function lineHeight(style: TextStyle, scale: number): number;
export function fieldHitBox(f: FieldView, scale: number): { left: number; top: number; width: number; height: number };
```

- [ ] **Step 1: Tests qui échouent**

`stageFields.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import type { FieldView, SlideView, TextStyle } from "./api";
import { fieldHitBox, fieldKey, fieldLabel, findField, firstFieldOf, nextField } from "./stageFields";

const style: TextStyle = { size: 20, leading: 6, tracking: 0, font: ["Inter"], weight: 400, fill: null, align: "left", case: null, bg: null };
const f = (owner: string, field: string, extra: Partial<FieldView> = {}): FieldView => ({
  owner, owner_kind: owner === "s" ? "slide" : "element", widget: "card", field, kind: "plain", label: null,
  text: "x", empty: false, side: null, x: 10, y: 100, w: 50, h: 20, ax: 10, aw: 200, style, ...extra,
});
const view = (fields: FieldView[]) => ({ fields } as unknown as SlideView);
const v = view([f("s", "title"), f("c1", "kicker"), f("c1", "title"), f("c2", "title")]);

describe("nextField", () => {
  test("nextField forward returns the following field", () => {
    expect(nextField(v, fieldKey({ owner: "c1", field: "kicker" }), 1)?.field).toBe("title");
  });
  test("nextField backward from the first returns null", () => {
    expect(nextField(v, fieldKey({ owner: "s", field: "title" }), -1)).toBeNull();
  });
  test("nextField forward from the last returns null", () => {
    expect(nextField(v, fieldKey({ owner: "c2", field: "title" }), 1)).toBeNull();
  });
  test("nextField with a vanished key returns null", () => {
    expect(nextField(v, fieldKey({ owner: "zz", field: "title" }), 1)).toBeNull();
  });
});

test("findField by key survives a reordered view", () => {
  const k = fieldKey({ owner: "c1", field: "title" });
  expect(findField(view([...v.fields].reverse()), k)?.owner).toBe("c1");
});

test("firstFieldOf an element returns its first field in tab order", () => {
  expect(firstFieldOf(v, "c1")?.field).toBe("kicker");
});

test("fieldLabel of a known field uses the translation, else the schema label", () => {
  expect(fieldLabel({ field: "title", label: null })).not.toBe("title");
  expect(fieldLabel({ field: "zz-inconnu", label: "Légende" })).toBe("Légende");
  expect(fieldLabel({ field: "zz-inconnu", label: null })).toBe("zz-inconnu");
});

describe("fieldHitBox", () => {
  test("fieldHitBox of an empty field before its anchor ends at the anchor", () => {
    const b = fieldHitBox(f("c1", "kicker", { empty: true, side: "before", h: 0, w: 0 }), 1);
    expect(b.top + b.height).toBeCloseTo(100);
    expect(b.height).toBeGreaterThan(0);
    expect(b.width).toBeCloseTo(200);
  });
  test("fieldHitBox of an empty field after its anchor starts at the anchor", () => {
    const b = fieldHitBox(f("c1", "sub", { empty: true, side: "after", h: 0, w: 0 }), 2);
    expect(b.top).toBeCloseTo(200);
  });
  test("fieldHitBox of a filled field is its rendered box", () => {
    expect(fieldHitBox(f("c1", "title"), 2)).toEqual({ left: 20, top: 200, width: 100, height: 40 });
  });
});
```

Dans `deck.test.ts`, remplacer les tests de `textOps`/`textChanged` par :

```ts
test("fieldOps of a slide field sets the slide param as text", () => {
  const ops = fieldOps(viewWith(slide), { ...fv, owner: slide.id, owner_kind: "slide", field: "title", kind: "plain" }, "Nouveau");
  expect(ops).toEqual([{ op: "set_param", target: { slide: slide.id }, param: "title", value: "Nouveau" }]);
});
test("fieldOps of an element rich field sets paragraphs", () => {
  const ops = fieldOps(viewWith(slide), { ...fv, owner: "c1", owner_kind: "element", field: "body", kind: "rich" }, "- a");
  expect(ops[0]).toMatchObject({ op: "set_param", target: { slide: slide.id, element: "c1" }, param: "body" });
  expect(Array.isArray((ops[0] as { value: unknown }).value)).toBe(true);
});
test("fieldOps of an element plain field sets its text", () => {
  const ops = fieldOps(viewWith(slide), { ...fv, owner: "c1", owner_kind: "element", field: "title", kind: "plain" }, "T");
  expect(ops).toEqual([{ op: "set_param", target: { slide: slide.id, element: "c1" }, param: "title", value: "T" }]);
});
test("fieldChanged of a plain field ignores newlines turned into spaces", () => {
  expect(fieldChanged({ kind: "plain", text: "a\nb" }, "a b")).toBe(false);
});
```

en réutilisant les fixtures `slide`/`viewWith` du fichier (adapter les noms à ceux qui existent ; `fv` est un `FieldView` de base construit comme `f()` ci-dessus).

Run: `cd ui && bun test stageFields deck.test`
Expected: FAIL (module et fonctions absents).

- [ ] **Step 2: `api.ts`**

Remplacer `EditTarget` par `ObjectTarget` et ajouter `FieldView` (interfaces ci-dessus, avec une ligne de doc chacune) ; `SlideView.targets: ObjectTarget[]` et `fields: FieldView[]`.

- [ ] **Step 3: `deck.ts`**

```ts
/** Le brouillon change-t-il le champ ? Un champ `plain` compare ses retours à la ligne en espaces. */
export function fieldChanged(f: Pick<FieldView, "kind" | "text">, text: string): boolean {
  return text !== (f.kind === "plain" ? f.text.replaceAll("\n", " ") : f.text);
}

/** Ops pour écrire le brouillon d'un champ : `set_param`, en paragraphes pour un champ `rich`. */
export function fieldOps(view: SlideView, f: FieldView, value: string): Op[] {
  const slide = view.slide;
  const target = f.owner_kind === "slide" ? { slide: slide.id } : { slide: slide.id, element: f.owner };
  if (f.kind === "plain") return [{ op: "set_param", target, param: f.field, value }];
  const e = findElement(slide, f.owner);
  const previous = { paragraphs: (e && rich(e, f.field)) ?? [], text: f.text };
  return [{ op: "set_param", target, param: f.field, value: textToParagraphs(value, previous) }];
}
```

Supprimer `textChanged` et `textOps` ; mettre à jour leurs appelants à la tâche 9 (`App.tsx`, `Stage.tsx`) — la compilation TS casse entre les tâches 7 et 9 : **les tâches 7, 8 et 9 se commitent ensemble**, à la fin de la tâche 9 (voir son step final). Les tests purs de cette tâche passent avant.

- [ ] **Step 4: `stageFields.ts`**

```ts
/**
 * Champs de la scène : clé stable, navigation au clavier, libellé du
 * placeholder et boîte cliquable (un champ vide a une ligne de haut).
 */
import type { FieldView, SlideView, TextStyle } from "./api";
import { t } from "./i18n";

export type FieldKey = string;

/** Clé d'un champ : objet et paramètre (séparateur d'unité ASCII, absent des ids). */
export const fieldKey = (f: Pick<FieldView, "owner" | "field">): FieldKey => `${f.owner}\u001f${f.field}`;

export function findField(view: SlideView, key: FieldKey | null): FieldView | null {
  if (key === null) return null;
  return view.fields.find((f) => fieldKey(f) === key) ?? null;
}

/** Champ suivant (`1`) ou précédent (`-1`) dans l'ordre de Tab ; `null` au bout. */
export function nextField(view: SlideView, key: FieldKey, dir: 1 | -1): FieldView | null {
  const i = view.fields.findIndex((f) => fieldKey(f) === key);
  if (i < 0) return null;
  return view.fields[i + dir] ?? null;
}

export function firstFieldOf(view: SlideView, owner: string): FieldView | null {
  return view.fields.find((f) => f.owner === owner) ?? null;
}

/** Libellé du placeholder : traduction du nom, sinon libellé du schéma, sinon le nom. */
export function fieldLabel(f: Pick<FieldView, "field" | "label">): string {
  const translated = t("ui-field", { kind: f.field });
  return translated !== f.field ? translated : f.label ?? f.field;
}

/** Hauteur d'une ligne (px écran), comme l'éditeur : capitale + interligne, au moins 1,05 em. */
export function lineHeight(style: TextStyle, scale: number): number {
  return Math.max(style.size * 0.7 + style.leading, style.size * 1.05) * scale;
}

/** Boîte cliquable (px écran). Un champ vide a une ligne de haut, avant ou après son ancre. */
export function fieldHitBox(f: FieldView, scale: number) {
  if (!f.empty || f.h > 0) {
    return { left: f.x * scale, top: f.y * scale, width: Math.max(f.w * scale, 8), height: Math.max(f.h * scale, 8) };
  }
  const line = f.style ? lineHeight(f.style, scale) : 16;
  const top = f.side === "after" ? f.y * scale : f.y * scale - line;
  return { left: f.ax * scale, top, width: f.aw * scale, height: line };
}
```

Le test `fieldHitBox of a filled field` attend `height: 40` : `Math.max(20 * 2, 8)` = 40, cohérent. Remplacer dans `Stage.tsx` le calcul de `line` d'`editorGeometry` par `lineHeight` (tâche 9) : une seule formule.

- [ ] **Step 5: Tests purs**

Run: `cd ui && bun test stageFields deck.test`
Expected: PASS. (Pas de commit : voir tâche 9.)

---

### Task 8: i18n : clés des champs

**Files:**
- Modify: `crates/oku-slides-i18n/src/locales/{ar,bn,de,en,es,fr,hi,it,ja,ko,nl,pl,pt,ru,th,tr,uk,vi,zh-Hans,zh-Hant}/ui.ftl`

**Interfaces:**
- Produces : clés `ui-field` complétée (`body`, `author`, `date`, `organization`), `ui-undo-edit-slide`, `ui-toast-field-gone`, `ui-stage-hint-tab` ; retrait de `ui-stage-hint-field` et `ui-stage-hint-block` remplacés par `ui-stage-hint-plain` et `ui-stage-hint-rich`.

- [ ] **Step 1: Français (référence) et anglais**

`fr/ui.ftl` :

```ftl
ui-undo-edit-slide = Modifier la slide
ui-toast-field-gone = Le champ modifié n'existe plus : la saisie est perdue.
ui-stage-hint-plain = Entrée ou Tab pour passer au suivant · Échap pour annuler
ui-stage-hint-rich = Tab pour passer au suivant · ⌘ Entrée pour valider · « - » pour une puce · ⌘B gras, ⌘I italique, ⌘K lien
ui-field = { $kind ->
    [title] Titre
    [subtitle] Sous-titre
    [kicker] Surtitre
    [text] Texte
    [body] Corps
    [author] Auteur
    [date] Date
    [organization] Organisation
   *[other] { $kind }
}
```

`en/ui.ftl` : `Edit slide` ; `The field you were editing no longer exists: your input is lost.` ; `Enter or Tab for next · Esc to cancel` ; `Tab for next · ⌘ Enter to confirm · "-" for a bullet · ⌘B bold, ⌘I italic, ⌘K link` ; `Title`, `Subtitle`, `Kicker`, `Text`, `Body`, `Author`, `Date`, `Organization`.

- [ ] **Step 2: Les 18 autres langues**

Traduire les mêmes clés dans chaque `ui.ftl`, en gardant la ponctuation typographique de la langue (espaces fines en français, guillemets locaux). Retirer `ui-stage-hint-field` et `ui-stage-hint-block` des 20 fichiers.

- [ ] **Step 3: Tests**

Run: `cargo nextest run -p oku-slides-i18n && (cd ui && bun test i18n)`
Expected: PASS (`every_locale_has_exactly_the_reference_keys`, `i18n-keys.test.ts` : ce dernier échoue si `Stage.tsx` cite encore les clés retirées — c'est attendu jusqu'à la tâche 9, relancer alors).

---

### Task 9: UI : couches, gestes et éditeur par champ

**Files:**
- Modify: `ui/src/Stage.tsx`
- Modify: `ui/src/App.tsx` (état `editing`, `apply` avec clé, dictée, copier, touches)
- Modify: `ui/src/agent/ui.ts` (+ son test)
- Modify: `ui/src/styles.css` (`.field`, `.field-placeholder`)
- Create: `ui/src/Stage.render.test.tsx`

**Interfaces:**
- Consumes : `FieldView`, `ObjectTarget`, `fieldOps`, `fieldChanged` (tâche 7), `stageFields.ts`, clés i18n (tâche 8), `api.apply(label, ops, merge)` (tâche 5).
- Produces :
  - `Stage` props : `editing: FieldKey | null`, `onEdit: (key: FieldKey | null) => void`, `onApply: (label: string, ops: Op[], merge?: string) => void`, reste inchangé.
  - `agentUi(current: string | null, selected: string | null, editing: FieldView | null): AgentUi`.

- [ ] **Step 1: Test de rendu qui échoue**

`Stage.render.test.tsx` (modèle : `SlidePile.render.test.tsx` pour l'installation de happy-dom via `./test/dom`) :

```tsx
import "./test/dom";
import { expect, test } from "bun:test";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { FieldView, SlideView } from "./api";
import Stage from "./Stage";

const style = { size: 20, leading: 6, tracking: 0, font: ["Inter"], weight: 400, fill: null, align: "left", case: null, bg: null } as const;
const fv = (owner: string, field: string, text: string, y: number): FieldView => ({
  owner, owner_kind: "element", widget: "card", field, kind: "plain", label: null, text, empty: false, side: null,
  x: 10, y, w: 100, h: 20, ax: 10, aw: 300, style,
});
const view = {
  slide: { id: "s", kind: "content", params: { blocks: [{ id: "c1", kind: "card", params: { title: "Titre", kicker: "Sur" } }] } },
  page: 1, hash: "h", width: 1920, height: 1080, zone: null,
  targets: [{ id: "c1", kind: "card", free: false, x: 0, y: 0, w: 400, h: 200 }],
  fields: [fv("c1", "kicker", "Sur", 20), fv("c1", "title", "Titre", 60)],
} as unknown as SlideView;

function mount(props: Partial<Parameters<typeof Stage>[0]> = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const calls: { edit: (string | null)[]; apply: unknown[][] } = { edit: [], apply: [] };
  const root = createRoot(host);
  act(() => root.render(
    <Stage view={view} svgUrl={null} selected={null} editing={null} palette={[]}
      onSelect={() => {}} onEdit={(k) => calls.edit.push(k)} onApply={(...a) => calls.apply.push(a)} {...props} />,
  ));
  return { host, calls, root };
}

test("Stage double click on a card title edits that field", () => {
  const { host, calls } = mount();
  const title = host.querySelector('.field[aria-label="Titre"]')!;
  act(() => title.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
  expect(calls.edit).toEqual(["c1\u001ftitle"]);
});

test("Stage editing a field opens the editor on its text", () => {
  const { host } = mount({ editing: "c1\u001ftitle" });
  expect(host.querySelector(".editor .cm-content")?.textContent).toBe("Titre");
});

test("Stage empty field of the selected element shows its placeholder", () => {
  const empty = { ...fv("c1", "kicker", "", 60), empty: true, side: "before" as const, h: 0, w: 0 };
  const v = { ...view, fields: [empty, fv("c1", "title", "Titre", 60)] } as SlideView;
  const { host } = mount({ view: v, selected: "c1" });
  expect(host.querySelector(".field-placeholder")?.textContent).toBe("Surtitre");
});
```

Le libellé attendu (`Surtitre`) suppose l'interface en français dans les tests de rendu : vérifier la langue que fixe `./test/dom` et adapter (ou utiliser `fieldLabel(...)` pour calculer l'attendu).

Run: `cd ui && bun test Stage.render`
Expected: FAIL.

- [ ] **Step 2: `Stage.tsx` — couches**

- Supprimer `fieldLabel` exporté (déplacé dans `stageFields.ts`), le tri `ordered`, `TargetKind`/`t.target` et les `@`.
- Couche objets : `view.targets.map(…)` comme aujourd'hui, sans `onDoubleClick` de texte ; un double-clic sur un objet hors champ ouvre `firstFieldOf(view, tg.id)` :
  `onDoubleClick={() => { const f = firstFieldOf(view, tg.id); if (f) onEdit(fieldKey(f)); }}`.
- Couche champs, après la couche objets (au-dessus), masquée si `history` — la scène le reçoit déjà par `onEdit === noop` ; ajouter une prop explicite `readOnly?: boolean` passée par `App` (`history !== null`) et ne pas rendre la couche champs quand elle est vraie :

```tsx
<div className="fields" role="group" aria-label={t("ui-stage-fields")}>
  {view.fields.map((f) => {
    const key = fieldKey(f);
    const box = fieldHitBox(f, scale);
    const showPlaceholder = f.empty && key !== editing && (templates || f.owner === selected || (editField?.owner === f.owner));
    return (
      <div
        key={key}
        className={`field${f.empty ? " empty" : ""}`}
        style={box}
        aria-label={fieldLabel(f)}
        onPointerDown={(e) => {
          if (editing) {
            e.stopPropagation();
            if (key !== editing) switchTo(key);
            return;
          }
          const owner = f.owner_kind === "element" ? view.targets.find((x) => x.id === f.owner) : undefined;
          if (owner) startDrag(e, owner, "move");
        }}
        onDoubleClick={() => onEdit(key)}
      >
        {showPlaceholder && <span className="field-placeholder" style={placeholderStyle(f, scale)}>{fieldLabel(f)}</span>}
      </div>
    );
  })}
</div>
```

`templates` : nouvelle prop booléenne `templates?: boolean` (mode modèles), passée par `App` (`!!snap.editing`). Ajouter la clé `ui-stage-fields` (« Champs de la slide ») aux 20 langues avec la tâche 8 si elle n'y est pas encore (la tâche 8 est commitée avec celle-ci).

`placeholderStyle(f, scale)` : même typographie que l'éditeur (police, taille, graisse, casse, alignement dans `ax`/`aw`), `opacity: 0.45`. Extraire de `editorGeometry` une fonction `typography(style, scale)` qui rend `{ fontFamily, fontSize, fontWeight, lineHeight, letterSpacing, textAlign, textTransform }`, utilisée par les deux.

- [ ] **Step 3: `Stage.tsx` — édition**

- `const editField = findField(view, editing);` remplace `editTarget`. Il est relu à chaque rendu : quand un nouveau `SlideView` arrive, la géométrie se recale.
- Si `editing !== null && editField === null` (champ disparu après un nouveau rendu) : effet qui appelle `onEdit(null)` et `say`… la scène n'a pas de toast : ajouter une prop `onNotice?: (msg: string) => void` fournie par `App` (`(m) => say(m, true)`), appelée avec `t("ui-toast-field-gone")`. Le brouillon n'est pas commité : le champ n'est plus dans la vue, donc son objet ou son paramètre n'existe plus.
- Clé de session : `const session = useRef<string | null>(null)`. À l'entrée en édition (transition `editing` de `null` à une clé) : `session.current = crypto.randomUUID()`. À la sortie (`editing` repasse à `null`) : `session.current = null`.
- Valider un champ :

```ts
const save = (f: FieldView, text: string) => {
  if (fieldChanged(f, text)) onApply(t("ui-undo-edit-slide"), fieldOps(view, f, text), session.current ?? undefined);
};
```

- `TextEditor` reçoit de nouveaux rappels : `onCommit(text)` (⌘Entrée, clic hors champ, blur) → `save(editField, text); onEdit(null)` ; `onNext(text, dir)` (Tab, Shift-Tab, Entrée en `plain`) → `save(editField, text); const n = nextField(view, editing, dir); onEdit(n ? fieldKey(n) : null)` ; `onCancel` → `onEdit(null)`.
- `switchTo(key)` (clic sur un autre champ en édition) : lit le texte courant via la poignée (`editor.current?.text()`, à ajouter à `TextEditorHandle`), `save`, puis `onEdit(key)`. Marquer la fin pour que le `blur` qui suit ne valide pas une seconde fois : le `TextEditor` démonté par le changement de `key` ignore son blur (déjà le cas : `done = true` au nettoyage).
- `editorGeometry(f: FieldView, draft, scale)` : partir de `f` ; `plain` → largeur naturelle bornée par `aw` (code actuel de la branche champ) ; `rich` → `f.w` ; hauteur de ligne par `lineHeight`. Supprimer la branche sans style : sans `style`, utiliser `fieldHitBox` et la police de l'interface.
- Masque (`editor-mask`) : sur `fieldHitBox(editField, scale)`, élargi de 2 px.
- Indication : `t(editField.kind === "plain" ? "ui-stage-hint-plain" : "ui-stage-hint-rich")`.

- [ ] **Step 4: `TextEditor.tsx`**

- Props : `field: boolean` devient `plain: boolean` ; ajouter `onNext: (text: string, dir: 1 | -1) => void`.
- Touches : `{ key: "Tab", run: (v) => next(v, 1), shift: (v) => next(v, -1) }` pour les deux sortes ; en `plain`, `Enter` → `next(v, 1)` au lieu de `finish(v, true)`. `next` suit le même verrou `done` que `finish`.
- `TextEditorHandle` gagne `text(): string` (contenu courant).
- Mettre à jour `TextEditor.test.ts` si des tests nomment `field`.

- [ ] **Step 5: `App.tsx`**

- `const [editing, setEditing] = useState<FieldKey | null>(null)` ; `const editField = view ? findField(view, editing) : null`.
- `apply` : `async (label: string, ops: Op[], merge?: string) => { await run(api.apply(label, ops, merge)); }`.
- `Stage` : `editing`, `onEdit={history ? noop : setEditing}`, `onApply={history ? noop : apply}`, `readOnly={!!history}`, `templates={!!snap.editing}`, `onNotice={(m) => say(m, true)}`.
- Dictée hors édition : `writable` devient `const writable = view && selected ? firstFieldOf(view, selected) : null;` ; `insertTo` : `writable ? (writable.kind === "plain" ? t("ui-dictation-to-field", { field: fieldLabel(writable).toLowerCase() }) : t("ui-dictation-to-block")) : …` ; `insertText` : `const keep = writable.kind === "rich" && previous.trim() !== "" && previous !== t("ui-new-text"); apply(t("ui-undo-dictation"), fieldOps(view, writable, keep ? \`${previous}\n${text}\` : text));`.
- Copier (ligne 245) : retirer `selected.startsWith("@")` ; le texte copié est `firstFieldOf(view, selected)?.text || element.kind`.
- Touche Entrée (ligne 565) : `const f = firstFieldOf(view, selected); if (f) { e.preventDefault(); setEditing(fieldKey(f)); }`. Suppression (ligne 570) : retirer la condition `@`.
- Agent : `agentUi(current, selected, editField)`.

- [ ] **Step 6: `agent/ui.ts`**

```ts
/** Ce que voit l'agent : slide, élément sélectionné ou en édition, champ en édition. */
export function agentUi(current: string | null, selected: string | null, editing: FieldView | null): AgentUi {
  const element = editing ? (editing.owner_kind === "element" ? editing.owner : null) : selected;
  return { slide: current, element, field: editing?.field ?? null, editing: editing !== null };
}
```

Réécrire son test (`ui/src/agent/*.test.ts` qui appelle `agentUi` : `grep -rn agentUi ui/src`) : champ de slide en édition → `{ element: null, field: "title", editing: true }` ; champ de carte → `{ element: "c1", field: "body", editing: true }` ; sélection seule → `{ element: "c1", field: null, editing: false }`.

- [ ] **Step 7: CSS**

`styles.css` :

```css
.fields { position: absolute; inset: 0; pointer-events: none; }
.field { position: absolute; pointer-events: auto; }
/* Champ de l'objet sélectionné ou en édition : on y écrit. */
.field.hot { cursor: text; }
.field.hot.empty:hover { background: color-mix(in srgb, var(--accent) 8%, transparent); border-radius: 4px; }
.field-placeholder { position: absolute; inset: 0; opacity: 0.45; white-space: nowrap; pointer-events: none; }
```

La scène pose `hot` sur un champ quand `f.owner === selected`, quand un champ du même objet est en édition, ou en mode modèles.

- [ ] **Step 8: Tests et vérifications**

Run: `cd ui && bun test && bun run typecheck`
Expected: PASS (tests purs, rendu, `i18n-keys.test.ts`).
Run: `cargo nextest run -p oku-slides-i18n`
Expected: PASS.

Vérification manuelle dans l'app (`preview_start`, voir `.claude/launch.json`) : double-clic sur le titre d'une carte, Tab jusqu'au corps, ⌘Z annule les deux en une fois ; placeholder « Surtitre » visible sur une carte sélectionnée sans surtitre ; capture d'écran jointe au rapport de tâche.

- [ ] **Step 9: Commit (tâches 7, 8 et 9)**

```bash
git add ui crates/oku-slides-i18n
```
```bash
git commit -m "feat(ui) : édition en place par champ, Tab entre champs, placeholders (t-271)"
```

---

### Task 10: E2E : éditeur et alignement des champs

**Files:**
- Modify: `ui/e2e/editor.cjs` (sélecteurs)
- Create: `ui/e2e/fields.cjs`
- Modify: `ui/package.json` (`e2e`)
- Modify: `crates/oku-slides-render/ARCHITECTURE.md`, `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-core/ARCHITECTURE.md` (une ligne chacun)

**Interfaces:**
- Consumes : la scène de la tâche 9 (`.field[aria-label=…]`, `.editor .cm-content`), `oku-slides serve fixtures/roadmap.json`.

- [ ] **Step 1: `editor.cjs`**

`open(p, label)` double-cliquait `.target[aria-label=…]`. Le remplacer par le champ : `p.dblclick(\`.field[aria-label="${label}"] >> nth=0\`)`. Les étiquettes utilisées (`Texte`…) deviennent celles des champs (`Corps` pour le corps d'un texte). Relancer `bun e2e/editor.cjs` contre un serveur neuf et corriger chaque sélecteur.

- [ ] **Step 2: `fields.cjs` — le test qui porte la condition de l'utilisateur**

```js
// Alignement : l'éditeur ouvert recouvre exactement le texte rendu (bord gauche,
// ligne de base de chaque ligne à 1 px près à l'échelle 1, mêmes retours à la ligne).
// Lancer : just e2e. À la main : bun e2e/fields.cjs, E2E_URL = adresse d'un serveur neuf.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TOL = 1;

/**
 * Lignes du texte rendu dans une boîte (px page), lues dans le SVG de la page :
 * glyphes regroupés par ligne ; bord gauche = le plus à gauche, ligne de base =
 * médiane des bas de glyphes (la plupart des lettres posent sur la ligne de base,
 * les jambages p, q, g… sont minoritaires).
 */
const renderedLines = (p, box) => p.evaluate(async ({ box }) => {
  const img = document.querySelector('.stage img');
  const at = img.getBoundingClientRect();
  const host = document.createElement('div');
  host.style.cssText = `position:fixed;left:${at.left}px;top:${at.top}px;width:${at.width}px;height:${at.height}px;opacity:0;pointer-events:none`;
  host.innerHTML = await fetch(img.src).then((r) => r.text());
  const svg = host.firstElementChild;
  svg.setAttribute('width', at.width);
  svg.setAttribute('height', at.height);
  document.body.append(host);
  const glyphs = [...svg.querySelectorAll('use')].map((g) => g.getBoundingClientRect())
    .filter((r) => r.width > 0 && r.left >= box.left - 2 && r.right <= box.right + 2
      && r.top >= box.top - 2 && r.bottom <= box.bottom + 2);
  host.remove();
  // Regroupe par ligne : un glyphe dont le centre est à plus d'une demi-hauteur
  // de glyphe du précédent ouvre une nouvelle ligne.
  glyphs.sort((a, b) => (a.top + a.bottom) - (b.top + b.bottom));
  const lines = [];
  for (const r of glyphs) {
    const c = (r.top + r.bottom) / 2;
    const last = lines[lines.length - 1];
    if (last && Math.abs(c - last.center) < r.height / 2) {
      last.rects.push(r);
    } else {
      lines.push({ center: c, rects: [r] });
    }
  }
  const median = (xs) => xs.sort((a, b) => a - b)[Math.floor(xs.length / 2)];
  return lines.map((l) => ({ left: Math.min(...l.rects.map((r) => r.left)), baseline: median(l.rects.map((r) => r.bottom)) }));
}, { box });

/**
 * Lignes de l'éditeur : un rectangle par ligne visuelle (Range sur chaque
 * `.cm-line`). Pour du texte en ligne, le rectangle est la zone de contenu de
 * la police : ligne de base = haut + ascendante de la police (`fontBoundingBoxAscent`).
 */
const editorLines = (p) => p.evaluate(() => {
  const content = document.querySelector('.editor .cm-content');
  const cs = getComputedStyle(content);
  const ctx = document.createElement('canvas').getContext('2d');
  ctx.font = `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
  const ascent = ctx.measureText('Hx').fontBoundingBoxAscent;
  const out = [];
  for (const line of content.querySelectorAll('.cm-line')) {
    const range = document.createRange();
    range.selectNodeContents(line);
    for (const r of range.getClientRects()) {
      if (r.width > 0) out.push({ left: r.left, baseline: r.top + ascent });
    }
  }
  return out.sort((a, b) => a.baseline - b.baseline);
});

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  // Échelle 1 : la slide (1920×1080 pt) affichée à 1 px par pt.
  const p = await b.newPage({ viewport: { width: 2600, height: 1500 } });
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  await p.evaluate(() => document.fonts.ready);
  const cases = [
    ['s-priorites', 'Titre', 0], ['s-priorites', 'Surtitre', 0], ['s-constat', 'Titre', 0],
    ['s-ch1', 'Sous-titre', 0], ['s-title', 'Titre', 0],
  ];
  for (const [slide, label, nth] of cases) {
    await p.click(`.pile [data-slide="${slide}"]`);
    await p.waitForSelector(`.field[aria-label="${label}"]`);
    const field = p.locator(`.field[aria-label="${label}"]`).nth(nth);
    const box = await field.boundingBox();
    const scale = await p.evaluate(() => document.querySelector('.stage').getBoundingClientRect().width / 1920);
    assert.ok(Math.abs(scale - 1) < 0.02, `échelle ${scale} : agrandir la fenêtre`);
    const rendered = await renderedLines(p, { left: box.x, top: box.y, right: box.x + box.width, bottom: box.y + box.height });
    await field.dblclick();
    await p.waitForSelector('.editor .cm-editor.cm-focused');
    const edited = await editorLines(p);
    assert.equal(edited.length, rendered.length, `${slide}/${label} : ${edited.length} lignes éditées, ${rendered.length} rendues`);
    edited.forEach((l, i) => {
      assert.ok(Math.abs(l.left - rendered[i].left) <= TOL, `${slide}/${label} ligne ${i + 1} : bord gauche ${l.left} ≠ ${rendered[i].left}`);
      assert.ok(Math.abs(l.baseline - rendered[i].baseline) <= TOL, `${slide}/${label} ligne ${i + 1} : ligne de base ${l.baseline} ≠ ${rendered[i].baseline}`);
    });
    await p.keyboard.press('Escape');
  }
  await b.close();
  console.log('fields : ok');
})().catch((e) => { console.error(e); process.exit(1); });
```

Avant d'exécuter : prendre le sélecteur de slide dans la pile (`[data-slide=…]` ci-dessus) qu'utilisent déjà `panel.cjs` ou `toolbar.cjs`, et vérifier que les glyphes du SVG de Typst sont bien des `<use>` (sinon adapter le sélecteur de `renderedLines`, pas la tolérance). Pour valider la mesure elle-même, faire échouer le test exprès une fois : décaler l'éditeur de 2 px dans `editorGeometry`, constater l'échec, annuler.

Si le test échoue sur l'écart réel (et non sur la mesure), c'est `editorGeometry` qui est faux : le corriger (tâche 9, `Stage.tsx`) en suivant G13–G15 (test rouge d'abord, diagnostic avant correction).

- [ ] **Step 3: Brancher dans `e2e`**

`ui/package.json` : ajouter `&& bun e2e/fields.cjs` à la fin du script `e2e`.

- [ ] **Step 4: ARCHITECTURE.md**

- render : « `field()` émet une boîte par champ, même vide ; `fields-stack` pose les champs vides sans place (`FieldBox.empty`, `side`). »
- core : « `Slide::text_fields` : champs texte dans l'ordre de lecture ; `apply(…, merge)` fusionne les étapes d'annulation de même clé. »
- session : « `SlideView.fields` (`fields.rs`) : champs éditables dans l'ordre de Tab ; `targets` : boîtes d'éléments. »

- [ ] **Step 5: Tout vérifier**

Run: `just check && cargo nextest run --workspace && (cd ui && bun test) && just e2e`
Expected: tout PASS ; sortie de `fields : ok` collée au rapport.

- [ ] **Step 6: Commit**

```bash
git add ui/e2e ui/package.json crates/oku-slides-render/ARCHITECTURE.md crates/oku-slides-session/ARCHITECTURE.md crates/oku-slides-core/ARCHITECTURE.md
```
```bash
git commit -m "test(ui) : alignement de l'éditeur sur les champs rendus, e2e mis à jour (t-271)"
```

---

## Couverture de la spec

| Spec | Tâche |
|---|---|
| `field()` vide, marqueur | 1 |
| `fields-stack`, ancre `before`/`after` | 1, 2 |
| Carte, slides (content, title, chapter, toc, end) | 1, 2 |
| `FieldBox.empty/side` | 1 |
| Ordre de Tab (slide, slots, schéma) | 3, 6 |
| Fusion d'annulation par clé, agent intercalé | 4, 5 |
| `Target` réduit, `FieldView`, `SlideView.fields`, `bind` | 6 |
| Libellés des placeholders | 7, 8 |
| Couches, gestes, Tab/Entrée/⌘Entrée/Échap, clic en édition | 9 |
| Géométrie unifiée, recalage au rendu | 9 |
| Champ disparu (toast), lecture seule de l'historique | 9 |
| Placeholders (objet sélectionné, partout en modèles) | 9 |
| Dictée | 9 |
| Test d'alignement ≤ 1 px, mêmes retours à la ligne | 10 |
| Export identique | 1, 2 (test doré) |
