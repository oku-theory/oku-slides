# Widgets Typst, plan 2 : bascule du modèle en v2

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remplacer le modèle fermé du deck (`ElementBody`, `SlideKind`, champs fixes de `Slide`) par le modèle générique `{id, kind, params}` validé par le registre des widgets, dans tout le workspace (cœur, Loro, ops, rendu, session, MCP, agent, CLI, UI), sans changer un pixel du rendu.

**Architecture:** Le cœur définit `Params`/`ParamValue`, déduits de la forme du JSON. Le registre (`WidgetRegistry`, plan 1) s'applique dans les ops, `validate`, `canonicalize` et `reconcile`. La lecture Loro reste indépendante du registre. Le rendu Typst devient générique (`data.typ` construit les arguments d'un widget depuis son schéma) et les slides émettent leurs champs par `field()`. La session tient les registres (base et pack) et les passe au cœur et au rendu. Les packs de modèles deviennent des dossiers.

**Tech Stack:** Rust (serde, snafu, loro, speculoos, rstest, insta), Typst 0.15, TypeScript/React (bun), Fluent (20 langues).

**Spec:** `docs/superpowers/specs/2026-09-27-widgets-typst-design.md` (t-294, suite de t-270). Plan précédent : `docs/superpowers/plans/2026-09-27-widgets-typst-1.md`.

## Global Constraints

- Branche et worktree : `294-feature/widgets-v2`, sous `.claude/worktrees/294-widgets-v2`. `main` reçoit du travail en parallèle. Dans le worktree : `git config oku.review deferred`.
- `git add` dans une commande à part, puis `git commit`. Chaque commit cite `(t-294)`. Pas de ligne `Co-Authored-By`.
- **Pas de rétrocompatibilité** : `DECK_VERSION = 2`, `SOURCES_VERSION = 3`. Pas de lecture ni de conversion des decks v1 dans le code livré. Les fixtures et les templates du dépôt sont convertis une fois, par un script de travail non commité.
- **Rendu identique** : le test doré (`crates/oku-slides-render/tests/golden.rs`) passe sur ses decks convertis en v2. Un écart est toléré seulement s'il est d'antialiasing, mesuré contre la page d'avant : moins de 0,1 % des pixels de la page, delta ≤ 2/255 par canal, toutes les autres pages identiques. La mesure n'est pas faite à l'œil : le test ignoré `dump_reference_pages` (tâche 1) écrit les PNG des pages dans `$GOLDEN_DUMP_DIR`, on le lance sur le commit d'avant et sur le commit courant, puis `.superpowers/sdd/2026-09-28-widgets-typst-2/pixel_diff.py AVANT APRES` compare les deux dossiers et **échoue** hors seuil. Seulement s'il passe, on ré-enregistre exactement les lignes concernées avec `INSTA_UPDATE=always`, et on colle sa sortie dans le rapport. Pas de contournement pour préserver une empreinte.
- **Rien ne se perd** : un paramètre ou un `kind` inconnus du registre sont conservés à l'identique ; `validate` les signale.
- Pas de `.unwrap()` ni de `.expect()` hors tests. Erreurs Snafu opaques avec backtrace, messages en minuscules.
- Tout texte destiné à l'utilisateur passe par Fluent, dans les 20 langues ; `every_locale_has_exactly_the_reference_keys` reste vert. Une clé retirée l'est des 20 langues.
- Tests nommés `<unité>_<scénario>_<résultat>`, `*_tests.rs` à côté du code, speculoos, rstest.
- Vocabulaire : **champ** (`field`) = région de texte éditable liée à un paramètre ; **zone** = zone de corps des éléments libres ; **slot** = paramètre qui contient des éléments.
- Vert par tâche : seul le crate de la tâche doit être vert (`cargo nextest run -p <crate>`), avec le filtre indiqué quand la tâche le précise. Le workspace entier et l'UI sont verts à la fin de la tâche 8.

## Décisions de ce plan

- **`ParamValue` par la forme du JSON** (spec, option A) : chaîne → `Text` ; tableau non vide dont chaque objet a `id` et `kind` → `Slot` ; autre tableau d'objets (vide compris) → `Rich` ; le reste → `Value`. Les accesseurs traitent `Rich(vec![])` comme un slot vide quand le schéma dit `slot`.
- **Params du deck** : l'actuel `model::Params` (titre, sous-titre… du deck) devient `DeckParams`, pour libérer `Params`.
- **Adressage** : `Target { slide: String, element: Option<String> }` (JSON `{"slide":"s1"}` ou `{"slide":"s1","element":"e1"}`), `SlotRef { owner: Target, param: String }`.
- **`SetSlideField` reste pour les métadonnées** : restreint à `name` et `description` (champs fixes de `Slide`, hors rendu). `kind` passe par `SetKind`, les autres champs par `SetParam`.
- **Placement libre** : un élément est libre s'il a un `frame` ou une `cell` (`Located::is_free`). Le mode du slot (`flow`/`free`) est contrôlé par `validate` : dans un slot `free`, un enfant sans `frame` ni `cell` est une issue ; dans un slot `flow`, un `frame` ou une `cell` aussi.
- **Registre dans le replica** : `DeckReplica` et `SourceBundle` portent un `Arc<WidgetRegistry>` (écriture et création d'avance des paramètres). La lecture n'en a pas besoin.
- **Registre de test** : `fixtures/widgets/base-registry.json` est la sonde du registre de base (`{"base": […], "pack": null}`). Un test du rendu vérifie qu'il est égal au registre vivant ; les tests des autres crates le chargent.
- **Chargeur d'images** : `load-image(e, asset: none, fit: "cover", chart: false)`. Le widget `image` appelle `(ctx.load)(asset: args.asset, fit: args.fit)`, le widget `chart` `(ctx.load)(chart: true)`. Le rendu Rust pose `chart-svg` sur chaque élément dont un paramètre est `host("chart")`.
- **Champs de slide** : les fonctions de slide (`slide`, `title-slide`, `chapter`, `end-slide`) prennent un `wrap` comme `card`. Les widgets de slide y passent `field(ctx, …)` ; `tracked()` disparaît. Côté session, les champs d'une slide gardent l'id de cible `@<param>` (l'UI s'appuie dessus) ; les champs des éléments restent pour t-271.
- **Libellés localisés des widgets** : hors plan (spec, « Hors plan 2 »).
- **Profondeur des slots bornée** : `MAX_SLOT_DEPTH = 16` (`locate.rs`). `Located` porte sa `depth` (0 pour un enfant direct d'un slot de la slide). `AddElement`, `ReplaceElement` et `AddSlide` refusent un élément dont un descendant dépasserait la borne (`InvalidValue`, raison « elements nest too deeply ») ; `validate` signale `issue-slot-too-deep` ; la lecture Loro descend au plus `MAX_DEPTH` (64) niveaux : le lecteur actuel compte deux niveaux par slot (entrée dans la liste, puis dans la carte `params` de l'élément ; la carte de l'élément ne compte pas), donc 16 slots = 32 niveaux, sous la borne. Le test hostile `read_deeply_nested_slots_stops_at_the_depth_bound` (tâche 3) fait foi, pas ce calcul. L'analyse d'un texte JSON est déjà bornée par `serde_json` (128). Les fonctions récursives (`find_mut`, `canonical::params`) ne voient donc jamais plus de 16 niveaux.
- **`choice` et `asset` sont atomiques dans Loro** (dernière écriture gagnante) : fusionner « cover » et « contain » caractère par caractère donnerait une valeur invalide. Seul `plain` est une prose fusionnée au caractère ; seuls `plain`, `rich` et `slot` sont créés d'avance.
- **`ChoiceOutOfRange`** : erreur d'op distincte, comme la spec l'écrit (`op-choice-out-of-range`).
- **Paramètres liés** : un `bind` n'existe que sur un `plain` ; `SetParam` le convertit par `param_value` (→ `Text` ou `None`) puis écrit le texte (ou `None`) dans le champ `DeckParams` de même nom (`title`, `subtitle`, `author`, `date`, `organization`), sinon dans `DeckParams.extra[bind]` en chaîne JSON. `DeckParams` garde ses `Option<String>`.
- **Cible éditable côté UI** : le type TS de la vue (`Target` d'aujourd'hui) devient `EditTarget`, pour ne pas le confondre avec le `Target` d'adressage des ops. Une cible d'élément sans `param` (widget sans paramètre `rich`, comme `shape` ou `image`) n'est pas éditable en place.
- **`Params::slot_mut` rend une `Option`** : toujours `Some` en pratique, mais sans branche qui panique ; les ops traduisent `None` en `InvalidValue`.
- **Doublon assumé** : `registry()` de test existe dans `src/test_registry.rs` (tests unitaires, `pub(crate)`) et dans `tests/common/mod.rs` (tests d'intégration, qui ne voient pas le code `#[cfg(test)]` de la crate).

## Carte du v1 vers le v2

| v1 | v2 |
|---|---|
| élément `text {text}` | `kind: "text"`, `params.body` |
| `card {kicker, title, highlight, text}` | `kind: "card"`, `params.{kicker, title, highlight, body}` |
| `image {asset, fit}` | `kind: "image"`, `params.{asset, fit}` |
| `shape {shape}` | `kind: "shape"`, `params.shape` |
| `chart {option}` | `kind: "chart"`, `params.option` |
| `group {layout: {type: "free"}, children}` | `kind: "canvas"`, `params.children` |
| `group {layout: {type, count, gap}, children}` | `kind: "stack"`, `params.{layout: type, count, gap, children}` |
| slide `title` | `kind: "title"`, `params: {}` ; ses `title`/`subtitle` éventuels vont dans `deck.params` (lien `bind`) |
| slide `toc {title}` | `params.title` |
| slide `chapter {title, subtitle}` | `params.{title, subtitle}` |
| slide `content {kicker, title, layout, grid, blocks, elements}` | `params.{kicker, title, layout, grid, blocks, elements}` |
| slide `end {text}` | `params.text` |
| slide `blank {grid, elements}` | `params.{grid, elements}` |
| op `set_text {slide, element, text}` | `set_param {target: {slide, element}, param: "body", value: text}` |
| op `set_slide_field {field: title/subtitle/kicker/layout/text}` | `set_param {target: {slide}, param: field, value}` |
| op `set_slide_field {field: kind}` | `set_kind {target: {slide}, kind: value}` |
| op `set_param {key, value}` (deck) | `set_deck_param {key, value}` |
| op `set_grid {slide, grid}` | `set_param {target: {slide}, param: "grid", value: grid}` |
| op `add_element {slide, zone, parent?, index?, element}` | `add_element {slot: {owner: {slide, element: parent?}, param: zone ou "children"}, index?, element}` |
| op `set_chart_option {slide, element, patch}` | `set_chart_option {target: {slide, element}, param: "option", patch}` |
| op `set_chart_data {slide, element, change}` | `set_chart_data {target: {slide, element}, param: "option", change}` |

## Structure des fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-core/src/model.rs` | `Deck`, `DeckParams`, `Slide`, `Element`, `Style`, `Frame`, `Cell`, `Grid`, `Paragraph`, `Run` (le reste disparaît) |
| `crates/oku-slides-core/src/params.rs` (créé) | `Params`, `ParamValue`, accesseurs |
| `crates/oku-slides-core/src/locate.rs` | parcours générique des slots |
| `crates/oku-slides-core/src/canonical.rs` | forme canonique générique |
| `crates/oku-slides-core/src/validate.rs` | `validate(deck, registry, assets)` |
| `crates/oku-slides-core/src/ops.rs` (+ `ops/params.rs` créé) | ops v2 ; conversion d'une valeur selon son type |
| `crates/oku-slides-core/src/replica/*` | écriture guidée par le registre, lecture par la forme |
| `crates/oku-slides-core/src/test_registry.rs` (créé, `#[cfg(test)]`) | registre de test |
| `fixtures/widgets/base-registry.json` (créé) | sonde du registre de base |
| `typst/oku-slides/src/data.typ` | rendu générique |
| `typst/oku-slides/src/slides.typ`, `widgets.typ`, `core.typ` | `wrap` des slides, `tracked` retiré |
| `crates/oku-slides-render/src/{world,charts,lib}.rs` | widgets du pack, chargeur v2, graphiques v2 |
| `crates/oku-slides-session/src/*` | registre, vues, templates en dossiers |
| `templates/<id>/pack.json` | packs intégrés |
| `crates/oku-slides-{mcp,agent,cli}` | ops et lectures v2, outil `widgets` |
| `ui/src/*` | types et ops v2 |

---

### Task 1: Modèle v2 du cœur, parcours, forme canonique, validation

**Files:**
- Create: `crates/oku-slides-core/src/params.rs`, `crates/oku-slides-core/src/params_tests.rs`
- Create: `crates/oku-slides-core/src/test_registry.rs`
- Create: `fixtures/widgets/base-registry.json`
- Modify: `crates/oku-slides-core/src/model.rs`, `locate.rs`, `canonical.rs`, `validate.rs`, `lib.rs`
- Modify: `crates/oku-slides-core/src/{locate,canonical,validate}_tests.rs`
- Modify (conversion) : `fixtures/roadmap.json`, `fixtures/markdown.json`, `fixtures/markdown-corpus.json`, `crates/oku-slides-render/tests/golden/widgets.json`, `fixtures/agent/e2e.json`
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl` (20 langues)
- Modify: `crates/oku-slides-render/tests/golden.rs` (test ignoré `dump_reference_pages`)

**Interfaces:**
- Consumes: `WidgetRegistry`, `WidgetSchema`, `ParamType`, `SlotMode`, `Category` (plan 1).
- Produces:
  - `model::{Deck, DeckParams, Slide, Element, …}` ; `Deck.params: DeckParams`.
  - `Slide { id: String, kind: String, params: Params, name: Option<String>, description: Option<String>, notes: String }`, `Slide::new(id, kind: &str)`.
  - `Element { id: String, kind: String, frame: Option<Frame>, cell: Option<Cell>, style: Style, params: Params }`, `Element::new(id, kind: &str)`, `Element::text(id, Vec<Paragraph>)` (kind `text`, `params.body`).
  - `Params` (voir code) et `ParamValue::{Text, Rich, Slot, Value}`.
  - `Located<'a> { element: &'a Element, owner: Option<&'a str>, slot: &'a str, index: usize, depth: usize }`, `Located::is_free()`, `Slide::walk()`, `Slide::locate(id)`, `Slide::element_ids()`, `Walk::over(params: &'a Params, owner: Option<&'a str>)`, `pub const MAX_SLOT_DEPTH: usize = 16`, `pub fn slot_depth(e: &Element) -> usize` (0 pour un élément sans enfant).
  - `canonicalize(deck: &mut Deck)` (sans registre).
  - `validate(deck: &Deck, registry: &WidgetRegistry, assets: Option<&HashSet<String>>) -> Vec<Issue>`.
  - `Slide::grid(&self) -> Grid` (paramètre `grid`, sinon la grille par défaut).
  - Test : `crate::test_registry::registry() -> WidgetRegistry` (lit `fixtures/widgets/base-registry.json`).

- [ ] **Step 1: Freeze the base registry as a fixture**

Écrire `fixtures/widgets/base-registry.json` : la sonde du registre de base, au format `{"base": [schéma, …], "pack": null}`. Pour l'obtenir, écrire dans `crates/oku-slides-render/tests/widgets.rs` le test :

```rust
#[test]
fn base_registry_fixture_matches_the_live_registry() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap();
    let fixture = oku_slides_core::WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap()).unwrap();
    assert_that!(load_registry(&Fonts::default(), None).unwrap()).is_equal_to(fixture);
}
```

Générer le fichier à partir de `crates/oku-slides-render/tests/snapshots/widgets__base_registry.snap` (le JSON du registre sérialisé : `{"widgets": {nom: schéma}}`) : `{"base": [les schémas, dans l'ordre des noms], "pack": null}`. Ce test ne tournera qu'à la tâche 4 (le rendu est rouge d'ici là) ; le lancer tout de suite, avant toute autre modification : `cargo nextest run -p oku-slides-render --test widgets base_registry_fixture` → PASS.

`crates/oku-slides-core/src/test_registry.rs` :

```rust
//! Registre de base pour les tests du cœur (`fixtures/widgets/base-registry.json`).

use crate::WidgetRegistry;

pub(crate) fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let probe = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("{path} : {e}"))
}
```

déclaré dans `lib.rs` par `#[cfg(test)] pub(crate) mod test_registry;`. Les tests d'intégration du cœur (`crates/oku-slides-core/tests/`) ont leur copie de cette fonction dans `tests/common/mod.rs` (à créer, `pub fn registry()`).

- [ ] **Step 1b: Golden dump test and pixel-diff script**

Ajouter à `crates/oku-slides-render/tests/golden.rs`, **avant toute autre modification** (il doit tourner sur le code v1 pour produire la référence « avant ») :

```rust
/// Écrit les PNG des pages de référence dans `$GOLDEN_DUMP_DIR` (mesure d'un
/// écart d'antialiasing ; voir le plan 2, contraintes globales).
#[test]
#[ignore = "outil : lancé à la main avec GOLDEN_DUMP_DIR"]
fn dump_reference_pages() {
    let dir = std::env::var("GOLDEN_DUMP_DIR").unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    for (name, path) in [("roadmap", "../../fixtures/roadmap.json"), ("widgets", "tests/golden/widgets.json")] {
        let compiled = r.render(&deck(path), &BTreeMap::new()).unwrap();
        for page in 0..compiled.page_count() {
            std::fs::write(format!("{dir}/{name}-p{}.png", page + 1), compiled.png(page, 0.5).unwrap()).unwrap();
        }
    }
}
```

Lancer : `GOLDEN_DUMP_DIR=$PWD/.superpowers/sdd/2026-09-28-widgets-typst-2/before cargo nextest run -p oku-slides-render --test golden --run-ignored only dump_reference_pages` → les 20 PNG de référence v1.

Écrire `.superpowers/sdd/2026-09-28-widgets-typst-2/pixel_diff.py` (non commité) :

```python
#!/usr/bin/env python3
"""Compare deux dossiers de PNG ; échoue hors seuil d'antialiasing (plan 2)."""
import sys, pathlib
from PIL import Image, ImageChops

MAX_SHARE, MAX_DELTA = 0.001, 2
before, after = map(pathlib.Path, sys.argv[1:3])
bad = False
for a in sorted(before.glob("*.png")):
    b = after / a.name
    ia, ib = Image.open(a).convert("RGBA"), Image.open(b).convert("RGBA")
    if ia.size != ib.size:
        print(f"{a.name}: taille {ia.size} -> {ib.size}"); bad = True; continue
    diff = ImageChops.difference(ia, ib)
    px = [max(p) for p in diff.getdata()]
    changed = sum(1 for d in px if d > 0)
    share, delta = changed / len(px), max(px)
    status = "identique" if changed == 0 else ("ok" if share < MAX_SHARE and delta <= MAX_DELTA else "HORS SEUIL")
    print(f"{a.name}: {changed} px ({share:.4%}), delta max {delta} -> {status}")
    bad |= status == "HORS SEUIL"
sys.exit(1 if bad else 0)
```

Usage à chaque tâche qui touche au rendu : produire `after` avec le même test ignoré, puis `python3 …/pixel_diff.py …/before …/after`. La référence `before` reste celle du v1 pendant tout le plan.

- [ ] **Step 2: Write the conversion script (not committed)**

Écrire `.superpowers/sdd/2026-09-28-widgets-typst-2/v1_to_v2.py` (dossier ignoré par git ; le créer). Il réécrit un fichier JSON v1 en v2 selon la « Carte du v1 vers le v2 » ci-dessus, en place, et reconnaît trois formes : un deck (`{"slides": …}`), un pack de modèles (`{"templates": [{"slide": …}]}`), un fichier d'ops (tout tableau d'objets `{"op": …}` rencontré, à n'importe quelle profondeur, comme dans `fixtures/agent/e2e.json`).

```python
#!/usr/bin/env python3
"""v1 -> v2 des decks, packs et ops du dépôt (t-294). Outil de travail, non commité."""
import json, sys

FIELDS = ("title", "subtitle", "kicker", "layout", "text", "grid")

def element(e):
    out = {k: e[k] for k in ("id", "frame", "cell", "style") if k in e}
    kind = e.get("kind", "text")
    p = {}
    if kind == "text":
        if "text" in e: p["body"] = e["text"]
    elif kind == "card":
        for k in ("kicker", "title"):
            if k in e: p[k] = e[k]
        if e.get("highlight"): p["highlight"] = True
        if "text" in e: p["body"] = e["text"]
    elif kind == "image":
        for k in ("asset", "fit"):
            if k in e: p[k] = e[k]
    elif kind == "shape":
        if "shape" in e: p["shape"] = e["shape"]
    elif kind == "chart":
        p["option"] = e["option"]
    elif kind == "group":
        lay = e.get("layout", {"type": "free"})
        children = [element(c) for c in e.get("children", [])]
        if lay.get("type", "free") == "free":
            kind = "canvas"
        else:
            kind = "stack"
            p["layout"] = lay["type"]
            for k in ("count", "gap"):
                if k in lay: p[k] = lay[k]
        if children: p["children"] = children
    else:
        raise SystemExit(f"kind inconnu : {kind}")
    out["kind"] = kind
    if p: out["params"] = p
    return out

def slide(s, deck_params=None):
    out = {"id": s["id"], "kind": s.get("kind", "content")}
    for k in ("name", "description", "notes"):
        if k in s: out[k] = s[k]
    p = {}
    if out["kind"] == "title":
        # Titre et sous-titre de la couverture : liés à deck.params.
        if deck_params is not None:
            for k in ("title", "subtitle"):
                if k in s and k not in deck_params: deck_params[k] = s[k]
    else:
        for k in FIELDS:
            if k in s: p[k] = s[k]
    for k in ("blocks", "elements"):
        if s.get(k): p[k] = [element(e) for e in s[k]]
    if p: out["params"] = p
    return out

def target(o):
    t = {"slide": o["slide"]}
    if o.get("element"): t["element"] = o["element"]
    return t

def op(o):
    name = o["op"]
    if name == "set_text":
        return {"op": "set_param", "target": target(o), "param": "body", "value": o["text"]}
    if name == "set_slide_field":
        if o["field"] == "kind":
            return {"op": "set_kind", "target": {"slide": o["slide"]}, "kind": o["value"]}
        if o["field"] in ("name", "description"):
            return o
        return {"op": "set_param", "target": {"slide": o["slide"]}, "param": o["field"], "value": o.get("value")}
    if name == "set_param":
        return {"op": "set_deck_param", "key": o["key"], "value": o.get("value")}
    if name == "set_grid":
        return {"op": "set_param", "target": {"slide": o["slide"]}, "param": "grid", "value": o.get("grid")}
    if name == "add_element":
        owner = {"slide": o["slide"]}
        if o.get("parent"): owner["element"] = o["parent"]
        param = "children" if o.get("parent") else o.get("zone", "blocks")
        out = {"op": "add_element", "slot": {"owner": owner, "param": param}, "element": element(o["element"])}
        if "index" in o: out["index"] = o["index"]
        return out
    if name in ("set_chart_option", "set_chart_data"):
        out = {k: v for k, v in o.items() if k not in ("slide", "element")}
        out["target"] = target(o); out["param"] = "option"
        return out
    if name == "replace_element":
        return {**o, "element": element(o["element"])}
    if name == "add_slide":
        return {**o, "slide": slide(o["slide"])}
    return o

def walk(v):
    if isinstance(v, dict):
        if "op" in v and isinstance(v["op"], str):
            return op(v)
        if "slides" in v and isinstance(v["slides"], list):
            params = v.setdefault("params", {})
            v["slides"] = [slide(s, params) for s in v["slides"]]
            if not params: del v["params"]
            v["version"] = 2
            return v
        if "templates" in v and isinstance(v["templates"], list):
            for t in v["templates"]: t["slide"] = slide(t["slide"])
            return v
        return {k: walk(x) for k, x in v.items()}
    if isinstance(v, list):
        return [walk(x) for x in v]
    return v

for path in sys.argv[1:]:
    data = json.load(open(path))
    json.dump(walk(data), open(path, "w"), ensure_ascii=False, indent=2)
    open(path, "a").write("\n")
```

Vérifier sur `fixtures/agent/e2e.json` que tout objet slide « variant » y est aussi converti (les variantes de slides sont des objets `Slide` hors d'un deck) : si le fichier contient des slides isolées (clé `variants`), ajouter au script leur conversion (`slide(...)`) et le noter dans le rapport.

- [ ] **Step 3: Convert the repository fixtures**

```bash
python3 .superpowers/sdd/2026-09-28-widgets-typst-2/v1_to_v2.py fixtures/roadmap.json fixtures/markdown.json fixtures/markdown-corpus.json crates/oku-slides-render/tests/golden/widgets.json fixtures/agent/e2e.json
```

Relire le diff de `fixtures/roadmap.json` en entier : chaque élément a `kind` et `params`, chaque slide a `params`, `"version": 2`. Les templates sont convertis à la tâche 6.

- [ ] **Step 4: Write the failing tests for `Params`**

`crates/oku-slides-core/src/params_tests.rs` :

```rust
use serde_json::json;
use speculoos::prelude::*;

use crate::{Element, ParamValue, Params, Paragraph};

fn params(v: serde_json::Value) -> Params {
    serde_json::from_value(v).unwrap()
}

#[test]
fn deserialize_string_gives_text() {
    let p = params(json!({ "title": "Titre" }));
    assert_that!(p.get("title")).is_equal_to(Some(&ParamValue::Text("Titre".into())));
}

#[test]
fn deserialize_paragraphs_gives_rich() {
    let p = params(json!({ "body": [{ "runs": [{ "t": "a" }] }] }));
    assert_that!(p.rich("body")).is_equal_to(Some(&[Paragraph::plain("a")][..]));
}

#[test]
fn deserialize_elements_gives_slot() {
    let p = params(json!({ "children": [{ "id": "e1", "kind": "text" }] }));
    assert_that!(p.slot("children").map(|s| s[0].id.as_str())).is_equal_to(Some("e1"));
}

#[test]
fn deserialize_empty_array_is_an_empty_rich_and_an_empty_slot() {
    let p = params(json!({ "x": [] }));
    assert_that!(p.rich("x")).is_equal_to(Some(&[][..]));
    assert_that!(p.slot("x")).is_equal_to(Some(&[][..]));
}

#[test]
fn deserialize_other_values_gives_value() {
    let p = params(json!({ "highlight": true, "grid": { "columns": 2, "rows": 1 } }));
    assert_that!(p.get("highlight")).is_equal_to(Some(&ParamValue::Value(json!(true))));
    assert_that!(p.value("grid")).is_equal_to(Some(&json!({ "columns": 2, "rows": 1 })));
}

#[test]
fn serialize_round_trips_every_shape() {
    let v = json!({ "a": "t", "b": [{ "runs": [{ "t": "x" }] }], "c": [{ "id": "e", "kind": "text" }], "d": 3 });
    let p = params(v.clone());
    assert_that!(serde_json::to_value(&p).unwrap()).is_equal_to(v);
}

#[test]
fn deserialize_paragraph_is_never_a_slot() {
    // Garde-fou : si `Paragraph` gagne un jour `id` et `kind`, ce test casse.
    let p = params(json!({ "body": [{ "runs": [{ "t": "a" }], "list": "bullet" }] }));
    assert_that!(matches!(p.get("body"), Some(ParamValue::Rich(_)))).is_true();
}

#[test]
fn deserialize_elements_missing_an_id_fails_instead_of_becoming_rich() {
    // Un tableau d'objets dont certains ont `kind` mais pas `id` est une
    // liste d'éléments malformée, pas du texte riche.
    let r = serde_json::from_value::<Params>(json!({ "children": [{ "id": "e1", "kind": "text" }, { "kind": "text" }] }));
    assert_that!(r.is_err()).is_true();
}

#[test]
fn slot_mut_turns_an_absent_or_empty_param_into_a_slot() {
    let mut p = Params::default();
    p.slot_mut("children").unwrap().push(Element::new("e1", "text"));
    assert_that!(p.slot("children").map(<[Element]>::len)).is_equal_to(Some(1));
}
```

- [ ] **Step 5: Write `params.rs`**

```rust
//! Paramètres d'un widget : valeurs nommées d'une slide ou d'un élément.
//!
//! Le type Rust d'une valeur se déduit de la forme de son JSON, sans le
//! registre : les ops arrivent désérialisées par serde aux frontières Tauri
//! et MCP. Le registre s'applique ensuite (ops, validation, réconciliation).

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::model::{Element, Paragraph};

/// Une valeur de paramètre.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ParamValue {
    /// Texte simple (`plain`), ou valeur de `choice` et d'`asset`.
    Text(String),
    /// Paragraphes à runs (`rich`). Un tableau vide se lit ici.
    Rich(Vec<Paragraph>),
    /// Éléments enfants (`slot`).
    Slot(Vec<Element>),
    /// Toute autre valeur : booléen, nombre, grille, options de graphique…
    Value(Value),
}

impl<'de> Deserialize<'de> for ParamValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        let is_element = |x: &Value| x.get("id").is_some_and(Value::is_string) && x.get("kind").is_some_and(Value::is_string);
        Ok(match v {
            Value::String(s) => ParamValue::Text(s),
            Value::Array(a) if !a.is_empty() && a.iter().all(is_element) => {
                ParamValue::Slot(serde_json::from_value(Value::Array(a)).map_err(serde::de::Error::custom)?)
            }
            Value::Array(a) if a.iter().any(|x| x.get("kind").is_some() || x.get("id").is_some()) => {
                return Err(serde::de::Error::custom("element list: every element needs a string `id` and `kind`"));
            }
            Value::Array(a) if a.iter().all(Value::is_object) => {
                ParamValue::Rich(serde_json::from_value(Value::Array(a)).map_err(serde::de::Error::custom)?)
            }
            other => ParamValue::Value(other),
        })
    }
}

/// Les paramètres d'une slide ou d'un élément, par nom.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Params(pub BTreeMap<String, ParamValue>);

impl Params {
    pub fn get(&self, name: &str) -> Option<&ParamValue> {
        self.0.get(name)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Pose une valeur ; renvoie l'ancienne.
    pub fn set(&mut self, name: impl Into<String>, value: ParamValue) -> Option<ParamValue> {
        self.0.insert(name.into(), value)
    }

    pub fn remove(&mut self, name: &str) -> Option<ParamValue> {
        self.0.remove(name)
    }

    /// Texte simple.
    pub fn text(&self, name: &str) -> Option<&str> {
        match self.get(name)? {
            ParamValue::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Paragraphes ; un slot vide se lit comme un texte vide.
    pub fn rich(&self, name: &str) -> Option<&[Paragraph]> {
        match self.get(name)? {
            ParamValue::Rich(p) => Some(p),
            ParamValue::Slot(s) if s.is_empty() => Some(&[]),
            _ => None,
        }
    }

    /// Éléments d'un slot ; un texte riche vide se lit comme un slot vide.
    pub fn slot(&self, name: &str) -> Option<&[Element]> {
        match self.get(name)? {
            ParamValue::Slot(s) => Some(s),
            ParamValue::Rich(p) if p.is_empty() => Some(&[]),
            _ => None,
        }
    }

    /// Le slot `name`, créé vide s'il est absent ou s'il porte une autre
    /// valeur. Toujours `Some` ; l'`Option` évite une branche qui panique.
    pub fn slot_mut(&mut self, name: &str) -> Option<&mut Vec<Element>> {
        let value = self.0.entry(name.to_owned()).or_insert_with(|| ParamValue::Slot(Vec::new()));
        if !matches!(value, ParamValue::Slot(_)) {
            *value = ParamValue::Slot(Vec::new());
        }
        match value {
            ParamValue::Slot(list) => Some(list),
            // Impossible : la valeur vient d'être posée en `Slot`. Branche
            // gardée pour ne pas paniquer (pas d'`unreachable!` hors tests).
            _ => None,
        }
    }

    pub fn value(&self, name: &str) -> Option<&Value> {
        match self.get(name)? {
            ParamValue::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Tous les slots, avec leur nom.
    pub fn slots(&self) -> impl Iterator<Item = (&str, &[Element])> {
        self.0.iter().filter_map(|(k, v)| match v {
            ParamValue::Slot(s) => Some((k.as_str(), s.as_slice())),
            _ => None,
        })
    }

    pub fn slots_mut(&mut self) -> impl Iterator<Item = (&str, &mut Vec<Element>)> {
        self.0.iter_mut().filter_map(|(k, v)| match v {
            ParamValue::Slot(s) => Some((k.as_str(), s)),
            _ => None,
        })
    }

    /// Tous les textes riches, avec leur nom.
    pub fn riches(&self) -> impl Iterator<Item = (&str, &[Paragraph])> {
        self.0.iter().filter_map(|(k, v)| match v {
            ParamValue::Rich(p) => Some((k.as_str(), p.as_slice())),
            _ => None,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &ParamValue)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut ParamValue)> {
        self.0.iter_mut().map(|(k, v)| (k.as_str(), v))
    }
}

#[cfg(test)]
#[path = "params_tests.rs"]
mod params_tests;
```

- [ ] **Step 6: Rewrite the model**

Dans `model.rs` :
- Renommer `Params` en `DeckParams` (même contenu) ; `Deck.params: DeckParams`.
- `DECK_VERSION = 2`.
- Supprimer `SlideKind`, `SlideLayout`, `GroupLayoutKind`, `GroupLayout`, `ShapeKind`, `Zone`, `ElementBody` et leurs `impl` (dont `kind_name`, `ElementBody::text`, `consistent_chart`, `Element::children`, `children_mut`).
- Nouvelles structures (dérivent `Debug, Clone, PartialEq, Serialize, Deserialize` ; `JsonSchema` : voir ci-dessous) :

```rust
/// Une slide : un widget de catégorie `slide` et ses paramètres.
pub struct Slide {
    pub id: String,
    /// Widget de slide (`title`, `content`…).
    pub kind: String,
    #[serde(default, skip_serializing_if = "Params::is_empty")]
    pub params: Params,
    /// Nom et description, pour une slide qui sert de modèle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

/// Un élément : un widget de catégorie `element`, placé et stylé.
pub struct Element {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
    /// Accroche à la grille de la slide ; prioritaire sur `frame`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<Cell>,
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: Style,
    #[serde(default, skip_serializing_if = "Params::is_empty")]
    pub params: Params,
}
```

avec `Slide::new(id: impl Into<String>, kind: &str)`, `Element::new(id: impl Into<String>, kind: &str)`, `Element::text(id, paragraphs)` (`kind: "text"`, `params.body = Rich(paragraphs)`), et :

```rust
impl Slide {
    /// La grille de la slide : son paramètre `grid`, sinon celle par défaut.
    pub fn grid(&self) -> Grid {
        self.params
            .value("grid")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| DEFAULT_GRID.clone())
    }
}
```

(`effective_grid` disparaît.) `JsonSchema` : `Params` et `ParamValue` n'ont pas de schéma dérivable fidèle ; implémenter `JsonSchema` pour `Params` à la main comme « objet à valeurs libres » (`schemars::json_schema!({"type": "object", "additionalProperties": true})` ou l'équivalent de la version de schemars du workspace, à vérifier dans `Cargo.toml`), ce qui garde `ops_schema` et les schémas MCP générables.

Exporter dans `lib.rs` : `pub mod params;`, `pub use params::{ParamValue, Params};`.

- [ ] **Step 7: Rewrite `locate.rs`**

```rust
//! Parcours des éléments d'une slide à travers ses slots, parents avant
//! enfants. Une seule traversée sert aux opérations, à la validation et à la
//! session.

use crate::model::{Element, Slide};
use crate::params::Params;

/// Un élément et sa place.
#[derive(Debug, Clone, Copy)]
pub struct Located<'a> {
    pub element: &'a Element,
    /// Élément qui porte le slot, ou `None` pour un slot de la slide.
    pub owner: Option<&'a str>,
    /// Nom du slot.
    pub slot: &'a str,
    pub index: usize,
    /// 0 pour un enfant direct d'un slot de la slide.
    pub depth: usize,
}

/// Profondeur maximale d'imbrication des slots (voir « Décisions »).
pub const MAX_SLOT_DEPTH: usize = 16;

/// Profondeur des descendants d'un élément : 0 s'il n'a pas d'enfant.
/// Itérative : un deck non fiable ne peut pas épuiser la pile.
pub fn slot_depth(e: &Element) -> usize {
    let mut max = 0;
    let mut stack = vec![(e, 0usize)];
    while let Some((e, d)) = stack.pop() {
        max = max.max(d);
        for (_, children) in e.params.slots() {
            stack.extend(children.iter().map(|c| (c, d + 1)));
        }
    }
    max
}

impl Located<'_> {
    /// Placé librement : avec un cadre ou une cellule.
    pub fn is_free(&self) -> bool {
        self.element.frame.is_some() || self.element.cell.is_some()
    }
}

/// Parcours en profondeur des slots, dans l'ordre des noms de paramètres.
#[derive(Debug)]
pub struct Walk<'a> {
    stack: Vec<(Option<&'a str>, &'a str, usize, std::iter::Enumerate<std::slice::Iter<'a, Element>>)>,
}

impl<'a> Walk<'a> {
    /// Parcourt les slots de `params`, portés par `owner`.
    pub fn over(params: &'a Params, owner: Option<&'a str>) -> Self {
        let mut stack: Vec<_> = params.slots().map(|(name, list)| (owner, name, 0, list.iter().enumerate())).collect();
        stack.reverse();
        Self { stack }
    }
}

impl<'a> Iterator for Walk<'a> {
    type Item = Located<'a>;

    fn next(&mut self) -> Option<Located<'a>> {
        loop {
            let (owner, slot, depth, list) = self.stack.last_mut()?;
            let (owner, slot, depth) = (*owner, *slot, *depth);
            let Some((index, element)) = list.next() else {
                self.stack.pop();
                continue;
            };
            let mut children: Vec<_> = element
                .params
                .slots()
                .map(|(name, l)| (Some(element.id.as_str()), name, depth + 1, l.iter().enumerate()))
                .collect();
            children.reverse();
            self.stack.extend(children);
            return Some(Located { element, owner, slot, index, depth });
        }
    }
}

impl Slide {
    pub fn walk(&self) -> Walk<'_> {
        Walk::over(&self.params, None)
    }

    pub fn locate(&self, id: &str) -> Option<Located<'_>> {
        self.walk().find(|l| l.element.id == id)
    }

    pub fn element_ids(&self) -> impl Iterator<Item = &str> {
        self.walk().map(|l| l.element.id.as_str())
    }
}
```

Réécrire `locate_tests.rs` sur des slides v2 construites en JSON (`serde_json::from_value::<Slide>(json!({…}))`) : ordre du parcours (parents avant enfants ; slots d'un même propriétaire dans l'ordre alphabétique de leur nom, `blocks` avant `elements` — l'ordre est désormais porteur pour la session), `owner`/`slot`/`index`/`depth` d'un enfant de `stack`, `is_free` avec cadre, avec cellule, sans les deux, `locate` d'un id absent → `None`, `slot_depth` d'une chaîne de 20 `canvas` imbriqués → 19 sans débordement de pile.

- [ ] **Step 8: Rewrite `canonical.rs`**

La forme canonique devient générique : pour chaque slide, `name`/`description` rognés (vides → `None`), `notes` vides retirées, puis `params(&mut s.params)` :

```rust
/// Forme canonique des paramètres : textes rognés (vides retirés), textes
/// riches canoniques (vides retirés), slots parcourus, nombres canoniques.
fn params(p: &mut Params) {
    p.0.retain(|_, v| match v {
        ParamValue::Text(s) => {
            let t = s.trim();
            if t.len() != s.len() {
                *s = t.to_owned();
            }
            !s.is_empty()
        }
        ParamValue::Rich(ps) => {
            paragraphs(ps);
            !ps.is_empty()
        }
        ParamValue::Slot(list) => {
            list.iter_mut().for_each(|e| params(&mut e.params));
            true
        }
        ParamValue::Value(v) => {
            numbers(v);
            !v.is_null()
        }
    });
}
```

`paragraphs`, `same_format` et `numbers` restent tels quels. Un slot vide est gardé (il a un sens). Adapter `canonical_tests.rs` : mêmes comportements qu'avant, sur des decks v2 (titre rogné d'une carte, paragraphes vides retirés, nombres d'options de graphique canoniques dans `params.option`).

- [ ] **Step 9: Rewrite `validate.rs`**

Signature : `pub fn validate(deck: &Deck, registry: &WidgetRegistry, assets: Option<&HashSet<String>>) -> Vec<Issue>`. Contrôles, dans cet ordre, pour chaque slide puis chaque élément de `slide.walk()` :

| Contrôle | Message (clé Fluent, arguments) |
|---|---|
| id de slide vide / répété | `issue-empty-slide-id`, `issue-duplicate-slide-id` (inchangés) |
| `kind` inconnu | `issue-unknown-widget` (`kind`) |
| widget d'une autre catégorie (slide qui n'est pas `slide`, élément qui n'est pas `element`) | `issue-wrong-category` (`kind`) |
| paramètre absent du schéma | `issue-unknown-param` (`param`) |
| valeur de forme incompatible avec son type (voir ci-dessous) | `issue-param-type` (`param`) |
| `choice` hors de ses valeurs | `issue-choice-out-of-range` (`param`) |
| enfant d'un slot dont `accepts` ne contient pas son `kind` | `issue-slot-rejects` (`kind`) |
| id d'élément vide / répété | `issue-empty-element-id`, `issue-duplicate-element-id` (inchangés) |
| slot `free` : enfant sans `frame` ni `cell` | `issue-free-without-frame` (inchangé) |
| slot `flow` : enfant avec `frame` ou `cell` | `issue-placed-in-flow` |
| `frame` de taille nulle | `issue-empty-frame` (inchangé) |
| `cell` d'un enfant d'un slot `free` de la slide qui sort de `slide.grid()` | `issue-cell-outside-grid` (inchangé) |
| paramètre `asset` absent de `assets` | `issue-missing-asset` (`asset`) (inchangé) |
| liens et pastilles de tous les paramètres `rich` | `issue-unsupported-link`, `issue-highlight-zero` (inchangés) |
| `bind` d'un paramètre vers une clé absente de `DeckParams` (ni champ nommé, ni `extra`) — contrôlé une fois par widget utilisé | `issue-unknown-bind` (`param`) |
| élément à une profondeur `> MAX_SLOT_DEPTH` | `issue-slot-too-deep` |

Forme compatible avec le type : `plain`/`choice`/`asset` → `Text` ; `rich` → `Rich` ou `Slot` vide ; `slot` → `Slot` ou `Rich` vide ; `flag` → `Value(bool)` ; `number` → `Value(number)` dans `min`/`max` ; `grid` → `Value(object)` qui se lit en `Grid` ; `host(chart)` → `Value` qui se lit en `ChartOption` et passe `oku_slides_chart::normalize`. Un widget inconnu : ses paramètres ne sont pas contrôlés (seul `issue-unknown-widget`), ses slots sont parcourus. Supprimer `issue-chapter-needs-title` et `issue-blocks-and-elements` des 20 `core.ftl` ; ajouter les clés nouvelles (en : `issue-unknown-widget = unknown widget: { $kind }`, `issue-wrong-category = { $kind } cannot be used here`, `issue-unknown-param = unknown parameter: { $param }`, `issue-param-type = invalid value for { $param }`, `issue-choice-out-of-range = value not allowed for { $param }`, `issue-slot-rejects = { $kind } is not accepted here`, `issue-placed-in-flow = element placed with a frame in a laid-out list`, `issue-unknown-bind = { $param } is linked to an unknown deck setting`, `issue-slot-too-deep = elements nest too deeply` ; fr : `widget inconnu : { $kind }`, `{ $kind } ne peut pas être utilisé ici`, `paramètre inconnu : { $param }`, `valeur invalide pour { $param }`, `valeur non permise pour { $param }`, `{ $kind } n'est pas accepté ici`, `élément placé avec un cadre dans une liste mise en page`, `{ $param } est lié à un réglage du deck inconnu`, `éléments imbriqués trop profondément` ; traduire les 18 autres). Vérifier par `grep` qu'aucune autre crate n'utilise les clés supprimées ; si c'est le cas, la tâche de cette crate les remplacera — les signaler dans le rapport.

Réécrire `validate_tests.rs` : un test par ligne du tableau (rstest), sur des decks v2 en JSON et `test_registry::registry()`. Pour `grid` et `host(chart)`, tester aussi un objet de la bonne forme mais illisible (`{"columns": "x"}`, option de graphique sans série valide) → `issue-param-type`. Pour `issue-unknown-bind`, construire le registre de test en JSON avec un widget dont un `plain` porte `bind: "nope"`.

- [ ] **Step 10: Run the core tests of this task**

Le reste du cœur (ops, replica) ne compile plus : pour faire tourner les tests de cette tâche, mettre temporairement les modules `ops` et `replica` et leurs usages hors compilation n'est pas permis. À la place, cette tâche **fait compiler tout le cœur** en adaptant `ops.rs`, `replica/*` et `sources.rs` au strict minimum pour les nouveaux types (compilation seulement, sans nouveau comportement ; les tests de ces modules peuvent échouer), et lance :

Run: `cargo nextest run -p oku-slides-core -E 'test(/params_tests|locate_tests|canonical_tests|validate_tests|widget_tests/)'`
Expected: PASS.

Consigner dans le rapport la liste des tests du cœur encore rouges (ops, replica, intégration) : les tâches 2 et 3 les reprennent.

- [ ] **Step 11: Commit**

```bash
git add crates/oku-slides-core crates/oku-slides-i18n/src/locales fixtures crates/oku-slides-render/tests
```
```bash
git commit -m "feat(core)!: modèle v2 {kind, params}, parcours des slots, validation par le registre (t-294)"
```

---

### Task 2: Ops v2

**Files:**
- Modify: `crates/oku-slides-core/src/ops.rs`
- Create: `crates/oku-slides-core/src/ops/value.rs` (si `ops.rs` devient `ops/mod.rs` ; sinon `crates/oku-slides-core/src/param_value.rs`) : conversion d'une valeur JSON selon son `ParamType`
- Modify: `crates/oku-slides-core/src/error.rs`
- Modify: `crates/oku-slides-core/tests/{ops,charts}.rs`, `tests/regressions/*` (hors replica)
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl`

**Interfaces:**
- Consumes: modèle v2, `Walk`, `Located`, `validate` (tâche 1), `WidgetRegistry`.
- Produces:
  - `Target { slide: String, element: Option<String> }` (`Serialize, Deserialize, JsonSchema`, `element` omis quand `None`), `SlotRef { owner: Target, param: String }`.
  - `Op` (voir tableau) ; `Op::apply(&self, deck: &mut Deck, registry: &WidgetRegistry) -> Result<Value, OpError>` (même valeur de retour qu'aujourd'hui : l'ancienne valeur ou `null`) ; `apply_all(deck: &Deck, ops: &[Op], registry: &WidgetRegistry) -> Result<Deck, TransactionError>` ; `ops_schema()`.
  - `SlideField::{Name, Description}`.
  - `OpErrorKind::{SlideNotFound, ElementNotFound, DuplicateId, IndexOutOfRange, InvalidValue, InvalidChart, UnknownWidget, UnknownParam, ParamType, ChoiceOutOfRange, SlotRejects, WrongCategory}` (`NotAGroup`, `NoText`, `NotAChart` disparaissent).
  - `pub fn param_value(ty: &ParamType, param: &str, value: Value) -> Result<Option<ParamValue>, OpError>` : `None` = retirer.

| Op v2 | Champs | Règles |
|---|---|---|
| `SetParam` | `target: Target, param: String, value: Value` | widget de la cible connu (`UnknownWidget`), paramètre déclaré (`UnknownParam`), valeur convertie par `param_value` ; un `slot` est refusé (`ParamType`, raison « use add_element ») ; un paramètre `bind` écrit le texte converti (ou `None`) dans le champ de `DeckParams` de ce nom, sinon dans `DeckParams.extra[bind]` (voir « Décisions ») ; un paramètre de type `grid` d'une slide dont un slot `flow` est non vide est refusé (`InvalidValue`, message `BLOCKS_OFF_GRID` réécrit : « flow slots are laid out by the slide and a grid hides them; move each element to a free slot first ») |
| `SetDeckParam` | `key, value` | code de l'actuel `SetParam` du deck |
| `SetKind` | `target, kind` | widget connu, de la bonne catégorie (`WrongCategory`) ; les paramètres sont gardés tels quels |
| `AddElement` | `slot: SlotRef, index: Option<usize>, element: Element` | propriétaire trouvé ; son widget déclare `param` de type `slot` (`ParamType` sinon) ; `accepts` (`SlotRejects`) ; widget de l'élément connu et de catégorie `element` ; graphiques contrôlés ; ids uniques dans la slide (`DuplicateId`) ; `index` ≤ longueur (`IndexOutOfRange`) ; profondeur du propriétaire + 1 + `slot_depth(element)` ≤ `MAX_SLOT_DEPTH` (`InvalidValue`) ; insertion par `Params::slot_mut` (`None` → `InvalidValue`) |
| `SetChartOption` | `target, param, patch` | le paramètre est `host(chart)` (`ParamType` sinon) ; logique actuelle sur l'option relue en `ChartOption`, réécrite en `Value` |
| `SetChartData` | `target, param, change` | idem |
| `SetCell` | inchangée | refusée si l'élément est dans un slot `flow` (message `BLOCKS_OFF_GRID`) |
| `SetSlideField` | `slide, field: SlideField, value` | `name`, `description` seulement |
| `SetText`, `SetGrid`, `SetParam` (deck) | — | supprimées |
| `SetStyle`, `ReplaceStyle`, `SetFrame`, `ReplaceElement`, `RemoveElement`, `AddSlide`, `RemoveSlide`, `MoveSlide`, `SetNotes`, `SetTheme`, `SetTemplates`, `SetToken`, `SetVar` | inchangés | parcours par `Walk` ; `ReplaceElement` et `AddSlide` contrôlent les graphiques et l'unicité des ids comme aujourd'hui |

`param_value` :

| `ParamType` | Valeur acceptée | Résultat |
|---|---|---|
| `Plain`, `Asset` | `null`, chaîne | `null` ou chaîne rognée vide → `None` ; sinon `Text` |
| `Choice { values }` | `null`, chaîne de `values` | `None` / `Text` ; hors liste → `ChoiceOutOfRange` |
| `Rich` | `null`, tableau de paragraphes | `null` ou `[]` → `None` ; sinon `Rich` |
| `Flag` | `null`, booléen | `None` / `Value` |
| `Number { min, max }` | `null`, nombre dans les bornes | `None` / `Value` |
| `Grid` | `null`, objet lisible en `Grid` | `None` / `Value` |
| `Host { Chart }` | objet lisible en `ChartOption`, qui passe `normalize` | `Value` (JSON de l'option) ; `InvalidChart` sinon |
| `Slot { .. }` | — | `ParamType` |

Erreurs à ajouter à `OpInner` (display en minuscules) : `UnknownWidget { kind }` (« unknown widget: {kind} »), `UnknownParam { widget, param }` (« {widget} has no parameter {param} »), `ParamType { param, expected: &'static str }` (« {param} expects {expected} »), `SlotRejects { slot, kind }` (« {slot} does not accept {kind} »), `WrongCategory { kind }` (« {kind} cannot be used here »), `ChoiceOutOfRange { param, value }` (« {value} is not allowed for {param} »). Messages Fluent (en / fr, puis 18 traductions) : `op-unknown-widget = Unknown widget: { $subject }` / `Widget inconnu : { $subject }` ; `op-unknown-param = Unknown parameter: { $subject }` / `Paramètre inconnu : { $subject }` ; `op-param-type = Invalid value for { $subject }` / `Valeur invalide pour { $subject }` ; `op-slot-rejects = { $subject } is not accepted here` / `{ $subject } n'est pas accepté ici` ; `op-wrong-category = { $subject } cannot be used here` / `{ $subject } ne peut pas être utilisé ici` ; `op-choice-out-of-range = Value not allowed for { $subject }` / `Valeur non permise pour { $subject }`. Retirer `op-not-a-group`, `op-no-text`, `op-not-a-chart` des 20 langues. Attention : `crates/oku-slides-session/src/error.rs:208` (`SessionError::NoChart`) utilise `op-not-a-chart` ; la tâche 5 lui donne sa propre clé. `subject()` rend `kind`, `param`, ou `kind`.

- [ ] **Step 1: Write the failing tests**

Réécrire `crates/oku-slides-core/tests/ops.rs` sur le modèle v2 (`common::registry()`) et ajouter, en rstest quand c'est possible :
- `set_param_plain_on_card_title_sets_the_text` ; `set_param_null_removes_the_param` ; `set_param_rich_sets_paragraphs` ; `set_param_on_bound_title_writes_deck_params` (slide `title`, `param: "title"` → `deck.params.title`) ;
- `set_param_unknown_param_returns_unknown_param` ; `set_param_on_unknown_widget_returns_unknown_widget` ; `set_param_choice_out_of_values_returns_choice_out_of_range` ; `set_param_number_out_of_bounds_returns_invalid_value` ; `set_param_on_slot_returns_param_type` ;
- la régression t-292 reste à sa place, `tests/regressions/issue_0292_grid_hides_blocks.rs`, réécrite en v2 : `set_param` sur `grid` d'une slide dont `blocks` est non vide → `InvalidValue`, et `set_cell` sur un élément de `blocks` → `InvalidValue` ;
- `set_kind_to_slide_widget_on_element_returns_wrong_category` ; `set_kind_keeps_params` ;
- `add_element_into_stack_children_inserts_at_index` ; `add_element_at_index_equal_to_len_appends` ; `add_element_at_index_past_len_returns_index_out_of_range` ; `add_element_into_slide_blocks_appends` ; `add_element_beyond_max_slot_depth_returns_invalid_value` (chaîne de 16 `canvas`, le 17ᵉ refusé) ; `replace_element_with_too_deep_tree_returns_invalid_value` ; `add_element_into_non_slot_param_returns_param_type` ; `add_element_rejected_by_accepts_returns_slot_rejects` (registre de test augmenté d'un widget à `accepts`, construit en JSON dans le test) ; `add_element_duplicate_id_returns_duplicate_id` ;
- `set_cell_in_flow_slot_returns_invalid_value` ;
- `set_deck_param_sets_title` ; `set_slide_field_name_sets_the_name`.

Réécrire `tests/charts.rs` (`SetChartOption`/`SetChartData` par `target` + `param`, élément `chart` v2) et `tests/regressions/issue_0164_chart_body_bloats_element_enum.rs` (supprimer ce test : son objet, la taille de l'enum `ElementBody`, disparaît ; le noter).

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-core --test ops --test charts`
Expected: FAIL (compilation : `Target`, `SetParam { target … }` absents).

- [ ] **Step 3: Implement**

Écrire `param_value`, les nouvelles variantes et leur application. Localisation des éléments : remplacer `find_mut`/`element_mut`/`container_mut` par :

```rust
/// Élément `id` d'une liste d'éléments, en descendant dans tous les slots.
fn find_mut<'a>(list: &'a mut [Element], id: &str) -> Option<&'a mut Element> {
    for e in list.iter_mut() {
        if e.id == id {
            return Some(e);
        }
        for (_, children) in e.params.slots_mut() {
            if let Some(found) = find_mut(children, id) {
                return Some(found);
            }
        }
    }
    None
}

fn element_mut<'a>(slide: &'a mut Slide, id: &str) -> Option<&'a mut Element> {
    for (_, list) in slide.params.slots_mut() {
        if let Some(e) = find_mut(list, id) {
            return Some(e);
        }
    }
    None
}
```

La profondeur étant bornée (`MAX_SLOT_DEPTH`), cette récursion est sûre. Vérifier ce code par `cargo check` dès le début de l'étape. `apply_all` appelle `canonicalize` comme aujourd'hui. `ops_schema` inchangé.

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p oku-slides-core -E 'not test(/replica/) and not binary(convergence) and not binary(untrusted)'` puis `cargo nextest run -p oku-slides-i18n`
Expected: PASS. Les tests restants (replica, convergence, untrusted, sources) sont repris à la tâche 3.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-core crates/oku-slides-i18n/src/locales
```
```bash
git commit -m "feat(core)!: ops v2, set_param validé par le schéma, set_kind, add_element par slot (t-294)"
```

---

### Task 3: Replica Loro et sources en v2

**Files:**
- Modify: `crates/oku-slides-core/src/replica/{schema,write,read,mod}.rs`, `replica/history.rs`
- Modify: `crates/oku-slides-core/src/sources.rs`
- Modify: tests du replica et d'intégration : `replica/*_tests.rs`, `sources_tests.rs`, `tests/{convergence,untrusted,zip_limits}.rs`, `tests/regressions/issue_0263_undo_reorder_loses_slides.rs`

**Interfaces:**
- Consumes: modèle v2 (tâche 1), ops v2 (tâche 2).
- Produces:
  - `DeckReplica::new(deck: &Deck, registry: Arc<WidgetRegistry>)`, `DeckReplica::from_snapshot(bytes, max_bytes, registry: Arc<WidgetRegistry>)`, `DeckReplica::set_registry(&mut self, Arc<WidgetRegistry>)`. `apply`, `reconcile`, `merge`, `undo`… gardent leur signature.
  - `SourceBundle::new(deck: Deck, registry: Arc<WidgetRegistry>)`, `SourceBundle::from_zip(bytes, limits, registry: Arc<WidgetRegistry>)`, `SourceBundle::set_registry(&mut self, Arc<WidgetRegistry>)`, `SourceBundle::registry(&self) -> &WidgetRegistry` (fusion et restauration appellent `validate` avec lui).
  - `SOURCES_VERSION = 3`.

- [ ] **Step 1: Write the failing tests**

Dans `replica/mod_tests.rs` (réécrit sur le v2, registre `test_registry::registry()` dans un `Arc`), ajouter :
- `reconcile_new_card_creates_its_text_params_before_any_value` : un `card` sans paramètre → dans le `LoroDoc`, `params.kicker`, `params.title` sont des `LoroText`, `params.body` une `LoroMap` à clé `rich`.
- `concurrent_first_edit_of_card_title_merges_both` : deux pairs partent du même replica (carte sans titre), chacun pose un titre, fusion → les deux textes fusionnés (même parade que t-188).
- `concurrent_edit_of_title_and_body_of_one_card_keeps_both`.
- `read_rich_param_without_registry_gives_paragraphs` : lecture d'un replica via `DeckReplica::from_snapshot` avec un **registre vide** (`WidgetRegistry::from_probe(json!({"base": []}))`) → `params.body` relu en paragraphes, `params.title` en texte.
- `concurrent_set_kind_and_param_edit_keep_the_param` : A change `kind` de `card` en `text`, B édite `title` → après fusion, `kind: "text"` et `params.title` conservé ; `validate` signale `issue-unknown-param`.
- `round_trip_roadmap_is_identical` : `fixtures/roadmap.json` → replica → deck identique.
- `concurrent_edit_of_image_fit_keeps_one_whole_value` : deux pairs posent `fit` à `contain` et à `stretch` → après fusion, l'une des deux valeurs entière (jamais un mélange).
- `concurrent_add_element_into_one_slot_keeps_both` : deux pairs ajoutent chacun un élément dans le même `blocks` → les deux présents.
- `concurrent_add_element_and_set_kind_of_owner_keeps_the_child` : A ajoute un enfant à un `stack`, B change le `stack` en `canvas` → l'enfant est conservé.
- `read_deeply_nested_slots_stops_at_the_depth_bound` : un replica forgé avec 200 niveaux de slots se lit sans débordement de pile (thread à petite pile, comme les tests hostiles existants de `mod_tests.rs`).

Dans `sources_tests.rs` : `from_zip_with_sources_version_2_returns_not_sources` (manifeste `version: 2`).

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-core -E 'test(/replica|sources/)'`
Expected: FAIL.

- [ ] **Step 3: Implement**

`schema.rs` :
- `Ctx::Slide` : `name`, `description`, `notes` → `Prose` ; `params` → `Nested(Ctx::Params)` ; le reste → `Nested(Opaque)`.
- `Ctx::Element` : `params` → `Nested(Ctx::Params)` ; `frame`, `cell`, `style` → `Nested(Opaque)` ; `id`, `kind` → `Atom`.
- `Ctx::Params` : le type d'une clé ne se lit pas dans la table (voir écriture et lecture).
- `texts()`/`lists()` : `Slide` → `name`, `description`, `notes` ; `Element` → rien ; les paramètres créés d'avance viennent du registre.

`write.rs`, dans `sync_map` pour `Ctx::Params` :
- le propriétaire (slide ou élément) passe son `kind` ; `let schema = registry.get(kind)` ;
- pour chaque clé de la valeur JSON : type déclaré s'il y en a un (`plain` → `Prose` ; `choice` et `asset` → `Atom`, voir « Décisions » ; `rich` → carte `{rich: LoroText}` via `rich::sync` ; `slot` → `Collection(Element)` ; le reste → `Atom`), sinon type par la forme (chaîne → `Prose`, tableau d'éléments → `Collection(Element)`, tableau d'objets → rich, le reste → `Atom`) ;
- création d'avance : chaque paramètre `plain`, `rich` ou `slot` déclaré et absent est créé vide (`LoroText`, carte `rich` vide, `LoroMovableList`) ;
- une clé présente dans Loro mais absente du JSON : retirée, **sauf** si c'est un paramètre `plain`/`rich`/`slot` déclaré (on vide son conteneur).

`read.rs` : le compteur `depth` existant (`MAX_DEPTH = 64`) est passé à travers le cycle `Params` → liste → `Element` → `Params`, sans remise à zéro. Pour `Ctx::Params` : `LoroText` → texte (vide = absent) ; `LoroMap` dont la seule clé est `rich` et qui porte un `LoroText` → paragraphes (`rich::read`, vides = absents) ; `LoroMovableList` → liste d'éléments (`read_list(…, Ctx::Element, …)`, vide = absente) ; valeur → JSON ; carte ordinaire → objet opaque. `clean_elements` descend dans toutes les listes de `params` au lieu de `children`. `read_slide` lit `params` au lieu de `blocks`/`elements`.

`mod.rs` : le registre est un champ `registry: Arc<WidgetRegistry>` de `DeckReplica`, passé à `write::sync_map`. `sources.rs` : `SOURCES_VERSION = 3` ; le registre traverse `SourceBundle` vers `DeckReplica`.

- [ ] **Step 4: Migrate the remaining core tests**

Réécrire sur le v2 : `read_tests`, `undo_tests`, `history_tests`, `authors_tests` (registre de test dans les constructeurs), `tests/convergence.rs` (proptest : les mutations portent sur `params.body` d'un `text` et `params.title` d'un `content`), `tests/untrusted.rs`, `tests/zip_limits.rs`, `tests/regressions/issue_0263_undo_reorder_loses_slides.rs`.

- [ ] **Step 5: Run the whole core**

Run: `cargo nextest run -p oku-slides-core && cargo clippy -p oku-slides-core --all-targets -- -D warnings`
Expected: PASS, clippy propre. **Le cœur est entièrement vert à partir d'ici.**

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-core
```
```bash
git commit -m "feat(core)!: replica Loro v2, paramètres créés d'avance par le registre, lecture sans registre (t-294)"
```

---

### Task 4: Rendu générique

**Files:**
- Modify: `typst/oku-slides/src/data.typ`, `slides.typ`, `widgets.typ`, `core.typ`, `lib.typ`, `README.md`
- Modify: `crates/oku-slides-render/src/{world,charts,lib,registry}.rs`
- Modify: `crates/oku-slides-render/tests/{render,widgets,golden}.rs`, `tests/regressions/*`, `crates/oku-slides-render/src/*_tests.rs`

**Interfaces:**
- Consumes: modèle v2, `WidgetRegistry`, `PackWidgets`, `Registries` (plan 1).
- Produces:
  - `Renderer::new(fonts: Fonts) -> Result<Renderer, RenderError>` (charge le registre de base) ; `Renderer::set_widgets(&mut self, pack: Option<&PackWidgets>, registry: Arc<WidgetRegistry>) -> Result<(), RenderError>` (source du pack injectée, graphiques redessinés) ; `Renderer::registry(&self) -> &Arc<WidgetRegistry>`.
  - Typst : `render-deck(data, theme: none, image-loader: none, widgets: none)` (`widgets` : dictionnaire nom → widget ; `none` → registre de base).
  - `Layout::fields` émis aussi par les slides (`owner` = id de la slide) ; `tracked` et les ids `@…` de `<oku-slides-box>` disparaissent.

- [ ] **Step 1: Write the failing tests**

Dans `tests/widgets.rs` :
- `compile_title_slide_emits_title_and_subtitle_fields_owned_by_the_slide` (roadmap v2 : `owner` = id de la couverture).
- `compile_content_slide_emits_kicker_and_title_fields`.
- `compile_pack_widget_renders_in_the_deck` : pack `citations` (widget `quote` du plan 1), deck v2 avec un élément `quote` ; `Renderer::set_widgets(Some(&pack), registry)` puis rendu → la boîte de l'élément existe et son champ `body` est émis.
- `compile_pack_override_of_card_replaces_the_base_render` : pack qui redéfinit `card` en ajoutant `icon: p.plain()` et en peignant tout le fond de la carte en rouge pur (`block(fill: rgb("#ff0000"), …)`) ; dans le PNG de la page (`Compiled::png`, décodé avec le crate `png` s'il est déjà une dépendance du workspace, sinon comparer les octets du PNG à ceux d'un rendu sans pack : ils doivent différer), le centre de la boîte de la carte est rouge. Vérifier aussi que le champ `title` est émis dans la boîte de la carte.
- `compile_unknown_element_kind_draws_the_unknown_box` (élément `kind: "nope"` : la compilation réussit).

Dans `tests/golden.rs` : les decks sont déjà en v2 (tâche 1). Le test doit passer à l'identique, sous la règle d'antialiasing.

Réécrire `tests/render.rs` : `Op::SetText` → `Op::SetParam { target, param: "body", value }` avec le registre du `Renderer` ; decks de graphiques en v2.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-render`
Expected: FAIL.

- [ ] **Step 3: Slides take `wrap`**

Dans `slides.typ`, ajouter `wrap: (name, body, style) => body` à `slide`, `title-slide`, `chapter` et `end-slide`. Chaque `tracked("@x", corps, style: s)` devient `wrap("x", corps, s)` (`_slide-title` reçoit `wrap`). Supprimer `tracked` de `core.typ` (plus aucun usage ; `_measured` reste pour `field`).

- [ ] **Step 4: Generic render in `data.typ`**

Supprimer `_element-args`, `_widget-of`, `_slide-args` et l'adaptateur v1. Écrire :

```typst
// Arguments d'un widget depuis son schéma : défauts, liens au deck, textes
// interpolés, paragraphes rendus, slots rendus en entrées.
#let _args(t, schema, params, vars, deck-params, image-loader, widgets) = {
  let out = (:)
  for d in schema.params {
    let bind = d.at("bind", default: none)
    let v = if bind != none { deck-params.at(bind, default: none) } else { params.at(d.name, default: d.default) }
    out.insert(d.name, if d.type == "plain" { interp(v, vars) }
      else if d.type == "rich" { if v == none or v == () { [] } else { _paragraphs(t, v, vars) } }
      else if d.type == "slot" {
        let list = if v == none { () } else { v }
        if d.mode == "free" { _placed(t, list, vars, image-loader, widgets) } else { _flow(t, list, vars, image-loader, widgets) }
      }
      else { v })
  }
  out
}
```

`render-element(t, e, vars, image-loader: none, widgets: none)` : widget `widgets.at(e.kind)`, de catégorie `element` (sinon l'encart « élément inconnu ») ; `ctx = (id, tokens, vars, style, framed, load: if image-loader != none { (..a) => image-loader(e, ..a) } else { none })` ; `render(_args(…), ctx)` ; enveloppe de boîte inchangée. `_flow`/`_placed` passent `widgets`. `render-slide(s, image-loader: none, widgets: none)` : widget `widgets.at(s.kind)` de catégorie `slide` (sinon une page avec l'encart d'erreur), `ctx.id` = id de la slide, `render(_args(…), ctx)`. `render-deck(data, theme: none, image-loader: none, widgets: none)` : `let widgets = if widgets == none { by-name(registry) } else { widgets }`.

- [ ] **Step 5: Slide widgets pass fields; image and chart use the new loader**

Dans `widgets.typ` : chaque widget de slide passe `wrap: (name, body, style) => field(ctx, name, body, style: style)`. `_toc` : `toc(title: if args.title == none { auto } else { args.title }, …)`. `_chapter` : `chapter(if args.title == none { "" } else { args.title }, …)`. `_image` : `(ctx.load)(asset: args.asset, fit: args.fit)`. `_chart` : `(ctx.load)(chart: true)`. `_end` : le texte arrive interpolé par `_args`.

- [ ] **Step 6: World, loader and charts in Rust**

`world.rs`, `main_source` : importe `widget.typ` (`by-name`), le registre de base et, si un pack est posé, `/packs/<id>/widgets.typ` ; passe `widgets: by-name(base + pack)` à `render-deck`. Le chargeur :

```rust
// Dans `main_source` (format!, accolades doublées).
#let load-image(e, asset: none, fit: "cover", chart: false) = {{
  if chart {{
    if "{svg_field}" not in e {{ fallback({chart_failed}) }}
    else if "frame" in e {{
      image("/charts/" + e.at("{svg_field}") + ".svg", width: 100%, height: 100%, fit: "contain")
    }} else {{
      // Dans le flux : toute la largeur, sans dépasser la hauteur disponible.
      layout(size => {{
        let h = size.width * {aspect}
        if size.height < h {{ h = size.height }}
        image("/charts/" + e.at("{svg_field}") + ".svg", width: 100%, height: h, fit: "contain")
      }})
    }}
  }} else {{
    let hex = if asset == none {{ "" }} else {{ asset.trim("sha256:", at: start) }}
    if hex in available {{
      image("/assets/" + hex, width: 100%, height: 100%, fit: fit)
    }} else {{ fallback({missing}) }}
  }}
}}
```

C'est le chargeur actuel (`world.rs`, `main_source`) avec trois changements seulement : la signature, le test `chart` au lieu de `e.kind == "chart"`, et `asset`/`fit` lus dans les arguments au lieu de `e.asset`/`e.at("fit")`.

`charts.rs`, `Charts::attach(json, theme, fonts, registry)` : parcourt `slides[].params` et les `params` des éléments (tout tableau d'objets ayant `id` et `kind`) ; pour un élément dont le widget a un paramètre `host(chart)`, lit l'option dans `params.<nom>` et pose `chart-svg` sur l'élément. `Renderer` garde `registry: Arc<WidgetRegistry>` et `pack: Option<PackWidgets>` ; `set_widgets` les remplace, ajoute la source du pack au monde, refait la source principale et redessine les graphiques.

- [ ] **Step 7: Run all render tests**

Run: `cargo nextest run -p oku-slides-render && cargo clippy -p oku-slides-render --all-targets -- -D warnings`
Expected: PASS ; test doré sous la règle d'antialiasing : produire `after` avec `dump_reference_pages`, lancer `pixel_diff.py before after` ; s'il échoue, corriger le Typst ; s'il passe avec des écarts, ré-enregistrer exactement ces lignes et coller sa sortie dans le rapport.

- [ ] **Step 8: README**

`typst/oku-slides/README.md` : format v2 du deck JSON (exemple), `render-deck(…, widgets:)`, champs émis par les slides.

- [ ] **Step 9: Commit**

```bash
git add typst/oku-slides crates/oku-slides-render
```
```bash
git commit -m "feat(render)!: rendu générique des widgets, champs des slides, widgets d'un pack rendus (t-294)"
```

---

### Task 5: Session sur le modèle v2

**Files:**
- Modify: `crates/oku-slides-session/src/{lib,views,charts,panel,text,templates_mode,error}.rs`, `crates/oku-slides-i18n/src/locales/*/session.ftl`, `src/agent/mod.rs` (compilation seulement ; le comportement de l'agent est à la tâche 7)
- Modify: tests de session (`src/*_tests.rs`, `tests/*.rs`, `tests/common/mod.rs`, `tests/regressions/*`)

**Interfaces:**
- Consumes: cœur v2, `Renderer::set_widgets`, `Registries`, `PackWidgets`.
- Produces:
  - `Session` tient `registries: Registries` et `registry: Arc<WidgetRegistry>` (celui du deck ouvert) ; `apply` utilise `registry`.
  - `Request::Widgets` → `Response` portant le registre sérialisé (`WidgetRegistry` est `Serialize`).
  - `SlideSummary { id, kind: String, title: Option<String>, … }` : titre = `params.title` de la slide, sinon (paramètre lié) `deck.params.title`.
  - `Target` : `id`, `target: "element" | "field"`, `kind` (nom du widget ou du paramètre), `param: Option<String>` (paramètre édité : pour un élément, son premier paramètre `rich` dans l'ordre du schéma ; pour un champ, son nom), `free`, `text`, géométrie, `style`. Champs de slide : id `@<param>`, construits depuis `Layout::fields_on(page)` dont `owner` = id de la slide. Texte d'un champ : `params.<param>` de la slide, ou la valeur liée du deck.
  - Deck neuf : slide `title` puis slide `content` avec un `text`.

- [ ] **Step 1: Write the failing tests**

Dans `src/lib_tests.rs` :
- `slide_view_of_cover_lists_title_field_with_deck_title_text` ;
- `slide_view_of_content_lists_element_target_with_its_rich_param` (`param: Some("body")` pour un `text`, `Some("body")` pour une `card`) ;
- `apply_set_param_on_card_title_sets_that_title` (la valeur posée, `"Nouveau titre"`, se relit dans `params.title` de la carte) ;
- `handle_widgets_returns_the_base_registry` ;
- `new_deck_is_valid_v2` (`validate` sans issue).

Dans `src/charts_tests.rs` : la coalescence des annulations de graphiques sur des éléments `chart` v2.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-session`
Expected: FAIL.

- [ ] **Step 3: Implement**

- `Session::new` : `Registries::new(fonts)`, registre de base ; le `Renderer` reçoit `set_widgets(None, base)`.
- Ouverture ou changement de deck, et `Op::SetTemplates` : pack du deck (`deck.templates` → widgets du pack s'il en a, tâche 6 ; en attendant, toujours la base) ; `registry` et `Renderer::set_widgets` mis à jour ; `SourceBundle::set_registry`.
- `slide()` : cibles d'éléments via `layout.boxes_on(page)` + `slide.locate(id)` ; texte = markdown (`paragraphs_to_text`) du premier paramètre `rich` du schéma de l'élément ; champs de slide via `layout.fields_on(page)` filtrés sur `owner == slide.id`, id `@<field>`.
- `snapshot()` : `SlideSummary.kind = slide.kind.clone()`, titre comme ci-dessus.
- `chart_kind()` : l'option est le paramètre `host(chart)` de l'élément ; `Op::ReplaceElement` inchangé.
- `charts.rs` : `canonicalize_tree` parcourt les éléments v2 et leurs paramètres `host(chart)`.
- `panel.rs` : `slide.params.slot("blocks").is_some_and(|b| !b.is_empty())` ; l'action `set_grid` (A2UI) émet maintenant `set_param` (paramètre `grid`) — l'action garde son nom côté A2UI.
- `templates_mode.rs` : `deck.params` devient `DeckParams`, logique inchangée.
- `error.rs` : `SessionError::NoChart` prend sa propre clé `session-no-chart` (en : `No chart here: { $subject }` ; fr : `Pas de graphique ici : { $subject }` ; 18 traductions) dans les 20 `session.ftl`, au lieu de `op-not-a-chart` retirée à la tâche 2. Test : `no_chart_error_localizes_with_its_own_key`.
- `agent/mod.rs` : compilation seulement (`SetSlideField` → `SetParam` sur la slide, `SetText` → `SetParam` sur `body`) ; la tâche 7 revoit son comportement.

Réécrire les tests de session sur les fixtures v2 (`fixtures/roadmap.json` converti) et les ops v2.

- [ ] **Step 4: Run the session tests**

Run: `cargo nextest run -p oku-slides-session -E 'not test(/templates/)'`
Expected: PASS hors templates (tâche 6).

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-session
```
```bash
git commit -m "feat(session)!: session sur le modèle v2, registre du deck, cibles et champs v2 (t-294)"
```

---

### Task 6: Packs de modèles en dossiers

**Files:**
- Create: `templates/essentiel/pack.json`, `templates/grilles/pack.json` (déplacés et convertis)
- Delete: `templates/essentiel.json`, `templates/grilles.json`
- Modify: `crates/oku-slides-session/src/templates.rs`, `lib.rs` (choix du pack), tests des templates

**Interfaces:**
- Consumes: `PackWidgets`, `Registries`, `Session` (tâche 5).
- Produces:
  - `Library::widgets(&self, id: &str) -> Option<String>` : source `widgets.typ` du pack (`None` pour un pack sans widgets ; les packs intégrés n'en ont pas).
  - Format disque : `<dossier des modèles>/<id>/pack.json` (+ `widgets.typ` optionnel). `Library::save` écrit `pack.json` dans le dossier du pack (en le créant) ; `Library::delete` retire le dossier. Un ancien fichier `<id>.json` n'est pas lu.

- [ ] **Step 1: Move and convert the built-in packs**

```bash
mkdir -p templates/essentiel templates/grilles
```
```bash
git mv templates/essentiel.json templates/essentiel/pack.json
```
```bash
git mv templates/grilles.json templates/grilles/pack.json
```
```bash
python3 .superpowers/sdd/2026-09-28-widgets-typst-2/v1_to_v2.py templates/essentiel/pack.json templates/grilles/pack.json
```

- [ ] **Step 2: Write the failing tests**

Dans les tests de `templates.rs` (répertoire temporaire) :
- `library_lists_builtin_and_user_packs_from_folders` ;
- `library_save_writes_pack_json_in_the_pack_folder` ;
- `library_delete_removes_the_pack_folder` ;
- `library_widgets_returns_the_pack_widgets_source` ;
- `library_ignores_a_legacy_json_file` ;
- `session_with_pack_widgets_uses_the_pack_registry` (pack utilisateur avec le `widgets.typ` `quote` du plan 1 ; `SetTemplates` → `Request::Widgets` contient `quote`).
- `every_builtin_template_is_valid_v2` : chaque slide des packs intégrés passe `validate` sans issue.

- [ ] **Step 3: Implement**

`BUILTIN` : `include_str!("../../../templates/essentiel/pack.json")` et `…/grilles/pack.json`. `user_sets`, `get`, `save`, `delete`, `list` lisent et écrivent des dossiers. Session : au changement de pack, `Library::widgets(id)` → `PackWidgets::new(id, source)` → `Registries::get(Some(&pack))` ; une erreur de pack est journalisée, montrée comme aujourd'hui les erreurs de rendu, et la session garde le registre de base.

- [ ] **Step 4: Run the whole session crate**

Run: `cargo nextest run -p oku-slides-session && cargo clippy -p oku-slides-session --all-targets -- -D warnings`
Expected: PASS. **La session est verte.**

- [ ] **Step 5: Commit**

```bash
git add templates crates/oku-slides-session
```
```bash
git commit -m "feat(session)!: packs de modèles en dossiers, widgets d'un pack chargés par la session (t-294)"
```

---

### Task 7: Agent, MCP, CLI et app

**Files:**
- Modify: `crates/oku-slides-session/src/agent/mod.rs` (+ tests)
- Modify: `crates/oku-slides-mcp/src/{tools,read_tools,edit_tools}.rs`, `crates/oku-slides-mcp/tests/*`
- Modify: `crates/oku-slides-agent/tests/*` (fixtures v2)
- Modify: `crates/oku-slides-cli/src/bench.rs`, tests CLI
- Modify: `src-tauri/src/*` si la compilation l'exige
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` si des messages de l'agent parlent de `@title` ou des ops v1

**Interfaces:**
- Consumes: session v2 (tâches 5-6).
- Produces:
  - Outil MCP `widgets` (« Widgets available in this deck: every widget with its parameters and their types. Read it before adding an element. ») → `Request::Widgets`.
  - Descriptions MCP à jour : `apply` décrit `set_param`, `set_deck_param`, `set_kind`, `add_element` par slot ; plus de mention de `set_text`/`set_slide_field` (sauf `name`/`description`)/`set_grid`. La ligne qui cite « @title, @subtitle, @kicker, @text » décrit désormais « @<param> : un paramètre texte de la slide ».
  - `deck_outline` : `kind` (chaîne), `title` (paramètre ou valeur liée).
  - Agent : `@<param>` désigne un paramètre texte de la slide ; les réécritures produisent `SetParam { target: {slide}, param, value }` ; un élément → `SetParam` sur son premier paramètre `rich`.

- [ ] **Step 1: Write the failing tests**

- MCP (`tests/surfaces.rs`, `tests/edit.rs`) : `widgets_tool_lists_card_with_its_params` ; `apply_set_param_through_mcp_changes_the_slide` ; les lectures d'éléments (`.body.text()`) remplacées par `params.rich("body")`.
- Agent (`crates/oku-slides-session/tests/agent.rs`, `crates/oku-slides-agent/tests/*`) : `agent_rewrite_of_at_title_sets_the_slide_title_param` ; les tests existants passent sur les fixtures v2 (`fixtures/agent/e2e.json` converti à la tâche 1).
- CLI : `bench` compile et tourne sur un deck v2 (premier élément du premier slot `blocks`).

- [ ] **Step 2: Agent**

`crates/oku-slides-session/src/agent/mod.rs` : `slide_field` disparaît. La lecture d'une cible de réécriture devient :

- `element` commence par `@` → nom de paramètre `p = &element[1..]` de la slide. Texte courant : `slide.params.text(p)`, sinon, si le schéma du widget de slide déclare `p` avec `bind: Some(k)`, la valeur de `k` dans `deck.params`. Op produite : `Op::SetParam { target: Target { slide, element: None }, param: p.into(), value: Value::String(texte) }`. Un `p` absent du schéma → la réécriture est refusée avec l'erreur d'op `UnknownParam` (même chemin qu'une op refusée aujourd'hui).
- sinon → élément ; paramètre = premier `rich` du schéma de son widget (aucun → refus, `ParamType`). Texte courant : `paragraphs_to_text(params.rich(p))`. Op : `Op::SetParam { target: Target { slide, element: Some(id) }, param, value: serde_json::to_value(paragraphes) }`.

Messages de l'agent (`agent.ftl`, 20 langues) qui citent `@title`, `set_text` ou `set_slide_field` : les réécrire en « @<paramètre> » et `set_param`.

- [ ] **Step 3: MCP**

- `tools.rs` : ajouter `tool::<NoArgs>("widgets", …)` → `Request::Widgets`, dans la liste des outils de lecture ; réécrire la description d'`apply` (ops v2, en une phrase par op : `set_param`, `set_deck_param`, `set_kind`, `add_element` avec `slot`, `set_chart_option`/`set_chart_data` avec `target` et `param`) et la ligne `@title, @subtitle, @kicker, @text` → « @<param> : a text parameter of the slide (see `widgets`) ».
- `read_tools.rs` : `deck_outline` écrit `"kind": sl.kind` (chaîne) et `"title"` = `sl.params.text("title")`, sinon la valeur liée du deck pour une slide dont le schéma lie `title`.
- `edit_tools.rs` : `ApplyArgs.ops: Vec<Op>` inchangé (le schéma suit `Op`).
- Tests : `tests/surfaces.rs` lit `slide.locate("e-texte")` puis `params.rich("body")` ; ajouter `widgets_tool_lists_card_with_its_params` et `apply_set_param_through_mcp_changes_the_slide`.

- [ ] **Step 4: CLI and app**

- `crates/oku-slides-cli/src/bench.rs` : premier élément trouvé par `deck.slides.iter().find_map(|s| s.params.slot("blocks").and_then(<[_]>::first).map(|e| (s.id.clone(), e.id.clone())))` ; op `Op::SetParam { target: Target { slide, element: Some(id) }, param: "body".into(), value }` ; `apply` avec le registre de base (`oku_slides_render::load_registry(&fonts, None)`).
- `src-tauri` : `cargo check -p oku-slides-app`. Il ne fait que transmettre `Request`/`Response` : ne rien changer si la compilation passe ; sinon, adapter les seuls endroits en erreur et les lister dans le rapport.

- [ ] **Step 5: Run**

Run: `cargo nextest run --workspace --exclude oku-slides-app && cargo check -p oku-slides-app && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS. **Tout le Rust est vert.**

- [ ] **Step 6: Commit**

```bash
git add crates src-tauri
```
```bash
git commit -m "feat(mcp)!: outil widgets, agent et CLI sur le modèle v2 (t-294)"
```

---

### Task 8: UI sur le modèle v2

**Files:**
- Modify: `ui/src/api.ts`, `deck.ts`, `slideTools.ts`, `a2ui/panelActions.ts`, `Stage.tsx`, `App.tsx`, `SlidePile.tsx`, `chart/ChartPanel.tsx`, `chart/chartDeck.ts`, `agent/rewrite.ts`
- Create: `ui/src/newElement.ts`, `ui/src/newElement.test.ts`
- Modify: `ui/src/{deck,slideTools}.test.ts`, `a2ui/panelActions.test.ts`, `chart/chartDeck.test.ts`, `agent/rewrite.test.ts`, et tout test de rendu qui construit un deck
- Modify: `ui/e2e/smoke.cjs`, `ui/e2e/editor.cjs`

**Interfaces:**
- Consumes: `Target` v2 (`param`), `SlideSummary.kind: string`, `Request::Widgets` (`cmd: "widgets"`), ops v2.
- Produces (TS) :
  - `type Params = Record<string, unknown>` ; `Element { id, kind: string, frame?, cell?, style?, params?: Params }` ; `Slide { id, kind: string, params?: Params, name?, description?, notes? }` ; `SlideKind = string`.
  - `EditTarget { id, target: "element" | "field", kind: string, param?: string, … }` (ex-`Target` de la vue, renommé partout) ; `Target = { slide: string; element?: string }` (adressage des ops).
  - Helpers dans `deck.ts` : `slot(owner: { params?: Params }, name: string): Element[]`, `rich(e: Element, name: string): Paragraph[] | undefined`.
  - Op builders v2 : `textOps` → `set_param` (élément : `target: {slide, element}`, `param: t.param` ; champ : `target: {slide}`, `param: t.kind`) ; une cible d'élément sans `param` n'est pas éditable (`Stage` n'ouvre pas d'éditeur, `textOps` rend `[]`) ; plus de branche « couverture » (le cœur écrit les paramètres liés) ; `blocksToGridOps`/`pasteOps` → `add_element` avec `slot: {owner: {slide}, param: "blocks" | "elements"}` ; `panelActions` : `set_grid` → `set_param` (`param: "grid"`), `rename_template`/`describe_template` → `set_slide_field` (`name`/`description`) ; `App.tsx` : titre du deck → `set_deck_param` ; nouvel élément construit par une fonction extraite `newElement(kind: AddKind, id: string): Element` dans `ui/src/newElement.ts` (avec son test), qui rend : `text` → `{kind: "text", params: {body: [{runs: [{t: ""}]}]}}` (même contenu de départ qu'aujourd'hui), `card` → `{kind: "card", params: {title: <titre de départ actuel>, body: <corps de départ actuel>}}`, `shape` → `{kind: "shape", style: <style actuel>}` (sans `params` : `shape` vaut `rect` par défaut), `chart` → `{kind: "chart", params: {option: starterChart()}}` ; `App.tsx` ne garde que l'appel (il fait 896 lignes, il ne doit pas grossir) ; `ChartPanel` → `set_chart_option {target, param: "option", patch}` ; `chartDeck` lit `el.params?.option`.

- [ ] **Step 1: Update tests first**

Réécrire les fixtures TS des tests listés en v2 et leurs attentes d'ops selon « Carte du v1 vers le v2 ». Lancer `cd ui && bun test` → FAIL.

- [ ] **Step 2: Implement**

Modifier les fichiers listés. `findElement` descend dans tous les slots (`Object.values(params)` dont les valeurs sont des tableaux d'éléments). Donner à `slot`, `rich`, `newElement` et aux builders d'ops un commentaire de doc d'une phrase. `slideTools.ts` n'a besoin que de l'élargissement des types (`canAdd` prend des `string`), sans changement de logique.

- [ ] **Step 3: Run the UI checks and e2e scripts**

Run: `cd ui && bun run typecheck && bun test && bun run test`
Expected: PASS. Mettre à jour `ui/e2e/smoke.cjs` (`snap.slide.params.title`, `slot(snap.slide, "elements")`) et `ui/e2e/editor.cjs` (`params.blocks`, `params.body`) ; les lancer seulement si l'environnement e2e est disponible (sinon le dire dans le rapport).

- [ ] **Step 4: Commit**

```bash
git add ui
```
```bash
git commit -m "feat(ui)!: UI sur le modèle v2, ops set_param et add_element par slot (t-294)"
```

---

### Task 9: Vérification finale et documentation

**Files:**
- Modify: `docs/superpowers/specs/2026-09-27-widgets-typst-design.md` (écarts constatés pendant l'exécution, s'il y en a)
- Modify: `README.md` du dépôt si le format du deck ou des modèles y est décrit
- Modify: `crates/oku-slides-render/src/lib.rs`, `crates/oku-slides-core/src/lib.rs`, `crates/oku-slides-session/src/lib.rs` : doc de crate (liste des types publics)
- Modify: `crates/oku-slides-core/ARCHITECTURE.md`, `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-mcp/ARCHITECTURE.md` ; Create: `crates/oku-slides-render/ARCHITECTURE.md` (moins de 50 lignes chacun : rôle, points d'extension, dépendances)

- [ ] **Step 1: Search for leftovers**

```bash
grep -rn "ElementBody\|SlideKind\|SlideLayout\|GroupLayout\|ShapeKind\|set_text\|set_slide_field\|set_grid\|tracked(\|\"@title\"\|effective_grid" crates src-tauri ui/src typst --include=*.rs --include=*.ts --include=*.tsx --include=*.typ
grep -rn "op-not-a-group\|op-no-text\|op-not-a-chart\|issue-chapter-needs-title\|issue-blocks-and-elements" crates ui/src src-tauri
```

Chaque occurrence restante est justifiée (par exemple `set_slide_field` pour `name`/`description`, `@title` comme id de champ de slide) ou supprimée.

- [ ] **Step 2: Full check**

Run: `just check` puis `just test`
Expected: PASS (fmt, clippy, typecheck, tests Rust et UI).

- [ ] **Step 3: Commit**

```bash
git add docs README.md crates
```
```bash
git commit -m "docs(core) : modèle v2 documenté, restes du v1 retirés (t-294)"
```
