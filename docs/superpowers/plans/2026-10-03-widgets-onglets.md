# Widgets en sous-onglets — plan d'implémentation

> **Pour les agents :** sous-skill requis : superpowers:subagent-driven-development
> (recommandé) ou superpowers:executing-plans, tâche par tâche. Les étapes
> utilisent des cases (`- [ ]`) pour le suivi.

**Objectif :** en mode modèles, chaque fichier de widget du pack s'ouvre dans
un sous-onglet de l'onglet du deck ; le panneau modal `WidgetWorkbench`
disparaît.

**Architecture :** aucun nouveau type d'onglet du `Workspace`. La session
garde les sous-onglets ouverts dans `UiState`, normalisés par
`UiState::sanitized` (vides hors du mode modèles). L'interface extrait la
logique d'écriture du panneau dans un état partagé (`useWidgetFiles`, une
seule file d'opérations, actif seulement en mode modèles) et l'affiche par
trois composants : barre de sous-onglets, section « Widgets » de la pile,
sous-onglet d'un fichier. `useTemplateWidgets` les assemble pour `App` et
porte leurs raccourcis, pour garder `App.tsx` sous 1000 lignes.

**Pile technique :** Rust (crate `oku-slides-session`, speculoos, nextest),
React 19 + TypeScript (bun test, happy-dom), CodeMirror 6, Fluent (20 langues).

**Spec :** `docs/superpowers/specs/2026-10-03-widgets-onglets-design.md`
(à lire avant toute tâche), et la section « Décisions d'implémentation » de
`docs/superpowers/specs/2026-10-02-creation-widgets-design.md`.

## Contraintes globales

- Tâche backlog parente : **t-394** ; chaque commit finit par `(t-394)` ;
  message en français, Conventional Commits, sans ligne `Co-Authored-By`.
- `git add` dans une commande à part, puis `git commit` (le crochet de
  commit annule toute la commande s'il bloque).
- Avant la première tâche : `cd ui && bun install --frozen-lockfile`
  (le worktree n'a pas de `node_modules`). Aucune dépendance ajoutée :
  `git diff main -- ui/package.json ui/bun.lock Cargo.toml` reste vide.
- TDD : test rouge, puis code, puis vert. Un comportement par test.
- Fichiers : cible 500 lignes, limite 1000. `ui/src/App.tsx` en fait 931 :
  il doit **rester sous 1000** ; les raccourcis des sous-onglets vivent dans
  `useTemplateWidgets`, pas dans `App`.
- i18n : toute clé ajoutée ou retirée l'est dans les **20 langues** de
  `crates/oku-slides-i18n/src/locales/*/ui.ftl` (ar bn de en es fr hi it ja
  ko nl pl pt ru th tr uk vi zh-Hans zh-Hant). Les textes `en` et `fr` sont
  donnés ici ; les autres se traduisent. `ui/src/i18n-keys.test.ts` exige
  que chaque clé `en` soit utilisée et que `fr` ait les mêmes clés ;
  `crates/oku-slides-i18n/src/lib_tests.rs` vérifie les 20.
- L'aperçu SVG reste dans un `<img>` (jamais `<object>`, `<iframe>`,
  `dangerouslySetInnerHTML`) : un SVG produit par un widget ne s'exécute pas.
- Vérifications : `cd ui && bun test` (deux passes : logique, puis
  `.render.test.`), `cd ui && bun run typecheck`, et pour Rust
  `cargo nextest run -p oku-slides-session` (ou `just test`).
- Ne jamais toucher aux fichiers non commités du dépôt principal.

## Fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-session/src/views.rs` | `UiState` : `widgets`, `widget`, `sanitized` ; `Snapshot.pack_widgets` |
| `crates/oku-slides-session/src/lib.rs` | `Request::Ui` normalisé ; `load_document` vide ; snapshot |
| `crates/oku-slides-session/src/templates_mode.rs` | `edit_templates`, `close_templates` vident |
| `crates/oku-slides-session/src/resume.rs` | reprise normalisée |
| `ui/src/api.ts` | `UiState`, `Snapshot.pack_widgets` |
| `ui/src/useViewOptions.ts` | garde aussi `widgets`/`widget` |
| `ui/src/widgetTabs.ts` (+ test) | logique pure des sous-onglets (`OpenWidget`) |
| `ui/src/useWidgetFiles.ts` (+ tests) | état partagé des fichiers, file unique |
| `ui/src/test/widgetApi.ts` | api en mémoire et outils de test |
| `ui/src/TypstEditor.tsx` | `Rewrite` exporté ; commandes et historique |
| `ui/src/WidgetTab.tsx` (+ test) | un fichier : éditeur, aperçu ; Commun |
| `ui/src/PileWidgets.tsx` (+ test) | section « Widgets » de la pile |
| `ui/src/SlidePile.tsx` (+ test) | `children` sous le bouton d'ajout |
| `ui/src/TemplateTabs.tsx` (+ test) | barre des sous-onglets |
| `ui/src/useTemplateWidgets.ts` (+ test) | assemble tout pour `App`, raccourcis, entrée, fin |
| `ui/src/WidgetPane.tsx` (+ test) | les sous-onglets montés, seul l'actif visible |
| `ui/src/icons.tsx` | `Icon.add` |
| `ui/src/App.tsx` | intégration ; retrait du panneau |
| `ui/src/Stage.tsx` (+ test) | étiquette d'un widget du pack : bouton |
| `ui/src/styles.css` | styles ; retrait de `.workbench*` ; `.unsaved-dot` partagé |
| retirés | `WidgetWorkbench.tsx`, `WidgetWorkbench*.render.test.tsx`, `test/workbench.ts` |

## Correspondance des tests retirés

Chaque test de `WidgetWorkbench*.render.test.tsx` a un équivalent avant son
retrait (tâche 8). `R` = `WidgetWorkbench.render`, `W` = `.writes`, `T` = `.trials`.

| Test retiré | Nouveau test |
|---|---|
| R:27 liste, `_commun` en tête, pastilles | T6 `PileWidgets` n°1 |
| R:36 frappe en attente survit à un nouveau rendu du parent | T3 n°15 |
| R:66 diagnostics de `_commun` sur le widget qui l'importe, avec leur fichier | T5 n°2 |
| R:86 fermer pendant le délai envoie la frappe | T3 n°16 (démontage) |
| R:95 changer de fichier pendant le délai | obsolète : chaque fichier a son sous-onglet ; couvert par T3 n°3 (deux fichiers) |
| R:105 supprimer pendant le délai | T3 n°8 |
| R:118 réponse de source après une frappe | T5 n°4 |
| R:142 réponse lente, sans écriture partie, après une frappe | T5 n°5 |
| R:154 pack changé ailleurs : source relue | T5 n°6 |
| R:165 Nouveau, nom pris refusé | T6 n°4 |
| R:184 supprimer attend l'écriture partie | T3 n°9 |
| R:202 écriture refusée : erreur dite, texte gardé après relecture | T3 n°5 + T5 n°7 |
| R:216 aperçu ancien et lent | T5 n°9 |
| R:236 frappe non envoyée pas écrasée par une relecture | T5 n°8 |
| R:247 deux écritures, la seconde attend la première | T3 n°4 |
| R:271 widget d'élément cadré | T5 n°10 |
| W:15 un relevé des statuts et un aperçu par écriture | T5 n°11 |
| W:31 statuts de l'écriture (essai refusé) | T3 n°6 |
| W:47 création attend la suppression | T3 n°10 |
| W:66 écriture attend le renommage, dans l'ordre | T3 n°12 |
| W:87 deux suppressions de suite | T3 n°11 |
| W:112 refus marque le fichier, jusqu'à la prochaine réussie | T3 n°5 |
| W:141 essai refusé : marqué, texte gardé à la relecture | T3 n°6 + T5 n°7 |
| W:160 Nouveau refusé : pas de fichier fantôme | T3 n°13 |
| W:176 supprimer efface la marque ; recréer n'en hérite pas | T3 n°14 |
| W:196 avant le premier statut, Nouveau attend | T6 n°5 |
| W:211 `_commun` : créé s'il manque, pas en double, pas par renommage | T6 n°6 |
| W:252 frappe en attente à la fin du renommage | T3 n°17 |
| W:261 frappe partie pendant le renommage | T3 n°18 |
| W:271 sans frappe : nom déclaré renommé sans écriture | T3 n°19 |
| W:283 renommage refusé : frappe sous l'ancien nom | T3 n°20 |
| T:16 essai refusé : relecture sans ses diagnostics | T3 n°22 |
| T:56 relecture garde les diagnostics du rendu | T3 n°21 |
| T:65 pack changé de nouveau : ils ne valent plus | T3 n°21 |
| Échap ferme le panneau ; bouton Fermer | obsolètes : plus de panneau |

---

### Tâche 1 : Session — sous-onglets gardés et normalisés dans `UiState`

**Fichiers :**
- Modifier : `crates/oku-slides-session/src/views.rs` (`UiState`, `Default`, `sanitized`, doc)
- Modifier : `crates/oku-slides-session/src/lib.rs:541` (`Request::Ui`), `:610` (`load_document`)
- Modifier : `crates/oku-slides-session/src/templates_mode.rs:126` (`edit_templates`), `:279` (`close_templates`)
- Modifier : `crates/oku-slides-session/src/resume.rs:606` (`restore`)
- Modifier : `ui/src/api.ts` (`UiState`), `ui/src/useViewOptions.ts`, `ui/src/App.tsx:224`
- Tests : `crates/oku-slides-session/src/views_tests.rs` (créer, `#[path]` dans `views.rs`),
  `resume_tests.rs`, `templates_mode_tests.rs`, `crates/oku-slides-session/tests/session.rs:408-413`,
  `ui/src/useViewOptions.render.test.tsx`

**Interfaces :**
- Produit (Rust) :

```rust
/// Nombre maximal de sous-onglets gardés.
pub const MAX_WIDGET_TABS: usize = 64;
impl UiState {
    /// L'état admis : sous-onglets vidés hors du mode modèles, sinon noms
    /// valides, sans doublon, au plus `MAX_WIDGET_TABS`, actif parmi eux.
    pub fn sanitized(self, templates: bool) -> Self;
}
// UiState { …, widgets: Vec<String>, widget: Option<String> }
```

- Produit (TS) : `UiState { …; widgets: string[]; widget: string | null }` ;
  `useViewOptions(kept, current, save, tabs: Pick<UiState, "widgets" | "widget">)`.

- [ ] **Étape 1 : tests rouges (Rust)**

`views_tests.rs` :

```rust
use speculoos::prelude::*;

use super::{MAX_WIDGET_TABS, UiState};

fn tabs(widgets: &[&str], widget: Option<&str>) -> UiState {
    UiState {
        widgets: widgets.iter().map(|s| (*s).to_owned()).collect(),
        widget: widget.map(str::to_owned),
        ..UiState::default()
    }
}

#[test]
fn sanitized_outside_templates_clears_the_tabs() {
    let ui = tabs(&["kpi"], Some("kpi")).sanitized(false);
    assert_that!(ui.widgets).is_empty();
    assert_that!(ui.widget).is_none();
}

#[test]
fn sanitized_in_templates_drops_invalid_and_duplicate_names() {
    let ui = tabs(&["kpi", "../x", "Kpi", "kpi", "_commun"], Some("kpi")).sanitized(true);
    assert_that!(ui.widgets).is_equal_to(vec!["kpi".to_owned(), "_commun".to_owned()]);
}

#[test]
fn sanitized_in_templates_caps_the_list() {
    let names: Vec<String> = (0..MAX_WIDGET_TABS + 10).map(|i| format!("w{i}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    assert_that!(tabs(&refs, None).sanitized(true).widgets).has_length(MAX_WIDGET_TABS);
}

#[test]
fn sanitized_with_an_active_tab_not_open_drops_it() {
    assert_that!(tabs(&["kpi"], Some("quote")).sanitized(true).widget).is_none();
}
```

`templates_mode_tests.rs` (le module de test est dans le crate : `s.ui` est lisible) :

```rust
fn open_tabs(s: &mut crate::Session) {
    let ui = crate::views::UiState {
        widgets: vec!["_commun".into()],
        widget: Some("_commun".into()),
        ..crate::views::UiState::default()
    };
    s.handle(crate::Request::Ui(ui)).unwrap();
}

/// Sortir du mode modèles : le deck ne garde aucun sous-onglet.
#[test]
fn close_templates_with_open_widget_tabs_clears_them() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    s.edit_templates(DEFAULT_PACK).unwrap();
    open_tabs(&mut s);
    s.close_templates().unwrap();
    assert_that!(s.ui.widgets).is_empty();
    assert_that!(s.ui.widget).is_none();
}

/// Passer à un autre pack : les sous-onglets de l'ancien ne le suivent pas.
#[test]
fn edit_templates_of_another_pack_clears_the_widget_tabs() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    s.edit_templates(DEFAULT_PACK).unwrap();
    open_tabs(&mut s);
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(s.ui.widgets).is_empty();
}

/// En mode deck, l'interface ne peut pas poser de sous-onglets.
#[test]
fn ui_request_in_deck_mode_keeps_no_widget_tabs() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    open_tabs(&mut s);
    assert_that!(s.ui.widgets).is_empty();
}
```

(Le second test rouvre le même pack : `edit_templates` en mode modèles
passe par la branche « déjà en édition », le cas à couvrir. Si
`session_on` ne donne pas de `Session` avec `handle`, utiliser l'aide de
test qui l'expose dans `test_support.rs`.)

`resume_tests.rs` :
- dans `write_state_then_read_state_round_trips_mode_and_ui`, le littéral
  gagne `widgets: vec!["_commun".into(), "kpi".into()], widget: Some("kpi".into())` ;
- dans `read_state_without_ghosts_or_columns_returns_defaults` (renommer en
  `read_state_without_ghosts_columns_or_tabs_returns_defaults`), l'attendu
  gagne `widgets: Vec::new(), widget: None` ;
`tests/session.rs` :
- l. 411-412 : l'attendu devient
  `json!({"slide": "s-1", "grid": false, "ghosts": true, "pile": null, "notes": null, "widgets": [], "widget": null})` ;
- deux tests de reprise, à côté de `resume_with_unreadable_templates_returns_to_the_deck_without_loss`
  (l. ~728), avec ses aides `data_dir`, `resumed`, `req`, `snapshot_json` :

```rust
/// t-394 : reprise en mode modèles, les sous-onglets de widgets reviennent.
#[test]
fn resume_in_templates_mode_returns_the_widget_tabs() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    a.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "widgets": ["_commun"], "widget": "_commun"})))
        .unwrap();
    drop(a);
    let mut b = resumed(&data);
    let ui = snapshot_json(&mut b)["ui"].clone();
    assert_that!((ui["widgets"].clone(), ui["widget"].clone())).is_equal_to((json!(["_commun"]), json!("_commun")));
}

/// t-394 : modèles illisibles, la reprise revient au deck sans sous-onglets.
#[test]
fn resume_with_unreadable_templates_returns_no_widget_tabs() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    a.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "widgets": ["_commun"], "widget": "_commun"})))
        .unwrap();
    drop(a);
    std::fs::write(data.join("resume/templates/document.json"), b"pas du json").unwrap();
    let mut b = resumed(&data);
    assert_that!(snapshot_json(&mut b)["ui"]["widgets"].clone()).is_equal_to(json!([]));
}
```

(Si l'écriture de l'état de reprise est différée, reprendre ce que fait le
test voisin entre `handle` et `drop` ; il n'appelle rien de plus.)

- [ ] **Étape 2 : vérifier l'échec**

Run : `cargo nextest run -p oku-slides-session`
Attendu : erreur de compilation (champs et `sanitized` inconnus).

- [ ] **Étape 3 : implémenter (Rust)**

`views.rs` : doc de `UiState` → « Ce que l'interface fait garder à l'hôte :
slide courante, grille, fantômes, largeurs des colonnes, sous-onglets de
widgets. » ; champs après `notes` :

```rust
    /// Sous-onglets de widgets ouverts en mode modèles, dans l'ordre d'ouverture.
    #[serde(default)]
    pub widgets: Vec<String>,
    /// Sous-onglet actif ; `None` : « Modèles ».
    #[serde(default)]
    pub widget: Option<String>,
```

`Default` : `widgets: Vec::new(), widget: None`. Puis :

```rust
/// Nombre maximal de sous-onglets gardés.
pub const MAX_WIDGET_TABS: usize = 64;

impl UiState {
    /// L'état admis : sous-onglets vidés hors du mode modèles, sinon noms
    /// valides, sans doublon, au plus `MAX_WIDGET_TABS`, actif parmi eux.
    pub fn sanitized(mut self, templates: bool) -> Self {
        if !templates {
            self.widgets.clear();
            self.widget = None;
            return self;
        }
        let mut seen = std::collections::BTreeSet::new();
        self.widgets.retain(|n| WidgetFile::parse(n).is_some() && seen.insert(n.clone()));
        self.widgets.truncate(MAX_WIDGET_TABS);
        if self.widget.as_ref().is_some_and(|w| !self.widgets.contains(w)) {
            self.widget = None;
        }
        self
    }
}

#[cfg(test)]
#[path = "views_tests.rs"]
mod views_tests;
```

(`WidgetFile` : `oku_slides_core::WidgetFile`, vérifier le chemin
d'import réexporté par `oku_slides_core`.) Les noms restent des `String` :
un nom invalide est écarté sans rendre `state.json` illisible.

`lib.rs:541` :

```rust
            Request::Ui(ui) => self.ui = ui.sanitized(matches!(self.mode, Mode::Templates { .. })),
```

`lib.rs:610` (`load_document`), `templates_mode.rs:126` (`edit_templates`)
et `templates_mode.rs:279` (`close_templates`), après `self.ui.slide = None;` :

```rust
        self.ui = std::mem::take(&mut self.ui).sanitized(false);
```

(`sanitized(false)` vide les deux champs ; à l'entrée en mode modèles,
l'interface pose ensuite ses sous-onglets par `Request::Ui`.)

`resume.rs:606` :

```rust
        self.ui = state.ui.sanitized(matches!(self.mode, Mode::Templates { .. }));
```

- [ ] **Étape 4 : tests rouges (TS)**

`useViewOptions.render.test.tsx` :
- `kept` (l. 23) gagne `widgets: [], widget: null` ; la sonde (l. 37) appelle
  `useViewOptions(ui, current, save, tabs)` avec `tabs` en prop
  (défaut `{ widgets: [], widget: null }`) ; l'attendu exact de la l. 111
  gagne `widgets: [], widget: null` ;
- nouveaux tests :

```tsx
test("useViewOptions sauve les sous-onglets passés avec le reste", async () => {
  const { out, render } = await mount();
  await render(kept, "s1", { widgets: ["kpi"], widget: "kpi" });
  expect(out.kept.at(-1)).toMatchObject({ widgets: ["kpi"], widget: "kpi" });
});

test("useViewOptions avec une nouvelle liste de même contenu ne sauve pas de nouveau", async () => {
  const { out, render } = await mount();
  await render(kept, "s1", { widgets: ["kpi"], widget: null });
  const n = out.saved.length;
  await render(kept, "s1", { widgets: ["kpi"], widget: null });
  expect(out.saved.length).toBe(n);
});
```

(`render` gagne un troisième paramètre `tabs`, passé à la sonde.)

- [ ] **Étape 5 : implémenter (TS)**

`api.ts`, `UiState` :

```ts
  /** Sous-onglets de widgets ouverts en mode modèles, dans l'ordre d'ouverture. */
  widgets: string[];
  /** Sous-onglet actif ; `null` : « Modèles ». */
  widget: string | null;
```

`useViewOptions.ts` : quatrième paramètre
`tabs: Pick<UiState, "widgets" | "widget">` (doc : « Sous-onglets à garder
avec le reste »), sauvegarde `save({ slide: current, grid, ghosts, ...columns, ...tabs })`,
dépendances de l'effet : `[restored, current, grid, ghosts, columns, tabs.widgets.join("\n"), tabs.widget]`.

`App.tsx:224` : `useViewOptions(snap?.ui ?? null, current, api.ui, NO_WIDGET_TABS)`
avec, en tête de fichier, `const NO_WIDGET_TABS = { widgets: [], widget: null };`
— remplacé à la tâche 8 par `widgets.kept`.

- [ ] **Étape 6 : vérifier le vert**

Run : `cargo nextest run -p oku-slides-session && cd ui && bun test && bun run typecheck`
Attendu : tout passe.

- [ ] **Étape 7 : commit**

```bash
git add crates/oku-slides-session ui/src/api.ts ui/src/useViewOptions.ts ui/src/useViewOptions.render.test.tsx ui/src/App.tsx
```
```bash
git commit -m "feat(session) : sous-onglets de widgets gardés et normalisés dans l'état d'interface (t-394)"
```

---

### Tâche 2 : Sous-onglets — logique pure

**Fichiers :**
- Créer : `ui/src/widgetTabs.ts`
- Test : `ui/src/widgetTabs.test.ts`

**Interfaces :**
- Produit :

```ts
export interface OpenWidget { id: number; name: string }
export interface WidgetTabs { tabs: OpenWidget[]; active: number | null; next: number }
export const NO_TABS: WidgetTabs;
export function withRestored(s: WidgetTabs, names: string[], active: string | null): WidgetTabs;
export function opened(s: WidgetTabs, name: string): WidgetTabs;
export function closed(s: WidgetTabs, id: number): WidgetTabs;
export function activated(s: WidgetTabs, id: number | null): WidgetTabs;
export function renamed(s: WidgetTabs, from: string, to: string): WidgetTabs;
export function pruned(s: WidgetTabs, names: ReadonlySet<string>): WidgetTabs;
export function activeName(s: WidgetTabs): string | null;
export function kept(s: WidgetTabs): { widgets: string[]; widget: string | null };
```

- [ ] **Étape 1 : tests rouges** (`widgetTabs.test.ts`)

```ts
import { describe, expect, test } from "bun:test";
import { NO_TABS, activated, activeName, closed, kept, opened, pruned, renamed, withRestored } from "./widgetTabs";

const names = (s: { tabs: { name: string }[] }) => s.tabs.map((t) => t.name);

describe("widgetTabs", () => {
  test("opened ajoute en fin et active", () => {
    const s = opened(opened(NO_TABS, "kpi"), "quote");
    expect(names(s)).toEqual(["kpi", "quote"]);
    expect(activeName(s)).toBe("quote");
  });

  test("opened d'un nom déjà ouvert l'active sans doublon", () => {
    const s = opened(opened(opened(NO_TABS, "kpi"), "quote"), "kpi");
    expect(names(s)).toEqual(["kpi", "quote"]);
    expect(activeName(s)).toBe("kpi");
  });

  test("closed de l'actif au milieu active son voisin de gauche", () => {
    let s = opened(opened(opened(NO_TABS, "a"), "b"), "c");
    s = closed(activated(s, s.tabs[1].id), s.tabs[1].id);
    expect(names(s)).toEqual(["a", "c"]);
    expect(activeName(s)).toBe("a");
  });

  // Décision : à gauche du premier sous-onglet, il y a « Modèles ».
  test("closed du premier actif revient à Modèles", () => {
    let s = opened(opened(NO_TABS, "a"), "b");
    s = closed(activated(s, s.tabs[0].id), s.tabs[0].id);
    expect(s.active).toBeNull();
  });

  test("closed d'un autre que l'actif garde l'actif", () => {
    let s = opened(opened(NO_TABS, "a"), "b");
    s = closed(s, s.tabs[0].id);
    expect(activeName(s)).toBe("b");
  });

  test("renamed garde l'id et la place", () => {
    const s = opened(opened(NO_TABS, "a"), "b");
    const r = renamed(s, "a", "z");
    expect(names(r)).toEqual(["z", "b"]);
    expect(r.tabs[0].id).toBe(s.tabs[0].id);
  });

  test("pruned retire les noms disparus ; actif disparu → Modèles", () => {
    const s = pruned(opened(opened(NO_TABS, "a"), "b"), new Set(["a"]));
    expect(names(s)).toEqual(["a"]);
    expect(s.active).toBeNull();
  });

  test("pruned sans changement rend le même objet", () => {
    const s = opened(NO_TABS, "a");
    expect(pruned(s, new Set(["a"]))).toBe(s);
  });

  test("withRestored sur rien puis kept : aller-retour", () => {
    expect(kept(withRestored(NO_TABS, ["_commun", "kpi"], "kpi"))).toEqual({ widgets: ["_commun", "kpi"], widget: "kpi" });
  });

  test("withRestored garde un onglet déjà ouvert et son activité", () => {
    const s = withRestored(opened(NO_TABS, "quote"), ["kpi"], "kpi");
    expect(names(s)).toEqual(["quote", "kpi"]);
    expect(activeName(s)).toBe("quote");
  });

  test("withRestored dédoublonne et ignore un actif absent", () => {
    const s = withRestored(NO_TABS, ["kpi", "kpi"], "autre");
    expect(names(s)).toEqual(["kpi"]);
    expect(s.active).toBeNull();
  });

  test("après withRestored, opened donne un id neuf", () => {
    const s = opened(withRestored(NO_TABS, ["a", "b"], null), "c");
    expect(new Set(s.tabs.map((t) => t.id)).size).toBe(3);
  });
});
```

- [ ] **Étape 2 : vérifier l'échec** — Run : `cd ui && bun test src/widgetTabs.test.ts` ; attendu : module introuvable.

- [ ] **Étape 3 : implémenter** (`widgetTabs.ts`)

```ts
// Sous-onglets de widgets du mode modèles : logique pure.

/** Un sous-onglet ouvert ; `id` stable : un renommage ne remonte pas l'éditeur. */
export interface OpenWidget {
  id: number;
  name: string;
}

/** Sous-onglets ouverts, dans l'ordre d'ouverture ; `active` : `null` pour « Modèles ». */
export interface WidgetTabs {
  tabs: OpenWidget[];
  active: number | null;
  next: number;
}

/** Aucun sous-onglet : « Modèles » seul. */
export const NO_TABS: WidgetTabs = { tabs: [], active: null, next: 1 };

/** Ajoute les sous-onglets gardés absents ; un actif déjà choisi reste. */
export function withRestored(s: WidgetTabs, names: string[], active: string | null): WidgetTabs {
  let out = s;
  for (const name of names) {
    if (!out.tabs.some((t) => t.name === name)) {
      out = { ...out, tabs: [...out.tabs, { id: out.next, name }], next: out.next + 1 };
    }
  }
  if (out.active === null) out = { ...out, active: out.tabs.find((t) => t.name === active)?.id ?? null };
  return out;
}

/** Active `name`, ouvert en fin de barre s'il ne l'est pas. */
export function opened(s: WidgetTabs, name: string): WidgetTabs {
  const found = s.tabs.find((t) => t.name === name);
  if (found) return activated(s, found.id);
  return { tabs: [...s.tabs, { id: s.next, name }], active: s.next, next: s.next + 1 };
}

/** Ferme `id` ; s'il était actif, son voisin de gauche le devient (aucun : « Modèles »). */
export function closed(s: WidgetTabs, id: number): WidgetTabs {
  const i = s.tabs.findIndex((t) => t.id === id);
  if (i < 0) return s;
  const tabs = s.tabs.filter((t) => t.id !== id);
  const active = s.active !== id ? s.active : (tabs[i - 1]?.id ?? null);
  return { ...s, tabs, active };
}

/** Active `id` ; `null` : « Modèles ». */
export function activated(s: WidgetTabs, id: number | null): WidgetTabs {
  return s.active === id ? s : { ...s, active: id };
}

/** Le sous-onglet de `from` porte désormais `to`, même id, même place. */
export function renamed(s: WidgetTabs, from: string, to: string): WidgetTabs {
  if (!s.tabs.some((t) => t.name === from)) return s;
  return { ...s, tabs: s.tabs.map((t) => (t.name === from ? { ...t, name: to } : t)) };
}

/** Retire les sous-onglets dont le fichier n'existe plus ; actif retiré : « Modèles ». */
export function pruned(s: WidgetTabs, names: ReadonlySet<string>): WidgetTabs {
  const tabs = s.tabs.filter((t) => names.has(t.name));
  if (tabs.length === s.tabs.length) return s;
  const active = tabs.some((t) => t.id === s.active) ? s.active : null;
  return { ...s, tabs, active };
}

/** Nom du sous-onglet actif ; `null` : « Modèles ». */
export function activeName(s: WidgetTabs): string | null {
  return s.tabs.find((t) => t.id === s.active)?.name ?? null;
}

/** Ce que l'hôte garde pour la reprise. */
export function kept(s: WidgetTabs): { widgets: string[]; widget: string | null } {
  return { widgets: s.tabs.map((t) => t.name), widget: activeName(s) };
}
```

- [ ] **Étape 4 : vérifier le vert** — Run : `cd ui && bun test src/widgetTabs.test.ts` ; attendu : PASS.

- [ ] **Étape 5 : commit**

```bash
git add ui/src/widgetTabs.ts ui/src/widgetTabs.test.ts
```
```bash
git commit -m "feat(ui) : logique des sous-onglets de widgets (t-394)"
```

---

### Tâche 3 : État partagé des fichiers — `useWidgetFiles`

Extraction de la logique de `ui/src/WidgetWorkbench.tsx` (statuts, essai,
file, écriture, renommage, suppression, création) en tables par fichier.
`WidgetWorkbench` reste en place, inchangé, jusqu'à la tâche 8.

**Fichiers :**
- Créer : `ui/src/useWidgetFiles.ts`
- Créer : `ui/src/test/widgetApi.ts` (déplacer `memory`, `stub`, `deferred`, `wait`
  de `test/workbench.ts`, l. 62-141 ; `workbench.ts` les réexporte :
  `export { deferred, memory, stub, wait } from "./widgetApi";`)
- Modifier : `ui/src/TypstEditor.tsx` (exporter `Rewrite`, utilisé par sa prop `rewrite`)
- Test : `ui/src/useWidgetFiles.render.test.tsx`

**Interfaces :**
- Consomme : `widgetList.ts` (`ordered`, `merged`, `withTrial`, `declaredAs`, `skeleton`), `debounce.ts`, `TabApi`.
- Produit :

```ts
// TypstEditor.tsx
/** Remplacement à faire dans le texte courant (nom déclaré après un renommage). */
export interface Rewrite { find: string; replace: string; silent?: boolean }

// useWidgetFiles.ts
export interface WidgetFiles {
  /** Statuts du pack, `_commun` en tête ; `null` hors mode modèles ou pas encore lus. */
  statuses: WidgetStatus[] | null;
  /** La `revision` pour laquelle `statuses` a été lu ; `null` sans statuts. */
  statusesFor: string | null;
  /** Fichiers dont le texte tapé n'est pas enregistré. */
  unsaved: ReadonlySet<string>;
  /** Par fichier : le texte dont les statuts affichés sont le reflet. */
  checked: ReadonlyMap<string, string>;
  /** Par fichier : nombre d'écritures enregistrées (l'aperçu se redemande). */
  writes: ReadonlyMap<string, number>;
  /** Par fichier (nouveau nom) : remplacement à faire après un renommage. */
  rewrites: ReadonlyMap<string, Rewrite>;
  /** Frappe dans `name`, écrite PREVIEW_DELAY_MS après la dernière. */
  edit(name: string, text: string): void;
  /** Compteur des frappes et écritures de `name`, pour dater une lecture. */
  edits(name: string): number;
  /** Une source de `name` lue à `asked` est périmée : ne pas l'afficher. */
  stale(name: string, asked: number, source: string): boolean;
  /** La source de `name` vient d'être affichée. */
  loaded(name: string, source: string): void;
  /** Envoie la frappe en attente de `name`, ou de tous les fichiers. */
  flush(name?: string): void;
  /** Toutes les frappes envoyées et la file vide. */
  settle(): Promise<void>;
  /** Crée `name` avec son squelette ; vrai si créé. */
  create(name: string): Promise<boolean>;
  /** Renomme ; vrai si fait. */
  rename(from: string, to: string): Promise<boolean>;
  /** Supprime ; vrai si fait. */
  remove(name: string): Promise<boolean>;
  /** Dit une erreur (toast du parent). */
  fail(e: unknown): void;
}

/**
 * `pack` : le pack édité (`null` hors mode modèles : rien n'est lu). Un
 * autre pack remet tout à zéro, frappes en attente annulées.
 * `on.renamed` est appelé dans la file, avant les nouveaux statuts.
 */
export function useWidgetFiles(
  api: TabApi,
  pack: string | null,
  revision: string,
  on: { changed(): void; error(message: string): void; renamed(from: string, to: string): void },
): WidgetFiles;
```

- [ ] **Étape 1 : déplacer les outils de test** (voir Fichiers), puis
  `cd ui && bun test .render.test.` — attendu : PASS (rien ne change).

- [ ] **Étape 2 : tests rouges** (`useWidgetFiles.render.test.tsx`)

```tsx
// État partagé des fichiers de widgets : statuts, frappes, file des opérations.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { TabApi, WidgetChange } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { PREVIEW_DELAY_MS } = await import("./debounce");
const { skeleton } = await import("./widgetList");
const { useWidgetFiles } = await import("./useWidgetFiles");
const { deferred, memory, stub, wait } = await import("./test/widgetApi");

type Files = ReturnType<typeof useWidgetFiles>;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

interface Out { files: Files | null; errors: string[]; changed: string[]; renamed: string[] }

/** Monte le hook ; `render` le relance (pack, revision, rappels). */
async function mount(fake: TabApi) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: Out = { files: null, errors: [], changed: [], renamed: [] };
  function Probe({ pack, rev, tag }: { pack: string | null; rev: string; tag: string }) {
    out.files = useWidgetFiles(fake, pack, rev, {
      changed: () => out.changed.push(tag),
      error: (m) => out.errors.push(m),
      renamed: (f, t) => out.renamed.push(`${f}>${t}`),
    });
    return null;
  }
  const render = (pack: string | null = "p", rev = "r", tag = "a") =>
    act(async () => root.render(createElement(Probe, { pack, rev, tag })));
  await render();
  return { out, render, unmount: () => act(async () => root.unmount()) };
}

const typed = async (out: Out, name: string, text: string) => {
  await act(async () => out.files!.edit(name, text));
};
const later = () => wait(PREVIEW_DELAY_MS + 50);
const decl = (n: string, b: string) => `#let widget = widget("${n}", ${b})`;

/** `key` de l'api retenu jusqu'à `release`. */
function held(fake: TabApi, key: "setWidget" | "removeWidget" | "renameWidget") {
  const gate = deferred<void>();
  const real = (fake as unknown as Record<string, (...a: unknown[]) => Promise<unknown>>)[key];
  stub(fake, key, async (...a: never[]) => {
    await gate.promise;
    return real(...a);
  });
  return { release: () => act(async () => gate.resolve()) };
}

describe("useWidgetFiles, statuts", () => {
  // 1
  test("lit les statuts, _commun en tête, avec leur revision", async () => {
    const { fake } = memory({ kpi: "k", _commun: "c" });
    const { out } = await mount(fake);
    expect(out.files?.statuses?.map((s) => s.name)).toEqual(["_commun", "kpi"]);
    expect(out.files?.statusesFor).toBe("r");
  });

  // 2
  test("hors mode modèles (pack null) : rien n'est lu", async () => {
    const { fake, calls } = memory({ kpi: "k" });
    const { out, render } = await mount(fake);
    calls.length = 0;
    await render(null);
    expect(out.files?.statuses).toBeNull();
    expect(calls).toEqual([]);
  });

  // 2b
  test("une relecture ancienne arrivée après une plus récente est ignorée", async () => {
    const { fake } = memory({ kpi: "k" });
    const slow = deferred<unknown>();
    let first = true;
    const status = fake.widgetStatus;
    stub(fake, "widgetStatus", () => {
      if (!first) return status();
      first = false;
      return slow.promise;
    });
    const { out, render } = await mount(fake);
    await render("p", "r2");
    await act(async () => slow.resolve([]));
    expect(out.files?.statuses?.map((s) => s.name)).toEqual(["kpi"]);
    expect(out.files?.statusesFor).toBe("r2");
  });

  // 2c
  test("un autre pack : frappe en attente annulée, tables remises à zéro", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out, render } = await mount(fake);
    await typed(out, "kpi", "k2");
    await render("autre");
    await later();
    expect(log).toEqual([]);
    expect(out.files!.checked.size).toBe(0);
  });
});

describe("useWidgetFiles, frappes et file", () => {
  // 3
  test("frappes dans deux fichiers : chacune écrite, dans l'ordre d'envoi", async () => {
    const { fake, log } = memory({ a: "a", b: "b" });
    const { out } = await mount(fake);
    const h = held(fake, "setWidget");
    await typed(out, "a", "a1");
    await typed(out, "b", "b1");
    await act(async () => out.files!.flush());
    await wait(10);
    expect(log).toEqual([]);
    await h.release();
    await wait(10);
    expect(log).toEqual(["set a:a1", "set b:b1"]);
  });

  // 4
  test("deux écritures du même fichier : la seconde attend la première, même lente", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const h = held(fake, "setWidget");
    await typed(out, "kpi", "k1");
    await later();
    await typed(out, "kpi", "k2");
    await later();
    expect(log).toEqual([]);
    await h.release();
    await wait(10);
    expect(log).toEqual(["set kpi:k1", "set kpi:k2"]);
  });

  // 5
  test("une écriture refusée marque le fichier et dit l'erreur ; la suivante réussie efface la marque", async () => {
    const { fake } = memory({ kpi: "k" });
    const set = fake.setWidget;
    stub(fake, "setWidget", async () => { throw new Error("non"); });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    expect(out.files!.unsaved.has("kpi")).toBe(true);
    expect(out.errors).toEqual(["non"]);
    stub(fake, "setWidget", set);
    await typed(out, "kpi", "k3");
    await later();
    expect(out.files!.unsaved.has("kpi")).toBe(false);
  });

  // 6
  test("essai refusé (saved: false) : non enregistré, statuts de l'essai, pas de changed", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "setWidget", async (): Promise<WidgetChange> => ({
      status: { name: "kpi", schema: null, diagnostics: [{ file: "kpi", line: 1, column: 1, error: true, message: "rendu" }] },
      broken: [],
      saved: false,
    }));
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    expect(out.files!.unsaved.has("kpi")).toBe(true);
    expect(out.files!.statuses?.[0].diagnostics.map((d) => d.message)).toEqual(["rendu"]);
    expect(out.files!.checked.get("kpi")).toBe("k2");
    expect(out.changed).toEqual([]);
  });

  // 7
  test("une écriture enregistrée compte dans writes et appelle changed", async () => {
    const { fake } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    expect(out.files!.writes.get("kpi")).toBe(1);
    expect(out.changed).toEqual(["a"]);
  });

  // 8
  test("remove pendant le délai : la frappe en attente ne recrée pas le fichier", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await act(async () => { await out.files!.remove("kpi"); });
    await later();
    expect(log).toEqual(["remove kpi"]);
  });

  // 9
  test("remove attend l'écriture déjà partie", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const h = held(fake, "setWidget");
    await typed(out, "kpi", "k2");
    await later();
    let removing: Promise<boolean> | null = null;
    await act(async () => { removing = out.files!.remove("kpi"); });
    await wait(10);
    expect(log).toEqual([]);
    await h.release();
    await act(async () => { await removing; });
    expect(log).toEqual(["set kpi:k2", "remove kpi"]);
  });

  // 10
  test("create attend la suppression en cours", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const h = held(fake, "removeWidget");
    let created: Promise<boolean> | null = null;
    await act(async () => { void out.files!.remove("kpi"); created = out.files!.create("abc"); });
    await wait(10);
    expect(log).toEqual([]);
    await h.release();
    await act(async () => { await created; });
    expect(log).toEqual(["remove kpi", `set abc:${skeleton("abc")}`]);
  });

  // 11
  test("deux suppressions de suite : la seconde attend la première", async () => {
    const { fake, log } = memory({ a: "a", b: "b" });
    const { out } = await mount(fake);
    const h = held(fake, "removeWidget");
    await act(async () => { void out.files!.remove("a"); void out.files!.remove("b"); });
    await wait(10);
    expect(log).toEqual([]);
    await h.release();
    await wait(10);
    expect(log).toEqual(["remove a", "remove b"]);
  });

  // 12
  test("une écriture d'un autre fichier attend le renommage en cours", async () => {
    const { fake, log } = memory({ kpi: decl("kpi", "a"), quote: "q" });
    const { out } = await mount(fake);
    const h = held(fake, "renameWidget");
    await act(async () => { void out.files!.rename("kpi", "kpi2"); });
    await typed(out, "quote", "q2");
    await later();
    expect(log).toEqual([]);
    await h.release();
    await wait(10);
    expect(log).toEqual(["rename kpi>kpi2", "set quote:q2"]);
  });

  // 13
  test("create refusé (saved: false) : pas de fichier fantôme, l'erreur est dite, faux", async () => {
    const { fake } = memory({});
    stub(fake, "setWidget", async (): Promise<WidgetChange> => ({
      status: { name: "abc", schema: null, diagnostics: [{ file: "abc", line: 1, column: 1, error: true, message: "rendu" }] },
      broken: [],
      saved: false,
    }));
    const { out } = await mount(fake);
    let ok = true;
    await act(async () => { ok = await out.files!.create("abc"); });
    expect(ok).toBe(false);
    expect(out.errors).toEqual(["rendu"]);
    expect(out.files!.statuses?.some((s) => s.name === "abc")).toBe(false);
  });

  // 14
  test("supprimer un fichier non enregistré efface sa marque ; le recréer n'en hérite pas", async () => {
    const { fake } = memory({ kpi: "k" });
    const set = fake.setWidget;
    stub(fake, "setWidget", async () => { throw new Error("non"); });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    stub(fake, "setWidget", set);
    await act(async () => { await out.files!.remove("kpi"); });
    expect(out.files!.unsaved.has("kpi")).toBe(false);
    await act(async () => { await out.files!.create("kpi"); });
    expect(out.files!.unsaved.has("kpi")).toBe(false);
  });

  // 15
  test("une frappe en attente survit à un nouveau rendu du parent : seuls les nouveaux rappels servent", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out, render } = await mount(fake);
    await typed(out, "kpi", "k2");
    await render("p", "r", "b");
    await later();
    expect(log).toEqual(["set kpi:k2"]);
    expect(out.changed).toEqual(["b"]);
  });

  // 16
  test("démontage pendant le délai : la frappe part quand même", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out, unmount } = await mount(fake);
    await typed(out, "kpi", "k2");
    await unmount();
    mounted = null;
    await wait(10);
    expect(log).toEqual(["set kpi:k2"]);
  });

  // 16b
  test("settle attend la frappe en attente et l'écriture partie", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const h = held(fake, "setWidget");
    await typed(out, "kpi", "k2");
    let done = false;
    const settling = out.files!.settle().then(() => { done = true; });
    await wait(10);
    expect(done).toBe(false);
    await h.release();
    await act(async () => settling);
    expect(log).toEqual(["set kpi:k2"]);
  });
});

describe("useWidgetFiles, renommage", () => {
  /** kpi lu, son renommage en kpi2 lancé et retenu. */
  async function renaming() {
    const { fake, log } = memory({ kpi: decl("kpi", "a") });
    const r = await mount(fake);
    await act(async () => r.out.files!.loaded("kpi", decl("kpi", "a")));
    const h = held(fake, "renameWidget");
    let done: Promise<boolean> | null = null;
    await act(async () => { done = r.out.files!.rename("kpi", "kpi2"); });
    const release = async () => {
      await h.release();
      await act(async () => { await done; });
      await later();
    };
    return { ...r, fake, log, release };
  }

  // 17
  test("frappe en attente à la fin du renommage : écrite sous le nouveau nom, remplacement non silencieux", async () => {
    const { out, log, release } = await renaming();
    await typed(out, "kpi", decl("kpi", "tapé"));
    await release();
    expect(log.filter((l) => l.startsWith("set kpi:"))).toEqual([]);
    expect(log.at(-1)).toBe(`set kpi2:${decl("kpi2", "tapé")}`);
    expect(out.files!.rewrites.get("kpi2")).toEqual({ find: 'widget("kpi"', replace: 'widget("kpi2"' });
  });

  // 18
  test("frappe partie pendant le renommage : redirigée vers le nouveau nom", async () => {
    const { out, log, release } = await renaming();
    await typed(out, "kpi", decl("kpi", "tapé"));
    await later();
    await release();
    expect(log.filter((l) => l.startsWith("set kpi:"))).toEqual([]);
    expect(log).toContain(`set kpi2:${decl("kpi2", "tapé")}`);
  });

  // 19
  test("sans frappe : remplacement silencieux, aucune écriture, texte connu", async () => {
    const { out, log, release } = await renaming();
    await release();
    expect(log).toEqual(["rename kpi>kpi2"]);
    expect(out.files!.rewrites.get("kpi2")?.silent).toBe(true);
    expect(out.files!.checked.get("kpi2")).toBe(decl("kpi2", "a"));
    expect(out.renamed).toEqual(["kpi>kpi2"]);
  });

  // 19b
  test("marqué non enregistré mais rien tapé depuis : remplacement non silencieux, texte connu sous le nouveau nom", async () => {
    const { fake } = memory({ kpi: decl("kpi", "a") });
    const set = fake.setWidget;
    stub(fake, "setWidget", async () => { throw new Error("non"); });
    const { out } = await mount(fake);
    await typed(out, "kpi", decl("kpi", "b"));
    await later();
    stub(fake, "setWidget", set);
    await act(async () => { await out.files!.rename("kpi", "kpi2"); });
    expect(out.files!.rewrites.get("kpi2")?.silent).toBeUndefined();
    expect(out.files!.unsaved.has("kpi2")).toBe(true);
  });

  // 20
  test("renommage refusé : la frappe reste écrite sous l'ancien nom, renamed non appelé", async () => {
    const { fake, log } = memory({ kpi: decl("kpi", "a") });
    const { out } = await mount(fake);
    const gate = deferred<void>();
    stub(fake, "renameWidget", async () => {
      await gate.promise;
      throw new Error("utilisé");
    });
    let done: Promise<boolean> | null = null;
    await act(async () => { done = out.files!.rename("kpi", "kpi2"); });
    await typed(out, "kpi", decl("kpi", "tapé"));
    await later();
    await act(async () => gate.resolve());
    let ok = true;
    await act(async () => { ok = await done!; });
    await wait(10);
    expect(ok).toBe(false);
    expect(log).toEqual([`set kpi:${decl("kpi", "tapé")}`]);
    expect(out.renamed).toEqual([]);
  });

  // 20b (l'ordre « renamed avant les statuts » est vérifié par son effet,
  // l'absence d'élagage du sous-onglet renommé : T8 n°9)
  test("renommage réussi : renamed une fois, statuts sous le nouveau nom", async () => {
    const { fake } = memory({ kpi: decl("kpi", "a") });
    const { out } = await mount(fake);
    await act(async () => { await out.files!.rename("kpi", "kpi2"); });
    expect(out.renamed).toEqual(["kpi>kpi2"]);
    expect(out.files!.statuses?.map((s) => s.name)).toEqual(["kpi2"]);
  });
});

describe("useWidgetFiles, essais et relectures", () => {
  // 21
  test("relecture après une écriture : diagnostics du rendu gardés ; pack changé de nouveau : plus", async () => {
    const { fake } = memory({ kpi: "k" });
    const set = fake.setWidget;
    stub(fake, "setWidget", async (n: string, s: string): Promise<WidgetChange> => {
      const c = await set(n, s);
      return { ...c, status: { ...c.status, diagnostics: [{ file: "kpi", line: 1, column: 1, error: true, message: "rendu" }] } };
    });
    const { out, render } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    await render("p", "r1");
    await wait(10);
    expect(out.files!.statuses?.[0].diagnostics.length).toBe(1);
    await render("p", "r2");
    await wait(10);
    expect(out.files!.statuses?.[0].diagnostics.length).toBe(0);
  });

  // 21b
  test("deux fichiers écrits avant la relecture : chacun garde ses diagnostics du rendu", async () => {
    const { fake } = memory({ a: "a", b: "b" });
    const set = fake.setWidget;
    stub(fake, "setWidget", async (n: string, s: string): Promise<WidgetChange> => {
      const c = await set(n, s);
      return { ...c, status: { ...c.status, diagnostics: [{ file: n, line: 1, column: 1, error: true, message: `rendu ${n}` }] } };
    });
    const { out, render } = await mount(fake);
    await typed(out, "a", "a2");
    await typed(out, "b", "b2");
    await act(async () => out.files!.flush());
    await wait(10);
    await render("p", "r1");
    await wait(10);
    expect(out.files!.statuses?.map((s) => s.diagnostics.length)).toEqual([1, 1]);
  });

  // 22
  test("essai refusé : une relecture du pack ne lui colle pas ses diagnostics", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "setWidget", async (): Promise<WidgetChange> => ({
      status: { name: "kpi", schema: null, diagnostics: [{ file: "kpi", line: 1, column: 1, error: true, message: "rendu" }] },
      broken: [],
      saved: false,
    }));
    const { out, render } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    await render("p", "r1");
    await wait(10);
    expect(out.files!.statuses?.[0].diagnostics.length).toBe(0);
  });

  // 23
  test("stale : périmée après une frappe du même fichier, pas après une frappe d'un autre", async () => {
    const { fake } = memory({ a: "a", b: "b" });
    const { out } = await mount(fake);
    await act(async () => out.files!.loaded("b", "b"));
    const askedB = out.files!.edits("b");
    await typed(out, "a", "a2");
    await later();
    expect(out.files!.stale("b", askedB, "b-agent")).toBe(false);
    const askedA = out.files!.edits("a");
    await typed(out, "a", "a3");
    expect(out.files!.stale("a", askedA, "a-agent")).toBe(true);
  });

  // 24
  test("stale : la source déjà affichée est périmée (notre propre texte)", async () => {
    const { fake } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    await act(async () => out.files!.loaded("kpi", "k"));
    expect(out.files!.stale("kpi", out.files!.edits("kpi"), "k")).toBe(true);
  });
});
```

(Le test 20b n'a pas besoin de `seen` : supprimer ces deux lignes s'il ne
sert pas après écriture ; l'assertion utile est l'ordre `renamed` puis
statuts, garantie par l'implémentation qui appelle `on.renamed` avant
`setStatuses`.)

- [ ] **Étape 3 : vérifier l'échec** — Run : `cd ui && bun test src/useWidgetFiles.render.test.tsx` ; attendu : module introuvable.

- [ ] **Étape 4 : implémenter**

`TypstEditor.tsx` : extraire le type de la prop `rewrite` :

```ts
/** Remplacement à faire dans le texte courant (nom déclaré après un renommage). */
export interface Rewrite {
  find: string;
  replace: string;
  /** Le parent sait déjà le texte obtenu : pas d'`onChange`. */
  silent?: boolean;
}
```

et `rewrite?: Rewrite | null` dans `Props`.

`useWidgetFiles.ts` :

```ts
/**
 * Fichiers de widgets du pack édité : statuts, frappes, une file d'opérations.
 *
 * Partagé par la pile et les sous-onglets. Chaque fichier a son délai de
 * frappe (400 ms) ; toutes les opérations passent l'une après l'autre, dans
 * l'ordre. Règles de t-390 conservées : essai refusé (`saved: false`),
 * diagnostics du rendu gardés à la première relecture, frappe pendant un
 * renommage redirigée vers le nouveau nom.
 */
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { errorText, type TabApi, type WidgetStatus } from "./api";
import { PREVIEW_DELAY_MS, debounced } from "./debounce";
import { t } from "./i18n";
import type { Rewrite } from "./TypstEditor";
import { declaredAs, merged, ordered, skeleton, withTrial } from "./widgetList";

// (interface WidgetFiles : recopier celle de « Interfaces » ci-dessus, avec ses commentaires)

type Push = ReturnType<typeof debounced<[string]>>;
interface Trial { statuses: WidgetStatus[]; text: string }
interface Callbacks { changed(): void; error(message: string): void; renamed(from: string, to: string): void }

/** `map` où `key` vaut `value` (nouvelle table, pour React). */
function withEntry<V>(map: ReadonlyMap<string, V>, key: string, value: V): ReadonlyMap<string, V> {
  const next = new Map(map);
  next.set(key, value);
  return next;
}

/** `set` avec ou sans `name` ; le même objet s'il ne change pas. */
function toggled(set: ReadonlySet<string>, name: string, on: boolean): ReadonlySet<string> {
  if (set.has(name) === on) return set;
  const next = new Set(set);
  if (on) next.add(name);
  else next.delete(name);
  return next;
}

const EMPTY_SET: ReadonlySet<string> = new Set();
const EMPTY_MAP = new Map<string, never>();

export function useWidgetFiles(api: TabApi, pack: string | null, revision: string, on: Callbacks): WidgetFiles {
  const [statuses, setStatuses] = useState<WidgetStatus[] | null>(null);
  const [statusesFor, setStatusesFor] = useState<string | null>(null);
  const [unsaved, setUnsaved] = useState<ReadonlySet<string>>(EMPTY_SET);
  const [checked, setChecked] = useState<ReadonlyMap<string, string>>(EMPTY_MAP);
  const [writes, setWrites] = useState<ReadonlyMap<string, number>>(EMPTY_MAP);
  const [rewrites, setRewrites] = useState<ReadonlyMap<string, Rewrite>>(EMPTY_MAP);
  /** Copie synchrone de `unsaved`, lue dans la file (jamais pendant le rendu). */
  const unsavedNow = useRef(new Set<string>());
  /** Dernier texte enregistré (ou lu) de chaque fichier. */
  const sent = useRef(new Map<string, string>());
  /** Fichiers dont une frappe attend son écriture. */
  const unsent = useRef(new Set<string>());
  /** Par fichier : frappes et écritures (date une lecture), écritures en cours. */
  const edits = useRef(new Map<string, number>());
  const writing = useRef(new Map<string, number>());
  const inflight = useRef<Promise<unknown>>(Promise.resolve());
  const moves = useRef(new Map<string, { to: string; done: boolean }>());
  const trials = useRef(new Map<string, Trial>());
  const pushes = useRef(new Map<string, Push>());
  const refreshSeq = useRef(0);
  const onRef = useRef(on);
  const apiRef = useRef(api);
  useLayoutEffect(() => {
    onRef.current = on;
    apiRef.current = api;
  });

  const bump = (m: Map<string, number>, name: string, by: number) => m.set(name, (m.get(name) ?? 0) + by);
  const fail = useCallback((e: unknown) => onRef.current.error(errorText(e)), []);
  const markUnsaved = useCallback((name: string, yes: boolean) => {
    if (yes) unsavedNow.current.add(name);
    else unsavedNow.current.delete(name);
    setUnsaved((u) => toggled(u, name, yes));
  }, []);
  const check = useCallback((name: string, text: string) => setChecked((c) => withEntry(c, name, text)), []);

  // Un autre pack (ou la sortie du mode modèles) : tout repart de zéro.
  useEffect(() => {
    pushes.current.forEach((p) => p.cancel());
    pushes.current.clear();
    for (const r of [sent, unsent, edits, writing, moves, trials]) r.current.clear();
    unsavedNow.current.clear();
    setStatuses(null);
    setStatusesFor(null);
    setUnsaved(EMPTY_SET);
    setChecked(EMPTY_MAP);
    setWrites(EMPTY_MAP);
    setRewrites(EMPTY_MAP);
  }, [pack]);

  // Statuts relus quand le pack change ; une réponse plus ancienne qu'une autre est ignorée.
  useEffect(() => {
    if (pack === null) return;
    const seq = ++refreshSeq.current;
    const pending = [...trials.current];
    trials.current.clear();
    const written = Promise.all(pending.map(([file]) => api.widgetSource(file).catch(() => null)));
    Promise.all([api.widgetStatus(), written])
      .then(([list, sources]) => {
        if (seq !== refreshSeq.current) return;
        let shown = list;
        pending.forEach(([, tr], i) => {
          if (sources[i] === tr.text) shown = withTrial(shown, tr.statuses);
        });
        setStatuses(ordered(shown));
        setStatusesFor(revision);
      })
      .catch(fail);
  }, [api, pack, revision, fail]);

  /** `task` à la suite de la file ; sa valeur, ou `false` après un échec dit. */
  const enqueue = useCallback(
    <T,>(task: () => Promise<T>): Promise<T | false> => {
      const run = inflight.current.then(task).catch((e: unknown) => {
        fail(e);
        return false as const;
      });
      inflight.current = run;
      return run;
    },
    [fail],
  );

  /** Écrit `text` dans `name` : statuts, essai, marque, compte des écritures. */
  const write = useCallback(
    (name: string, text: string) => {
      bump(writing.current, name, 1);
      return apiRef.current
        .setWidget(name, text)
        .then(
          (change) => {
            setStatuses((s) => merged(s, [change.status, ...change.broken]));
            check(name, text);
            if (!change.saved) {
              markUnsaved(name, true);
              return;
            }
            sent.current.set(name, text);
            markUnsaved(name, false);
            trials.current.set(name, { statuses: [change.status, ...change.broken], text });
            setWrites((w) => withEntry(w, name, (w.get(name) ?? 0) + 1));
            onRef.current.changed();
          },
          (e: unknown) => {
            markUnsaved(name, true);
            throw e;
          },
        )
        .finally(() => bump(writing.current, name, -1));
    },
    [check, markUnsaved],
  );

  /** Le délai de frappe de `name`, créé au premier besoin. */
  const pushOf = useCallback(
    (name: string): Push => {
      let p = pushes.current.get(name);
      if (!p) {
        p = debounced((text: string) => {
          unsent.current.delete(name);
          // Un fichier vide vaudrait suppression : on attend du texte.
          if (text === "") return;
          bump(edits.current, name, 1);
          const move = moves.current.get(name);
          void enqueue(() => (move?.done ? write(move.to, declaredAs(text, name, move.to)) : write(name, text)));
        }, PREVIEW_DELAY_MS);
        pushes.current.set(name, p);
      }
      return p;
    },
    [enqueue, write],
  );
  // Démontage : les frappes partent quand même.
  useEffect(() => () => pushes.current.forEach((p) => p.flush()), []);

  const edit = useCallback(
    (name: string, text: string) => {
      bump(edits.current, name, 1);
      unsent.current.add(name);
      pushOf(name)(text);
    },
    [pushOf],
  );

  const flush = useCallback((name?: string) => {
    if (name === undefined) pushes.current.forEach((p) => p.flush());
    else pushes.current.get(name)?.flush();
  }, []);

  const settle = useCallback(async () => {
    flush();
    await inflight.current;
  }, [flush]);

  const create = useCallback(
    (name: string) => {
      flush();
      return enqueue(() =>
        apiRef.current.setWidget(name, skeleton(name)).then((c) => {
          if (!c.saved) {
            onRef.current.error(c.status.diagnostics[0]?.message ?? t("ui-widgets-unsaved"));
            return false;
          }
          markUnsaved(name, false);
          sent.current.set(name, skeleton(name));
          trials.current.set(name, { statuses: [c.status, ...c.broken], text: skeleton(name) });
          setStatuses((s) => merged(s, [c.status, ...c.broken]));
          onRef.current.changed();
          return true;
        }),
      ).then((ok) => ok === true);
    },
    [enqueue, flush, markUnsaved],
  );

  const rename = useCallback(
    (from: string, to: string) => {
      const move = { to, done: false };
      moves.current.set(from, move);
      const typedSince = edits.current.get(from) ?? 0;
      return enqueue(() =>
        apiRef.current
          .renameWidget(from, to)
          .then((list) => {
            move.done = true;
            const keep = unsavedNow.current.has(from);
            // La frappe encore en attente part maintenant, vers `to`.
            pushes.current.get(from)?.flush();
            pushes.current.delete(from);
            if (keep) {
              markUnsaved(from, false);
              markUnsaved(to, true);
            }
            const find = `widget("${from}"`;
            const replace = `widget("${to}"`;
            const text = declaredAs(sent.current.get(from) ?? "", from, to);
            sent.current.set(to, text);
            sent.current.delete(from);
            if (keep || (edits.current.get(from) ?? 0) !== typedSince) {
              setRewrites((r) => withEntry(r, to, { find, replace }));
            } else {
              // Rien de tapé : l'éditeur montre ce que la session a écrit, sans écriture de plus.
              check(to, text);
              setRewrites((r) => withEntry(r, to, { find, replace, silent: true }));
            }
            edits.current.set(to, (edits.current.get(from) ?? 0) + 1);
            // Le sous-onglet suit avant les statuts sans `from` : il n'est pas élagué.
            onRef.current.renamed(from, to);
            setStatuses(ordered(list));
            onRef.current.changed();
            return true;
          })
          .finally(() => moves.current.delete(from)),
      ).then((ok) => ok === true);
    },
    [enqueue, check, markUnsaved],
  );

  const remove = useCallback(
    (name: string) => {
      // Le fichier part : sa frappe en attente aussi ; l'écriture déjà partie est attendue.
      pushes.current.get(name)?.cancel();
      pushes.current.delete(name);
      unsent.current.delete(name);
      return enqueue(() =>
        apiRef.current.removeWidget(name).then((list) => {
          markUnsaved(name, false);
          sent.current.delete(name);
          setStatuses(ordered(list));
          onRef.current.changed();
          return true;
        }),
      ).then((ok) => ok === true);
    },
    [enqueue, markUnsaved],
  );

  const loaded = useCallback(
    (name: string, source: string) => {
      sent.current.set(name, source);
      check(name, source);
    },
    [check],
  );

  const stale = useCallback(
    (name: string, asked: number, source: string) =>
      unsent.current.has(name) ||
      (edits.current.get(name) ?? 0) !== asked ||
      (writing.current.get(name) ?? 0) > 0 ||
      moves.current.has(name) ||
      source === sent.current.get(name),
    [],
  );

  const editsOf = useCallback((name: string) => edits.current.get(name) ?? 0, []);

  return useMemo(
    () => ({
      statuses, statusesFor, unsaved, checked, writes, rewrites,
      edit, edits: editsOf, stale, loaded, flush, settle, create, rename, remove, fail,
    }),
    [statuses, statusesFor, unsaved, checked, writes, rewrites, edit, editsOf, stale, loaded, flush, settle, create, rename, remove, fail],
  );
}
```

Notes :
- `enqueue` : `run` ne rejette jamais (le `catch` rend `false`) ; la file
  avance donc toujours.
- Les délais de frappe passent par `enqueue`/`write`, eux-mêmes stables
  (ils lisent `apiRef`) : un délai créé tôt n'appelle jamais une api périmée.
- `moves.current.has(name)` dans `stale` : pendant un renommage, une source
  relue de l'ancien ou du nouveau nom est périmée (cas W:271).

- [ ] **Étape 5 : vérifier le vert** — Run : `cd ui && bun test && bun run typecheck` ; attendu : PASS (les tests du panneau aussi).

- [ ] **Étape 6 : commit**

```bash
git add ui/src/useWidgetFiles.ts ui/src/useWidgetFiles.render.test.tsx ui/src/test/widgetApi.ts ui/src/test/workbench.ts ui/src/TypstEditor.tsx
```
```bash
git commit -m "refactor(ui) : état partagé des fichiers de widgets, file unique d'opérations (t-394)"
```

---

### Tâche 4 : `TypstEditor` — annuler et rétablir pilotables

**Fichiers :**
- Modifier : `ui/src/TypstEditor.tsx`
- Test : `ui/src/TypstEditor.render.test.tsx`

**Interfaces :**
- Produit :

```ts
/** Commandes d'un éditeur, pour les boutons du plateau. */
export interface EditorCommands { undo(): void; redo(): void }
/** Pas d'annulation disponibles. */
export interface EditorHistory { undo: number; redo: number }
// Props ajoutées :
//   onCommands?: (c: EditorCommands | null) => void;  // au montage ; null au démontage
//   onHistory?: (h: EditorHistory) => void;           // au montage, puis à chaque changement
```

- [ ] **Étape 1 : tests rouges** (dans `TypstEditor.render.test.tsx`)

Ajouter un montage qui accepte des props en plus :

```tsx
type Extra = Partial<Parameters<typeof TypstEditor>[0]>;

async function mountWith(extra: Extra, text = "a") {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const render = (more: Extra = {}) =>
    act(async () =>
      root.render(createElement(TypstEditor, { text, diagnostics: [], label: "kpi", onChange: () => {}, ...extra, ...more })),
    );
  await render();
  const view = () => EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement)!;
  const typeText = (insert: string) =>
    act(async () => view().dispatch({ changes: { from: 0, to: view().state.doc.length, insert }, userEvent: "input.type" }));
  return { host, render, view, typeText };
}
```

Tests :

```tsx
describe("TypstEditor, commandes", () => {
  test("au montage : commandes posées et historique vide signalé", async () => {
    let commands: EditorCommands | null = null;
    const histories: EditorHistory[] = [];
    await mountWith({ onCommands: (c) => (commands = c), onHistory: (h) => histories.push(h) });
    expect(commands).not.toBeNull();
    expect(histories).toEqual([{ undo: 0, redo: 0 }]);
  });

  test("undo annule la frappe, onChange suit ; redo la refait", async () => {
    let commands: EditorCommands | null = null;
    const seen: string[] = [];
    const { typeText } = await mountWith({ onCommands: (c) => (commands = c), onChange: (t) => seen.push(t) });
    await typeText("ab");
    await act(async () => commands!.undo());
    expect(seen.at(-1)).toBe("a");
    await act(async () => commands!.redo());
    expect(seen.at(-1)).toBe("ab");
  });

  test("l'historique n'est signalé que s'il change (pas pour un déplacement du curseur)", async () => {
    const histories: EditorHistory[] = [];
    const { typeText, view } = await mountWith({ onHistory: (h) => histories.push(h) });
    await typeText("ab");
    const n = histories.length;
    await act(async () => view().dispatch({ selection: { anchor: 1 } }));
    expect(histories.length).toBe(n);
    expect(histories.at(-1)).toEqual({ undo: 1, redo: 0 });
  });

  test("démontage : commandes retirées", async () => {
    let commands: EditorCommands | null = null;
    await mountWith({ onCommands: (c) => (commands = c) });
    await act(async () => mounted!.root.unmount());
    mounted = null;
    expect(commands).toBeNull();
  });

  test("un remplacement (renommage) n'entre pas dans l'historique", async () => {
    const histories: EditorHistory[] = [];
    const { render } = await mountWith({ onHistory: (h) => histories.push(h) }, 'widget("kpi"');
    await render({ rewrite: { find: 'widget("kpi"', replace: 'widget("kpi2"' } });
    expect(histories.at(-1)).toEqual({ undo: 0, redo: 0 });
  });
});
```

(Importer `type EditorCommands, type EditorHistory` depuis `./TypstEditor`.)

- [ ] **Étape 2 : vérifier l'échec** — Run : `cd ui && bun test src/TypstEditor.render.test.tsx` ; attendu : FAIL.

- [ ] **Étape 3 : implémenter**

```ts
import { history, historyKeymap, indentWithTab, redo, redoDepth, standardKeymap, undo, undoDepth } from "@codemirror/commands";
import { EditorState, Transaction } from "@codemirror/state";
```

Dans le composant :

```ts
  const report = useEffectEvent((h: EditorHistory) => onHistory?.(h));
  const give = useEffectEvent((c: EditorCommands | null) => onCommands?.(c));
```

Dans l'`updateListener` existant, après l'appel de `changed` :

```ts
            const h = { undo: undoDepth(u.state), redo: redoDepth(u.state) };
            const before = { undo: undoDepth(u.startState), redo: redoDepth(u.startState) };
            if (h.undo !== before.undo || h.redo !== before.redo) report(h);
```

Après `view.current = v;` :

```ts
    give({ undo: () => undo(v), redo: () => redo(v) });
    report({ undo: 0, redo: 0 });
```

et `give(null);` dans le nettoyage, avant `v.destroy()`.

Dans l'effet de `rewrite`, l'annotation devient :

```ts
      annotations: [Transaction.addToHistory.of(false), ...(rewrite.silent ? [quiet.of(true)] : [])],
```

- [ ] **Étape 4 : vérifier le vert** — Run : `cd ui && bun test src/TypstEditor.render.test.tsx` ; attendu : PASS.

- [ ] **Étape 5 : commit**

```bash
git add ui/src/TypstEditor.tsx ui/src/TypstEditor.render.test.tsx
```
```bash
git commit -m "feat(ui) : annuler et rétablir de l'éditeur Typst pilotables (t-394)"
```

---

### Tâche 5 : `WidgetTab` — un fichier dans son sous-onglet

**Fichiers :**
- Créer : `ui/src/WidgetTab.tsx`
- Test : `ui/src/WidgetTab.render.test.tsx`
- Modifier : `ui/src/styles.css` (`.widget-tab*`)
- i18n (20 langues) : `ui-widgets-in-error` (en « Widgets with errors », fr
  « Widgets en erreur ») ; `ui-widgets-none-in-error` (en « No widget has
  errors », fr « Aucun widget en erreur »)

**Interfaces :**
- Consomme : `WidgetFiles` (T3), `EditorCommands`/`EditorHistory` (T4), `croppedSvg`, `widgetList`.
- Produit :

```ts
export interface WidgetTabProps {
  api: TabApi;
  files: WidgetFiles;
  /** Id stable du sous-onglet (`OpenWidget.id`), pour les ids ARIA. */
  id: number;
  /** Nom courant du fichier ; un renommage le change sans remonter le composant. */
  name: string;
  revision: string;
  /** Seul l'actif est visible, relit sa source et demande un aperçu. */
  active: boolean;
  /** Commun : ouvre le widget cliqué. */
  onOpen: (name: string) => void;
  onCommands: (c: EditorCommands | null) => void;
  onHistory: (h: EditorHistory) => void;
}
export default function WidgetTab(props: WidgetTabProps): JSX.Element;
```

Règles :
- Source lue au montage, puis à chaque `revision` **si actif** ; un
  sous-onglet caché note qu'une relecture est due et la fait en devenant
  actif. Le changement de `name` (renommage) ne relit pas : le remplacement
  `files.rewrites.get(name)` suffit.
- Une source périmée (`files.stale(name, files.edits(name) à la demande, source)`)
  n'est pas affichée une fois qu'une source l'a été.
- Aperçu : actif seulement (pas pour Commun), après la première source puis
  à chaque écriture enregistrée (`files.writes.get(name)`) ; un aperçu plus
  ancien qu'un autre est ignoré.
- En redevenant actif : `EditorView.requestMeasure()` (l'éditeur caché a pu
  mal se mesurer).
- Panneau : `role="tabpanel"`, `id="widget-panel-${id}"`,
  `aria-labelledby="widget-tab-${id}"` (ids posés par `TemplateTabs`, T7).

- [ ] **Étape 1 : tests rouges** (`WidgetTab.render.test.tsx`)

Outils : une sonde qui appelle `useWidgetFiles(fake, "p", rev, …)` et rend
`WidgetTab` avec `files`, `name`, `active`, `revision` en props, plus
`EditorView.findFromDOM` pour l'éditeur :

```tsx
// Sous-onglet d'un fichier de widget : source, diagnostics, aperçu, Commun.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { TabApi, WidgetStatus } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const { PREVIEW_DELAY_MS } = await import("./debounce");
const { useWidgetFiles } = await import("./useWidgetFiles");
const WidgetTab = (await import("./WidgetTab")).default;
const { deferred, memory, stub, wait } = await import("./test/widgetApi");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

interface Shown { name: string; active?: boolean; revision?: string }

async function mount(fake: TabApi, first: Shown) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const opened: string[] = [];
  const files: { current: ReturnType<typeof useWidgetFiles> | null } = { current: null };
  function Probe({ name, active = true, revision = "r" }: Shown) {
    const f = useWidgetFiles(fake, "p", revision, { changed: () => {}, error: () => {}, renamed: () => {} });
    files.current = f;
    return createElement(WidgetTab, {
      api: fake, files: f, id: 1, name, revision, active,
      onOpen: (n: string) => opened.push(n), onCommands: () => {}, onHistory: () => {},
    });
  }
  const render = (p: Shown) => act(async () => root.render(createElement(Probe, p)));
  await render(first);
  await wait(10);
  const view = () => EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement);
  const typeText = (insert: string) =>
    act(async () => {
      const v = view()!;
      v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert }, userEvent: "input.type" });
    });
  const text = () => host.querySelector(".cm-content")?.textContent;
  return { host, render, view, typeText, text, opened, files };
}
const later = () => wait(PREVIEW_DELAY_MS + 50);
```

Tests (corps à écrire avec ces outils) :

1. « affiche la source du fichier » : `memory({ kpi: "k" })` → `text()` vaut `"k"`.
2. « les diagnostics de _commun restent visibles sur le widget qui l'importe, avec leur fichier » :
   `widgetStatus` rend `kpi` avec un diagnostic `{ file: "_commun", line: 1, column: 1, error: true, message: "x" }` ;
   la liste `.widget-tab-diagnostics li` contient exactement `["_commun:1:1 x"]`
   (format de `diagnosticPlace` : le vérifier dans `widgetList.ts` et
   reprendre l'attendu de `WidgetWorkbench.render.test.tsx:66`).
3. « une frappe est écrite après le délai » : `typeText("k2")`, `later()` → `log` = `["set kpi:k2"]`.
4. « une réponse de source arrivée après une frappe ne ramène pas l'ancien texte » :
   `widgetSource` retenu par un `deferred` à la relecture (rendre avec
   `revision: "r1"`), `typeText("k2")`, libérer avec `"k"` → `text()` vaut `"k2"`.
5. « une réponse de source lente, sans écriture partie mais après une frappe, est ignorée » :
   idem 4 sans attendre le délai ; libérer avant `later()` → `text()` vaut `"k2"`.
6. « le pack changé ailleurs (l'agent) : la source affichée est relue » :
   `files.kpi = "agent"` dans la mémoire, rendre avec `revision: "r1"` → `text()` vaut `"agent"`.
7. « une écriture refusée : le texte tapé reste, même après relecture » :
   `setWidget` qui lève, `typeText("k2")`, `later()`, rendre `revision: "r1"` → `text()` vaut `"k2"`.
8. « une frappe pas encore envoyée n'est pas écrasée par une relecture demandée après elle » :
   `typeText("k2")` puis aussitôt rendre `revision: "r1"` → `text()` vaut `"k2"`.
9. « un aperçu ancien et lent n'écrase pas un plus récent » : premier
   `previewWidget` retenu, second immédiat avec `svg: "<svg id='new'/>"` ;
   libérer le premier avec `svg: "<svg id='old'/>"` → l'`img` montre l'URL du second
   (comparer au `src` relevé après le second).
10. « un widget d'élément est cadré sur sa boîte » : `previewWidget` rend
    `{ svg: '<svg viewBox="0 0 960 540" width="960pt" height="540pt"></svg>', bbox: { x: 100, y: 100, w: 200, h: 100 }, diagnostics: [] }` ;
    reprendre l'attendu de `WidgetWorkbench.render.test.tsx:271` (lire le blob de l'`img`).
11. « après une écriture : un seul aperçu redemandé » : compter `calls` « preview »
    après `typeText` + `later()` + rendu `revision: "r1"` → exactement un de plus.
12. « inactif : ni aperçu, ni relecture ; activé : relecture puis aperçu » :
    monter `active: false` → `calls` sans « preview » ; rendre `revision: "r1"` →
    pas de « source » de plus ; rendre `active: true` → une « source » et un « preview ».
13. « Commun : liste les widgets en erreur ; un clic appelle onOpen » :
    statuts `_commun` valide, `kpi` en erreur, `quote` valide ; nom `_commun` →
    un seul bouton `kpi` dans `.widget-tab-broken`, clic → `opened` = `["kpi"]` ; aucun `img`.
14. « Commun sans widget en erreur : le texte ui-widgets-none-in-error ».
15. « nom changé (renommage) : même vue CodeMirror, remplacement appliqué, pas de relecture » :
    `memory({ kpi: 'widget("kpi", a)' })` ; garder `view()` ; `typeText('widget("kpi", b)')` ;
    `await act(() => files.current!.rename("kpi", "kpi2"))` ; rendre `name: "kpi2"` →
    `view()` est le même objet, `text()` contient `widget("kpi2"`, aucune « source » de plus.

- [ ] **Étape 2 : vérifier l'échec** — Run : `cd ui && bun test src/WidgetTab.render.test.tsx`.

- [ ] **Étape 3 : implémenter** (`WidgetTab.tsx`)

```tsx
/**
 * Sous-onglet d'un fichier de widget : éditeur, aperçu et diagnostics.
 *
 * Pour Commun, les widgets en erreur à la place de l'aperçu. Caché quand il
 * n'est pas actif, il reste monté : l'éditeur garde son historique.
 * L'aperçu reste dans un `<img>` : un SVG de widget ne s'y exécute pas.
 */
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { EditorView } from "@codemirror/view";
import type { TabApi, WidgetPreview } from "./api";
import { t } from "./i18n";
import TypstEditor, { type EditorCommands, type EditorHistory } from "./TypstEditor";
import type { WidgetFiles } from "./useWidgetFiles";
import { croppedSvg } from "./widgetPreview";
import { COMMON, diagnosticPlace, fileDiagnostics, listedDiagnostics, stateOf, widgetLabel } from "./widgetList";

// (interface WidgetTabProps : recopier celle de « Interfaces » ci-dessus)

export default function WidgetTab({ api, files, id, name, revision, active, onOpen, onCommands, onHistory }: WidgetTabProps) {
  const [source, setSource] = useState<string | null>(null);
  const [editorKey, setEditorKey] = useState(0);
  const [preview, setPreview] = useState<WidgetPreview | null>(null);
  const host = useRef<HTMLElement>(null);
  const nameRef = useRef(name);
  const filesRef = useRef(files);
  useLayoutEffect(() => {
    nameRef.current = name;
    filesRef.current = files;
  });
  /** Revision dont la source est lue (ou due). */
  const readFor = useRef<string | null>(null);
  const shown = useRef(false);
  const previewSeq = useRef(0);

  // Source : au montage, puis à chaque revision quand actif (un caché la lit en devenant actif).
  useEffect(() => {
    if (readFor.current === revision || (!active && shown.current)) return;
    readFor.current = revision;
    let alive = true;
    const file = nameRef.current;
    const asked = filesRef.current.edits(file);
    api
      .widgetSource(file)
      .then((s) => {
        if (!alive || (shown.current && filesRef.current.stale(file, asked, s))) return;
        shown.current = true;
        filesRef.current.loaded(file, s);
        setSource(s);
        setEditorKey((k) => k + 1);
      })
      .catch((e: unknown) => alive && filesRef.current.fail(e));
    return () => {
      alive = false;
    };
  }, [api, revision, active]);

  // Aperçu : actif seulement, après la source puis chaque écriture enregistrée.
  const written = files.writes.get(name) ?? 0;
  useEffect(() => {
    if (!active || name === COMMON || source === null) return;
    const seq = ++previewSeq.current;
    api
      .previewWidget(name)
      .then((p) => {
        if (seq === previewSeq.current) setPreview(p);
      })
      .catch((e: unknown) => seq === previewSeq.current && filesRef.current.fail(e));
  }, [api, active, name, written, editorKey, source === null]);

  // Redevenu visible : l'éditeur caché se remesure.
  useEffect(() => {
    if (!active) return;
    const el = host.current?.querySelector(".cm-content");
    if (el) EditorView.findFromDOM(el as HTMLElement)?.requestMeasure();
  }, [active]);

  const status = files.statuses?.find((s) => s.name === name);
  const diagnostics = useMemo(() => fileDiagnostics(status, name), [status, name]);
  const previewUrl = useMemo(() => {
    if (!preview?.svg) return null;
    // Un widget d'élément est cadré sur ce qu'il dessine, pas sur la slide entière.
    const svg = croppedSvg(preview.svg, preview.bbox ?? null);
    return URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
  }, [preview]);
  useEffect(() => () => {
    if (previewUrl) URL.revokeObjectURL(previewUrl);
  }, [previewUrl]);

  const inError = (files.statuses ?? []).filter((s) => s.name !== COMMON && stateOf(s) === "errors");

  return (
    <section
      ref={host}
      className="widget-tab"
      hidden={!active}
      role="tabpanel"
      id={`widget-panel-${id}`}
      aria-labelledby={`widget-tab-${id}`}
    >
      <div className="widget-tab-editor">
        {source !== null && (
          <TypstEditor
            key={editorKey}
            text={source}
            rewrite={files.rewrites.get(name) ?? null}
            diagnostics={diagnostics}
            diagnosticsFor={files.checked.get(name) ?? null}
            label={widgetLabel(name)}
            onChange={(text) => files.edit(name, text)}
            onCommands={onCommands}
            onHistory={onHistory}
          />
        )}
      </div>
      <aside className="widget-tab-side" aria-label={t(name === COMMON ? "ui-widgets-in-error" : "ui-widgets-preview")}>
        {name === COMMON ? (
          <div className="widget-tab-broken">
            <h3>{t("ui-widgets-in-error")}</h3>
            {inError.length === 0 ? (
              <p className="caption">{t("ui-widgets-none-in-error")}</p>
            ) : (
              <ul>
                {inError.map((s) => (
                  <li key={s.name}>
                    <button className="btn" onClick={() => onOpen(s.name)}>{s.name}</button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ) : previewUrl ? (
          <img src={previewUrl} alt={t("ui-widgets-preview")} />
        ) : (
          <p className="caption">{t("ui-widgets-no-preview")}</p>
        )}
        <ul className="widget-tab-diagnostics">
          {listedDiagnostics(status, preview?.diagnostics ?? []).map((d, i) => {
            const place = diagnosticPlace(d);
            return (
              <li key={i} className={d.error ? "error" : undefined}>
                {place ? `${place} ` : ""}{d.message}
              </li>
            );
          })}
        </ul>
      </aside>
    </section>
  );
}
```

(L'effet d'aperçu a `source === null` dans ses dépendances : le linter peut
demander une variable ; écrire `const ready = source !== null;` et dépendre
de `ready`.)

CSS :

```css
.widget-tab { display: grid; grid-template-columns: 1fr 360px; gap: var(--space-4); height: 100%; min-height: 0; }
.widget-tab[hidden] { display: none; }
.widget-tab-editor { min-width: 0; overflow: auto; border-radius: var(--radius-sm); background: var(--paper-200); }
.widget-tab-side { min-width: 0; overflow: auto; }
.widget-tab-side img { width: 100%; max-height: 50vh; object-fit: contain; border-radius: var(--radius-sm); box-shadow: var(--shadow-sheet-lifted); }
.widget-tab-broken h3 { margin: 0 0 var(--space-2); font: 500 13px/20px var(--sans); }
.widget-tab-broken ul { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: var(--space-2); }
.widget-tab-diagnostics { margin: var(--space-2) 0 0; padding: 0; list-style: none; font-size: 12px; }
.widget-tab-diagnostics .error { color: var(--danger); }
```

Clés Fluent dans les 20 `ui.ftl`, à la suite des `ui-widgets-*`.

- [ ] **Étape 4 : vérifier le vert** — Run : `cd ui && bun test && bun run typecheck && cd .. && cargo nextest run -p oku-slides-i18n`.

- [ ] **Étape 5 : commit**

```bash
git add ui/src/WidgetTab.tsx ui/src/WidgetTab.render.test.tsx ui/src/styles.css crates/oku-slides-i18n/src/locales
```
```bash
git commit -m "feat(ui) : sous-onglet d'un fichier de widget, Commun et ses widgets en erreur (t-394)"
```

---

### Tâche 6 : `PileWidgets` — section « Widgets » de la pile

**Fichiers :**
- Créer : `ui/src/PileWidgets.tsx`
- Modifier : `ui/src/icons.tsx` (`add`), `ui/src/SlidePile.tsx` (`children`)
- Tests : `ui/src/PileWidgets.render.test.tsx`, `ui/src/SlidePile.render.test.tsx`
- Modifier : `ui/src/styles.css` (`.pile-widgets*`, `.unsaved-dot` qui remplace `.workbench-unsaved`)
- i18n (20 langues) : `ui-widgets-section` (en « Widgets », fr « Widgets ») ;
  `ui-widgets-delete-named` (en « Delete { $name } », fr « Supprimer { $name } ») ;
  `ui-widgets-confirm-delete` (en « Delete the widget “{ $name }”? », fr
  « Supprimer le widget « { $name } » ? ») ;
  `ui-widgets-rename-hint` (en « Double-click or F2 to rename », fr
  « Double-clic ou F2 pour renommer ») ; `ui-widgets-new` change de texte
  (en « New widget », fr « Nouveau widget »).

**Interfaces :**
- Produit :

```ts
// icons.tsx, dans Icon (section Actions) :
//   add: stroke(<path d="M12 5v14M5 12h14" />),

export interface PileWidgetsProps {
  files: WidgetFiles;
  /** Fichier du sous-onglet actif ; `null` : « Modèles ». */
  current: string | null;
  onOpen: (name: string) => void;
  /** Après une suppression réussie (le sous-onglet se ferme). */
  onRemoved: (name: string) => void;
  /** Change : le focus va au titre de la section (☰ « Éditer les widgets »). */
  focus: number;
  /** Confirmation (injectable pour les tests) ; défaut : `window.confirm`. */
  confirm?: (message: string) => boolean;
}
// SlidePile : children?: ReactNode — « Sous la pile, dans la même colonne (section Widgets du mode modèles). »
```

(Le renommage d'un sous-onglet passe par `on.renamed` de `useWidgetFiles`,
pas par la pile.)

- [ ] **Étape 1 : tests rouges**

`SlidePile.render.test.tsx` : « les enfants sont rendus sous le bouton
d'ajout, dans la pile » (`createElement(SlidePile, { …props existants, children: createElement("p", { id: "x" }) })`
→ `nav.pile #x` existe).

`PileWidgets.render.test.tsx` : sonde `useWidgetFiles(memory(...), "p", "r", …)` + `PileWidgets` ;
outils `row(host, label)`, `nameInput(host)`, `submitName(host, value)`
(repris de `test/workbench.ts`, adaptés aux classes `.pile-widgets-*`).

1. « liste : Commun en tête, pastilles d'état, marque non enregistré » :
   statuts `kpi` valide, `_commun` en erreur → pastilles `["pill errors", "pill valid"]` ;
   après une écriture refusée de `kpi`, `.unsaved-dot` dans sa ligne.
2. « clic sur un fichier : onOpen(nom) ».
3. « + puis un nom libre : créé avec son squelette, onOpen(nom) » :
   `log` = ``[`set abc:${skeleton("abc")}`]``, `opened` = `["abc"]`.
4. « + avec un nom pris : aria-invalid, rien n'est écrit ».
5. « avant le premier statut, + est désactivé » (`widgetStatus` retenu).
6. « `_commun` : + le crée s'il manque ; pas en double ; pas par un renommage ».
7. « double-clic sur un nom : champ ; valider → renameWidget » ;
   « F2 sur le bouton : même champ ».
8. « double-clic sur Commun : pas de champ ».
9. « Échap abandonne le champ et rend le focus à la ligne ».
10. « valider un renommage rend le focus à la ligne renommée ».
11. « perdre le focus ne perd pas le nom saisi » (blur → le champ reste, valeur gardée).
12. « supprimer : confirmation refusée → rien ; acceptée → removeWidget, onRemoved(nom) ».
13. « suppression refusée par la session (modèle qui l'utilise) : erreur dite, onRemoved non appelé ».
14. « création refusée : onOpen non appelé ».
15. « pack vide : texte ui-widgets-empty ».
16. « focus change : le titre de la section prend le focus ».
17. « le bouton Supprimer est atteignable au clavier » (il n'a ni `hidden`
    ni `tabIndex=-1` ; le masquage est en `opacity`).

- [ ] **Étape 2 : vérifier l'échec** — Run : `cd ui && bun test src/PileWidgets.render.test.tsx src/SlidePile.render.test.tsx`.

- [ ] **Étape 3 : implémenter**

```tsx
/** Section « Widgets » de la pile : ouvrir, créer, renommer, supprimer. */
import { useEffect, useRef, useState } from "react";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { WidgetFiles } from "./useWidgetFiles";
import { COMMON, nameAvailable, stateOf, widgetLabel } from "./widgetList";

// (interface PileWidgetsProps : recopier celle de « Interfaces » ci-dessus)

export default function PileWidgets({ files, current, onOpen, onRemoved, focus, confirm = (m) => window.confirm(m) }: PileWidgetsProps) {
  const [naming, setNaming] = useState<{ from: string | null; value: string } | null>(null);
  const title = useRef<HTMLHeadingElement>(null);
  const rows = useRef(new Map<string, HTMLButtonElement>());
  /** Ligne à refocaliser après le champ du nom. */
  const refocus = useRef<string | null>(null);
  useEffect(() => {
    if (focus > 0) title.current?.focus();
  }, [focus]);
  useEffect(() => {
    if (naming || !refocus.current) return;
    rows.current.get(refocus.current)?.focus();
    refocus.current = null;
  });
  const ok = (n: { from: string | null; value: string }) => nameAvailable(files.statuses, n.value, n.from);

  const close = (focusOn: string | null) => {
    refocus.current = focusOn;
    setNaming(null);
  };
  const commit = () => {
    if (!naming || !ok(naming)) return;
    const { from, value } = naming;
    close(from ? value : null);
    if (from) void files.rename(from, value);
    else void files.create(value).then((done) => done && onOpen(value));
  };
  const remove = (name: string) => {
    if (!confirm(t("ui-widgets-confirm-delete", { name: widgetLabel(name) }))) return;
    void files.remove(name).then((done) => done && onRemoved(name));
  };
  const startRename = (name: string) => {
    if (name !== COMMON) setNaming({ from: name, value: name });
  };

  const field = naming && (
    <form className="pile-widgets-name" onSubmit={(e) => { e.preventDefault(); commit(); }}>
      <input
        className="select"
        autoFocus
        aria-label={t("ui-widgets-name")}
        aria-invalid={naming.value !== "" && !ok(naming)}
        value={naming.value}
        onChange={(e) => setNaming({ ...naming, value: e.target.value })}
        onKeyDown={(e) => {
          if (e.key !== "Escape") return;
          e.stopPropagation();
          close(naming.from);
        }}
      />
    </form>
  );

  return (
    <section className="pile-widgets" aria-label={t("ui-widgets-files")}>
      <div className="pile-widgets-head">
        <h3 ref={title} tabIndex={-1}>{t("ui-widgets-section")}</h3>
        <button
          className="token ghost"
          aria-label={t("ui-widgets-new")}
          title={t("ui-widgets-new")}
          disabled={files.statuses === null}
          onClick={() => setNaming({ from: null, value: "" })}
        >
          {Icon.add}
        </button>
      </div>
      {naming?.from === null && field}
      <ul>
        {(files.statuses ?? []).map((s) => {
          if (naming?.from === s.name) return <li key={s.name}>{field}</li>;
          const state = stateOf(s);
          const label = widgetLabel(s.name);
          return (
            <li key={s.name} className="pile-widgets-row">
              <button
                ref={(el) => { if (el) rows.current.set(s.name, el); else rows.current.delete(s.name); }}
                className="pile-widgets-file"
                aria-current={s.name === current}
                title={s.name === COMMON ? undefined : t("ui-widgets-rename-hint")}
                onClick={() => onOpen(s.name)}
                onDoubleClick={() => startRename(s.name)}
                onKeyDown={(e) => {
                  if (e.key !== "F2") return;
                  e.preventDefault();
                  startRename(s.name);
                }}
              >
                <span className={`pill ${state}`} role="img" aria-label={t(state === "valid" ? "ui-widgets-valid" : "ui-widgets-errors")} />
                {label}
                {files.unsaved.has(s.name) && <span className="unsaved-dot" role="img" aria-label={t("ui-widgets-unsaved")} title={t("ui-widgets-unsaved")} />}
              </button>
              <button className="token ghost pile-widgets-remove" aria-label={t("ui-widgets-delete-named", { name: label })} title={t("ui-widgets-delete-named", { name: label })} onClick={() => remove(s.name)}>
                {Icon.trash}
              </button>
            </li>
          );
        })}
      </ul>
      {files.statuses?.length === 0 && <p className="caption">{t("ui-widgets-empty")}</p>}
    </section>
  );
}
```

`SlidePile.tsx` : `children?: ReactNode` dans `Props` (doc ci-dessus),
`{children}` juste après le bouton `add-slide` (l. 194).

CSS :

```css
.pile-widgets { margin-top: var(--space-4); }
.pile-widgets-head { display: flex; align-items: center; justify-content: space-between; }
.pile-widgets-head h3 { margin: 0; font: 500 13px/20px var(--sans); }
.pile-widgets ul { list-style: none; margin: 0; padding: 0; }
.pile-widgets-row { display: flex; align-items: center; }
.pile-widgets-file { flex: 1; min-width: 0; display: flex; align-items: center; gap: var(--space-2); padding: 6px 8px; border: 0; border-radius: var(--radius-sm); background: transparent; color: inherit; text-align: start; }
.pile-widgets-file[aria-current="true"] { background: var(--accent-soft); }
.pile-widgets-remove { opacity: 0; }
.pile-widgets-row:hover .pile-widgets-remove, .pile-widgets-row:focus-within .pile-widgets-remove { opacity: 1; }
.pile-widgets-name input[aria-invalid="true"] { outline: 2px solid var(--danger); }
.unsaved-dot { flex: none; width: 7px; height: 7px; margin-inline-start: auto; border-radius: 50%; background: currentColor; }
```

- [ ] **Étape 4 : vérifier le vert** — Run : `cd ui && bun test && bun run typecheck && cd .. && cargo nextest run -p oku-slides-i18n`.

- [ ] **Étape 5 : commit**

```bash
git add ui/src/PileWidgets.tsx ui/src/PileWidgets.render.test.tsx ui/src/SlidePile.tsx ui/src/SlidePile.render.test.tsx ui/src/icons.tsx ui/src/styles.css crates/oku-slides-i18n/src/locales
```
```bash
git commit -m "feat(ui) : section Widgets de la pile des modèles (t-394)"
```

---

### Tâche 7 : `TemplateTabs` — barre des sous-onglets

**Fichiers :**
- Créer : `ui/src/TemplateTabs.tsx`
- Test : `ui/src/TemplateTabs.render.test.tsx`
- Modifier : `ui/src/styles.css` (`.template-tabs*`)
- i18n (20 langues) : `ui-widgets-tabs` (en « Template tabs », fr « Onglets des modèles ») ;
  `ui-widgets-templates-tab` (en « Templates », fr « Modèles ») ;
  `ui-widgets-close-tab` (en « Close { $name } », fr « Fermer { $name } »).

**Interfaces :**
- Consomme : `OpenWidget` (T2).
- Produit :

```ts
export interface TemplateTabsProps {
  tabs: OpenWidget[];
  active: number | null;
  unsaved: ReadonlySet<string>;
  onActivate: (id: number | null) => void;
  onClose: (id: number) => void;
}
export default function TemplateTabs(props: TemplateTabsProps): JSX.Element;
```

Motif d'accessibilité : `role="tablist"` ne contient que des `role="tab"`.
Chaque onglet : `id="widget-tab-${id}"`, `aria-controls="widget-panel-${id}"`,
`aria-selected`, `tabIndex` 0 pour l'actif, -1 sinon ; ←/→ (inversés en
`rtl`), Début/Fin déplacent le focus et activent ; Suppr ferme l'onglet
widget focalisé. « Modèles » : `id="widget-tab-0"`,
`aria-controls="templates-panel"` (posé sur la table des modèles par T8).
Le × de chaque onglet est un `<button tabIndex={-1} aria-hidden>` à côté
du `role="tab"`, dans une enveloppe `role="presentation"` : la souris
s'en sert, le clavier passe par Suppr.

- [ ] **Étape 1 : tests rouges**

1. « Modèles en premier, puis les widgets, Commun libellé « Commun » ».
2. « clic : onActivate(id) ; clic sur Modèles : onActivate(null) ».
3. « × : onClose(id), sans onActivate ».
4. « clic du milieu (auxclick, button 1) : onClose(id) ».
5. « aria-selected et tabIndex 0 sur l'actif seulement ; Modèles n'a pas de × ».
6. « → depuis Modèles active le premier widget ; ← depuis le premier revient à Modèles ».
7. « Suppr sur un onglet widget : onClose(id) ; sur Modèles : rien ».
8. « aria-controls de chaque onglet = widget-panel-<id> ».
9. « marque non enregistré sur le libellé (`.unsaved-dot`) ».

- [ ] **Étape 2 : vérifier l'échec** — Run : `cd ui && bun test src/TemplateTabs.render.test.tsx`.

- [ ] **Étape 3 : implémenter**

```tsx
/** Barre des sous-onglets du mode modèles : « Modèles », puis un par widget ouvert. */
import { useRef, type KeyboardEvent } from "react";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { OpenWidget } from "./widgetTabs";
import { widgetLabel } from "./widgetList";

// (interface TemplateTabsProps : recopier celle de « Interfaces » ci-dessus)

export default function TemplateTabs({ tabs, active, unsaved, onActivate, onClose }: TemplateTabsProps) {
  const els = useRef(new Map<number, HTMLButtonElement>());
  /** 0 : « Modèles » ; sinon l'id du sous-onglet. */
  const order = [0, ...tabs.map((tab) => tab.id)];
  const rtl = typeof document !== "undefined" && document.dir === "rtl";
  const go = (key: number) => {
    els.current.get(key)?.focus();
    onActivate(key === 0 ? null : key);
  };
  const onKey = (e: KeyboardEvent, key: number) => {
    const i = order.indexOf(key);
    const step = { ArrowRight: rtl ? -1 : 1, ArrowLeft: rtl ? 1 : -1 }[e.key];
    if (step !== undefined) go(order[(i + step + order.length) % order.length]);
    else if (e.key === "Home") go(order[0]);
    else if (e.key === "End") go(order[order.length - 1]);
    else if (e.key === "Delete" && key !== 0) onClose(key);
    else return;
    e.preventDefault();
  };
  const tabProps = (key: number, selected: boolean) => ({
    role: "tab" as const,
    id: `widget-tab-${key}`,
    "aria-selected": selected,
    tabIndex: selected ? 0 : -1,
    className: "template-tab",
    ref: (el: HTMLButtonElement | null) => {
      if (el) els.current.set(key, el);
      else els.current.delete(key);
    },
    onKeyDown: (e: KeyboardEvent) => onKey(e, key),
  });

  return (
    <div role="tablist" aria-label={t("ui-widgets-tabs")} className="template-tabs">
      <button {...tabProps(0, active === null)} aria-controls="templates-panel" onClick={() => onActivate(null)}>
        {t("ui-widgets-templates-tab")}
      </button>
      {tabs.map((tab) => {
        const label = widgetLabel(tab.name);
        // L'enveloppe sort de l'arbre d'accessibilité ; le ×, caché aux
        // lecteurs d'écran et hors tabulation, laisse la liste aux seuls onglets
        // (au clavier : Suppr).
        return (
          <div key={tab.id} role="presentation" className="template-tab-slot">
            <button
              {...tabProps(tab.id, active === tab.id)}
              aria-controls={`widget-panel-${tab.id}`}
              onClick={() => onActivate(tab.id)}
              onAuxClick={(e) => {
                if (e.button !== 1) return;
                e.preventDefault();
                onClose(tab.id);
              }}
            >
              {label}
              {unsaved.has(tab.name) && <span className="unsaved-dot" role="img" aria-label={t("ui-widgets-unsaved")} />}
            </button>
            <button
              className="token ghost"
              tabIndex={-1}
              aria-hidden
              title={t("ui-widgets-close-tab", { name: label })}
              onClick={() => onClose(tab.id)}
            >
              {Icon.close}
            </button>
          </div>
        );
      })}
    </div>
  );
}
```

Les tests trouvent le × par `.template-tab-slot button[aria-hidden]`.

```css
.template-tabs { display: flex; gap: var(--space-1); overflow-x: auto; scrollbar-width: none; }
.template-tab-slot { display: inline-flex; align-items: center; }
.template-tab { padding: 4px 10px; border: 0; border-bottom: 2px solid transparent; background: transparent; color: inherit; font: 500 13px/20px var(--sans); display: inline-flex; align-items: center; gap: var(--space-2); }
.template-tab[aria-selected="true"] { border-bottom-color: var(--accent); }
.template-tab-slot .token { width: 20px; height: 20px; }
```

- [ ] **Étape 4 : vérifier le vert** — Run : `cd ui && bun test && bun run typecheck && cd .. && cargo nextest run -p oku-slides-i18n`.

- [ ] **Étape 5 : commit**

```bash
git add ui/src/TemplateTabs.tsx ui/src/TemplateTabs.render.test.tsx ui/src/styles.css crates/oku-slides-i18n/src/locales
```
```bash
git commit -m "feat(ui) : barre des sous-onglets du mode modèles (t-394)"
```

---

### Tâche 8 : Assemblage, intégration dans `App` et retrait du panneau

**Fichiers :**
- Créer : `ui/src/useTemplateWidgets.ts`, `ui/src/useTemplateWidgets.render.test.tsx`
- Créer : `ui/src/WidgetPane.tsx`, `ui/src/WidgetPane.render.test.tsx`
- Modifier : `ui/src/App.tsx`
- Supprimer : `ui/src/WidgetWorkbench.tsx`, `ui/src/WidgetWorkbench.render.test.tsx`,
  `ui/src/WidgetWorkbench.trials.render.test.tsx`, `ui/src/WidgetWorkbench.writes.render.test.tsx`,
  `ui/src/test/workbench.ts`
- Modifier : `ui/src/styles.css` (retirer `.workbench*` ; garder `.pill`, `.typst-editor`)
- i18n (20 langues) : retirer `ui-widgets-menu`, `ui-widgets-title`,
  `ui-widgets-close`, `ui-widgets-pick`, `ui-widgets-rename`,
  `ui-widgets-delete` (remplacé par `ui-widgets-delete-named`).
- Modifier la spec : tableau « Découpage » (voir étape 7).

**Interfaces :**
- Consomme : T2 à T7.
- Produit :

```ts
export interface TemplateWidgets {
  files: WidgetFiles;
  tabs: WidgetTabs;
  /** Nom du sous-onglet actif ; `null` : « Modèles » ou hors mode modèles. */
  activeName: string | null;
  /** Pour `useViewOptions` : vide hors mode modèles, dès le même rendu. */
  kept: { widgets: string[]; widget: string | null };
  open(name: string): void;
  activate(id: number | null): void;
  close(id: number): void;
  removed(name: string): void;
  undo(): void;
  redo(): void;
  /** Profondeurs d'annulation de l'éditeur actif. */
  history: EditorHistory;
  commandsOf(id: number): (c: EditorCommands | null) => void;
  historyOf(id: number): (h: EditorHistory) => void;
  /** Compteur : le focus va à la section Widgets. */
  pileFocus: number;
  /** Entre en mode modèles par `enter`, puis ouvre `name` ou met le focus sur la pile. */
  enterAndOpen(enter: () => Promise<boolean>, name: string | null): Promise<void>;
  /** Frappes écrites et file vide, puis `close` (✓). */
  finish(close: () => Promise<unknown>): Promise<void>;
}

export function useTemplateWidgets(
  api: TabApi,
  snap: Snapshot | null,
  on: { changed(): void; error(message: string): void; blocked(): boolean },
): TemplateWidgets;
```

`on.blocked()` : vrai quand une fenêtre de l'agent, l'historique ou une
saisie de champ de slide est ouverte (les raccourcis ⌘W/⌘Z des sous-onglets
se taisent alors).

Règles :
- `pack = snap?.editing && !snap.history ? snap.editing.set : null` ;
  `revision = \`${snap?.templates?.version ?? ""}|${snap?.editing?.base ?? ""}\`` ;
  `files = useWidgetFiles(api, pack, revision, { changed, error, renamed: (f, t) => setTabs((s) => renamed(s, f, t)) })`.
- `pack` change : `tabs = NO_TABS`, reprise réarmée.
- Reprise : quand `pack !== null` et `files.statusesFor === revision` et la
  reprise de ce pack n'est pas faite : `withRestored(tabs, snap.ui.widgets, snap.ui.widget)`
  (fusion : un sous-onglet ouvert avant, par `enterAndOpen`, reste), puis
  élagage, puis reprise marquée faite.
- Élagage : seulement quand `files.statusesFor === revision` (jamais sur des
  statuts d'une autre revision ou d'un autre pack).
- `kept = pack ? kept(tabs) : { widgets: [], widget: null }` calculé dans
  le rendu.
- `close(id)` : `files.flush(nom)` puis `closed`. `removed(name)` : ferme
  son sous-onglet. `enterAndOpen` : si déjà en mode modèles, `open(name)`
  ou focus ; sinon `await enter()`, puis pose `pending = name` (appliqué par
  `opened` dès le rendu suivant où `pack !== null`, avant la reprise ; la
  reprise fusionne).
- Raccourcis (écouteur `keydown` **en capture** sur `window`, posé par le
  hook, actif seulement si `pack !== null`, un sous-onglet widget est
  actif et `!on.blocked()`, `!e.repeat`) :
  - ⌘W / Ctrl+W (`tabShortcut(e, isMac)?.kind === "close-tab"`) : ferme le
    sous-onglet actif, `preventDefault` + `stopPropagation` (le `Workspace`
    ne ferme pas l'onglet du deck) ; une cible `input`, `textarea`,
    `select` hors de l'éditeur (`.typst-editor`) laisse passer ;
  - ⌘Z / ⌘⇧Z hors de l'éditeur (cible pas dans `.typst-editor`) :
    `undo()`/`redo()`, `preventDefault` + `stopPropagation` (l'`App` ne
    lance pas l'annulation du document des modèles).
  Sur « Modèles », rien n'est intercepté.
- `history` : profondeurs par id dans une `useRef(new Map())` ; `setHistory`
  seulement pour l'id actif et si la valeur change ; activer un autre
  sous-onglet relit sa valeur gardée.

`WidgetPane.tsx` (doc : « Les sous-onglets ouverts, montés ; seul l'actif est visible. ») :

```tsx
import type { TabApi } from "./api";
import type { TemplateWidgets } from "./useTemplateWidgets";
import WidgetTab from "./WidgetTab";

export default function WidgetPane({ api, widgets, revision }: { api: TabApi; widgets: TemplateWidgets; revision: string }) {
  return (
    <div className="widget-pane" hidden={widgets.tabs.active === null}>
      {widgets.tabs.tabs.map((tab) => (
        <WidgetTab
          key={tab.id}
          api={api}
          files={widgets.files}
          id={tab.id}
          name={tab.name}
          revision={revision}
          active={tab.id === widgets.tabs.active}
          onOpen={widgets.open}
          onCommands={widgets.commandsOf(tab.id)}
          onHistory={widgets.historyOf(tab.id)}
        />
      ))}
    </div>
  );
}
```

(`commandsOf`/`historyOf` rendent des fonctions stables par id — tenues
dans une `Map` — pour ne pas relancer les effets de l'éditeur.) `revision`
est exposé par `TemplateWidgets` s'il est plus simple de le lire là :
ajouter alors `revision: string` à l'interface et l'enlever des props.

- [ ] **Étape 1 : tests rouges**

`useTemplateWidgets.render.test.tsx` — sonde qui appelle le hook avec un
`snap` construit (`{ editing: { set: "p", base: "b" }, templates: { version: "v" }, history: null, ui: { …, widgets, widget } }`
casté en `Snapshot`) et `memory(...)` ; tests :

1. « reprise : les sous-onglets gardés sont rouverts, l'actif aussi, une fois les statuts lus ».
2. « reprise : un nom absent des statuts est retiré ; aucune source n'est lue pour lui »
   (monter aussi un `WidgetPane` et vérifier les appels `source`).
3. « des statuts d'une autre revision n'élaguent pas » : statuts retenus
   pour la nouvelle revision, anciens sans `kpi` → `kpi` reste ouvert.
4. « sortie du mode modèles : kept vide dès le même rendu ; plus de sous-onglets ».
5. « autre pack : sous-onglets remis à zéro ».
6. « enterAndOpen depuis le deck : le sous-onglet ouvert reste actif après la reprise »
   (`enter` fait passer `snap` en mode modèles avec `ui.widgets = []`).
7. « enterAndOpen sans nom : pileFocus augmente ».
8. « close envoie la frappe en attente du fichier » (log `set` avant fermeture).
9. « removed ferme le sous-onglet ; un renommage garde l'id et le
   sous-onglet n'est pas élagué » (rendre un `React.StrictMode` autour de
   la sonde, renommer un fichier ouvert : `tabs` contient `kpi2` avec le
   même id, à aucun rendu intermédiaire il n'a disparu — relever
   `kept(tabs).widgets` à chaque rendu).
10. « undo/redo appellent les commandes de l'actif seulement ; history suit l'actif ».
11. « ⌘W sur un sous-onglet widget le ferme et ne se propage pas » (un
    écouteur `window` en phase de bulle ne reçoit rien) ; « ⌘W sur Modèles
    se propage » ; « ⌘W avec on.blocked() : se propage ».
12. « ⌘W avec le focus dans l'éditeur ferme aussi le sous-onglet ».
13. « ⌘Z hors de l'éditeur, sous-onglet widget actif : undo de l'éditeur, ne se propage pas ».
14. **Régression** « ✓ : la frappe en attente d'un sous-onglet caché est
    écrite avant close_templates » : deux sous-onglets montés (avec
    `WidgetPane`), frappe dans le premier, activer le second,
    `await w.finish(() => fake.closeTemplates())` avec un `closeTemplates`
    qui écrit `close` dans `log` → `log` = `["set kpi:…", "close"]`.

`WidgetPane.render.test.tsx` :
1. « tous les sous-onglets sont montés ; seul l'actif est visible ».
2. « sur Modèles, le volet entier est caché ».

- [ ] **Étape 2 : vérifier l'échec** — Run : `cd ui && bun test src/useTemplateWidgets.render.test.tsx src/WidgetPane.render.test.tsx`.

- [ ] **Étape 3 : implémenter** `useTemplateWidgets.ts` et `WidgetPane.tsx`
  selon les règles et l'interface ci-dessus. Doc de module de
  `useTemplateWidgets.ts` : « Sous-onglets de widgets du mode modèles, pour `App`. »

- [ ] **Étape 4 : brancher dans `App.tsx`**

1. Retirer : l. 55 (`const WidgetWorkbench = lazy(...)`), l. 382
   (`widgetsOpen`), les lectures de `widgetsOpen` l. 247, 475, 530, la
   définition de `widgetsEntry` (l. ~692) et ses deux usages, le bloc
   `{widgetsOpen && …}` (l. 910-920), la constante `NO_WIDGET_TABS` de T1.
2. Après `say`/`run` :

```tsx
  const widgets = useTemplateWidgets(api, snap, {
    changed: () => void run(api.snapshot()),
    error: (m) => say(m, true),
    blocked: () => !!modal || editing !== null || picking || !!snapRef.current?.history,
  });
  const widgetActive = !!snap?.editing && widgets.tabs.active !== null;
```

   et `useViewOptions(snap?.ui ?? null, current, api.ui, widgets.kept)`.
3. Gardes existantes : l. 247 et l. 530, remplacer `widgetsOpen` par
   `widgetActive` ; l. 475 : `!editing && !picking && !widgetActive`.
4. Sous le `<header>` :
   `{snap.editing && !history && <TemplateTabs tabs={widgets.tabs.tabs} active={widgets.tabs.active} unsaved={widgets.files.unsaved} onActivate={widgets.activate} onClose={widgets.close} />}`.
5. `SlidePile` : enfant
   `{snap.editing && !history && <PileWidgets files={widgets.files} current={widgets.activeName} onOpen={widgets.open} onRemoved={widgets.removed} focus={widgets.pileFocus} />}`.
6. Zone de la scène : envelopper `Stage` dans
   `<div id="templates-panel" role={snap.editing ? "tabpanel" : undefined} aria-labelledby={snap.editing ? "widget-tab-0" : undefined} hidden={widgetActive}>`
   (`Stage` reste monté : sélection et défilement sont gardés) ; à côté,
   `{snap.editing && !history && <WidgetPane api={api} widgets={widgets} revision={…} />}`.
   Le groupe d'ajout du plateau est masqué si `widgetActive`.
7. Boutons Annuler/Rétablir : si `widgetActive`, `onClick` → `widgets.undo()`/`redo()`,
   `disabled` → `widgets.history.undo === 0` / `.redo === 0`, libellés
   `t("ui-toolbar-undo")`/`t("ui-toolbar-redo")`.
8. `finishTemplates` : remplacer `if (await run(api.closeTemplates()))` par

```tsx
    let closed = false;
    await widgets.finish(async () => { closed = !!(await run(api.closeTemplates())); });
    if (closed) {
```

   et appeler `widgets.files.settle()` n'est plus nécessaire ailleurs ;
   `autosave.flush()` reste avant.
9. Menu ☰ du mode modèles : plus d'entrée Widgets. Menu du mode deck : plus
   d'entrée Widgets (elle revient à la tâche 9 sous « Éditer les widgets »).
10. `wc -l ui/src/App.tsx` : moins de 1000.

- [ ] **Étape 5 : retirer le panneau**

```bash
git rm ui/src/WidgetWorkbench.tsx ui/src/WidgetWorkbench.render.test.tsx ui/src/WidgetWorkbench.trials.render.test.tsx ui/src/WidgetWorkbench.writes.render.test.tsx ui/src/test/workbench.ts
```

Contrôler la table « Correspondance des tests retirés » : chaque ligne a
son test vert. Retirer `.workbench*` de `styles.css` et les six clés
Fluent dans les 20 langues.

- [ ] **Étape 6 : vérifier le vert**

Run : `cd ui && bun test && bun run typecheck && bun run build && cd .. && cargo nextest run -p oku-slides-i18n`.

- [ ] **Étape 7 : spec conforme (G9)**

Relire `docs/superpowers/specs/2026-10-03-widgets-onglets-design.md`
(tableau « Découpage », comportements) contre ce qui a été construit ; la
mettre à jour si un détail a changé en route.

- [ ] **Étape 8 : vérifier dans l'app**

`preview_start` (configuration de `.claude/launch.json`) : ouvrir un deck,
☰ → Modèles, créer un widget depuis la pile, taper, voir l'aperçu ; ouvrir
Commun ; ⌘Z dans l'éditeur ; fermer par ⌘W ; ✓. Capture d'écran.

- [ ] **Étape 9 : commit**

```bash
git add -A ui crates/oku-slides-i18n/src/locales docs/superpowers/specs/2026-10-03-widgets-onglets-design.md
```
```bash
git commit -m "feat(ui)! : widgets en sous-onglets du mode modèles, panneau Widgets retiré (t-394)"
```

---

### Tâche 9 : Ouvrir un widget depuis la scène ou le menu d'un deck

Les statuts (et leur sonde) ne sont chargés qu'**au clic** : en mode deck,
l'étiquette sait qu'un élément vient du pack par la liste des fichiers du
pack embarqué, lue sans sonde dans le snapshot.

**Fichiers :**
- Modifier : `crates/oku-slides-session/src/views.rs` (`Snapshot.pack_widgets`), `lib.rs:772-799`
- Test : `crates/oku-slides-session/tests/session.rs`
- Modifier : `ui/src/api.ts` (`Snapshot.pack_widgets: string[]`)
- Modifier : `ui/src/Stage.tsx` (étiquette `.tag`, l. 320) ; test `ui/src/Stage.render.test.tsx`
- Modifier : `ui/src/App.tsx` (`editTemplates` rend un booléen ; menu du mode deck ; props de `Stage`)
- Modifier : `ui/src/styles.css` (`.tag-open`)
- i18n (20 langues) : `ui-widgets-edit` (en « Edit widgets », fr « Éditer
  les widgets ») ; `ui-widgets-open` (en « Open widget { $name } », fr
  « Ouvrir le widget { $name } »).

**Interfaces :**
- Produit :

```rust
// Snapshot :
    /// Widgets du pack en cours (fichiers, sans `_commun`), sans sonde.
    pub pack_widgets: Vec<&'a str>,
```

```ts
// Stage, props ajoutées :
  /** Widgets du pack : leur étiquette ouvre le widget. */
  packWidgets?: ReadonlySet<string>;
  onOpenWidget?: (name: string) => void;
```

- [ ] **Étape 1 : tests rouges**

Rust (`tests/session.rs`) :

```rust
/// Le snapshot nomme les widgets du pack du deck, sans `_commun`.
#[test]
fn snapshot_lists_the_pack_widget_files_without_common() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "_commun", "source": "#let x = 1"}))).unwrap();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_WIDGET}))).unwrap();
    assert_that!(snapshot_json(&mut s)["pack_widgets"]).is_equal_to(json!(["kpi"]));
}
```

(`KPI_WIDGET` : la source de widget valide déjà utilisée par les tests de
widgets de ce fichier ou de `tests/` ; la reprendre, ou écrire
`#import "@local/oku-slides:0.1.0": *` + `#let widget = widget("kpi", …)`
comme dans `widgets_edit_tests.rs`.)

TS (`Stage.render.test.tsx`, avec le montage existant) :
1. « élément sélectionné d'un widget du pack : l'étiquette est un bouton ;
   clic → onOpenWidget(kind), sans déplacement ni sélection changée ».
2. « double-clic sur ce bouton n'ouvre pas l'édition du champ » (`onEdit` non appelé).
3. « élément intégré (`text`) : étiquette simple, pas de bouton ».
4. « double-clic sur l'élément (hors étiquette) édite toujours son champ principal ».

`useTemplateWidgets.render.test.tsx` (déjà là, T8, n°6 et 7) couvre
l'entrée depuis le deck.

- [ ] **Étape 2 : vérifier l'échec** — Run : `cargo nextest run -p oku-slides-session -E 'test(pack_widget)'` et `cd ui && bun test src/Stage.render.test.tsx`.

- [ ] **Étape 3 : implémenter**

Rust, dans la construction du `Snapshot` (`lib.rs`) :

```rust
            pack_widgets: self
                .widget_target()
                .map(|t| t.files.widgets().keys().filter(|n| !n.is_common()).map(|n| n.as_str()).collect())
                .unwrap_or_default(),
```

Si l'emprunt ne passe pas (`widget_target` rend des fichiers clonés),
rendre `Vec<String>` et `pack_widgets: Vec<String>` ; vérifier que
`widget_target` ne lance aucune sonde (il ne lit que les fichiers).

`Stage.tsx`, à la place de `<span className="tag">{tagFor(tg)}</span>` :

```tsx
{sel && !editing && (packWidgets?.has(tg.kind) && onOpenWidget ? (
  <button
    className="tag tag-open"
    aria-label={t("ui-widgets-open", { name: tg.kind })}
    title={t("ui-widgets-open", { name: tg.kind })}
    onPointerDown={(e) => e.stopPropagation()}
    onDoubleClick={(e) => e.stopPropagation()}
    onClick={() => onOpenWidget(tg.kind)}
  >
    {tagFor(tg)}
  </button>
) : (
  <span className="tag">{tagFor(tg)}</span>
))}
```

`App.tsx` :
- `editTemplates` rend `Promise<boolean>` (`return !!(await run(...))` et
  `false` sans pack).
- `const packWidgets = useMemo(() => new Set(snap?.pack_widgets ?? []), [snap?.pack_widgets]);`
- `Stage` : `packWidgets={packWidgets}` et
  `onOpenWidget={(name) => void widgets.enterAndOpen(editTemplates, name)}`.
- Menu du mode deck, à la place de l'ancienne entrée Widgets :
  `{ kind: "item", label: t("ui-widgets-edit"), onSelect: () => void widgets.enterAndOpen(editTemplates, null) }`.

CSS : `.tag-open { cursor: pointer; text-decoration: underline dotted; }`
(dans le style de `.tag`, l. 278).

- [ ] **Étape 4 : vérifier le vert** — Run : `cargo nextest run -p oku-slides-session && cd ui && bun test && bun run typecheck && cd .. && cargo nextest run -p oku-slides-i18n` ; `wc -l ui/src/App.tsx` < 1000.

- [ ] **Étape 5 : vérifier dans l'app** — en mode deck, sélectionner un
  élément d'un widget du pack, cliquer son étiquette → mode modèles,
  sous-onglet ouvert ; ☰ « Éditer les widgets » → mode modèles, focus sur
  la section Widgets.

- [ ] **Étape 6 : commit**

```bash
git add crates/oku-slides-session ui crates/oku-slides-i18n/src/locales
```
```bash
git commit -m "feat(ui) : ouvrir un widget depuis la scène ou le menu d'un deck (t-394)"
```

---

### Tâche 10 : Vérification d'ensemble

- [ ] `just check` (fmt, clippy, nextest) : vert.
- [ ] `cd ui && bun test && bun run typecheck && bun run build` : vert.
- [ ] `cd ui && bun run e2e` : vert (aucun scénario ne vise le panneau retiré).
- [ ] Doublons (G10) : `git grep -n "workbench\|WidgetWorkbench" ui crates` vide ;
  une seule file d'opérations ; `Rewrite` défini une fois ; pas de copie de
  `croppedSvg`/`ordered`.
- [ ] `git diff main -- ui/package.json ui/bun.lock Cargo.toml` vide.
- [ ] Aucun raccourci de menu Tauri ne prend ⌘W (`git grep -n "CmdOrCtrl+W\|accelerator" src-tauri`).
- [ ] Reprise réelle : deux sous-onglets ouverts, quitter, relancer :
  sous-onglets et actif revenus ; glisser l'onglet du deck vers une autre
  fenêtre : sous-onglets suivis ; changer d'onglet de deck puis revenir :
  sous-onglets là (historique d'annulation de l'éditeur perdu, limite
  connue de la spec).

---

## Auto-revue

- Couverture de la spec : décisions 1 (T7, T8), 2 (T5), 3 (T6, T9), 4 (T6),
  5 (T3, T4, T8), 6 (T8, T9), 7 (T1, T8) ; cas limites : historique (T8,
  `pack` nul et `!history`), restauration (T1, T8), pack vide (T6) ;
  retraits (T8) ; tests 1-5 de la spec (T1, T2, T3, T5-T8, régression T8 n°14).
- Noms vérifiés d'une tâche à l'autre : `WidgetFiles`, `Rewrite`,
  `EditorCommands`, `EditorHistory`, `OpenWidget`, `WidgetTabs`,
  `TemplateWidgets`, `MAX_WIDGET_TABS`, `pack_widgets`.
- Relecture d'équipe (14 relecteurs) intégrée : élagage sur la bonne
  revision, reprise fusionnée, normalisation Rust, tests portés et table de
  correspondance, `trial`/`stale` par fichier, `unsavedRef`, historique
  hors remplacements, garde-fous clavier, accessibilité, statuts chargés
  seulement en mode modèles.
