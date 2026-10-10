# Import PPTX, lot 1 — `SourceDeck` et lecteur PPTX : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** une crate `oku-slides-import` qui lit un `.pptx` (octets en mémoire) et rend une `SourceDeck` résolue — géométrie en pt, styles texte calculés par la cascade, couleurs avec leur provenance, tableaux, graphiques, SmartArt, replis, polices — sans projecteur (lots 2 et 3) (t-692).

**Architecture:**
- `source/` : la représentation intermédiaire, neutre vis-à-vis du format (`Serialize` pour les snapshots), exposée telle quelle (`oku_slides_import::source::*`), sans ré-export à plat (`Frame`, `Cell`, `Paragraph`, `Run`, `Series` existent aussi dans `oku-slides-core`/`-chart`).
- `collect.rs` : sorties neutres de toute lecture (assets dédupliqués, avertissements bornés, erreur fatale différée, assainissement des chaînes) — le futur lecteur PDF s'en sert aussi.
- `pptx/` : le lecteur. `package.rs` (zip borné, relations en cache), `xml.rs` (analyse bornée en nœuds **et en profondeur**, accès par nom local, attributs avec leur espace de noms), une étape par sorte de part, `cascade.rs` (seule résolution des styles texte). Couleurs, remplissages et traits sont lus **non résolus** (`ColorSpec`, `FillSpec`, `LineSpec`) et résolus avec le contexte de couleur de la part qui les affiche.
- Point d'entrée unique : `read_pptx(bytes, &ReadLimits) -> Result<SourceDeck, ImportError>`.

**Tech Stack:** Rust 2024, `zip` 8 (workspace), `roxmltree` 0.21 (nouveau au workspace, déjà dans le lockfile), `snafu`, `serde`, `oku-slides-core::asset_id`, `oku-slides-i18n` (20 langues) ; tests `speculoos`, `rstest`, `insta` (json), nextest.

**Spec:** `docs/superpowers/specs/2026-10-09-import-pptx-design.md` (« Représentation intermédiaire », « Lecteur PPTX », « Erreurs », « Tests », lot 1).

**Revues :** v1 → v2 : revue d'équipe du 2026-10-09 (12 relecteurs) et analyse d'un vrai deck (`fixtures/import/alto-wealth.pptx`, PptxGenJS). v2 → v3 : revue de contrôle (12 relecteurs), dont un **montage réel du plan** dans un worktree jetable (clippy `-D warnings` et 257 tests verts après correctifs) ; v3 montée à son tour (rustc 1.98 : clippy propre, 245 tests de l'import verts, LibreOffice compris). Différés au backlog : t-721 à t-732, t-752 à t-755.

## Décisions de la revue (elles priment sur le croquis de la spec)

1. **Attributs et espaces de noms.** `xml::attr_r` ne lit que l'espace des relations (transitional ou strict) ; `attr_str`/`attr_i64`/`attr_f64`/`attr_bool`/`attr_u32` ne lisent que les attributs **sans** espace de noms (`<p:sldId id="256" r:id="rId2"/>`).
2. **Profondeur bornée, deux fois.** (a) `xml::parse` refuse un document de plus de `MAX_XML_DEPTH = 256` niveaux **avant** roxmltree, dont l'analyse est récursive (débordement de pile mesuré entre 2 000 et 4 000 niveaux). (b) `GroupCtx.depth` coupe groupes et `AlternateContent` à `MAX_NESTING = 32` (`Opaque{Unknown("nesting")}` + `NestingTooDeep`) ; un SmartArt dans le dessin d'un SmartArt n'est jamais suivi (`in_drawing`).
3. **Couleurs : provenance complète, résolution tardive partout.** `Origin::Scheme { name, slot, mods }` : `name` tel qu'écrit (`tx1`, `bg1`, `accent1`, `phClr` remplacé), `slot` après `clrMap` (`dk1`…), `mods` dans l'ordre d'application. Couleurs, remplissages et traits se lisent en `ColorSpec`/`FillSpec`/`LineSpec` et sont résolus avec le `ColorCtx` de la part **qui affiche** : le `clrMapOvr` d'une slide atteint le texte, les fonds et les remplissages hérités du layout et du master. `phClr` est remplacé syntaxiquement (`ColorSpec::with_placeholder`) — `ColorCtx` ne porte plus de `ph`.
4. **Un thème par master.** `SourceDeck.themes`, `Master.theme: usize`. Thème absent ou illisible → thème Office intégré + `UnreadablePart`.
5. **Lien placeholder** (`Item.placeholder: PlaceholderLink { idx, layout, master }`) ; les placeholders héritent aussi remplissage, trait et géométrie de leurs parents.
6. **Styles par niveau typés** : `Paragraph { level, style: ParaStyle, runs }`, `Placeholder.levels: Vec<LevelStyle { para, run }>`.
7. **Ajouts pour le vrai deck et les lots suivants** : `RunStyle.char_spacing`, `Run.field`, `Item.alt`, `Table.flags`, `FontFace.{fs_type, subset}`, avertissements `Approximated`, `UnreadablePart`, `UnknownTableStyle`, `UnsafeLink`, `NestingTooDeep`, `Truncated`.
8. **Fatal restreint** : paquet illisible, pas une présentation, protégé, aucun master utilisable, limite dépassée. Slide ou layout illisible → omis/`layout: None` + `UnreadablePart`. *(Précisé à l'exécution :)* dépasser `max_xml_nodes` est `TooLarge` (`error::xml_error` convertit le `NodesLimitReached` de roxmltree) ; les notes (`slide.rs`), `frame.rs::chart_item` et `smartart.rs::parse_part` envoient une erreur `TooLarge` à `cx.fail` au lieu de l'avaler.
9. **Lire une fois.** Relations en cache ; assets dédupliqués par chemin avant lecture ; thème analysé une fois et ses listes de styles indexées (`NodeId`) ; `tableStyles.xml` analysé une fois ; masters dédupliqués par chemin ; nombre d'entrées du zip contrôlé **avant** d'indexer le répertoire central.
10. **Bornes sur les valeurs** (amplification mémoire comprise) : `attr_f64` rejette les non-finis ; échelles de groupe dans `[1e-6, 1e6]`, cadres dans `±1e6` pt, rotation ramenée à `[0, 360)`, rognages tels que `l + r < 1` et `t + b < 1` ; `gridSpan`/`rowSpan` bornés à la grille ; `numCol` dans `[1, 16]` (illisible → 1) ; au plus `MAX_COLOR_MODS = 16` modificateurs par couleur, `MAX_GRADIENT_STOPS = 64`, `MAX_PATHS = 64` sous-tracés et `MAX_PATH_CMDS = 10_000` commandes par géométrie ; noms de police ≤ 128 caractères, caractère de puce ≤ 8 ; `clrMap` limité à ses 12 clés et 12 valeurs ; points de graphique : longueur `min(max(ptCount, idx max + 1), nombre de points présents + 64, 10 000)` (proportionnelle à ce que le fichier contient vraiment), au plus `MAX_SERIES = 256` séries ; liens : `http`, `https`, `mailto`, ≤ 2048 caractères, sans contrôle ni espace (sinon `UnsafeLink`).
11. **`AlternateContent` : la branche est choisie avant d'être lue.** Équation (`oMath` dans `Choice`) → `Opaque { Math, fallback }`. Sinon `Choice` si chaque préfixe de `Requires` désigne un espace de noms connu (`a14`, `p14`, `p15`, `a16`, `asvg`) ; sinon `Fallback` ; sinon `Choice`. La branche écartée n'est jamais analysée (aucun id consommé, aucun asset lu, aucune erreur fatale).
12. **Polices embarquées** : sfnt direct, EOT (`.fntdata` : en-tête, XOR 0x50 ; MicroType → `FontEmbeddedUnreadable`), ODTTF ; `fs_type` (droits d'incorporation) et `subset` (`saveSubsetFonts`) sont gardés pour que les lots suivants n'incorporent pas une police d'aperçu seul ou partielle.
13. **Styles de tableau** absents de `tableStyles.xml` → `UnknownTableStyle` (styles intégrés : t-725).
14. **Pas de `#[non_exhaustive]`** sur la représentation (les projecteurs `match`ent exhaustivement) ; seul `ImportErrorKind` l'est. `ReadLimits` implémente `Default` (valeurs conservatrices), pour qu'une limite ajoutée plus tard ne casse pas les appelants.
15. **Identifiants stables** : `cNvPr@id` lu comme entier (sinon `x`), `dsp:sp@modelId` pour les formes de SmartArt (PowerPoint y écrit `cNvPr id="0"`), suffixe `~2`, `~3`… jusqu'à un id libre.

**Revue finale** (revue de toute la branche, après la tâche 16 ; correctifs appliqués après coup, les blocs de code des tâches ne les reprennent pas) :

16. **Bombe zip relue : paquet épuisé.** `Package::bytes` décompresse au plus `min(max_part_bytes, budget restant) + 1` octets, compte ce qui a été décompressé même en cas d'échec, et après un premier `TooLarge` (drapeau `exhausted`) toute lecture échoue sans rien décompresser ; `PartCtx::asset` garde aussi les échecs par chemin (`Sink::fail_asset`) : ni relecture ni avertissement répété. Régression : `issue_692_bomb_rereads`.
17. **Une seule lecture dégradable des parts liées** : `PartCtx::soft` (limite dépassée → fatal ; autre erreur → `None` + l'avertissement donné) et `PartCtx::linked_doc` (analyse d'une part liée) servent graphique (`UnreadableItem`), SmartArt (`UnreadableItem`, relations du dessin comprises : dessin lu sans ses images) et notes (`UnreadablePart`) ; `part_text` passe aussi par `soft`.
18. **Layouts lus une fois par master** : un layout cité deux fois par les relations d'un même master n'est lu qu'une fois ; partagé entre masters, il reste lu par chacun. Régression : `issue_692_layout_dedup`.
19. **Cellules fusionnées** : bord droit/bas et parties `lastCol`/`lastRow` se lisent à la fin de la fusion (`c + gridSpan == nc`, `r + rowSpan == nr`), plus à l'ancre. Régression : `issue_692_merged_cell_edges`.
20. **`collect::clip`** retire aussi U+061C, U+180E, U+2028, U+2029 et U+FFF9 à U+FFFB. Régression : `issue_692_invisible_chars`.

## Global Constraints

- Crate `crates/oku-slides-import`, déclarée dans `[workspace.dependencies]`. Dépendance : import → core, i18n ; **core ne dépend jamais d'import**.
- Aucune E/S dans la crate : elle reçoit des octets. Les tests d'intégration lisent leurs fixtures avec `std::fs`.
- Pas de `.unwrap()`/`.expect()`/`unreachable!` hors `#[test]` et modules `cfg(test)` ; dans `tests/` (helpers compris), `unwrap_or_else(|e| panic!("…: {e}"))`. Clippy `-D warnings` (dont `too-many-arguments-threshold = 6`), `cargo fmt --all` **à chaque tâche**.
- Unités : EMU → pt = /12 700 ; angles en 60 000ᵉ de degré ; pourcentages en 100 000ᵉ ; `sz`, `spc`, `spcPts` en centièmes de pt.
- `Frame` en **pt source**, absolus (transformations de groupe appliquées), finis, bornés.
- Provenance : `Origin::Scheme { name, slot, mods }`, `slot` ∈ `dk1 lt1 dk2 lt2 accent1..accent6 hlink folHlink` ou `major`/`minor` (polices) ; `Origin::Explicit` ; `Origin::Default`.
- Identifiants d'items : `"<propriétaire>/<id>"`, propriétaire `m<n>`/`l<n>`/`s<n>` ; enfants de SmartArt : propriétaire = id du cadre ; doublon → `~2`, `~3`…
- Assets : `oku_slides_core::asset_id(bytes)`, dédupliqués par chemin puis par contenu. Ce sont des **octets non fiables** : les lots qui les servent ou les affichent les traitent comme tels (t-752).
- Chaînes venues du fichier dans les `Warning` et erreurs : passées par `collect::clip` (≤ 200 caractères, sans contrôle ni caractère de mise en forme bidirectionnelle).
- Fluent : clés `import-*` dans `document.ftl` des **20 langues**.
- Fichiers ≤ 500 lignes. Tests à côté (`foo_tests.rs`, `#[cfg(test)] #[path = "foo_tests.rs"] mod foo_tests;`), noms `<unité>_<scénario>_<résultat>`. Constantes, paquets et contextes de test : **uniquement** dans `src/pptx/test_support.rs` ; les XML de test qui sont aussi des fichiers de fixture sont lus par `include_str!`. Tests d'intégration : `tests/common/mod.rs`. Un bug trouvé en exécutant ce plan → test de régression dans `tests/regressions/` (créé au premier bug : `main.rs` + `issue_692_<sujet>.rs`) — la tâche 2 en crée un pour la profondeur XML.
- Quand un bloc « Ajouter à `X` » apporte des `use`, les fusionner en tête de fichier.
- Snapshots : `INSTA_UPDATE=new cargo nextest run …`, lecture du `.snap.new`, puis `cargo insta accept --manifest-path crates/oku-slides-import/Cargo.toml` ; jamais `INSTA_UPDATE=always`.
- Commits : `feat(import) : … (t-692)` ; message dans un fichier du scratchpad, `git add` des seuls fichiers de la tâche, vérifier l'index (`git diff --cached --name-only` : d'autres sessions indexent sur `main`), puis `git commit -F <fichier>`. Jamais de `Co-Authored-By`. Marquer la sous-tâche Oku `done` après chaque tâche.

## Structure des fichiers

```
crates/oku-slides-import/
  Cargo.toml, ARCHITECTURE.md
  src/lib.rs                       doc, pub mod source, ré-exports read_pptx/ReadLimits/ImportError/ImportErrorKind
  src/error.rs (+_tests)           ImportError(Box<Inner>), message_id, Localize
  src/limits.rs (+_tests)          ReadLimits (+ Default)
  src/collect.rs (+_tests)         Sink, clip, clip_to
  src/source/{mod,style,text,shape,table,chart,font}.rs (+ style_tests, mod_tests)
  src/pptx/mod.rs                  read_pptx
  src/pptx/package.rs (+_tests)    Package<'b>, Rels, resolve_path, declared_entries
  src/pptx/xml.rs (+_tests)        parse borné (nœuds, profondeur), accès
  src/pptx/units.rs
  src/pptx/color.rs (+_tests)      ColorSpec, ColorCtx
  src/pptx/fill.rs (+_tests)       FillSpec, LineSpec
  src/pptx/geometry.rs (+_tests)
  src/pptx/theme.rs (+_tests)      ThemeData<'x>
  src/pptx/text.rs (+_tests), cascade.rs (+_tests)
  src/pptx/placeholder.rs (+_tests), context.rs (+_tests)
  src/pptx/shapes.rs (+_tests), frame.rs
  src/pptx/master.rs (+_tests), slide.rs
  src/pptx/table.rs (+_tests), chart.rs (+_tests), smartart.rs (+_tests), fonts.rs (+_tests), eot.rs (+_tests)
  src/pptx/test_support.rs
  tests/common/mod.rs, tests/read.rs, tests/alto.rs, tests/kitchen.rs, tests/regressions/{main.rs,issue_692_xml_depth.rs}
fixtures/import/README.md          provenance des fixtures
fixtures/import/minimal/…, fixtures/import/kitchen/…   OOXML décompressé écrit à la main
fixtures/import/libreoffice.pptx   `kitchen` réécrit par LibreOffice
fixtures/import/alto-wealth.pptx   vrai deck (PptxGenJS) ; alto-wealth.pdf (LFS) : rendu de référence pour les lots 4 et 6 (aucun test du lot 1 ne le lit)
```

---

### Task 1: Crate et types de la représentation intermédiaire

**Files:**
- Modify: `Cargo.toml` (workspace), `README.md` (tableau des crates)
- Create: `crates/oku-slides-import/{Cargo.toml,ARCHITECTURE.md}`, `src/lib.rs`, `src/source/{mod,style,text,shape,table,chart,font}.rs`, `src/source/{style_tests,mod_tests}.rs`

**Interfaces:**
- Produces : tout `source::*` ci-dessous ; `Rgba::{new, to_hex}`, `Styled::{explicit, scheme, mapped, default_value}`, `Origin::{scheme, slot}`, `Frame::new`, `Fill::is_none`, `TextBox::default()` (1 colonne, retour à la ligne).

- [ ] **Step 1 : workspace, manifeste, README**

`Cargo.toml` racine : après `oku-slides-render-worker = …`, `oku-slides-import = { path = "crates/oku-slides-import" }` ; après `zip = …` :

```toml
# Lecture des parts XML d'un .pptx (import, t-692) : DOM en lecture seule, sans dépendance.
roxmltree = "0.21"
```

`crates/oku-slides-import/Cargo.toml` :

```toml
[package]
name = "oku-slides-import"
description = "Import d'une présentation (PPTX) : représentation intermédiaire et lecteur."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
publish.workspace = true

[dependencies]
oku-slides-core.workspace = true
oku-slides-i18n.workspace = true
roxmltree.workspace = true
serde.workspace = true
snafu.workspace = true
zip.workspace = true

[dev-dependencies]
insta.workspace = true
rstest.workspace = true
serde_json.workspace = true
speculoos.workspace = true

[lints]
workspace = true
```

`README.md` : dans le tableau des crates (lignes 17–25), juste après la ligne de `crates/oku-slides-pdf`, ajouter `crates/oku-slides-import` — « Import PPTX : représentation intermédiaire `SourceDeck` et lecteur » (même format que les voisines).

- [ ] **Step 2 : tests qui échouent**

`src/source/style_tests.rs` :

```rust
use serde_json::json;
use speculoos::prelude::*;

use super::*;

#[test]
fn rgba_opaque_serializes_as_six_digit_hex() {
    assert_that!(serde_json::to_value(Rgba::new(0x1f, 0x4e, 0x79)).unwrap()).is_equal_to(json!("#1f4e79"));
}

#[test]
fn rgba_translucent_serializes_as_eight_digit_hex() {
    let c = Rgba { r: 255, g: 0, b: 0, a: 128 };
    assert_that!(serde_json::to_value(c).unwrap()).is_equal_to(json!("#ff000080"));
}

#[test]
fn styled_scheme_color_serializes_name_and_slot_without_mods() {
    let c = Styled::scheme(Rgba::new(0x44, 0x72, 0xc4), "accent1");
    assert_that!(serde_json::to_value(&c).unwrap())
        .is_equal_to(json!({"value": "#4472c4", "origin": {"scheme": {"name": "accent1", "slot": "accent1"}}}));
}

#[test]
fn styled_mapped_color_serializes_written_name_slot_and_mods() {
    let c = Styled::mapped(Rgba::new(0xff, 0xff, 0xff), "tx1", "lt1", vec![ColorMod::LumMod(0.75)]);
    assert_that!(serde_json::to_value(&c).unwrap()).is_equal_to(json!({
        "value": "#ffffff",
        "origin": {"scheme": {"name": "tx1", "slot": "lt1", "mods": [{"op": "lum_mod", "value": 0.75}]}}
    }));
}

#[test]
fn styled_explicit_color_says_explicit() {
    assert_that!(serde_json::to_value(Styled::explicit(Rgba::new(0, 0, 0))).unwrap())
        .is_equal_to(json!({"value": "#000000", "origin": "explicit"}));
}

#[test]
fn origin_slot_reads_the_mapped_slot_only_for_scheme_values() {
    let o = Origin::Scheme { name: "bg1".into(), slot: "dk1".into(), mods: vec![] };
    assert_that!((o.slot(), Origin::Explicit.slot())).is_equal_to((Some("dk1"), None));
}
```

`src/source/mod_tests.rs` :

```rust
use serde_json::json;
use speculoos::prelude::*;

use super::*;

fn text_item() -> Item {
    Item {
        id: "s1/2".into(),
        name: "Titre 1".into(),
        alt: None,
        frame: Frame::new(10.0, 20.0, 300.0, 40.0),
        z: 0,
        role: Some(Role::Title),
        placeholder: None,
        kind: ItemKind::Text(TextBox::default()),
    }
}

#[test]
fn item_text_serializes_with_kind_tag_and_role() {
    let v = serde_json::to_value(text_item()).unwrap();
    assert_that!((v["kind"]["type"].clone(), v["role"].clone())).is_equal_to((json!("text"), json!("title")));
}

#[test]
fn frame_serializes_position_size_rotation_and_flips() {
    let v = serde_json::to_value(text_item()).unwrap();
    assert_that!(v["frame"])
        .is_equal_to(json!({"x": 10.0, "y": 20.0, "w": 300.0, "h": 40.0, "rot": 0.0, "flip_h": false, "flip_v": false}));
}

#[test]
fn default_textbox_has_one_wrapping_column() {
    let tb = TextBox::default();
    assert_that!((tb.columns, tb.wrap)).is_equal_to((1, true));
}

#[test]
fn asset_serializes_its_length_not_its_bytes() {
    let a = Asset { mime: "image/png".into(), bytes: vec![1, 2, 3] };
    assert_that!(serde_json::to_value(a).unwrap()).is_equal_to(json!({"mime": "image/png", "len": 3}));
}

#[test]
fn fill_none_is_none_and_solid_is_not() {
    assert_that!(Fill::None.is_none()).is_true();
    assert_that!(Fill::Solid { color: Styled::explicit(Rgba::new(1, 2, 3)) }.is_none()).is_false();
}
```

- [ ] **Step 3 : lancer, constater l'échec** — `cargo nextest run -p oku-slides-import` : FAIL à la compilation.

- [ ] **Step 4 : les types**

`src/lib.rs` (état de la tâche 1) :

```rust
//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! Le module [`source`] décrit la représentation : géométrie en pt, styles
//! texte résolus, couleurs avec leur provenance, tableaux, graphiques,
//! SmartArt, replis, polices. Le lecteur arrive aux tâches suivantes.

pub mod source;
```

`src/source/style.rs` :

```rust
//! Couleurs, provenance, remplissages et traits.

use serde::{Serialize, Serializer};

/// Couleur sRGB 8 bits avec opacité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    /// Rouge, 0..255.
    pub r: u8,
    /// Vert, 0..255.
    pub g: u8,
    /// Bleu, 0..255.
    pub b: u8,
    /// Opacité, 0 (transparent) à 255 (opaque).
    pub a: u8,
}

impl Rgba {
    /// Couleur opaque.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// `#rrggbb`, ou `#rrggbbaa` si elle n'est pas opaque.
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

impl Serialize for Rgba {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

/// Modificateur DrawingML appliqué à une couleur de thème ; valeurs en fraction (0.75 = 75 %).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "op", content = "value")]
pub enum ColorMod {
    /// Luminance multipliée (HSL).
    LumMod(f64),
    /// Luminance décalée (HSL).
    LumOff(f64),
    /// Saturation multipliée (HSL).
    SatMod(f64),
    /// Éclaircie vers le blanc (sRGB linéaire).
    Tint(f64),
    /// Assombrie vers le noir (sRGB linéaire).
    Shade(f64),
    /// Opacité.
    Alpha(f64),
}

/// D'où vient une valeur stylée.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Couleur ou police du thème.
    Scheme {
        /// Nom tel qu'écrit (`tx1`, `bg1`, `accent1`… ; `major`/`minor` pour une police).
        name: String,
        /// Emplacement du thème après `clrMap` (`dk1`, `lt1`, `accent1`… ; `major`/`minor`).
        slot: String,
        /// Modificateurs appliqués, dans l'ordre ; vide = la valeur du thème telle quelle.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        mods: Vec<ColorMod>,
    },
    /// Valeur écrite telle quelle dans le fichier (éventuellement modifiée).
    Explicit,
    /// Valeur par défaut du lecteur.
    Default,
}

impl Origin {
    /// Emplacement de thème écrit sous son propre nom, sans modificateur.
    pub fn scheme(slot: &str) -> Self {
        Origin::Scheme { name: slot.to_owned(), slot: slot.to_owned(), mods: Vec::new() }
    }

    /// Emplacement du thème (après `clrMap`), si la valeur en vient.
    pub fn slot(&self) -> Option<&str> {
        match self {
            Origin::Scheme { slot, .. } => Some(slot),
            Origin::Explicit | Origin::Default => None,
        }
    }
}

/// Une valeur résolue et sa provenance.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Styled<T> {
    /// Valeur résolue.
    pub value: T,
    /// Provenance.
    pub origin: Origin,
}

impl<T> Styled<T> {
    /// Valeur écrite telle quelle.
    pub fn explicit(value: T) -> Self {
        Self { value, origin: Origin::Explicit }
    }

    /// Valeur de l'emplacement `slot` du thème, écrit sous ce nom.
    pub fn scheme(value: T, slot: &str) -> Self {
        Self { value, origin: Origin::scheme(slot) }
    }

    /// Valeur du thème écrite `name`, lue dans `slot` après `clrMap`, modifiée par `mods`.
    pub fn mapped(value: T, name: &str, slot: &str, mods: Vec<ColorMod>) -> Self {
        Self { value, origin: Origin::Scheme { name: name.to_owned(), slot: slot.to_owned(), mods } }
    }

    /// Valeur par défaut du lecteur.
    pub fn default_value(value: T) -> Self {
        Self { value, origin: Origin::Default }
    }
}

/// Couleur résolue avec sa provenance.
pub type Color = Styled<Rgba>;

/// Arrêt de dégradé.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GradientStop {
    /// Position, 0..1.
    pub pos: f64,
    /// Couleur à cette position.
    pub color: Color,
}

/// Remplissage d'une forme, d'un fond ou d'une cellule.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Fill {
    /// Rien n'est peint.
    None,
    /// Couleur unie.
    Solid {
        /// Couleur.
        color: Color,
    },
    /// Dégradé linéaire.
    Gradient {
        /// Angle en degrés (0 = de gauche à droite).
        angle: f64,
        /// Arrêts (64 au plus).
        stops: Vec<GradientStop>,
    },
    /// Image étirée.
    Picture {
        /// Identifiant d'asset.
        asset: String,
    },
}

impl Fill {
    /// Vrai si rien n'est peint.
    pub fn is_none(&self) -> bool {
        matches!(self, Fill::None)
    }
}

/// Motif d'un trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Dash {
    /// Continu.
    Solid,
    /// Tirets.
    Dash,
    /// Points.
    Dot,
    /// Tiret-point.
    DashDot,
    /// Tirets longs.
    LongDash,
}

/// Trait (contour, bordure, connecteur).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stroke {
    /// Épaisseur en pt.
    pub width: f64,
    /// Couleur.
    pub color: Color,
    /// Motif.
    pub dash: Dash,
}

#[cfg(test)]
#[path = "style_tests.rs"]
mod style_tests;
```

`src/source/text.rs` :

```rust
//! Texte résolu : boîte, paragraphes, runs, styles par niveau.

use serde::Serialize;

use super::style::{Color, Styled};

/// Marges intérieures d'une boîte de texte, en pt.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Insets {
    /// Gauche.
    pub l: f64,
    /// Haut.
    pub t: f64,
    /// Droite.
    pub r: f64,
    /// Bas.
    pub b: f64,
}

impl Default for Insets {
    /// Défauts OOXML : 7,2 pt à gauche et à droite, 3,6 pt en haut et en bas.
    fn default() -> Self {
        Self { l: 7.2, t: 3.6, r: 7.2, b: 3.6 }
    }
}

/// Ancrage vertical du texte dans sa boîte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    /// En haut.
    #[default]
    Top,
    /// Au milieu.
    Middle,
    /// En bas.
    Bottom,
}

/// Ajustement automatique du texte à sa boîte.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Autofit {
    /// Aucun.
    #[default]
    None,
    /// Le texte rétrécit.
    Shrink {
        /// Échelle de police (fraction).
        font_scale: f64,
        /// Réduction d'interligne (fraction).
        line_reduction: f64,
    },
    /// La boîte grandit avec le texte.
    Resize,
}

/// Alignement horizontal d'un paragraphe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    /// À gauche.
    #[default]
    Left,
    /// Centré.
    Center,
    /// À droite.
    Right,
    /// Justifié.
    Justify,
}

/// Interligne ou espacement.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    /// Proportion de la hauteur de ligne (1.0 = simple).
    Percent(f64),
    /// Valeur absolue en pt.
    Points(f64),
}

/// Puce d'un paragraphe.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Bullet {
    /// Pas de puce.
    #[default]
    None,
    /// Caractère.
    Char {
        /// Le caractère (8 au plus).
        char: String,
        /// Couleur ; absente = celle du texte.
        color: Option<Color>,
        /// Taille relative au texte (1.0 = même taille).
        size_pct: f64,
        /// Police ; absente = celle du texte.
        font: Option<String>,
    },
    /// Numérotation automatique.
    AutoNum {
        /// Schéma OOXML (`arabicPeriod`, `alphaLcParenR`…).
        scheme: String,
        /// Premier numéro.
        start: u32,
    },
}

/// Majuscules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Caps {
    /// Telles qu'écrites.
    #[default]
    None,
    /// Toutes en capitales.
    All,
    /// Petites capitales.
    Small,
}

/// Style résolu d'un run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunStyle {
    /// Famille (128 caractères au plus) et provenance (`major`/`minor` pour le thème).
    pub font: Styled<String>,
    /// Taille en pt.
    pub size: f64,
    /// Gras.
    pub bold: bool,
    /// Italique.
    pub italic: bool,
    /// Souligné.
    pub underline: bool,
    /// Barré.
    pub strike: bool,
    /// Couleur du texte.
    pub color: Color,
    /// Surlignage.
    pub highlight: Option<Color>,
    /// Décalage de ligne de base en % (positif = exposant).
    pub baseline: i32,
    /// Majuscules.
    pub caps: Caps,
    /// Espacement entre caractères, en pt (`spc`).
    pub char_spacing: f64,
    /// Lien hypertexte externe (`http`, `https`, `mailto` seulement).
    pub link: Option<String>,
}

/// Morceau de texte au style uniforme ; un saut de ligne `a:br` est un run `"\n"`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Run {
    /// Texte (valeur en cache pour un champ).
    pub text: String,
    /// Champ dynamique (`slidenum`, `datetime1`…).
    pub field: Option<String>,
    /// Style résolu.
    pub style: RunStyle,
}

/// Style résolu d'un paragraphe.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParaStyle {
    /// Alignement.
    pub align: Align,
    /// Retrait de première ligne en pt (négatif = suspendu).
    pub indent: f64,
    /// Marge gauche en pt.
    pub margin_left: f64,
    /// Interligne.
    pub line_spacing: Spacing,
    /// Espace avant.
    pub space_before: Spacing,
    /// Espace après.
    pub space_after: Spacing,
    /// Puce.
    pub bullet: Bullet,
}

/// Paragraphe résolu.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Paragraph {
    /// Niveau 0..8 (`lvl`).
    pub level: u8,
    /// Style du paragraphe.
    pub style: ParaStyle,
    /// Runs, dans l'ordre.
    pub runs: Vec<Run>,
}

/// Style que prend un texte saisi à un niveau donné d'un placeholder.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LevelStyle {
    /// Style de paragraphe.
    pub para: ParaStyle,
    /// Style de run.
    pub run: RunStyle,
}

/// Boîte de texte résolue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextBox {
    /// Paragraphes, dans l'ordre.
    pub paragraphs: Vec<Paragraph>,
    /// Marges intérieures.
    pub insets: Insets,
    /// Ancrage vertical.
    pub anchor: Anchor,
    /// Ajustement automatique.
    pub autofit: Autofit,
    /// Retour à la ligne automatique.
    pub wrap: bool,
    /// Colonnes de texte, 1..16.
    pub columns: u32,
}

impl Default for TextBox {
    /// Boîte vide : une colonne, retour à la ligne, marges OOXML.
    fn default() -> Self {
        Self {
            paragraphs: Vec::new(),
            insets: Insets::default(),
            anchor: Anchor::Top,
            autofit: Autofit::None,
            wrap: true,
            columns: 1,
        }
    }
}
```

`src/source/shape.rs` :

```rust
//! Formes, géométries et images.

use std::collections::BTreeMap;

use serde::Serialize;

use super::style::{Fill, Stroke};
use super::text::TextBox;

/// Commande de tracé ; coordonnées en fraction (0..1) de la boîte de l'item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "cmd")]
pub enum PathCmd {
    /// Déplacement sans tracé.
    MoveTo { x: f64, y: f64 },
    /// Segment.
    LineTo { x: f64, y: f64 },
    /// Courbe de Bézier cubique.
    CubicTo { x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64 },
    /// Courbe de Bézier quadratique.
    QuadTo { x1: f64, y1: f64, x: f64, y: f64 },
    /// Arc OOXML : rayons (fraction de la boîte), départ et balayage en degrés.
    ArcTo { wr: f64, hr: f64, start: f64, swing: f64 },
    /// Fermeture du sous-tracé.
    Close,
}

/// Géométrie d'une forme.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Geometry {
    /// Forme prédéfinie OOXML.
    Preset {
        /// Nom (`rect`, `roundRect`, `line`…).
        name: String,
        /// Ajustements (`adj`, `adj1`…) en 100 000ᵉ.
        adj: BTreeMap<String, f64>,
    },
    /// Tracé libre (`custGeom`).
    Path {
        /// Sous-tracés (64 au plus ; 10 000 commandes au plus au total).
        paths: Vec<Vec<PathCmd>>,
    },
}

/// Forme dessinée, éventuellement porteuse de texte.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shape {
    /// Géométrie.
    pub geom: Geometry,
    /// Remplissage.
    pub fill: Fill,
    /// Contour, s'il est peint.
    pub stroke: Option<Stroke>,
    /// Texte, s'il n'est pas vide.
    pub text: Option<TextBox>,
}

/// Rognage d'une image, en fraction de chaque bord (`l + r < 1`, `t + b < 1`).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct Crop {
    /// Gauche.
    pub l: f64,
    /// Haut.
    pub t: f64,
    /// Droite.
    pub r: f64,
    /// Bas.
    pub b: f64,
}

/// Image posée.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Picture {
    /// Identifiant d'asset.
    pub asset: String,
    /// Rognage.
    pub crop: Crop,
}
```

> Les champs des variantes de `PathCmd` sont des coordonnées (`x`, `y`, points de contrôle `x1`…) : une doc par variante suffit, la doc de l'enum en donne l'unité.

`src/source/table.rs` :

```rust
//! Tableaux résolus (style de tableau appliqué).

use serde::Serialize;

use super::style::{Fill, Stroke};
use super::text::TextBox;

/// Bordures d'une cellule.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Borders {
    /// Gauche.
    pub l: Option<Stroke>,
    /// Haut.
    pub t: Option<Stroke>,
    /// Droite.
    pub r: Option<Stroke>,
    /// Bas.
    pub b: Option<Stroke>,
}

/// Cellule.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cell {
    /// Texte (vide si la cellule est couverte).
    pub text: TextBox,
    /// Remplissage.
    pub fill: Fill,
    /// Bordures.
    pub borders: Borders,
    /// Lignes couvertes, 1.. (bornées à la grille).
    pub row_span: u32,
    /// Colonnes couvertes, 1.. (bornées à la grille).
    pub col_span: u32,
    /// Couverte par la fusion d'une autre cellule.
    pub covered: bool,
}

/// Options de mise en forme du tableau (`tblPr`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct TableFlags {
    /// Première ligne mise en valeur (en-tête).
    pub first_row: bool,
    /// Dernière ligne mise en valeur (total).
    pub last_row: bool,
    /// Première colonne mise en valeur.
    pub first_col: bool,
    /// Dernière colonne mise en valeur.
    pub last_col: bool,
    /// Lignes en bandes.
    pub band_row: bool,
    /// Colonnes en bandes.
    pub band_col: bool,
}

/// Tableau.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Table {
    /// Largeur des colonnes, en pt.
    pub col_widths: Vec<f64>,
    /// Hauteur des lignes, en pt.
    pub row_heights: Vec<f64>,
    /// Options de mise en forme.
    pub flags: TableFlags,
    /// Cellules, ligne par ligne.
    pub rows: Vec<Vec<Cell>>,
}
```

`src/source/chart.rs` :

```rust
//! Graphiques : type et données lues dans le cache du XML.

use serde::Serialize;

use super::style::Color;

/// Type de graphique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    /// Barres verticales.
    Column,
    /// Barres horizontales.
    Bar,
    /// Courbes.
    Line,
    /// Secteurs.
    Pie,
    /// Anneau.
    Doughnut,
    /// Aires.
    Area,
    /// Nuage de points.
    Scatter,
    /// Radar.
    Radar,
}

/// Empilement des séries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Grouping {
    /// Côte à côte.
    #[default]
    Clustered,
    /// Empilées.
    Stacked,
    /// Empilées à 100 %.
    PercentStacked,
    /// Sans empilement (courbes, aires).
    Standard,
}

/// Une série.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Series {
    /// Nom.
    pub name: String,
    /// Valeurs ; absente ou non numérique = `None`.
    pub values: Vec<Option<f64>>,
    /// Couleur explicite de la série.
    pub color: Option<Color>,
}

/// Graphique.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chart {
    /// Type (le premier, pour un graphique combiné).
    pub kind: ChartKind,
    /// Empilement.
    pub grouping: Grouping,
    /// Titre.
    pub title: Option<String>,
    /// Libellés des catégories (les X d'un nuage de points, en texte).
    pub categories: Vec<String>,
    /// Séries (256 au plus).
    pub series: Vec<Series>,
}
```

`src/source/font.rs` :

```rust
//! Polices utilisées et embarquées, assets binaires.

use serde::{Serialize, Serializer};

/// Sérialise la longueur d'un contenu binaire, pas son contenu.
fn len_of<S: Serializer>(data: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u64(data.len() as u64)
}

/// Graisse et style d'une face embarquée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FaceStyle {
    /// Normal.
    Regular,
    /// Gras.
    Bold,
    /// Italique.
    Italic,
    /// Gras italique.
    BoldItalic,
}

/// Face embarquée lisible (sfnt : TrueType ou OpenType).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FontFace {
    /// Graisse et style.
    pub style: FaceStyle,
    /// Droits d'incorporation (`fsType` OS/2) ; `None` s'ils ne se lisent pas.
    pub fs_type: Option<u16>,
    /// La présentation n'embarque que les glyphes utilisés (`saveSubsetFonts`).
    pub subset: bool,
    /// Données sfnt (octets non fiables).
    #[serde(rename = "len", serialize_with = "len_of")]
    pub data: Vec<u8>,
}

/// Famille utilisée par le texte du deck ; `faces` vide si rien n'est embarqué.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceFont {
    /// Nom de famille.
    pub family: String,
    /// Faces embarquées lisibles.
    pub faces: Vec<FontFace>,
}

/// Contenu d'un asset (image) ; la clé de la table est son `asset_id`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Asset {
    /// Type, déduit de l'extension (à vérifier avant tout usage : octets non fiables).
    pub mime: String,
    /// Octets.
    #[serde(rename = "len", serialize_with = "len_of")]
    pub bytes: Vec<u8>,
}
```

`src/source/mod.rs` :

```rust
//! La représentation intermédiaire : un deck source résolu, neutre vis-à-vis du format.
//!
//! Tout est concret (plus d'héritage à calculer) ; les valeurs stylées gardent
//! leur provenance. Les `Frame` sont en pt source, absolus. Le vocabulaire
//! OOXML (noms de presets, types de layout) y reste : c'est la langue commune
//! des lecteurs (un lecteur PDF émet `Geometry::Path`, `Origin::Explicit`,
//! `Layout.kind = "cust"`). Les projecteurs `match`ent exhaustivement : pas
//! de `#[non_exhaustive]` ici.

mod chart;
mod font;
mod shape;
mod style;
mod table;
mod text;

use std::collections::BTreeMap;

use serde::Serialize;

pub use chart::{Chart, ChartKind, Grouping, Series};
pub use font::{Asset, FaceStyle, FontFace, SourceFont};
pub use shape::{Crop, Geometry, PathCmd, Picture, Shape};
pub use style::{Color, ColorMod, Dash, Fill, GradientStop, Origin, Rgba, Stroke, Styled};
pub use table::{Borders, Cell, Table, TableFlags};
pub use text::{
    Align, Anchor, Autofit, Bullet, Caps, Insets, LevelStyle, ParaStyle, Paragraph, Run, RunStyle, Spacing, TextBox,
};

/// Taille de page, en pt.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Size {
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
}

/// Boîte d'un item : position et taille en pt (page), rotation, miroirs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Frame {
    /// Bord gauche.
    pub x: f64,
    /// Bord haut.
    pub y: f64,
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
    /// Rotation en degrés, sens horaire, dans `[0, 360)`.
    pub rot: f64,
    /// Miroir horizontal.
    pub flip_h: bool,
    /// Miroir vertical.
    pub flip_v: bool,
}

impl Frame {
    /// Boîte sans rotation ni miroir.
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h, rot: 0.0, flip_h: false, flip_v: false }
    }
}

/// Rôle d'un placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Titre (`title`, `ctrTitle`).
    Title,
    /// Sous-titre.
    Subtitle,
    /// Corps de texte.
    Body,
    /// Contenu quelconque (`obj`, défaut).
    Object,
    /// Image.
    Picture,
    /// Graphique.
    Chart,
    /// Tableau.
    Table,
    /// Média.
    Media,
    /// Date.
    Date,
    /// Pied de page.
    Footer,
    /// Numéro de slide.
    SlideNum,
}

/// Thème source.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceTheme {
    /// Nom du thème.
    pub name: String,
    /// Couleurs par emplacement : `dk1 lt1 dk2 lt2 accent1..accent6 hlink folHlink`.
    pub colors: BTreeMap<String, Rgba>,
    /// Polices.
    pub fonts: ThemeFonts,
}

/// Polices du thème (écriture latine).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeFonts {
    /// Titres.
    pub major: String,
    /// Corps.
    pub minor: String,
}

/// Placeholder d'un master ou d'un layout.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Placeholder {
    /// Identifiant d'item.
    pub id: String,
    /// Rôle.
    pub role: Role,
    /// Index (`ph@idx`).
    pub idx: Option<u32>,
    /// Boîte.
    pub frame: Frame,
    /// Style d'un texte saisi à chaque niveau 0..8.
    pub levels: Vec<LevelStyle>,
    /// Texte d'invite.
    pub text: TextBox,
}

/// Master.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Master {
    /// Identifiant (`m1`…).
    pub id: String,
    /// Nom.
    pub name: String,
    /// Index de son thème dans `SourceDeck.themes`.
    pub theme: usize,
    /// Fond, résolu avec les couleurs du master.
    pub background: Fill,
    /// Items hors placeholders.
    pub decor: Vec<Item>,
    /// Placeholders.
    pub placeholders: Vec<Placeholder>,
}

/// Layout.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Layout {
    /// Identifiant (`l1`…).
    pub id: String,
    /// Nom.
    pub name: String,
    /// Attribut `type` brut (`title`, `secHead`, `obj`, `twoObj`, `titleOnly`, `blank`, `cust`…).
    pub kind: String,
    /// Identifiant de son master.
    pub master: String,
    /// Fond propre, résolu avec les couleurs du layout ; `None` = celui du master.
    pub background: Option<Fill>,
    /// Le décor du master est-il dessiné sous ce layout (`showMasterSp`) ?
    pub show_master_decor: bool,
    /// Items hors placeholders.
    pub decor: Vec<Item>,
    /// Placeholders.
    pub placeholders: Vec<Placeholder>,
}

/// Slide.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceSlide {
    /// Identifiant (`s1`…).
    pub id: String,
    /// Numéro 1.. dans l'ordre de la présentation.
    pub number: u32,
    /// Identifiant de son layout, s'il est lisible.
    pub layout: Option<String>,
    /// Fond résolu avec les couleurs de la slide (fond hérité compris).
    pub background: Fill,
    /// Le décor du layout (et du master) est-il dessiné sous la slide ?
    pub show_layout_decor: bool,
    /// Masquée en diaporama.
    pub hidden: bool,
    /// Items, dans l'ordre de dessin.
    pub items: Vec<Item>,
    /// Notes, en texte brut.
    pub notes: String,
}

/// Placeholders parents d'un item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaceholderLink {
    /// Index du placeholder (`ph@idx`).
    pub idx: Option<u32>,
    /// Id du placeholder de layout rempli.
    pub layout: Option<String>,
    /// Id du placeholder de master correspondant.
    pub master: Option<String>,
}

/// Pourquoi un item n'a pas été traduit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpaqueReason {
    /// Vidéo ou son.
    Media,
    /// Objet OLE.
    Ole,
    /// Équation.
    Math,
    /// SmartArt sans version dessinée lisible.
    SmartArt,
    /// Autre (URI du `graphicData`, `nesting`…).
    Unknown(String),
}

/// Item intraduisible.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Opaque {
    /// Raison.
    pub reason: OpaqueReason,
    /// Asset de l'image de repli fournie par le fichier.
    pub fallback: Option<String>,
}

/// Sorte d'item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ItemKind {
    /// Boîte de texte sans remplissage ni trait.
    Text(TextBox),
    /// Forme dessinée (en boîte : de loin la plus grosse variante).
    Shape(Box<Shape>),
    /// Image.
    Picture(Picture),
    /// Tableau.
    Table(Table),
    /// Graphique.
    Chart(Chart),
    /// Groupe (ou SmartArt dessiné).
    Group {
        /// Enfants, dans l'ordre de dessin.
        items: Vec<Item>,
    },
    /// Intraduisible.
    Opaque(Opaque),
}

/// Élément posé sur une slide, un layout ou un master.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    /// Identifiant unique dans le deck (`s3/12`, `s3/12/{GUID}`, `s3/12~2`).
    pub id: String,
    /// Nom donné dans l'éditeur (`cNvPr@name`).
    pub name: String,
    /// Texte alternatif (`cNvPr@descr`).
    pub alt: Option<String>,
    /// Boîte.
    pub frame: Frame,
    /// Rang de dessin dans son parent (0 = au fond).
    pub z: u32,
    /// Rôle, si c'est un placeholder.
    pub role: Option<Role>,
    /// Placeholders parents.
    pub placeholder: Option<PlaceholderLink>,
    /// Contenu.
    pub kind: ItemKind,
}

/// Ce que le lecteur a ignoré, approché ou dû replier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum WarningKind {
    /// Effet non traduit (`effectLst`, `scene3d`, `sp3d`, rotation ou miroir de groupe).
    EffectIgnored(String),
    /// Rendu approché (`pattFill`, dégradé non linéaire, `txXfrm`, valeurs tronquées).
    Approximated(String),
    /// Item illisible, retiré.
    UnreadableItem(String),
    /// Part de structure illisible (slide omise, layout absent, thème par défaut).
    UnreadablePart(String),
    /// Part référencée absente du paquet.
    MissingPart(String),
    /// Graphique non géré, ou combiné (seul le premier type est lu).
    UnsupportedChart(String),
    /// Image dans un format que Typst ne dessine pas (`emf`, `wmf`, `tiff`, `wdp`) : indication pour les projecteurs.
    UnsupportedImage(String),
    /// Style de tableau référencé mais absent de `tableStyles.xml`.
    UnknownTableStyle(String),
    /// Lien refusé (schéma, longueur, caractères).
    UnsafeLink(String),
    /// Imbrication trop profonde, coupée.
    NestingTooDeep(String),
    /// Police embarquée illisible (EOT compressé…).
    FontEmbeddedUnreadable(String),
    /// Trop d'avertissements : les suivants sont omis.
    Truncated,
}

/// Avertissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    /// Numéro de slide concernée.
    pub slide: Option<u32>,
    /// Id de l'item concerné.
    pub item: Option<String>,
    /// Nature.
    pub kind: WarningKind,
}

/// Le deck source résolu.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceDeck {
    /// Taille de page.
    pub size: Size,
    /// Un thème par master déclaré (même index).
    pub themes: Vec<SourceTheme>,
    /// Masters lisibles.
    pub masters: Vec<Master>,
    /// Layouts lisibles.
    pub layouts: Vec<Layout>,
    /// Slides lisibles.
    pub slides: Vec<SourceSlide>,
    /// Assets par identifiant.
    pub assets: BTreeMap<String, Asset>,
    /// Familles utilisées ou embarquées, triées.
    pub fonts: Vec<SourceFont>,
    /// Avertissements (1 000 au plus, puis `Truncated`).
    pub warnings: Vec<Warning>,
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
```

`ARCHITECTURE.md` :

```markdown
# oku-slides-import

Import d'une présentation PowerPoint (`.pptx`) en représentation
intermédiaire, `source::SourceDeck` : neutre vis-à-vis du format, résolue,
chaque valeur stylée avec sa provenance. Spec :
`docs/superpowers/specs/2026-10-09-import-pptx-design.md`.

## API

- `read_pptx(bytes, &ReadLimits) -> Result<SourceDeck, ImportError>` : fatal
  seulement pour un paquet illisible, pas une présentation, protégé, sans
  master utilisable, ou une limite dépassée.
- `source` : les types de la représentation, sans ré-export à plat.

## Couches

- `source/` ne dépend de rien ; `error.rs` et `collect.rs` (assets,
  avertissements, fatal différé, assainissement) sont neutres vis-à-vis du format.
- `pptx/` (et demain `pdf/`) dépend de `source/`, `error.rs`, `collect.rs`.
  Dans `pptx/`, `shapes` → `frame` → `smartart` → `shapes` forment un cycle
  borné par `GroupCtx.depth` et `in_drawing`.
- Projecteurs (lots 2-3) : `source/` + `oku-slides-core`, jamais `pptx/`.
  `oku-slides-core` ne dépend jamais de cette crate.

## Points d'extension

- Nouveau format source (PDF, lot 6) : un module frère de `pptx/` qui produit
  la même `SourceDeck` et réutilise `collect::Sink`.

## Dépendances

`zip`, `roxmltree`, `oku-slides-core` (`asset_id`), `oku-slides-i18n`.
```

- [ ] **Step 5 : lancer, constater le succès** — `cargo nextest run -p oku-slides-import` : 11 tests PASS.

- [ ] **Step 6 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add Cargo.toml Cargo.lock README.md crates/oku-slides-import
git diff --cached --name-only   # seulement ces fichiers
git commit -F <scratchpad>/msg.txt   # feat(import) : crate oku-slides-import et types de la représentation intermédiaire (t-692)
oku cli task done --project slider --id t-706
```

---

### Task 2: Erreurs, limites, sorties communes, analyse XML bornée et paquet OPC

**Files:**
- Create: `src/error.rs` (+`_tests`), `src/limits.rs` (+`_tests`), `src/collect.rs` (+`_tests`), `src/pptx/mod.rs`, `src/pptx/package.rs` (+`_tests`), `src/pptx/xml.rs` (analyse) + `src/pptx/xml_tests.rs`, `src/pptx/test_support.rs`, `tests/common/mod.rs`
- Modify: `src/lib.rs`, `crates/oku-slides-i18n/src/locales/*/document.ftl` (20 fichiers)

**Interfaces:**
- Produces :
  - `ReadLimits { max_input_bytes: u64, max_part_bytes: u64, max_total_bytes: u64, max_parts: usize, max_xml_nodes: u32, max_slides: usize }` + `Default` (256 Mio, 64 Mio, 512 Mio, 10 000, 5 000 000, 2 000).
  - `ImportError(Box<Inner>)`, `kind() -> ImportErrorKind` (`#[non_exhaustive]` : `Unreadable`, `NotAPresentation`, `Protected`, `TooLarge`) ; `error::message_id(kind)` (crate) ; `impl Localize`. Constructeurs crate : `zip_error`, `io_error`, `utf8_error`, `xml_error`, `structure`, `too_large`, `not_a_presentation`, `protected`.
  - `collect::Sink` : `new`, `warn(slide, item, kind)` (1 000 puis `Truncated`), `cached_asset(path)`, `intern(path, bytes, mime) -> String`, `set_fatal(e)`, `has_fatal()`, `finish(self) -> Result<(BTreeMap<String, Asset>, Vec<Warning>), ImportError>` ; `collect::clip(s) -> String` (200 car.), `collect::clip_to(s, n) -> String`.
  - `pptx::xml::{parse(text, part, nodes) -> Result<Document, ImportError>, MAX_XML_DEPTH = 256}`.
  - `pptx::package::Package<'b>` : `open(bytes, limits)`, `limits()`, `main_part()`, `exists(part)`, `bytes(part)`, `text(part)`, `rels(part) -> Result<Rc<Rels>, _>` (cache), `content_type(part)` ; `Rels::{target, by_type, external}` ; `resolve_path(base, target)` (décode les `%XX`) ; `declared_entries(bytes) -> Option<u64>`.
  - Tests (`pptx::test_support`) : `LIMITS`, `CT`, `ROOT_RELS`, `zip_of`, `TestPkg::{with, bytes, pkg}`. Intégration (`tests/common`) : `LIMITS = ReadLimits::default()`, `pptx(name)`, `pptx_with(name, &[(path, Option<&str>)])`, `file(name)`.

- [ ] **Step 1 : messages Fluent (20 langues)**

Ajouter en fin de chaque `crates/oku-slides-i18n/src/locales/<langue>/document.ftl`, après une ligne vide, les quatre lignes de la table (`<br>` = saut de ligne). En arabe, `.ppt`/`.pptx` sont écrits sans marque de direction, comme dans `cli.ftl`.

| langue | lignes |
|---|---|
| en | `import-unreadable = This presentation could not be read`<br>`import-not-a-presentation = This file is not a PowerPoint presentation`<br>`import-protected = This presentation is encrypted or in the old .ppt format: save it as .pptx first`<br>`import-too-large = This presentation is too large to import` |
| fr | `import-unreadable = Cette présentation est illisible`<br>`import-not-a-presentation = Ce fichier n'est pas une présentation PowerPoint`<br>`import-protected = Cette présentation est chiffrée ou à l'ancien format .ppt : enregistrez-la d'abord en .pptx`<br>`import-too-large = Cette présentation est trop volumineuse pour être importée` |
| de | `import-unreadable = Diese Präsentation kann nicht gelesen werden`<br>`import-not-a-presentation = Diese Datei ist keine PowerPoint-Präsentation`<br>`import-protected = Diese Präsentation ist verschlüsselt oder im alten .ppt-Format: Speichern Sie sie zuerst als .pptx`<br>`import-too-large = Diese Präsentation ist zu groß für den Import` |
| es | `import-unreadable = No se puede leer esta presentación`<br>`import-not-a-presentation = Este archivo no es una presentación de PowerPoint`<br>`import-protected = Esta presentación está cifrada o en el antiguo formato .ppt: guárdela primero como .pptx`<br>`import-too-large = Esta presentación es demasiado grande para importarla` |
| it | `import-unreadable = Impossibile leggere questa presentazione`<br>`import-not-a-presentation = Questo file non è una presentazione PowerPoint`<br>`import-protected = Questa presentazione è crittografata o nel vecchio formato .ppt: salvala prima come .pptx`<br>`import-too-large = Questa presentazione è troppo grande per essere importata` |
| pt | `import-unreadable = Não foi possível ler esta apresentação`<br>`import-not-a-presentation = Este arquivo não é uma apresentação do PowerPoint`<br>`import-protected = Esta apresentação está criptografada ou no antigo formato .ppt: salve-a primeiro como .pptx`<br>`import-too-large = Esta apresentação é grande demais para ser importada` |
| nl | `import-unreadable = Deze presentatie kan niet worden gelezen`<br>`import-not-a-presentation = Dit bestand is geen PowerPoint-presentatie`<br>`import-protected = Deze presentatie is versleuteld of in het oude .ppt-formaat: sla haar eerst op als .pptx`<br>`import-too-large = Deze presentatie is te groot om te importeren` |
| pl | `import-unreadable = Nie można odczytać tej prezentacji`<br>`import-not-a-presentation = Ten plik nie jest prezentacją PowerPoint`<br>`import-protected = Ta prezentacja jest zaszyfrowana lub w starym formacie .ppt: najpierw zapisz ją jako .pptx`<br>`import-too-large = Ta prezentacja jest zbyt duża, aby ją zaimportować` |
| ru | `import-unreadable = Не удаётся прочитать эту презентацию`<br>`import-not-a-presentation = Этот файл не является презентацией PowerPoint`<br>`import-protected = Эта презентация зашифрована или сохранена в старом формате .ppt: сначала сохраните её как .pptx`<br>`import-too-large = Эта презентация слишком велика для импорта` |
| uk | `import-unreadable = Не вдається прочитати цю презентацію`<br>`import-not-a-presentation = Цей файл не є презентацією PowerPoint`<br>`import-protected = Ця презентація зашифрована або має старий формат .ppt: спершу збережіть її як .pptx`<br>`import-too-large = Ця презентація завелика для імпорту` |
| tr | `import-unreadable = Bu sunu okunamıyor`<br>`import-not-a-presentation = Bu dosya bir PowerPoint sunusu değil`<br>`import-protected = Bu sunu şifreli veya eski .ppt biçiminde: önce .pptx olarak kaydedin`<br>`import-too-large = Bu sunu içe aktarılamayacak kadar büyük` |
| ar | `import-unreadable = تعذّرت قراءة هذا العرض التقديمي`<br>`import-not-a-presentation = هذا الملف ليس عرضًا تقديميًا من PowerPoint`<br>`import-protected = هذا العرض التقديمي مشفّر أو بتنسيق .ppt القديم: احفظه أولًا بتنسيق .pptx`<br>`import-too-large = هذا العرض التقديمي أكبر من أن يُستورد` |
| hi | `import-unreadable = यह प्रस्तुति पढ़ी नहीं जा सकी`<br>`import-not-a-presentation = यह फ़ाइल PowerPoint प्रस्तुति नहीं है`<br>`import-protected = यह प्रस्तुति एन्क्रिप्टेड है या पुराने .ppt फ़ॉर्मैट में है: पहले इसे .pptx के रूप में सहेजें`<br>`import-too-large = यह प्रस्तुति आयात करने के लिए बहुत बड़ी है` |
| bn | `import-unreadable = এই উপস্থাপনাটি পড়া যায়নি`<br>`import-not-a-presentation = এই ফাইলটি PowerPoint উপস্থাপনা নয়`<br>`import-protected = এই উপস্থাপনাটি এনক্রিপ্ট করা বা পুরোনো .ppt ফরম্যাটে: আগে এটি .pptx হিসেবে সংরক্ষণ করুন`<br>`import-too-large = এই উপস্থাপনাটি ইমপোর্ট করার জন্য খুব বড়` |
| ja | `import-unreadable = このプレゼンテーションを読み込めません`<br>`import-not-a-presentation = このファイルは PowerPoint プレゼンテーションではありません`<br>`import-protected = このプレゼンテーションは暗号化されているか旧形式の .ppt です。先に .pptx として保存してください`<br>`import-too-large = このプレゼンテーションは大きすぎてインポートできません` |
| ko | `import-unreadable = 이 프레젠테이션을 읽을 수 없습니다`<br>`import-not-a-presentation = 이 파일은 PowerPoint 프레젠테이션이 아닙니다`<br>`import-protected = 이 프레젠테이션은 암호화되었거나 이전 .ppt 형식입니다. 먼저 .pptx로 저장하세요`<br>`import-too-large = 이 프레젠테이션은 너무 커서 가져올 수 없습니다` |
| zh-Hans | `import-unreadable = 无法读取此演示文稿`<br>`import-not-a-presentation = 此文件不是 PowerPoint 演示文稿`<br>`import-protected = 此演示文稿已加密或为旧版 .ppt 格式：请先另存为 .pptx`<br>`import-too-large = 此演示文稿太大，无法导入` |
| zh-Hant | `import-unreadable = 無法讀取此簡報`<br>`import-not-a-presentation = 此檔案不是 PowerPoint 簡報`<br>`import-protected = 此簡報已加密或為舊版 .ppt 格式：請先另存為 .pptx`<br>`import-too-large = 此簡報太大，無法匯入` |
| th | `import-unreadable = ไม่สามารถอ่านงานนำเสนอนี้ได้`<br>`import-not-a-presentation = ไฟล์นี้ไม่ใช่งานนำเสนอ PowerPoint`<br>`import-protected = งานนำเสนอนี้ถูกเข้ารหัสหรือเป็นรูปแบบ .ppt แบบเก่า: โปรดบันทึกเป็น .pptx ก่อน`<br>`import-too-large = งานนำเสนอนี้ใหญ่เกินกว่าจะนำเข้าได้` |
| vi | `import-unreadable = Không thể đọc bản trình bày này`<br>`import-not-a-presentation = Tệp này không phải là bản trình bày PowerPoint`<br>`import-protected = Bản trình bày này đã được mã hóa hoặc ở định dạng .ppt cũ: hãy lưu thành .pptx trước`<br>`import-too-large = Bản trình bày này quá lớn để nhập` |

Run: `cargo nextest run -p oku-slides-i18n` → PASS.

- [ ] **Step 2 : tests qui échouent**

`src/error_tests.rs` :

```rust
use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(ImportErrorKind::Unreadable)]
#[case(ImportErrorKind::NotAPresentation)]
#[case(ImportErrorKind::Protected)]
#[case(ImportErrorKind::TooLarge)]
fn message_id_of_each_kind_exists_in_english(#[case] kind: ImportErrorKind) {
    assert_that!(english().try_tr(message_id(kind))).is_some();
}

#[test]
fn localize_unreadable_in_french_reads_naturally() {
    assert_that!(structure("x").localize(&Localizer::new(Some("fr")))).is_equal_to("Cette présentation est illisible".to_owned());
}

#[test]
fn kind_of_xml_error_is_unreadable() {
    let err = roxmltree::Document::parse("<a>").unwrap_err();
    assert_that!(xml_error("p.xml", err).kind()).is_equal_to(ImportErrorKind::Unreadable);
}
```

`src/limits_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;

#[test]
fn default_limits_are_conservative_and_non_zero() {
    let l = ReadLimits::default();
    assert_that!((l.max_input_bytes, l.max_part_bytes, l.max_total_bytes)).is_equal_to((256 << 20, 64 << 20, 512 << 20));
    assert_that!((l.max_parts, l.max_xml_nodes, l.max_slides)).is_equal_to((10_000, 5_000_000, 2_000));
}
```

`src/collect_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::error::too_large;
use crate::source::WarningKind;

#[test]
fn intern_same_path_twice_returns_same_id_and_one_asset() {
    let mut s = Sink::new();
    let a = s.intern("ppt/media/a.png", vec![1, 2, 3], "image/png");
    assert_that!(s.cached_asset("ppt/media/a.png")).is_equal_to(Some(a.clone()));
    assert_that!(a.starts_with("sha256:")).is_true();
    assert_that!(s.finish().unwrap().0.len()).is_equal_to(1);
}

#[test]
fn intern_same_bytes_at_two_paths_keeps_one_asset() {
    let mut s = Sink::new();
    let a = s.intern("ppt/media/a.png", vec![9], "image/png");
    let b = s.intern("ppt/media/b.png", vec![9], "image/png");
    assert_that!(a).is_equal_to(b);
    assert_that!(s.finish().unwrap().0.len()).is_equal_to(1);
}

#[test]
fn warnings_beyond_the_cap_collapse_into_one_truncated() {
    let mut s = Sink::new();
    for _ in 0..(MAX_WARNINGS + 10) {
        s.warn(None, None, WarningKind::MissingPart("x".into()));
    }
    let (_, w) = s.finish().unwrap();
    assert_that!(w.len()).is_equal_to(MAX_WARNINGS + 1);
    assert_that!(w.last().map(|w| w.kind.clone())).is_equal_to(Some(WarningKind::Truncated));
}

#[test]
fn finish_with_a_fatal_error_fails() {
    let mut s = Sink::new();
    s.set_fatal(too_large("p"));
    assert_that!(s.finish().is_err()).is_true();
}

#[test]
fn clip_strips_control_and_bidi_characters_and_truncates() {
    let long = format!("a\u{7}b\u{202e}\n{}", "x".repeat(500));
    let c = clip(&long);
    assert_that!(c.starts_with("abx")).is_true();
    assert_that!(c.chars().count()).is_equal_to(200);
}

#[test]
fn clip_to_cuts_at_the_given_length() {
    assert_that!(clip_to("abcdef", 3)).is_equal_to("abc".to_owned());
}
```

`src/pptx/xml_tests.rs` (cette tâche : l'analyse ; la tâche 3 ajoute les tests d'accès) :

```rust
use speculoos::prelude::*;

use super::*;
use crate::ImportErrorKind;

#[test]
fn parse_over_node_limit_is_unreadable() {
    let doc = format!("<r>{}</r>", "<a/>".repeat(50));
    assert_that!(parse(&doc, "p.xml", 10).map(|_| ()).map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::Unreadable));
}

#[test]
fn parse_ten_thousand_levels_is_refused_without_recursing() {
    let doc = format!("{}{}", "<a x='>'>".repeat(10_000), "</a>".repeat(10_000));
    assert_that!(parse(&doc, "p.xml", 1_000_000).map(|_| ()).map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::Unreadable));
}

#[test]
fn parse_at_the_depth_limit_with_comment_cdata_and_pi_is_accepted() {
    let inner = "<!-- <a> --><![CDATA[<a>]]><?pi <a>?><b/>";
    let doc = format!("{}{inner}{}", "<a>".repeat(MAX_XML_DEPTH - 1), "</a>".repeat(MAX_XML_DEPTH - 1));
    assert_that!(parse(&doc, "p.xml", 1_000_000).is_ok()).is_true();
}

#[test]
fn depth_counts_open_elements_not_self_closing_or_quoted_angle_brackets() {
    // a (1) > c (2) ; b et d sont auto-fermants ; le « /> » entre guillemets ne ferme rien.
    assert_that!(max_depth("<a q=\"/>\"><b/><c><d/></c></a>")).is_equal_to(2);
}
```

`src/pptx/package_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::ImportErrorKind;
use crate::pptx::test_support::{CT, LIMITS, ROOT_RELS, TestPkg, zip_of};

const PRES_RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/a%20b.png"/>
<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
</Relationships>"#;

fn minimal() -> TestPkg {
    TestPkg::with(&[("ppt/_rels/presentation.xml.rels", PRES_RELS.as_bytes())])
}

fn open_err(bytes: &[u8], limits: &ReadLimits) -> ImportErrorKind {
    match Package::open(bytes, limits) {
        Ok(_) => panic!("échec attendu"),
        Err(e) => e.kind(),
    }
}

#[test]
fn open_minimal_package_finds_main_part() {
    assert_that!(minimal().pkg().main_part().to_owned()).is_equal_to("ppt/presentation.xml".to_owned());
}

#[test]
fn rels_resolve_targets_relative_to_their_part() {
    let t = minimal();
    let pkg = t.pkg();
    let rels = pkg.rels("ppt/presentation.xml").unwrap();
    assert_that!(rels.target("rId2")).is_equal_to(Some("ppt/slides/slide1.xml"));
    assert_that!(rels.by_type("/slide")).is_equal_to(vec![("rId2", "ppt/slides/slide1.xml")]);
}

#[test]
fn rels_decode_percent_escapes_in_targets() {
    let t = minimal();
    let pkg = t.pkg();
    assert_that!(pkg.rels("ppt/presentation.xml").unwrap().target("rId3")).is_equal_to(Some("ppt/media/a b.png"));
}

#[test]
fn rels_keep_external_targets_apart() {
    let t = minimal();
    let pkg = t.pkg();
    let rels = pkg.rels("ppt/presentation.xml").unwrap();
    assert_that!((rels.target("rId9"), rels.external("rId9"))).is_equal_to((None, Some("https://example.com")));
}

#[test]
fn rels_are_read_once_then_cached() {
    let t = minimal();
    let pkg = t.pkg();
    let a = pkg.rels("ppt/presentation.xml").unwrap();
    let b = pkg.rels("ppt/presentation.xml").unwrap();
    assert_that!(std::rc::Rc::ptr_eq(&a, &b)).is_true();
}

#[test]
fn rels_of_part_without_rels_file_are_empty() {
    let t = minimal();
    let pkg = t.pkg();
    assert_that!(pkg.rels("ppt/slides/slide1.xml").unwrap().by_type("/slide")).is_empty();
}

#[test]
fn resolve_path_climbs_parent_directories() {
    assert_that!(resolve_path("ppt/slides/slide1.xml", "../media/image1.png")).is_equal_to("ppt/media/image1.png".to_owned());
    assert_that!(resolve_path("ppt/slides/slide1.xml", "/ppt/media/x.png")).is_equal_to("ppt/media/x.png".to_owned());
}

#[test]
fn declared_entries_reads_the_end_of_central_directory() {
    let bytes = zip_of(&[("a", b"1"), ("b", b"2"), ("c", b"3")]);
    assert_that!(declared_entries(&bytes)).is_equal_to(Some(3));
}

#[test]
fn open_garbage_is_unreadable() {
    assert_that!(open_err(b"not a zip", &LIMITS)).is_equal_to(ImportErrorKind::Unreadable);
}

#[test]
fn open_compound_file_is_protected() {
    let mut cfb = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    cfb.extend_from_slice(&[0; 504]);
    assert_that!(open_err(&cfb, &LIMITS)).is_equal_to(ImportErrorKind::Protected);
}

#[test]
fn open_word_document_is_not_a_presentation() {
    let ct = CT.replace("presentationml.presentation.main", "wordprocessingml.document.main").replace("/ppt/presentation.xml", "/word/document.xml");
    let rels = ROOT_RELS.replace("ppt/presentation.xml", "word/document.xml");
    let bytes = zip_of(&[("[Content_Types].xml", ct.as_bytes()), ("_rels/.rels", rels.as_bytes()), ("word/document.xml", b"<w/>")]);
    assert_that!(open_err(&bytes, &LIMITS)).is_equal_to(ImportErrorKind::NotAPresentation);
}

#[test]
fn open_without_office_document_relationship_is_unreadable() {
    let bytes = zip_of(&[("[Content_Types].xml", CT.as_bytes()), ("_rels/.rels", b"<Relationships/>")]);
    assert_that!(open_err(&bytes, &LIMITS)).is_equal_to(ImportErrorKind::Unreadable);
}

#[test]
fn open_input_over_limit_is_too_large() {
    let t = minimal();
    assert_that!(open_err(t.bytes(), &ReadLimits { max_input_bytes: 10, ..LIMITS })).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn open_with_too_many_declared_entries_is_too_large() {
    let t = minimal();
    assert_that!(open_err(t.bytes(), &ReadLimits { max_parts: 2, ..LIMITS })).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn bytes_exactly_at_part_limit_are_accepted_one_more_is_too_large() {
    let t = TestPkg::with(&[("ppt/media/a.bin", &[7u8; 1024]), ("ppt/media/b.bin", &[7u8; 1025])]);
    let pkg = Package::open(t.bytes(), &ReadLimits { max_part_bytes: 1024, ..LIMITS }).unwrap();
    assert_that!(pkg.bytes("ppt/media/a.bin").map(|b| b.len()).ok()).is_equal_to(Some(1024));
    assert_that!(pkg.bytes("ppt/media/b.bin").map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
}

#[test]
fn total_over_limit_is_too_large() {
    let t = TestPkg::with(&[("ppt/media/a.bin", &[7u8; 600]), ("ppt/media/b.bin", &[7u8; 600])]);
    // L'ouverture lit [Content_Types].xml et _rels/.rels ; il reste de quoi lire une image, pas deux.
    let budget = (CT.len() + ROOT_RELS.len() + 900) as u64;
    let pkg = Package::open(t.bytes(), &ReadLimits { max_total_bytes: budget, ..LIMITS }).unwrap();
    assert_that!(pkg.bytes("ppt/media/a.bin").is_ok()).is_true();
    assert_that!(pkg.bytes("ppt/media/b.bin").map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
}
```

- [ ] **Step 3 : lancer, constater l'échec** — FAIL (modules absents).

- [ ] **Step 4 : implémentation**

`src/limits.rs` :

```rust
//! Bornes de lecture d'un paquet (bombes zip, XML géants).

/// Bornes appliquées à la lecture d'un `.pptx` ; l'appelant les tire de sa configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadLimits {
    /// Taille maximale du fichier reçu.
    pub max_input_bytes: u64,
    /// Taille décompressée maximale d'une part.
    pub max_part_bytes: u64,
    /// Taille décompressée maximale cumulée de tout ce qui est lu.
    pub max_total_bytes: u64,
    /// Nombre maximal d'entrées dans le zip.
    pub max_parts: usize,
    /// Nombre maximal de nœuds d'une part XML.
    pub max_xml_nodes: u32,
    /// Nombre maximal de slides lues.
    pub max_slides: usize,
}

impl Default for ReadLimits {
    /// Bornes conservatrices pour un deck réel.
    fn default() -> Self {
        Self {
            max_input_bytes: 256 << 20,
            max_part_bytes: 64 << 20,
            max_total_bytes: 512 << 20,
            max_parts: 10_000,
            max_xml_nodes: 5_000_000,
            max_slides: 2_000,
        }
    }
}

#[cfg(test)]
#[path = "limits_tests.rs"]
mod limits_tests;
```

`src/error.rs` :

```rust
//! Erreurs fatales de l'import ; tout le reste devient `Opaque`, une part omise ou un `Warning`.

use std::string::FromUtf8Error;

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, IntoError, Snafu};

use crate::collect::clip;

/// La présentation ne peut pas être importée du tout.
#[derive(Debug, Snafu)]
pub struct ImportError(Box<Inner>);

/// Nature d'une [`ImportError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportErrorKind {
    /// Paquet, part essentielle ou XML illisible.
    Unreadable,
    /// Un paquet OOXML, mais pas une présentation.
    NotAPresentation,
    /// Fichier composé (chiffré, ou `.ppt` 97-2003).
    Protected,
    /// Une limite de [`crate::ReadLimits`] est dépassée.
    TooLarge,
}

impl ImportError {
    /// Nature de l'erreur, pour choisir le message affiché.
    pub fn kind(&self) -> ImportErrorKind {
        match *self.0 {
            Inner::Zip { .. } | Inner::Io { .. } | Inner::Utf8 { .. } | Inner::Xml { .. } | Inner::Structure { .. } => {
                ImportErrorKind::Unreadable
            }
            Inner::NotAPresentation { .. } => ImportErrorKind::NotAPresentation,
            Inner::Protected { .. } => ImportErrorKind::Protected,
            Inner::TooLarge { .. } => ImportErrorKind::TooLarge,
        }
    }
}

/// Clé Fluent du message d'une nature d'erreur.
pub(crate) fn message_id(kind: ImportErrorKind) -> &'static str {
    match kind {
        ImportErrorKind::Unreadable => "import-unreadable",
        ImportErrorKind::NotAPresentation => "import-not-a-presentation",
        ImportErrorKind::Protected => "import-protected",
        ImportErrorKind::TooLarge => "import-too-large",
    }
}

impl Localize for ImportError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(message_id(self.kind()))
    }
}

#[derive(Debug, Snafu)]
enum Inner {
    #[snafu(display("unreadable zip entry {part}"))]
    Zip { part: String, source: zip::result::ZipError, backtrace: Backtrace },
    #[snafu(display("could not read {part}"))]
    Io { part: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("{part} is not UTF-8"))]
    Utf8 { part: String, source: FromUtf8Error, backtrace: Backtrace },
    #[snafu(display("malformed XML in {part}"))]
    Xml { part: String, source: roxmltree::Error, backtrace: Backtrace },
    #[snafu(display("unexpected structure: {what}"))]
    Structure { what: String, backtrace: Backtrace },
    #[snafu(display("not a presentation: {content_type}"))]
    NotAPresentation { content_type: String, backtrace: Backtrace },
    #[snafu(display("protected file (encrypted or legacy .ppt)"))]
    Protected { backtrace: Backtrace },
    #[snafu(display("too large: {what}"))]
    TooLarge { what: String, backtrace: Backtrace },
}

// Constructeurs plutôt que `.context()` : `clip` n'est appelé que sur le chemin d'erreur, et
// l'erreur reste boxée (`clippy::result_large_err`).

pub(crate) fn zip_error(part: &str, source: zip::result::ZipError) -> ImportError {
    ImportError(Box::new(ZipSnafu { part: clip(part) }.into_error(source)))
}

pub(crate) fn io_error(part: &str, source: std::io::Error) -> ImportError {
    ImportError(Box::new(IoSnafu { part: clip(part) }.into_error(source)))
}

pub(crate) fn utf8_error(part: &str, source: FromUtf8Error) -> ImportError {
    ImportError(Box::new(Utf8Snafu { part: clip(part) }.into_error(source)))
}

pub(crate) fn xml_error(part: &str, source: roxmltree::Error) -> ImportError {
    ImportError(Box::new(XmlSnafu { part: clip(part) }.into_error(source)))
}

pub(crate) fn structure(what: &str) -> ImportError {
    ImportError(Box::new(StructureSnafu { what: clip(what) }.build()))
}

pub(crate) fn too_large(what: &str) -> ImportError {
    ImportError(Box::new(TooLargeSnafu { what: clip(what) }.build()))
}

pub(crate) fn not_a_presentation(content_type: &str) -> ImportError {
    ImportError(Box::new(NotAPresentationSnafu { content_type: clip(content_type) }.build()))
}

pub(crate) fn protected() -> ImportError {
    ImportError(Box::new(ProtectedSnafu.build()))
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
```

`src/collect.rs` :

```rust
//! Sorties communes à toute lecture : assets dédupliqués, avertissements, erreur fatale différée.

use std::collections::BTreeMap;

use oku_slides_core::asset_id;

use crate::error::ImportError;
use crate::source::{Asset, Warning, WarningKind};

/// Nombre d'avertissements gardés ; au-delà, un seul `Truncated`.
pub(crate) const MAX_WARNINGS: usize = 1000;
const CLIP_CHARS: usize = 200;

/// Caractère de contrôle ou de mise en forme invisible (bidi, largeur nulle).
fn is_unsafe(c: char) -> bool {
    c.is_control() || matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{feff}')
}

/// Chaîne venue du fichier, sans caractère dangereux, `n` caractères au plus.
pub(crate) fn clip_to(s: &str, n: usize) -> String {
    s.chars().filter(|c| !is_unsafe(*c)).take(n).collect()
}

/// Chaîne venue du fichier, rendue sûre pour un message (200 caractères au plus).
pub(crate) fn clip(s: &str) -> String {
    clip_to(s, CLIP_CHARS)
}

/// Accumulateur d'une lecture.
#[derive(Debug, Default)]
pub(crate) struct Sink {
    assets: BTreeMap<String, Asset>,
    by_path: BTreeMap<String, String>,
    warnings: Vec<Warning>,
    fatal: Option<ImportError>,
}

impl Sink {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Note un avertissement (borné à `MAX_WARNINGS`, puis un `Truncated`).
    pub(crate) fn warn(&mut self, slide: Option<u32>, item: Option<&str>, kind: WarningKind) {
        match self.warnings.len() {
            n if n < MAX_WARNINGS => self.warnings.push(Warning { slide, item: item.map(clip), kind }),
            MAX_WARNINGS => self.warnings.push(Warning { slide: None, item: None, kind: WarningKind::Truncated }),
            _ => {}
        }
    }

    /// Identifiant de l'asset déjà lu à `path`.
    pub(crate) fn cached_asset(&self, path: &str) -> Option<String> {
        self.by_path.get(path).cloned()
    }

    /// Range un asset lu à `path` ; rend son identifiant.
    pub(crate) fn intern(&mut self, path: &str, bytes: Vec<u8>, mime: &str) -> String {
        let id = asset_id(&bytes);
        self.assets.entry(id.clone()).or_insert_with(|| Asset { mime: mime.to_owned(), bytes });
        self.by_path.insert(path.to_owned(), id.clone());
        id
    }

    /// Garde la première erreur fatale rencontrée.
    pub(crate) fn set_fatal(&mut self, e: ImportError) {
        self.fatal.get_or_insert(e);
    }

    /// Une erreur fatale est-elle notée ?
    pub(crate) fn has_fatal(&self) -> bool {
        self.fatal.is_some()
    }

    /// Seule sortie : l'erreur fatale si elle existe, sinon assets et avertissements.
    pub(crate) fn finish(self) -> Result<(BTreeMap<String, Asset>, Vec<Warning>), ImportError> {
        match self.fatal {
            Some(e) => Err(e),
            None => Ok((self.assets, self.warnings)),
        }
    }
}

#[cfg(test)]
#[path = "collect_tests.rs"]
mod collect_tests;
```

> `clip_strips_control_and_bidi_characters_and_truncates` : `"a\u{7}b\u{202e}\n" + 500 × "x"` → `"ab" + 198 × "x"`.

`src/pptx/xml.rs` (cette tâche : l'analyse) :

```rust
//! XML : analyse bornée (nœuds, profondeur), et accès par nom local (`p:sp`, `dsp:sp`, `sp` se lisent pareil).

use roxmltree::{Document, ParsingOptions};

use crate::error::{ImportError, structure, xml_error};

/// Profondeur maximale d'un document : roxmltree analyse récursivement.
pub(crate) const MAX_XML_DEPTH: usize = 256;

/// Profondeur d'éléments d'un texte XML, par un balayage linéaire (commentaires, CDATA,
/// instructions et déclarations ignorés ; `>` entre guillemets ignoré).
pub(crate) fn max_depth(text: &str) -> usize {
    let b = text.as_bytes();
    let (mut i, mut depth, mut max) = (0usize, 0usize, 0usize);
    let skip_to = |from: usize, end: &[u8]| -> usize {
        b.get(from..).and_then(|rest| rest.windows(end.len()).position(|w| w == end)).map_or(b.len(), |p| from + p + end.len())
    };
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &b[i..];
        if rest.starts_with(b"<!--") {
            i = skip_to(i + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            i = skip_to(i + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            i = skip_to(i + 2, b"?>");
        } else if rest.starts_with(b"<!") {
            i = skip_to(i + 2, b">");
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            i = skip_to(i + 2, b">");
        } else {
            let (mut j, mut quote) = (i + 1, None::<u8>);
            while j < b.len() {
                match (quote, b[j]) {
                    (None, q @ (b'"' | b'\'')) => quote = Some(q),
                    (Some(q), c) if c == q => quote = None,
                    (None, b'>') => break,
                    _ => {}
                }
                j += 1;
            }
            if j > i + 1 && b.get(j - 1) != Some(&b'/') {
                depth += 1;
                max = max.max(depth);
            }
            i = j + 1;
        }
    }
    max
}

/// Analyse une part XML : sans DTD, au plus `nodes` nœuds et `MAX_XML_DEPTH` niveaux.
pub(crate) fn parse<'i>(text: &'i str, part: &str, nodes: u32) -> Result<Document<'i>, ImportError> {
    if max_depth(text) > MAX_XML_DEPTH {
        return Err(structure(&format!("{part}: XML nested deeper than {MAX_XML_DEPTH}")));
    }
    let opt = ParsingOptions { allow_dtd: false, nodes_limit: nodes, ..ParsingOptions::default() };
    Document::parse_with_options(text, opt).map_err(|e| xml_error(part, e))
}

#[cfg(test)]
#[path = "xml_tests.rs"]
mod xml_tests;
```

> `max_depth` n'indexe `b[i]`/`b[j]` que sous garde `i < b.len()`/`j < b.len()` : pas de panique.

`src/pptx/package.rs` :

```rust
//! Paquet OPC : zip borné, relations (en cache), types de contenu.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::rc::Rc;

use crate::ReadLimits;
use crate::error::{ImportError, io_error, not_a_presentation, protected, structure, too_large, utf8_error, zip_error};
use crate::pptx::xml::parse;

const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const EOCD: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
const ZIP64_LOCATOR: [u8; 4] = [0x50, 0x4b, 0x06, 0x07];
const PRESENTATION_TYPES: [&str; 4] = [
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
    "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml",
    "application/vnd.openxmlformats-officedocument.presentationml.slideshow.main+xml",
    "application/vnd.openxmlformats-officedocument.presentationml.template.main+xml",
];

/// Un `.pptx` ouvert sur ses octets ; chaque lecture compte dans les limites.
///
/// L'archive est clonée à chaque lecture (métadonnées en `Arc`, lecteur sur `&[u8]`) plutôt
/// que gardée dans un `RefCell` : `Package<'b>` reste covariant en `'b`, ce qui laisse les
/// contextes de part (`PartCtx<'p>`) raccourcir leurs durées de vie.
#[derive(Debug)]
pub(crate) struct Package<'b> {
    zip: zip::ZipArchive<Cursor<&'b [u8]>>,
    limits: ReadLimits,
    read_total: Cell<u64>,
    defaults: BTreeMap<String, String>,
    overrides: BTreeMap<String, String>,
    rels: RefCell<BTreeMap<String, Rc<Rels>>>,
    main: String,
}

/// Relations d'une part, cibles résolues en chemins du paquet.
#[derive(Debug, Default)]
pub(crate) struct Rels {
    internal: Vec<(String, String)>, // (type, cible), dans l'ordre du fichier
    by_id: BTreeMap<String, usize>,
    external: BTreeMap<String, String>,
}

impl Rels {
    /// Cible interne de la relation `id`.
    pub(crate) fn target(&self, id: &str) -> Option<&str> {
        self.by_id.get(id).and_then(|i| self.internal.get(*i)).map(|(_, t)| t.as_str())
    }

    /// Relations internes dont le type finit par `suffix` (`"/slide"`…), dans l'ordre du fichier.
    pub(crate) fn by_type(&self, suffix: &str) -> Vec<(&str, &str)> {
        let ids: BTreeMap<usize, &str> = self.by_id.iter().map(|(id, i)| (*i, id.as_str())).collect();
        self.internal
            .iter()
            .enumerate()
            .filter(|(_, (ty, _))| ty.ends_with(suffix))
            .filter_map(|(i, (_, t))| Some((*ids.get(&i)?, t.as_str())))
            .collect()
    }

    /// Cible d'une relation externe (lien, média lié).
    pub(crate) fn external(&self, id: &str) -> Option<&str> {
        self.external.get(id).map(String::as_str)
    }
}

/// `%XX` décodés (cibles de relation : ce sont des URI).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| char::from(c).to_digit(16);
        match (b[i], b.get(i + 1).copied().and_then(hex), b.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(h), Some(l)) => {
                out.push(u8::try_from(h * 16 + l).unwrap_or(b'?'));
                i += 3;
            }
            (c, _, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Chemin d'une cible relative à la part `base_part` (les `..` au-delà de la racine sont ignorés).
pub(crate) fn resolve_path(base_part: &str, target: &str) -> String {
    let target = percent_decode(target);
    if let Some(abs) = target.strip_prefix('/') {
        return abs.to_owned();
    }
    let mut segs: Vec<&str> = base_part.split('/').collect();
    segs.pop();
    for s in target.split('/') {
        match s {
            ".." => {
                segs.pop();
            }
            "." | "" => {}
            s => segs.push(s),
        }
    }
    segs.join("/")
}

/// Nombre d'entrées annoncé par la fin du répertoire central (Zip64 compris), sans l'indexer.
pub(crate) fn declared_entries(bytes: &[u8]) -> Option<u64> {
    let from = bytes.len().saturating_sub(65_557);
    let at = bytes.get(from..)?.windows(4).rposition(|w| w == EOCD)? + from;
    let le16 = |o: usize| bytes.get(o..o + 2).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes);
    let entries = le16(at + 10)?;
    if entries != 0xFFFF {
        return Some(u64::from(entries));
    }
    // Zip64 : le localisateur précède l'EOCD et pointe vers l'enregistrement Zip64.
    let loc = at.checked_sub(20)?;
    if bytes.get(loc..loc + 4)? != ZIP64_LOCATOR {
        return None;
    }
    let off = usize::try_from(u64::from_le_bytes(bytes.get(loc + 8..loc + 16)?.try_into().ok()?)).ok()?;
    Some(u64::from_le_bytes(bytes.get(off.checked_add(32)?..off.checked_add(40)?)?.try_into().ok()?))
}

fn rels_path(part: &str) -> String {
    match part.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{part}.rels"),
    }
}

impl<'b> Package<'b> {
    /// Ouvre le paquet et vérifie que c'est une présentation.
    pub(crate) fn open(bytes: &'b [u8], limits: &ReadLimits) -> Result<Self, ImportError> {
        if bytes.len() as u64 > limits.max_input_bytes {
            return Err(too_large("input"));
        }
        if bytes.starts_with(&CFB_MAGIC) {
            return Err(protected());
        }
        if declared_entries(bytes).is_some_and(|n| n > limits.max_parts as u64) {
            return Err(too_large("entries"));
        }
        let zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| zip_error("archive", e))?;
        if zip.len() > limits.max_parts {
            return Err(too_large("entries"));
        }
        let mut pkg = Self {
            zip,
            limits: *limits,
            read_total: Cell::new(0),
            defaults: BTreeMap::new(),
            overrides: BTreeMap::new(),
            rels: RefCell::new(BTreeMap::new()),
            main: String::new(),
        };
        pkg.read_content_types()?;
        let root = pkg.read_rels("_rels/.rels", "")?;
        let main = root.by_type("/officeDocument").first().map(|(_, t)| (*t).to_owned());
        let main = main.ok_or_else(|| structure("_rels/.rels: no officeDocument"))?;
        let ct = pkg.content_type(&main).unwrap_or_default().to_owned();
        if !PRESENTATION_TYPES.contains(&ct.as_str()) {
            return Err(not_a_presentation(&ct));
        }
        pkg.main = main;
        Ok(pkg)
    }

    /// Les limites de lecture.
    pub(crate) fn limits(&self) -> &ReadLimits {
        &self.limits
    }

    /// Chemin de la part principale (`ppt/presentation.xml` d'ordinaire).
    pub(crate) fn main_part(&self) -> &str {
        &self.main
    }

    /// Vrai si la part existe (sans lecture).
    pub(crate) fn exists(&self, part: &str) -> bool {
        self.zip.index_for_name(part).is_some()
    }

    /// Octets d'une part, bornés par les limites.
    pub(crate) fn bytes(&self, part: &str) -> Result<Vec<u8>, ImportError> {
        let mut zip = self.zip.clone();
        let file = zip.by_name(part).map_err(|e| zip_error(part, e))?;
        let cap = self.limits.max_part_bytes;
        let mut out = Vec::new();
        file.take(cap.saturating_add(1)).read_to_end(&mut out).map_err(|e| io_error(part, e))?;
        let n = out.len() as u64;
        if n > cap {
            return Err(too_large(part));
        }
        let total = self.read_total.get().saturating_add(n);
        if total > self.limits.max_total_bytes {
            return Err(too_large("total"));
        }
        self.read_total.set(total);
        Ok(out)
    }

    /// Texte UTF-8 d'une part (BOM retiré).
    pub(crate) fn text(&self, part: &str) -> Result<String, ImportError> {
        let s = String::from_utf8(self.bytes(part)?).map_err(|e| utf8_error(part, e))?;
        Ok(s.strip_prefix('\u{feff}').map(str::to_owned).unwrap_or(s))
    }

    /// Relations de `part` (lues une fois) ; vides si la part n'a pas de fichier de relations.
    pub(crate) fn rels(&self, part: &str) -> Result<Rc<Rels>, ImportError> {
        if let Some(r) = self.rels.borrow().get(part) {
            return Ok(Rc::clone(r));
        }
        let r = self.read_rels(&rels_path(part), part)?;
        self.rels.borrow_mut().insert(part.to_owned(), Rc::clone(&r));
        Ok(r)
    }

    /// Type de contenu d'une part (override, sinon défaut par extension).
    pub(crate) fn content_type(&self, part: &str) -> Option<&str> {
        if let Some(t) = self.overrides.get(part) {
            return Some(t);
        }
        let ext = part.rsplit_once('.')?.1.to_ascii_lowercase();
        self.defaults.get(&ext).map(String::as_str)
    }

    fn read_rels(&self, rels_part: &str, base: &str) -> Result<Rc<Rels>, ImportError> {
        if !self.exists(rels_part) {
            return Ok(Rc::new(Rels::default()));
        }
        let xml = self.text(rels_part)?;
        let doc = parse(&xml, rels_part, self.limits.max_xml_nodes)?;
        let mut rels = Rels::default();
        for r in doc.root_element().children().filter(|n| n.tag_name().name() == "Relationship") {
            let (Some(id), Some(ty), Some(target)) = (r.attribute("Id"), r.attribute("Type"), r.attribute("Target")) else {
                continue;
            };
            if r.attribute("TargetMode") == Some("External") {
                rels.external.insert(id.to_owned(), target.to_owned());
            } else if !rels.by_id.contains_key(id) {
                let path = if base.is_empty() { percent_decode(target.trim_start_matches('/')) } else { resolve_path(base, target) };
                rels.by_id.insert(id.to_owned(), rels.internal.len());
                rels.internal.push((ty.to_owned(), path));
            }
        }
        Ok(Rc::new(rels))
    }

    fn read_content_types(&mut self) -> Result<(), ImportError> {
        let xml = self.text("[Content_Types].xml")?;
        let doc = parse(&xml, "[Content_Types].xml", self.limits.max_xml_nodes)?;
        for n in doc.root_element().children().filter(roxmltree::Node::is_element) {
            match (n.tag_name().name(), n.attribute("ContentType")) {
                ("Default", Some(ct)) => {
                    if let Some(ext) = n.attribute("Extension") {
                        self.defaults.insert(ext.to_ascii_lowercase(), ct.to_owned());
                    }
                }
                ("Override", Some(ct)) => {
                    if let Some(p) = n.attribute("PartName") {
                        self.overrides.insert(p.trim_start_matches('/').to_owned(), ct.to_owned());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "package_tests.rs"]
mod package_tests;
```

`src/pptx/test_support.rs` :

```rust
//! Paquets et contextes de test en mémoire : seule source des aides de test du lecteur.

use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

use crate::ReadLimits;
use crate::pptx::package::Package;

pub(crate) const LIMITS: ReadLimits = ReadLimits {
    max_input_bytes: 64 << 20,
    max_part_bytes: 1 << 20,
    max_total_bytes: 8 << 20,
    max_parts: 1000,
    max_xml_nodes: 1_000_000,
    max_slides: 1000,
};

pub(crate) const CT: &str = r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/></Types>"#;
pub(crate) const ROOT_RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#;

/// Zip des parts données, dans l'ordre.
pub(crate) fn zip_of(parts: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in parts {
        w.start_file(*name, SimpleFileOptions::default()).unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap().into_inner()
}

/// Octets d'un paquet de test ; `pkg()` l'ouvre (le `Package` emprunte ces octets).
#[derive(Debug)]
pub(crate) struct TestPkg(Vec<u8>);

impl TestPkg {
    /// Présentation minimale (types, relation racine, `presentation.xml` vide) plus `parts`.
    pub(crate) fn with(parts: &[(&str, &[u8])]) -> Self {
        let mut all: Vec<(&str, &[u8])> =
            vec![("[Content_Types].xml", CT.as_bytes()), ("_rels/.rels", ROOT_RELS.as_bytes()), ("ppt/presentation.xml", b"<p/>")];
        all.extend_from_slice(parts);
        Self(zip_of(&all))
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn pkg(&self) -> Package<'_> {
        Package::open(&self.0, &LIMITS).unwrap()
    }
}
```

`tests/common/mod.rs` :

```rust
//! Fixtures d'import : dossiers OOXML décompressés, zippés à la volée, et fichiers binaires.

#![allow(dead_code, reason = "module partagé : chaque binaire de test n'en utilise qu'une partie")]

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use oku_slides_import::ReadLimits;
use zip::write::SimpleFileOptions;

pub const LIMITS: ReadLimits = ReadLimits {
    max_input_bytes: 256 << 20,
    max_part_bytes: 64 << 20,
    max_total_bytes: 512 << 20,
    max_parts: 10_000,
    max_xml_nodes: 5_000_000,
    max_slides: 2_000,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("lecture de {}: {e}", dir.display()));
    for e in entries {
        let p = e.unwrap_or_else(|e| panic!("entrée de {}: {e}", dir.display())).path();
        // `.DS_Store` (Finder) seulement : `_rels/.rels` commence aussi par un point.
        if p.file_name().is_some_and(|n| n == ".DS_Store") {
            continue;
        }
        if p.is_dir() { walk(&p, out) } else { out.push(p) }
    }
}

/// Le `.pptx` de la fixture `name` (un dossier), avec des parts remplacées (`Some`) ou retirées (`None`).
pub fn pptx_with(name: &str, overrides: &[(&str, Option<&str>)]) -> Vec<u8> {
    let dir = root().join(name);
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort_by_key(|p| (p.file_name() != Some("[Content_Types].xml".as_ref()), p.clone()));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for f in files {
        let rel = f.strip_prefix(&dir).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
        let data = match overrides.iter().find(|(p, _)| *p == rel) {
            Some((_, None)) => continue,
            Some((_, Some(text))) => text.as_bytes().to_vec(),
            None => fs::read(&f).unwrap_or_else(|e| panic!("lecture de {}: {e}", f.display())),
        };
        w.start_file(rel, SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip: {e}"));
        w.write_all(&data).unwrap_or_else(|e| panic!("zip: {e}"));
    }
    w.finish().unwrap_or_else(|e| panic!("zip: {e}")).into_inner()
}

/// Le `.pptx` de la fixture `name`.
pub fn pptx(name: &str) -> Vec<u8> {
    pptx_with(name, &[])
}

/// Un fichier binaire de `fixtures/import/`.
pub fn file(name: &str) -> Vec<u8> {
    fs::read(root().join(name)).unwrap_or_else(|e| panic!("lecture de {name}: {e}"))
}
```

> `LIMITS` y vaut les valeurs de `ReadLimits::default()` écrites en clair (une `const` ne peut pas appeler `Default::default()`).

`src/pptx/mod.rs` (provisoire ; réécrit à la tâche 11) :

```rust
//! Lecteur PPTX : du paquet OOXML à la [`crate::source::SourceDeck`].

pub(crate) mod package;
#[cfg(test)]
pub(crate) mod test_support;
pub(crate) mod xml;
```

`src/lib.rs` — **état de la tâche 2** (en entier) :

```rust
//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! Le module [`source`] décrit la représentation : géométrie en pt, styles
//! texte résolus, couleurs avec leur provenance, tableaux, graphiques,
//! SmartArt, replis, polices. Bornes : [`ReadLimits`] ; erreurs : [`ImportError`].

pub mod source;

// Branchés par `read_pptx` (tâche 11) : d'ici là, seule la build de test les utilise.
// `allow` et non `expect` : la build de test les utilise (l'attente y serait non satisfaite).
#[allow(dead_code, reason = "branché par read_pptx, tâche 11")]
mod collect;
#[allow(dead_code, reason = "branché par read_pptx, tâche 11")]
mod error;
mod limits;
#[allow(dead_code, reason = "branché par read_pptx (tâche 11) ; dernières fonctions utilisées à la tâche 13")]
mod pptx;

pub use error::{ImportError, ImportErrorKind};
pub use limits::ReadLimits;
```

- [ ] **Step 5 : lancer, constater le succès** — `cargo nextest run -p oku-slides-import` : PASS (le test de régression est ignoré).

- [ ] **Step 6 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import -p oku-slides-i18n --all-targets -- -D warnings && cargo nextest run -p oku-slides-i18n
git add crates/oku-slides-import crates/oku-slides-i18n/src/locales
git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : erreurs, limites, sorties communes, XML borné, paquet OPC (t-692)
oku cli task done --project slider --id t-707
```

---

### Task 3: Accès XML, unités et couleurs non résolues

**Files:**
- Modify: `src/pptx/xml.rs` (accès), `src/pptx/xml_tests.rs` (tests d'accès), `src/pptx/mod.rs` (`pub(crate) mod units; pub(crate) mod color;`)
- Create: `src/pptx/units.rs`, `src/pptx/color.rs`, `src/pptx/color_tests.rs`

**Interfaces:**
- Produces (`xml.rs`) : `elements(n)`, `child(n, local)`, `children(n, local)`, `descendant(n, local)` ; `attr_str`, `attr_i64`, `attr_u32`, `attr_f64` (finis), `attr_bool` (sans espace de noms) ; `attr_r(n, local)` (espace des relations) ; `text_of(n)`.
- Produces (`units.rs`) : `emu`, `angle`, `pct`, `hundredths`.
- Produces (`color.rs`) :

```rust
pub(crate) const MAX_COLOR_MODS: usize = 16;
pub(crate) enum ColorBase { Fixed(Rgba), Scheme(String) /* nom écrit : tx1, accent1… */, Placeholder /* phClr */ }
pub(crate) struct ColorSpec { pub base: ColorBase, pub mods: Vec<ColorMod> }
impl ColorSpec {
    pub(crate) fn with_placeholder(&self, ph: Option<&ColorSpec>) -> Option<ColorSpec>; // phClr remplacé ; mods de ph puis les siens (16 au plus)
    pub(crate) fn resolve(&self, ctx: &ColorCtx) -> Option<Color>;                     // phClr non remplacé → None
}
#[derive(Clone)] pub(crate) struct ColorCtx { pub scheme: Rc<BTreeMap<String, Rgba>>, pub map: Rc<BTreeMap<String, String>> }
impl ColorCtx { pub(crate) fn new(scheme: BTreeMap<String, Rgba>, map: BTreeMap<String, String>) -> Self; pub(crate) fn slot(&self, name: &str) -> Option<(String, Rgba)>; }
pub(crate) const SLOTS: [&str; 12];                       // dk1 lt1 dk2 lt2 accent1..6 hlink folHlink
pub(crate) fn default_map() -> BTreeMap<String, String>;
pub(crate) fn clean_map(attrs: impl Iterator<Item = (&str, &str)>, base: &BTreeMap<String, String>) -> BTreeMap<String, String>; // 12 clés, valeurs ∈ SLOTS
pub(crate) fn color_spec(c: Node) -> Option<ColorSpec>;      // élément couleur
pub(crate) fn color_spec_in(container: Node) -> Option<ColorSpec>; // premier enfant
pub(crate) fn parse_color(container: Node, ctx: &ColorCtx) -> Option<Color>;
```

Résolution : `Fixed` → `Origin::Explicit` (valeur modifiée par ses `mods`) ; `Scheme(nom)` → `Origin::Scheme { name: nom, slot: après clrMap, mods }`. Modificateurs dans l'ordre : `alpha`, `lumMod`/`lumOff`/`satMod` en HSL, `tint`/`shade` en sRGB linéaire.

- [ ] **Step 1 : tests qui échouent**

Ajouter à `src/pptx/xml_tests.rs` :

```rust
const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn doc(xml: &str) -> roxmltree::Document<'_> {
    parse(xml, "t", 10_000).unwrap()
}

#[test]
fn child_finds_by_local_name_whatever_the_prefix() {
    let d = doc(r#"<p:sp xmlns:p="urn:p" xmlns:a="urn:a"><p:spPr><a:xfrm><a:off x="12700"/></a:xfrm></p:spPr></p:sp>"#);
    assert_that!(child(d.root_element(), "spPr").map(|n| n.tag_name().name())).is_equal_to(Some("spPr"));
    assert_that!(descendant(d.root_element(), "off").and_then(|n| attr_i64(n, "x"))).is_equal_to(Some(12700));
}

#[test]
fn attr_r_skips_the_plain_id_written_first() {
    let xml = format!(r#"<p:sldId xmlns:p="urn:p" xmlns:r="{NS_R}" id="256" r:id="rId2"/>"#);
    let d = doc(&xml);
    assert_that!((attr_r(d.root_element(), "id"), attr_str(d.root_element(), "id"))).is_equal_to((Some("rId2"), Some("256")));
}

#[test]
fn attr_r_accepts_the_strict_relationships_namespace() {
    let d = doc(r#"<b xmlns:r="http://purl.oclc.org/ooxml/officeDocument/relationships" r:embed="rId3"/>"#);
    assert_that!(attr_r(d.root_element(), "embed")).is_equal_to(Some("rId3"));
}

#[test]
fn attr_r_ignores_other_namespaces() {
    let d = doc(r#"<b xmlns:x="urn:x" x:embed="rId3"/>"#);
    assert_that!(attr_r(d.root_element(), "embed")).is_none();
}

#[test]
fn attr_bool_reads_ooxml_booleans() {
    let d = doc(r#"<x a="1" b="false" c="on" d="maybe"/>"#);
    let n = d.root_element();
    assert_that!((attr_bool(n, "a"), attr_bool(n, "b"), attr_bool(n, "c"), attr_bool(n, "d"), attr_bool(n, "e")))
        .is_equal_to((Some(true), Some(false), Some(true), None, None));
}

#[test]
fn attr_f64_rejects_non_finite_values() {
    let d = doc(r#"<x a="NaN" b="inf" c="1.5"/>"#);
    let n = d.root_element();
    assert_that!((attr_f64(n, "a"), attr_f64(n, "b"), attr_f64(n, "c"))).is_equal_to((None, None, Some(1.5)));
}

#[test]
fn attr_u32_rejects_negative_and_too_large_values() {
    let d = doc(r#"<x a="-1" b="4294967296" c="7"/>"#);
    let n = d.root_element();
    assert_that!((attr_u32(n, "a"), attr_u32(n, "b"), attr_u32(n, "c"))).is_equal_to((None, None, Some(7)));
}

#[test]
fn units_convert_emu_angles_percents_and_hundredths() {
    use crate::pptx::units::{angle, emu, hundredths, pct};
    assert_that!((emu(12700), angle(5_400_000), pct(50_000), hundredths(1800))).is_equal_to((1.0, 90.0, 0.5, 18.0));
}
```

`src/pptx/color_tests.rs` :

```rust
use std::collections::BTreeMap;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::pptx::xml::parse;
use crate::source::{ColorMod, Origin, Rgba, Styled};

fn ctx() -> ColorCtx {
    let scheme = BTreeMap::from([
        ("dk1".to_owned(), Rgba::new(0, 0, 0)),
        ("lt1".to_owned(), Rgba::new(255, 255, 255)),
        ("accent1".to_owned(), Rgba::new(0x44, 0x72, 0xc4)),
    ]);
    ColorCtx::new(scheme, default_map())
}

fn spec(inner: &str) -> Option<ColorSpec> {
    let xml = format!(r#"<a:solidFill xmlns:a="urn:a">{inner}</a:solidFill>"#);
    let d = parse(&xml, "t", 10_000).unwrap();
    color_spec_in(d.root_element())
}

fn color_with(inner: &str, c: &ColorCtx) -> Option<Color> {
    spec(inner)?.resolve(c)
}

fn color(inner: &str) -> Option<Color> {
    color_with(inner, &ctx())
}

fn hex(inner: &str) -> String {
    color(inner).map(|c| c.value.to_hex()).unwrap_or_default()
}

#[test]
fn srgb_color_is_explicit() {
    assert_that!(color(r#"<a:srgbClr val="1F4E79"/>"#)).is_equal_to(Some(Styled::explicit(Rgba::new(0x1f, 0x4e, 0x79))));
}

#[test]
fn scheme_color_keeps_its_written_name_and_mapped_slot() {
    assert_that!(color(r#"<a:schemeClr val="tx1"/>"#)).is_equal_to(Some(Styled::mapped(Rgba::new(0, 0, 0), "tx1", "dk1", vec![])));
}

#[test]
fn overridden_color_map_changes_the_slot_not_the_name() {
    let mut map = default_map();
    map.insert("tx1".into(), "lt1".into());
    let dark = ColorCtx { map: std::rc::Rc::new(map), ..ctx() };
    assert_that!(color_with(r#"<a:schemeClr val="tx1"/>"#, &dark).map(|c| c.origin))
        .is_equal_to(Some(Origin::Scheme { name: "tx1".into(), slot: "lt1".into(), mods: vec![] }));
}

#[test]
fn sys_color_uses_last_clr() {
    assert_that!(hex(r#"<a:sysClr val="windowText" lastClr="1F1F1F"/>"#)).is_equal_to("#1f1f1f".to_owned());
}

#[test]
fn sys_color_without_last_clr_uses_the_system_default() {
    assert_that!(hex(r#"<a:sysClr val="window"/>"#)).is_equal_to("#ffffff".to_owned());
}

#[test]
fn preset_color_white_is_known() {
    assert_that!(hex(r#"<a:prstClr val="white"/>"#)).is_equal_to("#ffffff".to_owned());
}

#[test]
fn alpha_modifier_sets_opacity() {
    assert_that!(color(r#"<a:srgbClr val="FF0000"><a:alpha val="50000"/></a:srgbClr>"#).map(|c| c.value.a)).is_equal_to(Some(128));
}

#[rstest]
// Valeurs de PowerPoint pour accent1 = 4472C4 : « Plus sombre 25 % », « Plus clair 80 % », shade 50 % (trait par défaut), tint 40 %.
#[case(r#"<a:schemeClr val="accent1"><a:lumMod val="75000"/></a:schemeClr>"#, "#2f5597")]
#[case(r#"<a:schemeClr val="accent1"><a:lumMod val="20000"/><a:lumOff val="80000"/></a:schemeClr>"#, "#dae3f3")]
#[case(r#"<a:schemeClr val="accent1"><a:shade val="50000"/></a:schemeClr>"#, "#2f528f")]
#[case(r#"<a:schemeClr val="accent1"><a:tint val="40000"/></a:schemeClr>"#, "#cfd5ea")]
fn color_modifiers_match_powerpoint(#[case] inner: &str, #[case] expected: &str) {
    let got = color(inner).unwrap().value;
    let want = u32::from_str_radix(&expected[1..], 16).unwrap();
    let (r, g, b) = ((want >> 16) as u8, (want >> 8) as u8, want as u8);
    let off = |a: u8, b: u8| (i16::from(a) - i16::from(b)).abs();
    assert!(off(got.r, r) <= 1 && off(got.g, g) <= 1 && off(got.b, b) <= 1, "obtenu {}, attendu {expected}", got.to_hex());
}

#[test]
fn derived_scheme_color_keeps_its_modifiers_in_the_origin() {
    let c = color(r#"<a:schemeClr val="accent1"><a:lumMod val="75000"/></a:schemeClr>"#).unwrap();
    assert_that!(c.origin).is_equal_to(Origin::Scheme { name: "accent1".into(), slot: "accent1".into(), mods: vec![ColorMod::LumMod(0.75)] });
}

#[test]
fn placeholder_color_is_replaced_syntactically_and_mods_appended() {
    let ph = spec(r#"<a:schemeClr val="accent1"/>"#).unwrap();
    let styled = spec(r#"<a:schemeClr val="phClr"><a:tint val="40000"/></a:schemeClr>"#).unwrap();
    let got = styled.with_placeholder(Some(&ph)).unwrap().resolve(&ctx()).unwrap();
    assert_that!(got.origin).is_equal_to(Origin::Scheme { name: "accent1".into(), slot: "accent1".into(), mods: vec![ColorMod::Tint(0.4)] });
    assert_that!(got.value.to_hex()).is_equal_to("#cfd5ea".to_owned());
}

#[test]
fn placeholder_color_without_replacement_does_not_resolve() {
    let styled = spec(r#"<a:schemeClr val="phClr"/>"#).unwrap();
    assert_that!((styled.with_placeholder(None), styled.resolve(&ctx()))).is_equal_to((None, None));
}

#[test]
fn modifiers_beyond_the_cap_are_dropped() {
    let many = r#"<a:tint val="99000"/>"#.repeat(MAX_COLOR_MODS + 50);
    let s = spec(&format!(r#"<a:schemeClr val="accent1">{many}</a:schemeClr>"#)).unwrap();
    assert_that!(s.mods.len()).is_equal_to(MAX_COLOR_MODS);
}

#[test]
fn unknown_scheme_slot_is_none() {
    assert_that!(color(r#"<a:schemeClr val="accent6"/>"#)).is_none();
}

#[test]
fn clean_map_keeps_known_keys_and_slots_only() {
    let attrs = [("bg1", "dk1"), ("tx1", "lt1"), ("evil", "dk1"), ("bg2", "nonsense")];
    let m = clean_map(attrs.into_iter(), &default_map());
    assert_that!((m.get("bg1").map(String::as_str), m.get("tx1").map(String::as_str), m.get("bg2").map(String::as_str), m.len()))
        .is_equal_to((Some("dk1"), Some("lt1"), Some("lt2"), 12));
}
```

- [ ] **Step 2 : lancer, constater l'échec** — FAIL.

- [ ] **Step 3 : implémentation**

`src/pptx/xml.rs` — en-tête final (fusionner avec l'analyse de la tâche 2) :

```rust
//! XML : analyse bornée (nœuds, profondeur), et accès par nom local (`p:sp`, `dsp:sp`, `sp` se lisent pareil).

use roxmltree::{Document, Node, ParsingOptions};

use crate::error::{ImportError, structure, xml_error};

const NS_REL: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "http://purl.oclc.org/ooxml/officeDocument/relationships",
];
```

puis, après `parse` :

```rust
/// Enfants éléments.
pub(crate) fn elements<'a, 'i>(n: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(Node::is_element)
}

/// Premier enfant élément de nom local `local`.
pub(crate) fn child<'a, 'i>(n: Node<'a, 'i>, local: &str) -> Option<Node<'a, 'i>> {
    elements(n).find(|c| c.tag_name().name() == local)
}

/// Enfants éléments de nom local `local`.
pub(crate) fn children<'a, 'i>(n: Node<'a, 'i>, local: &str) -> impl Iterator<Item = Node<'a, 'i>> {
    elements(n).filter(move |c| c.tag_name().name() == local)
}

/// Premier descendant (lui-même exclu) de nom local `local`.
pub(crate) fn descendant<'a, 'i>(n: Node<'a, 'i>, local: &str) -> Option<Node<'a, 'i>> {
    n.descendants().skip(1).find(|c| c.is_element() && c.tag_name().name() == local)
}

/// Attribut sans espace de noms.
pub(crate) fn attr_str<'a>(n: Node<'a, '_>, name: &str) -> Option<&'a str> {
    n.attributes().find(|a| a.namespace().is_none() && a.name() == name).map(|a| a.value())
}

/// Attribut de l'espace des relations (`r:embed`, `r:id`…), par nom local.
pub(crate) fn attr_r<'a>(n: Node<'a, '_>, local: &str) -> Option<&'a str> {
    n.attributes().find(|a| a.name() == local && a.namespace().is_some_and(|ns| NS_REL.contains(&ns))).map(|a| a.value())
}

/// Entier signé.
pub(crate) fn attr_i64(n: Node<'_, '_>, name: &str) -> Option<i64> {
    attr_str(n, name)?.trim().parse().ok()
}

/// Entier non signé 32 bits.
pub(crate) fn attr_u32(n: Node<'_, '_>, name: &str) -> Option<u32> {
    attr_str(n, name)?.trim().parse().ok()
}

/// Nombre fini (`NaN`, `inf` rejetés).
pub(crate) fn attr_f64(n: Node<'_, '_>, name: &str) -> Option<f64> {
    attr_str(n, name)?.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Booléen OOXML (`1`/`true`/`on`, `0`/`false`/`off`).
pub(crate) fn attr_bool(n: Node<'_, '_>, name: &str) -> Option<bool> {
    match attr_str(n, name)? {
        "1" | "true" | "on" => Some(true),
        "0" | "false" | "off" => Some(false),
        _ => None,
    }
}

/// Texte concaténé des descendants `t` (titres de graphique).
pub(crate) fn text_of(n: Node<'_, '_>) -> String {
    n.descendants().filter(|d| d.is_element() && d.tag_name().name() == "t").filter_map(|d| d.text()).collect()
}
```

> `children` : en Rust 2024, `impl Trait` capture toutes les durées de vie, `local` compris.

`src/pptx/units.rs` :

```rust
//! Unités OOXML.

/// EMU → pt (12 700 EMU par pt).
pub(crate) fn emu(v: i64) -> f64 {
    v as f64 / 12_700.0
}

/// 60 000ᵉ de degré → degrés.
pub(crate) fn angle(v: i64) -> f64 {
    v as f64 / 60_000.0
}

/// 100 000ᵉ → fraction.
pub(crate) fn pct(v: i64) -> f64 {
    v as f64 / 100_000.0
}

/// Centièmes → unités (`sz`, `spc`, `spcPts`).
pub(crate) fn hundredths(v: i64) -> f64 {
    v as f64 / 100.0
}
```

`src/pptx/color.rs` :

```rust
//! Couleurs DrawingML : sRGB, thème (via `clrMap`), système, prédéfinies, `phClr`, modificateurs.
//!
//! Une couleur se lit en [`ColorSpec`] (non résolue) et se résout avec le
//! [`ColorCtx`] de la part qui l'affiche : le `clrMapOvr` d'une slide atteint
//! ainsi tout ce qu'elle hérite. `phClr` est remplacé syntaxiquement.

use std::collections::BTreeMap;
use std::rc::Rc;

use roxmltree::Node;

use crate::pptx::units::pct;
use crate::pptx::xml::{attr_i64, attr_str, elements};
use crate::source::{Color, ColorMod, Origin, Rgba, Styled};

/// Modificateurs gardés par couleur.
pub(crate) const MAX_COLOR_MODS: usize = 16;

/// Emplacements de couleur d'un thème.
pub(crate) const SLOTS: [&str; 12] =
    ["dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hlink", "folHlink"];

const CLR_MAP: [(&str, &str); 12] = [
    ("bg1", "lt1"),
    ("tx1", "dk1"),
    ("bg2", "lt2"),
    ("tx2", "dk2"),
    ("accent1", "accent1"),
    ("accent2", "accent2"),
    ("accent3", "accent3"),
    ("accent4", "accent4"),
    ("accent5", "accent5"),
    ("accent6", "accent6"),
    ("hlink", "hlink"),
    ("folHlink", "folHlink"),
];

/// `clrMap` par défaut.
pub(crate) fn default_map() -> BTreeMap<String, String> {
    CLR_MAP.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

/// `clrMap` lu dans des attributs, par-dessus `base` : seules les 12 clés et les 12 emplacements sont admis.
pub(crate) fn clean_map<'a>(attrs: impl Iterator<Item = (&'a str, &'a str)>, base: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut map = base.clone();
    for (k, v) in attrs {
        if CLR_MAP.iter().any(|(key, _)| *key == k) && SLOTS.contains(&v) {
            map.insert(k.to_owned(), v.to_owned());
        }
    }
    map
}

/// Thème et `clrMap` en vigueur dans une part (clones bon marché).
#[derive(Debug, Clone)]
pub(crate) struct ColorCtx {
    pub scheme: Rc<BTreeMap<String, Rgba>>,
    pub map: Rc<BTreeMap<String, String>>,
}

impl ColorCtx {
    pub(crate) fn new(scheme: BTreeMap<String, Rgba>, map: BTreeMap<String, String>) -> Self {
        Self { scheme: Rc::new(scheme), map: Rc::new(map) }
    }

    /// Emplacement de thème et valeur pour un nom (`tx1`, `accent1`, `dk2`…).
    pub(crate) fn slot(&self, name: &str) -> Option<(String, Rgba)> {
        let slot = self.map.get(name).map_or(name, String::as_str);
        self.scheme.get(slot).map(|c| (slot.to_owned(), *c))
    }
}

/// Base d'une couleur non résolue.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ColorBase {
    Fixed(Rgba),
    /// Nom du thème tel qu'écrit (`tx1`, `accent1`…), avant `clrMap`.
    Scheme(String),
    /// `phClr` : la couleur fournie par la référence de style.
    Placeholder,
}

/// Couleur non résolue : base et modificateurs (16 au plus).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ColorSpec {
    pub base: ColorBase,
    pub mods: Vec<ColorMod>,
}

impl ColorSpec {
    /// `phClr` remplacé par `ph` (ses modificateurs, puis les nôtres) ; `None` si `phClr` reste sans valeur.
    pub(crate) fn with_placeholder(&self, ph: Option<&ColorSpec>) -> Option<ColorSpec> {
        match (&self.base, ph) {
            (ColorBase::Placeholder, Some(p)) => {
                let mods = p.mods.iter().chain(&self.mods).copied().take(MAX_COLOR_MODS).collect();
                Some(ColorSpec { base: p.base.clone(), mods })
            }
            (ColorBase::Placeholder, None) => None,
            _ => Some(self.clone()),
        }
    }

    /// Couleur résolue dans `ctx`.
    pub(crate) fn resolve(&self, ctx: &ColorCtx) -> Option<Color> {
        match &self.base {
            ColorBase::Fixed(c) => Some(Styled::explicit(apply(*c, &self.mods))),
            ColorBase::Scheme(name) => {
                let (slot, v) = ctx.slot(name)?;
                Some(Styled { value: apply(v, &self.mods), origin: Origin::Scheme { name: name.clone(), slot, mods: self.mods.clone() } })
            }
            ColorBase::Placeholder => None,
        }
    }
}

/// Lit un élément couleur (`srgbClr`, `schemeClr`, `sysClr`, `prstClr`, `scrgbClr`).
pub(crate) fn color_spec(c: Node<'_, '_>) -> Option<ColorSpec> {
    let base = match c.tag_name().name() {
        "srgbClr" => ColorBase::Fixed(hex(attr_str(c, "val")?)?),
        "sysClr" => ColorBase::Fixed(match attr_str(c, "lastClr") {
            Some(last) => hex(last)?,
            None => sys_default(attr_str(c, "val")?),
        }),
        "prstClr" => ColorBase::Fixed(preset(attr_str(c, "val")?)?),
        "scrgbClr" => ColorBase::Fixed(Rgba::new(
            lin_to_srgb(pct(attr_i64(c, "r")?)),
            lin_to_srgb(pct(attr_i64(c, "g")?)),
            lin_to_srgb(pct(attr_i64(c, "b")?)),
        )),
        "schemeClr" => match attr_str(c, "val")? {
            "phClr" => ColorBase::Placeholder,
            name => ColorBase::Scheme(crate::collect::clip_to(name, 16)),
        },
        _ => return None,
    };
    let mods = elements(c)
        .filter_map(|m| {
            let v = pct(attr_i64(m, "val")?);
            Some(match m.tag_name().name() {
                "alpha" => ColorMod::Alpha(v),
                "lumMod" => ColorMod::LumMod(v),
                "lumOff" => ColorMod::LumOff(v),
                "satMod" => ColorMod::SatMod(v),
                "tint" => ColorMod::Tint(v),
                "shade" => ColorMod::Shade(v),
                _ => return None,
            })
        })
        .take(MAX_COLOR_MODS)
        .collect();
    Some(ColorSpec { base, mods })
}

/// Couleur du premier enfant élément de `container` (`solidFill`, `buClr`…), non résolue.
pub(crate) fn color_spec_in(container: Node<'_, '_>) -> Option<ColorSpec> {
    elements(container).next().and_then(color_spec)
}

/// Couleur du premier enfant de `container`, résolue (sans `phClr`).
pub(crate) fn parse_color(container: Node<'_, '_>, ctx: &ColorCtx) -> Option<Color> {
    color_spec_in(container)?.resolve(ctx)
}

fn hex(s: &str) -> Option<Rgba> {
    let v = u32::from_str_radix(s.get(..6)?, 16).ok()?;
    Some(Rgba::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn sys_default(name: &str) -> Rgba {
    match name {
        "window" | "highlightText" => Rgba::new(255, 255, 255),
        _ => Rgba::new(0, 0, 0),
    }
}

fn preset(name: &str) -> Option<Rgba> {
    Some(match name {
        "black" => Rgba::new(0, 0, 0),
        "white" => Rgba::new(255, 255, 255),
        "red" => Rgba::new(255, 0, 0),
        "green" => Rgba::new(0, 128, 0),
        "blue" => Rgba::new(0, 0, 255),
        "yellow" => Rgba::new(255, 255, 0),
        "gray" | "grey" => Rgba::new(128, 128, 128),
        "darkGray" | "dkGray" => Rgba::new(169, 169, 169),
        "lightGray" | "ltGray" => Rgba::new(211, 211, 211),
        "orange" => Rgba::new(255, 165, 0),
        _ => return None,
    })
}

fn srgb_to_lin(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn lin_to_srgb(c: f64) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    (s * 255.0).round() as u8
}

fn to_hsl(c: Rgba) -> (f64, f64, f64) {
    let (r, g, b) = (f64::from(c.r) / 255.0, f64::from(c.g) / 255.0, f64::from(c.b) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    if d.abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < f64::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

fn hue(p: f64, q: f64, t: f64) -> f64 {
    let t = t.rem_euclid(1.0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn from_hsl(h: f64, s: f64, l: f64, a: u8) -> Rgba {
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let l = l.clamp(0.0, 1.0);
    if s <= 0.0 {
        let v = byte(l);
        return Rgba { r: v, g: v, b: v, a };
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    Rgba { r: byte(hue(p, q, h + 1.0 / 3.0)), g: byte(hue(p, q, h)), b: byte(hue(p, q, h - 1.0 / 3.0)), a }
}

/// Applique les modificateurs dans l'ordre.
fn apply(mut c: Rgba, mods: &[ColorMod]) -> Rgba {
    for m in mods {
        c = match *m {
            ColorMod::Alpha(v) => Rgba { a: (v.clamp(0.0, 1.0) * 255.0).round() as u8, ..c },
            ColorMod::LumMod(v) => {
                let (h, s, l) = to_hsl(c);
                from_hsl(h, s, l * v, c.a)
            }
            ColorMod::LumOff(v) => {
                let (h, s, l) = to_hsl(c);
                from_hsl(h, s, l + v, c.a)
            }
            ColorMod::SatMod(v) => {
                let (h, s, l) = to_hsl(c);
                from_hsl(h, (s * v).clamp(0.0, 1.0), l, c.a)
            }
            ColorMod::Shade(v) => {
                let f = |x: u8| lin_to_srgb(srgb_to_lin(x) * v);
                Rgba { r: f(c.r), g: f(c.g), b: f(c.b), a: c.a }
            }
            ColorMod::Tint(v) => {
                let f = |x: u8| lin_to_srgb(srgb_to_lin(x) * v + (1.0 - v));
                Rgba { r: f(c.r), g: f(c.g), b: f(c.b), a: c.a }
            }
        };
    }
    c
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod color_tests;
```

> Les quatre valeurs de `color_modifiers_match_powerpoint` ont été recalculées avec ces formules lors des deux revues. Un écart > 1 = erreur de conversion (G14) : ne jamais toucher aux attentes.

- [ ] **Step 4 : lancer, constater le succès** — PASS.

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : XML par nom local et espace de noms, unités, couleurs non résolues (t-692)
oku cli task done --project slider --id t-708
```

---

### Task 4: Remplissages, traits et géométries (non résolus, bornés)

**Files:**
- Create: `src/pptx/fill.rs`, `src/pptx/fill_tests.rs`, `src/pptx/geometry.rs`, `src/pptx/geometry_tests.rs`
- Modify: `src/pptx/mod.rs` (`pub(crate) mod fill; pub(crate) mod geometry;`)

**Interfaces:**
- Consumes : `ColorSpec`, `color_spec_in`, `ColorCtx` (3).
- Produces :

```rust
pub(crate) const MAX_GRADIENT_STOPS: usize = 64;
pub(crate) enum FillSpec { None, Solid(ColorSpec), Gradient { angle: f64, stops: Vec<(f64, ColorSpec)> }, Picture(String) }
impl FillSpec {
    pub(crate) fn with_placeholder(&self, ph: Option<&ColorSpec>) -> FillSpec;
    pub(crate) fn resolve(&self, ctx: &ColorCtx) -> Fill;        // couleur irrésoluble → Fill::None (ou arrêt omis)
}
pub(crate) fn parse_fill(parent: Node, blip: &mut dyn FnMut(&str) -> Option<String>) -> Option<FillSpec>; // None = hériter ; grpFill → None
pub(crate) fn parse_fill_element(f: Node, blip: &mut dyn FnMut(&str) -> Option<String>) -> Option<FillSpec>;
pub(crate) fn approximation(parent: Node) -> Option<&'static str>; // "pattFill", "gradFill path", "gradFill stops"
#[derive(Default)] pub(crate) struct LineSpec { pub width: Option<f64>, pub color: Option<Option<ColorSpec>>, pub dash: Option<Dash> }
impl LineSpec {
    pub(crate) fn over(self, base: &LineSpec) -> LineSpec;
    pub(crate) fn with_placeholder(&self, ph: Option<&ColorSpec>) -> LineSpec;
    pub(crate) fn finish(&self, ctx: &ColorCtx) -> Option<Stroke>; // 0,75 pt par défaut ; None si noFill ou irrésoluble
}
pub(crate) fn parse_line(ln: Node) -> LineSpec;
// geometry.rs
pub(crate) const MAX_PATHS: usize = 64;
pub(crate) const MAX_PATH_CMDS: usize = 10_000;
pub(crate) fn parse_geometry(sp_pr: Node) -> Option<Geometry>;
```

- [ ] **Step 1 : tests qui échouent**

`src/pptx/fill_tests.rs` :

```rust
use std::collections::BTreeMap;

use speculoos::prelude::*;

use super::*;
use crate::pptx::color::{ColorBase, ColorCtx, default_map};
use crate::pptx::xml::parse;
use crate::source::{Dash, Fill, Origin, Rgba, Styled};

fn ctx() -> ColorCtx {
    ColorCtx::new(BTreeMap::from([("accent1".to_owned(), Rgba::new(0x44, 0x72, 0xc4)), ("lt1".to_owned(), Rgba::new(255, 255, 255))]), default_map())
}

fn with_sp_pr<R>(inner: &str, f: impl FnOnce(roxmltree::Node<'_, '_>) -> R) -> R {
    let xml = format!(r#"<p:spPr xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{inner}</p:spPr>"#);
    let d = parse(&xml, "t", 100_000).unwrap();
    f(d.root_element())
}

fn fill_of(inner: &str) -> Option<Fill> {
    with_sp_pr(inner, |n| parse_fill(n, &mut |id| Some(format!("asset-of-{id}")))).map(|s| s.resolve(&ctx()))
}

fn line_of(inner: &str) -> LineSpec {
    let xml = format!(r#"<a:ln xmlns:a="urn:a" {inner}"#);
    let d = parse(&xml, "t", 10_000).unwrap();
    parse_line(d.root_element())
}

fn explicit(r: u8, g: u8, b: u8) -> ColorSpec {
    ColorSpec { base: ColorBase::Fixed(Rgba::new(r, g, b)), mods: vec![] }
}

#[test]
fn no_fill_child_means_inherit() {
    assert_that!(fill_of(r#"<a:prstGeom prst="rect"/>"#)).is_none();
}

#[test]
fn no_fill_is_fill_none() {
    assert_that!(fill_of("<a:noFill/>")).is_equal_to(Some(Fill::None));
}

#[test]
fn solid_fill_resolves_its_color() {
    assert_that!(fill_of(r#"<a:solidFill><a:schemeClr val="accent1"/></a:solidFill>"#))
        .is_equal_to(Some(Fill::Solid { color: Styled::scheme(Rgba::new(0x44, 0x72, 0xc4), "accent1") }));
}

#[test]
fn solid_fill_is_resolved_by_the_consumer_color_map() {
    let spec = with_sp_pr(r#"<a:solidFill><a:schemeClr val="bg1"/></a:solidFill>"#, |n| parse_fill(n, &mut |_| None)).unwrap();
    let mut map = default_map();
    map.insert("bg1".into(), "accent1".into());
    let Fill::Solid { color } = spec.resolve(&ColorCtx::new((*ctx().scheme).clone(), map)) else { panic!("plein attendu") };
    assert_that!(color.origin).is_equal_to(Origin::Scheme { name: "bg1".into(), slot: "accent1".into(), mods: vec![] });
}

#[test]
fn gradient_fill_reads_stops_colors_and_angle() {
    let f = fill_of(r#"<a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="FFFFFF"/></a:gs><a:gs pos="100000"><a:srgbClr val="000000"/></a:gs></a:gsLst><a:lin ang="5400000"/></a:gradFill>"#);
    let Some(Fill::Gradient { angle, stops }) = f else { panic!("dégradé attendu, trouvé {f:?}") };
    let got: Vec<(f64, String)> = stops.iter().map(|s| (s.pos, s.color.value.to_hex())).collect();
    assert_that!((angle, got)).is_equal_to((90.0, vec![(0.0, "#ffffff".to_owned()), (1.0, "#000000".to_owned())]));
}

#[test]
fn gradient_stops_beyond_the_cap_are_dropped_and_reported() {
    let stops = r#"<a:gs pos="0"><a:srgbClr val="FFFFFF"/></a:gs>"#.repeat(MAX_GRADIENT_STOPS + 10);
    let xml = format!("<a:gradFill><a:gsLst>{stops}</a:gsLst></a:gradFill>");
    let Some(Fill::Gradient { stops, .. }) = fill_of(&xml) else { panic!("dégradé attendu") };
    assert_that!(stops.len()).is_equal_to(MAX_GRADIENT_STOPS);
    assert_that!(with_sp_pr(&xml, approximation)).is_equal_to(Some("gradFill stops"));
}

#[test]
fn blip_fill_maps_the_relationship_to_an_asset() {
    assert_that!(fill_of(r#"<a:blipFill><a:blip r:embed="rId4"/><a:stretch/></a:blipFill>"#))
        .is_equal_to(Some(Fill::Picture { asset: "asset-of-rId4".into() }));
}

#[test]
fn group_fill_means_inherit_from_group() {
    assert_that!(fill_of("<a:grpFill/>")).is_none();
}

#[test]
fn placeholder_fill_takes_the_reference_color() {
    let spec = with_sp_pr(r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>"#, |n| parse_fill(n, &mut |_| None)).unwrap();
    let ph = ColorSpec { base: ColorBase::Scheme("accent1".into()), mods: vec![] };
    assert_that!(spec.with_placeholder(Some(&ph)).resolve(&ctx()))
        .is_equal_to(Fill::Solid { color: Styled::scheme(Rgba::new(0x44, 0x72, 0xc4), "accent1") });
    assert_that!(spec.resolve(&ctx())).is_equal_to(Fill::None);
}

#[test]
fn pattern_and_path_gradient_are_reported_as_approximations() {
    assert_that!(with_sp_pr(r#"<a:pattFill prst="pct5"/>"#, approximation)).is_equal_to(Some("pattFill"));
    assert_that!(with_sp_pr(r#"<a:gradFill><a:path path="circle"/></a:gradFill>"#, approximation)).is_equal_to(Some("gradFill path"));
    assert_that!(with_sp_pr("<a:noFill/>", approximation)).is_none();
}

#[test]
fn line_over_base_keeps_base_color_when_absent() {
    let base = LineSpec { width: None, color: Some(Some(explicit(1, 2, 3))), dash: None };
    let s = line_of(r#"w="25400"><a:prstDash val="dash"/></a:ln>"#).over(&base).finish(&ctx()).unwrap();
    assert_that!((s.width, s.dash, s.color.value)).is_equal_to((2.0, Dash::Dash, Rgba::new(1, 2, 3)));
}

#[test]
fn line_no_fill_over_colored_base_has_no_stroke() {
    let base = LineSpec { width: Some(1.0), color: Some(Some(explicit(1, 2, 3))), dash: None };
    assert_that!(line_of(r#"w="12700"><a:noFill/></a:ln>"#).over(&base).finish(&ctx())).is_none();
}

#[test]
fn empty_line_element_inherits_everything() {
    assert_that!(line_of("></a:ln>")).is_equal_to(LineSpec::default());
}
```

`src/pptx/geometry_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::pptx::xml::parse;
use crate::source::{Geometry, PathCmd};

fn geom(inner: &str) -> Option<Geometry> {
    let xml = format!(r#"<p:spPr xmlns:p="urn:p" xmlns:a="urn:a">{inner}</p:spPr>"#);
    let d = parse(&xml, "t", 1_000_000).unwrap();
    parse_geometry(d.root_element())
}

#[test]
fn preset_reads_name_and_adjustments() {
    let g = geom(r#"<a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 25000"/></a:avLst></a:prstGeom>"#);
    let Some(Geometry::Preset { name, adj }) = g else { panic!("preset attendu") };
    assert_that!((name.as_str(), adj.get("adj").copied())).is_equal_to(("roundRect", Some(25000.0)));
}

#[test]
fn custom_path_is_normalized_to_the_box() {
    let g = geom(r#"<a:custGeom><a:pathLst><a:path w="200" h="100"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="200" y="50"/></a:lnTo><a:close/></a:path></a:pathLst></a:custGeom>"#);
    let Some(Geometry::Path { paths }) = g else { panic!("tracé attendu") };
    assert_that!(paths[0].clone()).is_equal_to(vec![PathCmd::MoveTo { x: 0.0, y: 0.0 }, PathCmd::LineTo { x: 1.0, y: 0.5 }, PathCmd::Close]);
}

#[test]
fn custom_path_is_capped_in_paths_and_commands() {
    let cmds = r#"<a:lnTo><a:pt x="1" y="1"/></a:lnTo>"#.repeat(MAX_PATH_CMDS);
    let paths = format!(r#"<a:path w="1" h="1">{cmds}</a:path>"#).repeat(2) + &r#"<a:path w="1" h="1"><a:close/></a:path>"#.repeat(MAX_PATHS + 5);
    let Some(Geometry::Path { paths }) = geom(&format!("<a:custGeom><a:pathLst>{paths}</a:pathLst></a:custGeom>")) else { panic!("tracé attendu") };
    assert_that!(paths.len() <= MAX_PATHS && paths.iter().map(Vec::len).sum::<usize>() <= MAX_PATH_CMDS).is_true();
}

#[test]
fn no_geometry_is_none() {
    assert_that!(geom("<a:noFill/>")).is_none();
}
```

- [ ] **Step 2 : lancer, constater l'échec** — FAIL.

- [ ] **Step 3 : implémentation**

`src/pptx/fill.rs` :

```rust
//! Remplissages et traits, lus non résolus (`FillSpec`, `LineSpec`) et résolus par la part qui les affiche.

use roxmltree::Node;

use crate::pptx::color::{ColorCtx, ColorSpec, color_spec_in};
use crate::pptx::units::{angle, emu, pct};
use crate::pptx::xml::{attr_i64, attr_r, attr_str, child, children, elements};
use crate::source::{Dash, Fill, GradientStop, Stroke};

/// Arrêts gardés par dégradé.
pub(crate) const MAX_GRADIENT_STOPS: usize = 64;
const FILLS: [&str; 6] = ["noFill", "solidFill", "gradFill", "blipFill", "pattFill", "grpFill"];
/// Largeur d'un trait sans `w` : 9525 EMU.
const DEFAULT_LINE_PT: f64 = 0.75;

/// Remplissage non résolu.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FillSpec {
    None,
    Solid(ColorSpec),
    Gradient { angle: f64, stops: Vec<(f64, ColorSpec)> },
    Picture(String),
}

impl FillSpec {
    /// `phClr` remplacé par `ph` partout.
    pub(crate) fn with_placeholder(&self, ph: Option<&ColorSpec>) -> FillSpec {
        let sub = |c: &ColorSpec| c.with_placeholder(ph).unwrap_or_else(|| c.clone());
        match self {
            FillSpec::Solid(c) => FillSpec::Solid(sub(c)),
            FillSpec::Gradient { angle, stops } => FillSpec::Gradient { angle: *angle, stops: stops.iter().map(|(p, c)| (*p, sub(c))).collect() },
            other => other.clone(),
        }
    }

    /// Remplissage résolu dans `ctx` (couleur irrésoluble : rien n'est peint, ou l'arrêt est omis).
    pub(crate) fn resolve(&self, ctx: &ColorCtx) -> Fill {
        match self {
            FillSpec::None => Fill::None,
            FillSpec::Solid(c) => c.resolve(ctx).map_or(Fill::None, |color| Fill::Solid { color }),
            FillSpec::Gradient { angle, stops } => Fill::Gradient {
                angle: *angle,
                stops: stops.iter().filter_map(|(pos, c)| Some(GradientStop { pos: *pos, color: c.resolve(ctx)? })).collect(),
            },
            FillSpec::Picture(asset) => Fill::Picture { asset: asset.clone() },
        }
    }
}

/// Premier remplissage enfant de `parent` ; `None` = hériter.
pub(crate) fn parse_fill(parent: Node<'_, '_>, blip: &mut dyn FnMut(&str) -> Option<String>) -> Option<FillSpec> {
    let f = elements(parent).find(|n| FILLS.contains(&n.tag_name().name()))?;
    parse_fill_element(f, blip)
}

/// Un élément qui *est* un remplissage ; `None` pour `grpFill` (hériter du groupe).
pub(crate) fn parse_fill_element(f: Node<'_, '_>, blip: &mut dyn FnMut(&str) -> Option<String>) -> Option<FillSpec> {
    match f.tag_name().name() {
        "noFill" => Some(FillSpec::None),
        "solidFill" => Some(color_spec_in(f).map_or(FillSpec::None, FillSpec::Solid)),
        "gradFill" => {
            let stops = child(f, "gsLst")
                .into_iter()
                .flat_map(|l| children(l, "gs"))
                .filter_map(|gs| Some((pct(attr_i64(gs, "pos")?), color_spec_in(gs)?)))
                .take(MAX_GRADIENT_STOPS)
                .collect();
            let ang = child(f, "lin").and_then(|l| attr_i64(l, "ang")).map_or(0.0, angle);
            Some(FillSpec::Gradient { angle: ang, stops })
        }
        "blipFill" => {
            let id = child(f, "blip").and_then(|b| attr_r(b, "embed"))?;
            Some(blip(id).map_or(FillSpec::None, FillSpec::Picture))
        }
        // Motif : approché par sa couleur de premier plan (signalé par `approximation`).
        "pattFill" => Some(child(f, "fgClr").and_then(color_spec_in).map_or(FillSpec::None, FillSpec::Solid)),
        _ => None,
    }
}

/// Rendu approché du remplissage de `parent`, à signaler.
pub(crate) fn approximation(parent: Node<'_, '_>) -> Option<&'static str> {
    let f = elements(parent).find(|n| FILLS.contains(&n.tag_name().name()))?;
    match f.tag_name().name() {
        "pattFill" => Some("pattFill"),
        "gradFill" if child(f, "path").is_some() => Some("gradFill path"),
        "gradFill" if child(f, "gsLst").is_some_and(|l| children(l, "gs").count() > MAX_GRADIENT_STOPS) => Some("gradFill stops"),
        _ => None,
    }
}

/// Trait non résolu, partiellement spécifié ; se combine à celui hérité et à celui du style.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LineSpec {
    pub width: Option<f64>,
    /// `Some(None)` : `noFill`.
    pub color: Option<Option<ColorSpec>>,
    pub dash: Option<Dash>,
}

impl LineSpec {
    /// `self` l'emporte sur `base`, champ par champ.
    pub(crate) fn over(self, base: &LineSpec) -> LineSpec {
        LineSpec { width: self.width.or(base.width), color: self.color.or_else(|| base.color.clone()), dash: self.dash.or(base.dash) }
    }

    /// `phClr` remplacé par `ph`.
    pub(crate) fn with_placeholder(&self, ph: Option<&ColorSpec>) -> LineSpec {
        let color = self.color.clone().map(|c| c.map(|c| c.with_placeholder(ph).unwrap_or(c)));
        LineSpec { color, ..self.clone() }
    }

    /// Le trait final dans `ctx` ; `None` s'il n'est pas peint.
    pub(crate) fn finish(&self, ctx: &ColorCtx) -> Option<Stroke> {
        let color = self.color.clone().flatten()?.resolve(ctx)?;
        Some(Stroke { width: self.width.unwrap_or(DEFAULT_LINE_PT), color, dash: self.dash.unwrap_or(Dash::Solid) })
    }
}

/// Lit un `a:ln` (et `lnL`/`lnR`/`lnT`/`lnB` de cellule).
pub(crate) fn parse_line(ln: Node<'_, '_>) -> LineSpec {
    let color = match elements(ln).find(|n| matches!(n.tag_name().name(), "noFill" | "solidFill" | "gradFill")) {
        Some(n) if n.tag_name().name() == "noFill" => Some(None),
        Some(n) if n.tag_name().name() == "solidFill" => color_spec_in(n).map(Some),
        Some(n) => child(n, "gsLst").and_then(|l| children(l, "gs").next()).and_then(color_spec_in).map(Some),
        None => None,
    };
    let dash = child(ln, "prstDash").and_then(|d| attr_str(d, "val")).map(|v| match v {
        "dash" | "sysDash" => Dash::Dash,
        "dot" | "sysDot" => Dash::Dot,
        "dashDot" | "sysDashDot" | "lgDashDot" | "lgDashDotDot" | "sysDashDotDot" => Dash::DashDot,
        "lgDash" => Dash::LongDash,
        _ => Dash::Solid,
    });
    LineSpec { width: attr_i64(ln, "w").map(emu).filter(|w| *w >= 0.0 && *w <= 1e4), color, dash }
}

#[cfg(test)]
#[path = "fill_tests.rs"]
mod fill_tests;
```

`src/pptx/geometry.rs` :

```rust
//! Géométrie d'une forme : prédéfinie (`prstGeom`) ou libre (`custGeom`, bornée).
//!
//! Les formules de guide (`gd fmla="*/ w 1 2"`) ne sont pas évaluées : seules
//! les constantes `val N` sont lues ; un point qui nomme un guide vaut 0 (t-727).

use std::collections::BTreeMap;

use roxmltree::Node;

use crate::collect::clip_to;
use crate::pptx::units::angle;
use crate::pptx::xml::{attr_f64, attr_i64, attr_str, child, children, elements};
use crate::source::{Geometry, PathCmd};

/// Sous-tracés gardés par géométrie.
pub(crate) const MAX_PATHS: usize = 64;
/// Commandes gardées par géométrie (tous sous-tracés confondus).
pub(crate) const MAX_PATH_CMDS: usize = 10_000;
const MAX_ADJ: usize = 16;

/// Géométrie déclarée dans `sp_pr` ; `None` si aucune.
pub(crate) fn parse_geometry(sp_pr: Node<'_, '_>) -> Option<Geometry> {
    if let Some(p) = child(sp_pr, "prstGeom") {
        let name = clip_to(attr_str(p, "prst").unwrap_or("rect"), 32);
        let adj: BTreeMap<String, f64> = child(p, "avLst")
            .into_iter()
            .flat_map(|l| children(l, "gd"))
            .filter_map(|gd| {
                let v: f64 = attr_str(gd, "fmla")?.strip_prefix("val ")?.trim().parse().ok()?;
                v.is_finite().then(|| (clip_to(attr_str(gd, "name").unwrap_or_default(), 16), v))
            })
            .take(MAX_ADJ)
            .collect();
        return Some(Geometry::Preset { name, adj });
    }
    let cust = child(sp_pr, "custGeom")?;
    let mut budget = MAX_PATH_CMDS;
    let paths = child(cust, "pathLst")
        .into_iter()
        .flat_map(|l| children(l, "path"))
        .take(MAX_PATHS)
        .map(|p| {
            let cmds = path(p, budget);
            budget -= cmds.len();
            cmds
        })
        .collect();
    Some(Geometry::Path { paths })
}

fn path(p: Node<'_, '_>, budget: usize) -> Vec<PathCmd> {
    let w = attr_f64(p, "w").filter(|v| *v > 0.0).unwrap_or(1.0);
    let h = attr_f64(p, "h").filter(|v| *v > 0.0).unwrap_or(1.0);
    let pts = |n: Node<'_, '_>| -> Vec<(f64, f64)> {
        children(n, "pt").take(3).map(|pt| (attr_f64(pt, "x").unwrap_or(0.0) / w, attr_f64(pt, "y").unwrap_or(0.0) / h)).collect()
    };
    elements(p)
        .filter_map(|c| match c.tag_name().name() {
            "moveTo" => pts(c).first().map(|&(x, y)| PathCmd::MoveTo { x, y }),
            "lnTo" => pts(c).first().map(|&(x, y)| PathCmd::LineTo { x, y }),
            "cubicBezTo" => match pts(c)[..] {
                [(x1, y1), (x2, y2), (x, y)] => Some(PathCmd::CubicTo { x1, y1, x2, y2, x, y }),
                _ => None,
            },
            "quadBezTo" => match pts(c)[..] {
                [(x1, y1), (x, y)] => Some(PathCmd::QuadTo { x1, y1, x, y }),
                _ => None,
            },
            "arcTo" => Some(PathCmd::ArcTo {
                wr: attr_f64(c, "wR").unwrap_or(0.0) / w,
                hr: attr_f64(c, "hR").unwrap_or(0.0) / h,
                start: attr_i64(c, "stAng").map_or(0.0, angle),
                swing: attr_i64(c, "swAng").map_or(0.0, angle),
            }),
            "close" => Some(PathCmd::Close),
            _ => None,
        })
        .take(budget)
        .collect()
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod geometry_tests;
```

- [ ] **Step 4 : lancer, constater le succès** — PASS.

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : remplissages, traits et géométries non résolus et bornés (t-692)
oku cli task done --project slider --id t-709
```

---

### Task 5: Thème (analysé une fois, styles indexés)

**Files:**
- Create: `src/pptx/theme.rs`, `src/pptx/theme_tests.rs`, `fixtures/import/minimal/ppt/theme/theme1.xml`
- Modify: `src/pptx/mod.rs` (`pub(crate) mod theme;`), `src/pptx/test_support.rs` (`THEME`)

**Interfaces:**
- Consumes : `FillSpec`, `LineSpec`, `parse_fill_element`, `parse_line` (4) ; `ColorCtx`, `ColorSpec`, `SLOTS`, `default_map`, `color_spec_in` (3) ; `xml::parse` (2) ; `clip_to` (2).
- Produces :
  - `ThemeData<'x> { pub theme: SourceTheme, … }` ; `ThemeData::parse(xml: &'x str, part, nodes) -> Result<_, ImportError>` ; `ThemeData::office() -> Result<ThemeData<'static>, ImportError>` ;
  - `.color_ctx(map: BTreeMap<String, String>) -> ColorCtx` ; `#[cfg(test)] .default_ctx() -> ColorCtx` ;
  - `.fill_style(idx: u32, ph: Option<&ColorSpec>, blip) -> Option<FillSpec>` (1.. formes, 1001.. fonds, 0 → `Some(FillSpec::None)`, `phClr` déjà remplacé) ;
  - `.line_style(idx: u32, ph: Option<&ColorSpec>) -> LineSpec` (0 → `color: Some(None)`).
  - `test_support::THEME = include_str!(…/minimal/ppt/theme/theme1.xml)`.

- [ ] **Step 1 : la fixture du thème**

`fixtures/import/minimal/ppt/theme/theme1.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Bureau">
<a:themeElements>
<a:clrScheme name="Office">
<a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
<a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
<a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
<a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4>
<a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6>
<a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
</a:clrScheme>
<a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
<a:fmtScheme name="Office">
<a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"><a:tint val="50000"/></a:schemeClr></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst>
<a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst>
<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>
<a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:srgbClr val="F2F2F2"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst>
</a:fmtScheme>
</a:themeElements>
</a:theme>
```

`test_support.rs` : `pub(crate) const THEME: &str = include_str!("../../../../fixtures/import/minimal/ppt/theme/theme1.xml");`

- [ ] **Step 2 : tests qui échouent**

`src/pptx/theme_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::pptx::color::{ColorBase, ColorSpec};
use crate::pptx::test_support::THEME;
use crate::source::{ColorMod, Fill, Origin, Rgba, Styled};

fn data() -> ThemeData<'static> {
    ThemeData::parse(THEME, "ppt/theme/theme1.xml", 100_000).unwrap()
}

fn accent1() -> ColorSpec {
    ColorSpec { base: ColorBase::Scheme("accent1".into()), mods: vec![] }
}

#[test]
fn parse_reads_all_twelve_color_slots() {
    let t = data();
    assert_that!(t.theme.colors.len()).is_equal_to(12);
    assert_that!((t.theme.colors.get("accent2").copied(), t.theme.colors.get("dk1").copied()))
        .is_equal_to((Some(Rgba::new(0xed, 0x7d, 0x31)), Some(Rgba::new(0, 0, 0))));
}

#[test]
fn parse_reads_name_and_major_minor_fonts() {
    let t = data();
    assert_that!((t.theme.name.as_str(), t.theme.fonts.major.as_str(), t.theme.fonts.minor.as_str())).is_equal_to(("Bureau", "Calibri Light", "Calibri"));
}

#[test]
fn fill_style_substitutes_the_placeholder_color() {
    let t = data();
    let f = t.fill_style(1, Some(&accent1()), &mut |_| None).map(|s| s.resolve(&t.default_ctx()));
    assert_that!(f).is_equal_to(Some(Fill::Solid { color: Styled::scheme(Rgba::new(0x44, 0x72, 0xc4), "accent1") }));
}

#[test]
fn fill_style_two_applies_its_tint_on_the_placeholder_color() {
    let t = data();
    let Some(Fill::Solid { color }) = t.fill_style(2, Some(&accent1()), &mut |_| None).map(|s| s.resolve(&t.default_ctx())) else { panic!("plein attendu") };
    assert_that!(color.origin).is_equal_to(Origin::Scheme { name: "accent1".into(), slot: "accent1".into(), mods: vec![ColorMod::Tint(0.5)] });
}

#[test]
fn fill_style_above_1000_reads_background_styles() {
    let t = data();
    assert_that!(t.fill_style(1002, None, &mut |_| None).map(|s| s.resolve(&t.default_ctx())))
        .is_equal_to(Some(Fill::Solid { color: Styled::explicit(Rgba::new(0xf2, 0xf2, 0xf2)) }));
}

#[test]
fn line_style_reads_width_and_placeholder_color() {
    let t = data();
    let s = t.line_style(2, Some(&accent1())).finish(&t.default_ctx()).unwrap();
    assert_that!((s.width, s.color.origin)).is_equal_to((1.0, Origin::scheme("accent1")));
}

#[test]
fn style_index_zero_is_nothing_and_out_of_range_is_none() {
    let t = data();
    assert_that!((t.fill_style(0, None, &mut |_| None), t.fill_style(9, None, &mut |_| None))).is_equal_to((Some(FillSpec::None), None));
    assert_that!((t.line_style(0, None).finish(&t.default_ctx()), t.line_style(9, None))).is_equal_to((None, LineSpec::default()));
}

#[test]
fn office_theme_has_the_office_palette() {
    let t = ThemeData::office().unwrap();
    assert_that!((t.theme.colors.get("accent1").copied(), t.theme.fonts.minor.as_str())).is_equal_to((Some(Rgba::new(0x44, 0x72, 0xc4)), "Calibri"));
}

#[test]
fn absurdly_long_font_name_is_clipped() {
    let xml = THEME.replace(r#"typeface="Calibri""#, &format!(r#"typeface="{}""#, "x".repeat(10_000)));
    let t = ThemeData::parse(&xml, "t", 100_000).unwrap();
    assert_that!(t.theme.fonts.minor.chars().count()).is_equal_to(MAX_FONT_NAME);
}
```

- [ ] **Step 3 : lancer, constater l'échec** — FAIL.

- [ ] **Step 4 : implémentation**

`src/pptx/theme.rs` :

```rust
//! Thème : emplacements de couleur, polices, styles de format, analysés une fois.

use std::collections::BTreeMap;
use std::rc::Rc;

use roxmltree::{Document, NodeId};

use crate::collect::clip_to;
use crate::error::{ImportError, structure};
use crate::pptx::color::{ColorCtx, ColorSpec, SLOTS, color_spec_in};
use crate::pptx::fill::{FillSpec, LineSpec, parse_fill_element, parse_line};
use crate::pptx::xml::{attr_str, child, descendant, elements, parse};
use crate::source::{Rgba, SourceTheme, ThemeFonts};

/// Longueur maximale d'un nom de police.
pub(crate) const MAX_FONT_NAME: usize = 128;

/// Thème Office (pour un master sans thème lisible).
const OFFICE: &str = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office"><a:themeElements><a:clrScheme name="Office"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme><a:fmtScheme name="Office"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#;

/// Thème lu ; ses listes de styles sont indexées une fois pour résoudre `fillRef`, `lnRef`, `bgRef`.
#[derive(Debug)]
pub(crate) struct ThemeData<'x> {
    pub theme: SourceTheme,
    scheme: Rc<BTreeMap<String, Rgba>>,
    doc: Document<'x>,
    fills: Vec<NodeId>,
    bg_fills: Vec<NodeId>,
    lines: Vec<NodeId>,
}

fn list_ids(doc: &Document<'_>, list: &str) -> Vec<NodeId> {
    descendant(doc.root_element(), list).map(|l| elements(l).take(16).map(|n| n.id()).collect()).unwrap_or_default()
}

impl<'x> ThemeData<'x> {
    /// Lit `theme1.xml`.
    pub(crate) fn parse(xml: &'x str, part: &str, nodes: u32) -> Result<Self, ImportError> {
        let doc = parse(xml, part, nodes)?;
        let root = doc.root_element();
        let clr = descendant(root, "clrScheme").ok_or_else(|| structure(part))?;
        // Les emplacements se lisent sans clrMap.
        let bare = ColorCtx::new(BTreeMap::new(), BTreeMap::new());
        let colors: BTreeMap<String, Rgba> = SLOTS
            .iter()
            .filter_map(|s| Some(((*s).to_owned(), color_spec_in(child(clr, s)?)?.resolve(&bare)?.value)))
            .collect();
        let font = |which: &str| {
            let name = descendant(root, which).and_then(|f| child(f, "latin")).and_then(|l| attr_str(l, "typeface")).unwrap_or("Calibri");
            clip_to(name, MAX_FONT_NAME)
        };
        let theme = SourceTheme {
            name: clip_to(attr_str(root, "name").unwrap_or_default(), MAX_FONT_NAME),
            colors: colors.clone(),
            fonts: ThemeFonts { major: font("majorFont"), minor: font("minorFont") },
        };
        let (fills, bg_fills, lines) = (list_ids(&doc, "fillStyleLst"), list_ids(&doc, "bgFillStyleLst"), list_ids(&doc, "lnStyleLst"));
        Ok(Self { theme, scheme: Rc::new(colors), doc, fills, bg_fills, lines })
    }

    /// Thème Office intégré.
    pub(crate) fn office() -> Result<ThemeData<'static>, ImportError> {
        ThemeData::parse(OFFICE, "office theme", 10_000)
    }

    /// Contexte de couleur de ce thème avec le `clrMap` donné.
    pub(crate) fn color_ctx(&self, map: BTreeMap<String, String>) -> ColorCtx {
        ColorCtx { scheme: Rc::clone(&self.scheme), map: Rc::new(map) }
    }

    /// Contexte avec le `clrMap` par défaut (tests).
    #[cfg(test)]
    pub(crate) fn default_ctx(&self) -> ColorCtx {
        self.color_ctx(crate::pptx::color::default_map())
    }

    /// Remplissage du style `idx` (1..3 formes, 1001.. fonds), `phClr` remplacé par `ph`.
    pub(crate) fn fill_style(&self, idx: u32, ph: Option<&ColorSpec>, blip: &mut dyn FnMut(&str) -> Option<String>) -> Option<FillSpec> {
        if idx == 0 {
            return Some(FillSpec::None);
        }
        let (list, i) = if idx > 1000 { (&self.bg_fills, idx - 1001) } else { (&self.fills, idx.checked_sub(1)?) };
        let node = self.doc.get_node(*list.get(usize::try_from(i).ok()?)?)?;
        Some(parse_fill_element(node, blip)?.with_placeholder(ph))
    }

    /// Trait du style `idx` (1..3), `phClr` remplacé par `ph`.
    pub(crate) fn line_style(&self, idx: u32, ph: Option<&ColorSpec>) -> LineSpec {
        if idx == 0 {
            return LineSpec { color: Some(None), ..LineSpec::default() };
        }
        idx.checked_sub(1)
            .and_then(|i| usize::try_from(i).ok())
            .and_then(|i| self.lines.get(i))
            .and_then(|id| self.doc.get_node(*id))
            .map(|ln| parse_line(ln).with_placeholder(ph))
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
```

- [ ] **Step 5 : lancer, constater le succès** — PASS.

- [ ] **Step 6 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import fixtures/import/minimal/ppt/theme && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : thème analysé une fois, styles de format indexés, thème Office de repli (t-692)
oku cli task done --project slider --id t-710
```

---

### Task 6: Texte brut — propriétés partielles (couleurs non résolues, valeurs bornées)

**Files:**
- Create: `src/pptx/text.rs`, `src/pptx/text_tests.rs`
- Modify: `src/pptx/mod.rs` (`pub(crate) mod text;`)

**Interfaces:**
- Consumes : `ColorSpec`, `color_spec_in` (3) ; `clip_to` (2) ; `MAX_FONT_NAME` (5).
- Produces (champs `pub`) :

```rust
pub(crate) const LEVELS: usize = 9;
pub(crate) enum FontRef { Major, Minor, Named(String) }
#[derive(Default, Clone)] pub(crate) struct RunProps { font, size, bold, italic, underline, strike, color: Option<ColorSpec>, highlight: Option<ColorSpec>, baseline, caps, char_spacing, link }
pub(crate) enum BulletKind { None, Char(String), AutoNum { scheme: String, start: u32 } }
#[derive(Default, Clone)] pub(crate) struct BulletProps { kind, color: Option<Option<ColorSpec>>, size_pct, font: Option<Option<String>> }
#[derive(Default, Clone)] pub(crate) struct ParaProps { align, indent, margin_left, line_spacing, space_before, space_after, bullet, run }
pub(crate) type LevelProps = Vec<ParaProps>;       // LEVELS entrées
#[derive(Default, Clone)] pub(crate) struct BodyProps { insets: [Option<f64>; 4], anchor, autofit, wrap, columns }
pub(crate) struct RawRun { text, field: Option<String>, props }
pub(crate) struct RawParagraph { level: u8, props, runs }
pub(crate) struct RawText { body, lst, paragraphs }
```

  `overlay` sur chaque structure ; `empty_levels()` ; `font_ref(fontRef) -> Option<FontRef>` ; `anchor(&str) -> Anchor` ; `parse_rpr(rpr, link)`, `parse_ppr(ppr)`, `parse_lst_style(lst)`, `parse_body_pr(bp)`, `parse_txbody(tx, link)`. Bornes : noms de police et de police de puce ≤ `MAX_FONT_NAME`, caractère de puce ≤ 8, schéma de numérotation ≤ 32, `numCol` dans `[1, 16]` (illisible ou nul → 1), `lvl` ≤ 8.

- [ ] **Step 1 : tests qui échouent**

`src/pptx/text_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::pptx::color::ColorBase;
use crate::pptx::xml::parse;
use crate::source::{Align, Anchor, Autofit, Spacing};

fn body(xml: &str) -> RawText {
    let full = format!(r#"<p:txBody xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{xml}</p:txBody>"#);
    let d = parse(&full, "t", 100_000).unwrap();
    parse_txbody(d.root_element(), &mut |id| (id == "rId7").then(|| "https://oku.example".to_owned()))
}

fn first_run(xml: &str) -> RunProps {
    body(&format!("<a:bodyPr/><a:p><a:r>{xml}<a:t>x</a:t></a:r></a:p>")).paragraphs[0].runs[0].props.clone()
}

#[test]
fn txbody_splits_paragraphs_runs_and_levels() {
    let t = body(r#"<a:bodyPr/><a:p><a:r><a:t>Bon</a:t></a:r><a:r><a:rPr b="1"/><a:t>jour</a:t></a:r></a:p><a:p><a:pPr lvl="1"/><a:r><a:t>Deux</a:t></a:r></a:p>"#);
    assert_that!(t.paragraphs.iter().map(|p| (p.level, p.runs.len())).collect::<Vec<_>>()).is_equal_to(vec![(0, 2), (1, 1)]);
    assert_that!(t.paragraphs[0].runs[1].props.bold).is_equal_to(Some(true));
}

#[test]
fn line_break_becomes_a_newline_run() {
    let t = body(r#"<a:bodyPr/><a:p><a:r><a:t>a</a:t></a:r><a:br/><a:r><a:t>b</a:t></a:r></a:p>"#);
    assert_that!(t.paragraphs[0].runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["a", "\n", "b"]);
}

#[test]
fn field_keeps_its_type_and_cached_text() {
    let t = body(r#"<a:bodyPr/><a:p><a:fld id="{1}" type="slidenum"><a:t>3</a:t></a:fld></a:p>"#);
    let r = &t.paragraphs[0].runs[0];
    assert_that!((r.text.as_str(), r.field.as_deref())).is_equal_to(("3", Some("slidenum")));
}

#[test]
fn rpr_reads_size_style_and_spacing() {
    let p = first_run(r#"<a:rPr sz="2400" i="1" u="sng" baseline="30000" cap="all" spc="50"/>"#);
    assert_that!((p.size, p.italic, p.underline, p.baseline, p.caps, p.char_spacing))
        .is_equal_to((Some(24.0), Some(true), Some(true), Some(30), Some(Caps::All), Some(0.5)));
}

#[test]
fn rpr_explicit_false_is_kept_to_override_inheritance() {
    let p = first_run(r#"<a:rPr b="0" u="none"/>"#);
    assert_that!((p.bold, p.underline)).is_equal_to((Some(false), Some(false)));
}

#[test]
fn rpr_keeps_theme_font_and_unresolved_scheme_color() {
    let p = first_run(r#"<a:rPr><a:solidFill><a:schemeClr val="tx1"/></a:solidFill><a:latin typeface="+mj-lt"/></a:rPr>"#);
    assert_that!((p.font.clone(), p.color.map(|c| c.base))).is_equal_to((Some(FontRef::Major), Some(ColorBase::Scheme("tx1".into()))));
}

#[test]
fn rpr_clips_absurd_font_names() {
    let p = first_run(&format!(r#"<a:rPr><a:latin typeface="{}"/></a:rPr>"#, "x".repeat(10_000)));
    let Some(FontRef::Named(n)) = p.font else { panic!("police nommée attendue") };
    assert_that!(n.chars().count()).is_equal_to(MAX_FONT_NAME);
}

#[test]
fn rpr_link_goes_through_the_link_resolver() {
    assert_that!(first_run(r#"<a:rPr><a:hlinkClick r:id="rId7"/></a:rPr>"#).link.as_deref()).is_equal_to(Some("https://oku.example"));
}

#[test]
fn ppr_reads_alignment_indents_spacing_and_char_bullet() {
    let t = body(r#"<a:bodyPr/><a:p><a:pPr algn="ctr" marL="342900" indent="-342900"><a:lnSpc><a:spcPct val="90000"/></a:lnSpc><a:spcBef><a:spcPts val="1000"/></a:spcBef><a:buFont typeface="Arial"/><a:buChar char="•"/></a:pPr><a:r><a:t>x</a:t></a:r></a:p>"#);
    let p = &t.paragraphs[0].props;
    assert_that!((p.align, p.margin_left, p.indent)).is_equal_to((Some(Align::Center), Some(27.0), Some(-27.0)));
    assert_that!((p.line_spacing, p.space_before)).is_equal_to((Some(Spacing::Percent(0.9)), Some(Spacing::Points(10.0))));
    assert_that!((p.bullet.kind.clone(), p.bullet.font.clone())).is_equal_to((Some(BulletKind::Char("•".into())), Some(Some("Arial".into()))));
}

#[test]
fn ppr_clips_a_long_bullet_character() {
    let t = body(&format!(r#"<a:bodyPr/><a:p><a:pPr><a:buChar char="{}"/></a:pPr></a:p>"#, "•".repeat(1000)));
    let Some(BulletKind::Char(c)) = t.paragraphs[0].props.bullet.kind.clone() else { panic!("puce attendue") };
    assert_that!(c.chars().count()).is_equal_to(8);
}

#[test]
fn ppr_reads_auto_numbering_none_and_text_colored_bullet() {
    let t = body(r#"<a:bodyPr/><a:p><a:pPr><a:buClrTx/><a:buAutoNum type="alphaLcParenR" startAt="3"/></a:pPr></a:p><a:p><a:pPr><a:buNone/></a:pPr></a:p>"#);
    assert_that!(t.paragraphs[0].props.bullet.kind.clone()).is_equal_to(Some(BulletKind::AutoNum { scheme: "alphaLcParenR".into(), start: 3 }));
    assert_that!((t.paragraphs[0].props.bullet.color.clone(), t.paragraphs[1].props.bullet.kind.clone())).is_equal_to((Some(None), Some(BulletKind::None)));
}

#[test]
fn lst_style_puts_def_ppr_under_each_level() {
    let t = body(r#"<a:bodyPr/><a:lstStyle><a:defPPr algn="r"/><a:lvl2pPr marL="12700"/></a:lstStyle><a:p/>"#);
    assert_that!((t.lst.len(), t.lst[1].align, t.lst[1].margin_left, t.lst[0].margin_left)).is_equal_to((LEVELS, Some(Align::Right), Some(1.0), None));
}

#[test]
fn body_pr_reads_insets_anchor_wrap_columns_and_autofit() {
    let t = body(r#"<a:bodyPr lIns="0" anchor="ctr" wrap="none" numCol="2"><a:normAutofit fontScale="62500" lnSpcReduction="20000"/></a:bodyPr><a:p/>"#);
    assert_that!((t.body.insets[0], t.body.insets[1], t.body.anchor, t.body.wrap, t.body.columns)).is_equal_to((Some(0.0), None, Some(Anchor::Middle), Some(false), Some(2)));
    assert_that!(t.body.autofit).is_equal_to(Some(Autofit::Shrink { font_scale: 0.625, line_reduction: 0.2 }));
}

#[test]
fn body_pr_bounds_column_counts() {
    assert_that!(body(r#"<a:bodyPr numCol="4294967295"/><a:p/>"#).body.columns).is_equal_to(Some(16));
    assert_that!(body(r#"<a:bodyPr numCol="-1"/><a:p/>"#).body.columns).is_equal_to(Some(1));
}

#[test]
fn norm_autofit_without_scale_is_a_neutral_shrink() {
    assert_that!(body(r#"<a:bodyPr><a:normAutofit/></a:bodyPr><a:p/>"#).body.autofit).is_equal_to(Some(Autofit::Shrink { font_scale: 1.0, line_reduction: 0.0 }));
}

#[test]
fn para_overlay_keeps_lower_values_not_overridden() {
    let mut low = ParaProps { align: Some(Align::Left), ..ParaProps::default() };
    low.run.size = Some(18.0);
    let mut top = ParaProps { align: Some(Align::Right), ..ParaProps::default() };
    top.run.bold = Some(true);
    low.overlay(&top);
    assert_that!((low.align, low.run.size, low.run.bold)).is_equal_to((Some(Align::Right), Some(18.0), Some(true)));
}
```

- [ ] **Step 2 : lancer, constater l'échec** — FAIL.

- [ ] **Step 3 : implémentation**

`src/pptx/text.rs` :

```rust
//! Texte tel qu'écrit dans une part : propriétés partielles, couleurs non résolues, valeurs bornées.

use roxmltree::Node;

use crate::collect::clip_to;
use crate::pptx::color::{ColorSpec, color_spec_in};
use crate::pptx::theme::MAX_FONT_NAME;
use crate::pptx::units::{emu, hundredths, pct};
use crate::pptx::xml::{attr_bool, attr_i64, attr_r, attr_str, attr_u32, child, elements};
use crate::source::{Align, Anchor, Autofit, Caps, Spacing};

/// Niveaux de texte d'OOXML (`lvl1pPr`..`lvl9pPr`).
pub(crate) const LEVELS: usize = 9;
const MAX_COLUMNS: u32 = 16;
const MAX_BULLET_CHARS: usize = 8;
const MAX_SCHEME_CHARS: usize = 32;

/// Police d'un run : de thème ou nommée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FontRef {
    Major,
    Minor,
    Named(String),
}

/// Propriétés de run partielles (`rPr`, `defRPr`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RunProps {
    pub font: Option<FontRef>,
    pub size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strike: Option<bool>,
    pub color: Option<ColorSpec>,
    pub highlight: Option<ColorSpec>,
    pub baseline: Option<i32>,
    pub caps: Option<Caps>,
    pub char_spacing: Option<f64>,
    pub link: Option<String>,
}

/// `$self.$f` prend `$top.$f` quand il est présent.
macro_rules! overlay_fields {
    ($self:ident, $top:ident, $($f:ident),+) => {
        $( if $top.$f.is_some() { $self.$f.clone_from(&$top.$f); } )+
    };
}

impl RunProps {
    pub(crate) fn overlay(&mut self, top: &RunProps) {
        overlay_fields!(self, top, font, size, bold, italic, underline, strike, color, highlight, baseline, caps, char_spacing, link);
    }
}

/// Sorte de puce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BulletKind {
    None,
    Char(String),
    AutoNum { scheme: String, start: u32 },
}

/// Propriétés de puce partielles ; chacune s'hérite séparément.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BulletProps {
    pub kind: Option<BulletKind>,
    /// `Some(None)` : la puce suit la couleur du texte (`buClrTx`).
    pub color: Option<Option<ColorSpec>>,
    pub size_pct: Option<f64>,
    /// `Some(None)` : la puce suit la police du texte (`buFontTx`).
    pub font: Option<Option<String>>,
}

impl BulletProps {
    pub(crate) fn overlay(&mut self, top: &BulletProps) {
        overlay_fields!(self, top, kind, color, size_pct, font);
    }
}

/// Propriétés de paragraphe partielles (`pPr`, `lvlNpPr`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ParaProps {
    pub align: Option<Align>,
    pub indent: Option<f64>,
    pub margin_left: Option<f64>,
    pub line_spacing: Option<Spacing>,
    pub space_before: Option<Spacing>,
    pub space_after: Option<Spacing>,
    pub bullet: BulletProps,
    pub run: RunProps,
}

impl ParaProps {
    pub(crate) fn overlay(&mut self, top: &ParaProps) {
        overlay_fields!(self, top, align, indent, margin_left, line_spacing, space_before, space_after);
        self.bullet.overlay(&top.bullet);
        self.run.overlay(&top.run);
    }
}

/// Propriétés par niveau 0..8.
pub(crate) type LevelProps = Vec<ParaProps>;

/// Neuf niveaux vides.
pub(crate) fn empty_levels() -> LevelProps {
    vec![ParaProps::default(); LEVELS]
}

/// Propriétés de corps partielles (`bodyPr`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BodyProps {
    /// l, t, r, b en pt.
    pub insets: [Option<f64>; 4],
    pub anchor: Option<Anchor>,
    pub autofit: Option<Autofit>,
    pub wrap: Option<bool>,
    pub columns: Option<u32>,
}

impl BodyProps {
    pub(crate) fn overlay(&mut self, top: &BodyProps) {
        for (mine, theirs) in self.insets.iter_mut().zip(top.insets) {
            if theirs.is_some() {
                *mine = theirs;
            }
        }
        overlay_fields!(self, top, anchor, autofit, wrap, columns);
    }
}

/// Run tel qu'écrit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawRun {
    pub text: String,
    pub field: Option<String>,
    pub props: RunProps,
}

/// Paragraphe tel qu'écrit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawParagraph {
    pub level: u8,
    pub props: ParaProps,
    pub runs: Vec<RawRun>,
}

/// Corps de texte tel qu'écrit (`txBody`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawText {
    pub body: BodyProps,
    pub lst: LevelProps,
    pub paragraphs: Vec<RawParagraph>,
}

/// Ancrage d'un attribut `anchor` (`t`, `ctr`, `b`).
pub(crate) fn anchor(v: &str) -> Anchor {
    match v {
        "ctr" => Anchor::Middle,
        "b" => Anchor::Bottom,
        _ => Anchor::Top,
    }
}

/// Police de thème d'un `fontRef` (`idx="major"|"minor"`).
pub(crate) fn font_ref(r: Node<'_, '_>) -> Option<FontRef> {
    match attr_str(r, "idx")? {
        "major" => Some(FontRef::Major),
        "minor" => Some(FontRef::Minor),
        _ => None,
    }
}

fn spacing(n: Node<'_, '_>) -> Option<Spacing> {
    if let Some(p) = child(n, "spcPct") {
        return attr_i64(p, "val").map(|v| Spacing::Percent(pct(v)));
    }
    child(n, "spcPts").and_then(|p| attr_i64(p, "val")).map(|v| Spacing::Points(hundredths(v)))
}

fn typeface(t: &str) -> FontRef {
    match t {
        "+mj-lt" | "+mj-ea" | "+mj-cs" => FontRef::Major,
        "+mn-lt" | "+mn-ea" | "+mn-cs" => FontRef::Minor,
        t => FontRef::Named(clip_to(t, MAX_FONT_NAME)),
    }
}

/// Lit un `rPr`/`defRPr`/`endParaRPr`.
pub(crate) fn parse_rpr(rpr: Node<'_, '_>, link: &mut dyn FnMut(&str) -> Option<String>) -> RunProps {
    RunProps {
        font: child(rpr, "latin").and_then(|l| attr_str(l, "typeface")).map(typeface),
        size: attr_i64(rpr, "sz").map(hundredths),
        bold: attr_bool(rpr, "b"),
        italic: attr_bool(rpr, "i"),
        underline: attr_str(rpr, "u").map(|u| u != "none"),
        strike: attr_str(rpr, "strike").map(|s| s != "noStrike"),
        color: child(rpr, "solidFill").and_then(color_spec_in),
        highlight: child(rpr, "highlight").and_then(color_spec_in),
        baseline: attr_i64(rpr, "baseline").map(|b| i32::try_from(b / 1000).unwrap_or(0)),
        caps: attr_str(rpr, "cap").map(|c| match c {
            "all" => Caps::All,
            "small" => Caps::Small,
            _ => Caps::None,
        }),
        char_spacing: attr_i64(rpr, "spc").map(hundredths),
        link: child(rpr, "hlinkClick").and_then(|h| attr_r(h, "id")).and_then(link),
    }
}

/// Lit un `pPr`/`lvlNpPr`/`defPPr`.
pub(crate) fn parse_ppr(ppr: Node<'_, '_>) -> ParaProps {
    let mut bullet = BulletProps::default();
    for b in elements(ppr) {
        match b.tag_name().name() {
            "buNone" => bullet.kind = Some(BulletKind::None),
            "buChar" => bullet.kind = attr_str(b, "char").map(|c| BulletKind::Char(clip_to(c, MAX_BULLET_CHARS))),
            "buAutoNum" => {
                bullet.kind = Some(BulletKind::AutoNum {
                    scheme: clip_to(attr_str(b, "type").unwrap_or("arabicPeriod"), MAX_SCHEME_CHARS),
                    start: attr_u32(b, "startAt").unwrap_or(1).max(1),
                });
            }
            "buClrTx" => bullet.color = Some(None),
            "buClr" => bullet.color = color_spec_in(b).map(Some),
            "buSzPct" => bullet.size_pct = attr_i64(b, "val").map(pct),
            "buFontTx" => bullet.font = Some(None),
            "buFont" => bullet.font = attr_str(b, "typeface").map(|t| Some(clip_to(t, MAX_FONT_NAME))),
            _ => {}
        }
    }
    ParaProps {
        align: attr_str(ppr, "algn").map(|a| match a {
            "ctr" => Align::Center,
            "r" => Align::Right,
            "just" | "dist" => Align::Justify,
            _ => Align::Left,
        }),
        indent: attr_i64(ppr, "indent").map(emu),
        margin_left: attr_i64(ppr, "marL").map(emu),
        line_spacing: child(ppr, "lnSpc").and_then(spacing),
        space_before: child(ppr, "spcBef").and_then(spacing),
        space_after: child(ppr, "spcAft").and_then(spacing),
        bullet,
        run: child(ppr, "defRPr").map(|r| parse_rpr(r, &mut |_| None)).unwrap_or_default(),
    }
}

/// Lit un `lstStyle` (ou `titleStyle`/`bodyStyle`/`otherStyle`, `defaultTextStyle`).
pub(crate) fn parse_lst_style(lst: Node<'_, '_>) -> LevelProps {
    let def = child(lst, "defPPr").map(parse_ppr).unwrap_or_default();
    (1..=LEVELS)
        .map(|i| {
            let mut p = def.clone();
            if let Some(l) = child(lst, &format!("lvl{i}pPr")) {
                p.overlay(&parse_ppr(l));
            }
            p
        })
        .collect()
}

/// Lit un `bodyPr`.
pub(crate) fn parse_body_pr(bp: Node<'_, '_>) -> BodyProps {
    let autofit = elements(bp).find_map(|c| match c.tag_name().name() {
        "noAutofit" => Some(Autofit::None),
        "spAutoFit" => Some(Autofit::Resize),
        "normAutofit" => Some(Autofit::Shrink {
            font_scale: attr_i64(c, "fontScale").map_or(1.0, pct),
            line_reduction: attr_i64(c, "lnSpcReduction").map_or(0.0, pct),
        }),
        _ => None,
    });
    let columns = attr_str(bp, "numCol").map(|_| match attr_i64(bp, "numCol") {
        Some(n) if n >= 1 => u32::try_from(n).unwrap_or(MAX_COLUMNS).min(MAX_COLUMNS),
        _ => 1,
    });
    BodyProps {
        insets: ["lIns", "tIns", "rIns", "bIns"].map(|a| attr_i64(bp, a).map(emu)),
        anchor: attr_str(bp, "anchor").map(anchor),
        autofit,
        wrap: attr_str(bp, "wrap").map(|w| w != "none"),
        columns,
    }
}

/// Lit un `txBody` (`p:txBody`, `a:txBody` de cellule, `dsp:txBody`).
pub(crate) fn parse_txbody(tx: Node<'_, '_>, link: &mut dyn FnMut(&str) -> Option<String>) -> RawText {
    let mut paragraphs = Vec::new();
    for p in elements(tx).filter(|n| n.tag_name().name() == "p") {
        let ppr = child(p, "pPr");
        let mut runs = Vec::new();
        for r in elements(p) {
            let cached = || child(r, "t").and_then(|t| t.text()).unwrap_or_default().to_owned();
            let (text, field) = match r.tag_name().name() {
                "r" => (cached(), None),
                "fld" => (cached(), attr_str(r, "type").map(|t| clip_to(t, MAX_SCHEME_CHARS))),
                "br" => ("\n".to_owned(), None),
                _ => continue,
            };
            let props = child(r, "rPr").map(|rp| parse_rpr(rp, link)).unwrap_or_default();
            runs.push(RawRun { text, field, props });
        }
        paragraphs.push(RawParagraph {
            level: ppr.and_then(|n| attr_u32(n, "lvl")).map_or(0, |l| u8::try_from(l.min(8)).unwrap_or(8)),
            props: ppr.map(parse_ppr).unwrap_or_default(),
            runs,
        });
    }
    RawText {
        body: child(tx, "bodyPr").map(parse_body_pr).unwrap_or_default(),
        lst: child(tx, "lstStyle").map(parse_lst_style).unwrap_or_else(empty_levels),
        paragraphs,
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;
```

- [ ] **Step 4 : lancer, constater le succès** — PASS.

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : texte brut — propriétés partielles, champs, espacement, valeurs bornées (t-692)
oku cli task done --project slider --id t-711
```

---

### Task 7: La cascade des styles texte (couleurs résolues par la part qui affiche)

**Files:**
- Create: `src/pptx/cascade.rs`, `src/pptx/cascade_tests.rs`
- Modify: `src/pptx/mod.rs` (`pub(crate) mod cascade;`)

**Interfaces:**
- Consumes : `RawText`, `ParaProps`, `RunProps`, `BodyProps`, `LevelProps`, `BulletKind`, `FontRef`, `LEVELS` (6) ; `ColorCtx`, `ColorSpec::resolve` (3).
- Produces :

```rust
pub(crate) struct TextChain<'a> { pub levels: Vec<&'a LevelProps>, pub bodies: Vec<&'a BodyProps> } // du plus faible au plus fort
pub(crate) struct TextEnv<'a> { pub major: &'a str, pub minor: &'a str, pub colors: &'a ColorCtx, pub font_ref: RunProps }
pub(crate) fn resolve_textbox(raw: &RawText, chain: &TextChain<'_>, env: &TextEnv<'_>) -> TextBox;
pub(crate) fn resolve_levels(chain: &TextChain<'_>, lst: &LevelProps, env: &TextEnv<'_>) -> Vec<LevelStyle>;
```

Ordre (niveau L, du plus faible au plus fort) : `chain.levels[i][L]` → `env.font_ref` (run seulement) → `raw.lst[L]` → `paragraph.props` → `run.props`. Couleurs résolues à la fin avec `env.colors`. Défauts : 18 pt ; police `minor` ; couleur `tx1` de `env.colors` (`Origin::Scheme { name: "tx1", slot, [] }`, sinon noir `Default`) ; booléens faux ; `baseline` 0 ; `caps` `None` ; `char_spacing` 0 ; gauche ; retraits 0 ; interligne `Percent(1.0)` ; espacements `Points(0.0)` ; puce `None` (taille 1.0) ; insets OOXML ; haut ; autofit `None` ; `wrap` vrai ; 1 colonne.

- [ ] **Step 1 : tests qui échouent**

`src/pptx/cascade_tests.rs` :

```rust
use std::collections::BTreeMap;
use std::rc::Rc;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::pptx::color::{ColorBase, ColorCtx, ColorSpec, default_map};
use crate::pptx::text::{BodyProps, BulletKind, FontRef, ParaProps, RawParagraph, RawRun, RawText, RunProps, empty_levels};
use crate::source::{Align, Anchor, Bullet, Rgba, Styled};

fn colors() -> ColorCtx {
    ColorCtx::new(BTreeMap::from([("dk1".to_owned(), Rgba::new(0, 0, 0)), ("lt1".to_owned(), Rgba::new(255, 255, 255))]), default_map())
}

fn env(c: &ColorCtx) -> TextEnv<'_> {
    TextEnv { major: "Calibri Light", minor: "Calibri", colors: c, font_ref: RunProps::default() }
}

fn levels_with(level: usize, p: ParaProps) -> LevelProps {
    let mut l = empty_levels();
    l[level] = p;
    l
}

fn one_run(level: u8, para: ParaProps, run: RunProps) -> RawText {
    RawText { body: BodyProps::default(), lst: empty_levels(), paragraphs: vec![RawParagraph { level, props: para, runs: vec![RawRun { text: "x".into(), field: None, props: run }] }] }
}

fn size(s: f64) -> ParaProps {
    ParaProps { run: RunProps { size: Some(s), ..RunProps::default() }, ..ParaProps::default() }
}

fn no_chain() -> TextChain<'static> {
    TextChain { levels: vec![], bodies: vec![] }
}

#[test]
fn empty_chain_gives_reader_defaults() {
    let c = colors();
    let tb = resolve_textbox(&one_run(0, ParaProps::default(), RunProps::default()), &no_chain(), &env(&c));
    let r = &tb.paragraphs[0].runs[0].style;
    assert_that!((r.size, r.char_spacing, r.bold)).is_equal_to((18.0, 0.0, false));
    assert_that!(r.font.clone()).is_equal_to(Styled::scheme("Calibri".to_owned(), "minor"));
    assert_that!(r.color.clone()).is_equal_to(Styled::mapped(Rgba::new(0, 0, 0), "tx1", "dk1", vec![]));
    assert_that!((tb.wrap, tb.columns)).is_equal_to((true, 1));
}

#[rstest]
// Une valeur distincte par couche : on vérifie laquelle gagne.
#[case(Some(10.0), Some(20.0), None, None, None, 20.0)]
#[case(Some(10.0), Some(20.0), Some(30.0), None, None, 30.0)]
#[case(Some(10.0), Some(20.0), Some(30.0), Some(40.0), None, 40.0)]
#[case(Some(10.0), Some(20.0), Some(30.0), Some(40.0), Some(50.0), 50.0)]
#[case(Some(10.0), None, None, None, None, 10.0)]
fn size_follows_the_stronger_layer(
    #[case] first: Option<f64>,
    #[case] second: Option<f64>,
    #[case] lst: Option<f64>,
    #[case] ppr: Option<f64>,
    #[case] rpr: Option<f64>,
    #[case] expected: f64,
) {
    let c = colors();
    let a = levels_with(0, first.map(size).unwrap_or_default());
    let b = levels_with(0, second.map(size).unwrap_or_default());
    let mut raw = one_run(0, ppr.map(size).unwrap_or_default(), RunProps { size: rpr, ..RunProps::default() });
    raw.lst = levels_with(0, lst.map(size).unwrap_or_default());
    let chain = TextChain { levels: vec![&a, &b], bodies: vec![] };
    assert_that!(resolve_textbox(&raw, &chain, &env(&c)).paragraphs[0].runs[0].style.size).is_equal_to(expected);
}

#[test]
fn level_picks_its_own_layer_entry() {
    let c = colors();
    let mut m = empty_levels();
    m[0] = size(32.0);
    m[1] = size(28.0);
    let tb = resolve_textbox(&one_run(1, ParaProps::default(), RunProps::default()), &TextChain { levels: vec![&m], bodies: vec![] }, &env(&c));
    assert_that!((tb.paragraphs[0].level, tb.paragraphs[0].runs[0].style.size)).is_equal_to((1, 28.0));
}

#[test]
fn font_ref_beats_the_chain() {
    let c = colors();
    let m = levels_with(0, ParaProps { run: RunProps { font: Some(FontRef::Named("Arial".into())), ..RunProps::default() }, ..ParaProps::default() });
    let mut e = env(&c);
    e.font_ref.font = Some(FontRef::Major);
    let tb = resolve_textbox(&one_run(0, ParaProps::default(), RunProps::default()), &TextChain { levels: vec![&m], bodies: vec![] }, &e);
    assert_that!(tb.paragraphs[0].runs[0].style.font.clone()).is_equal_to(Styled::scheme("Calibri Light".to_owned(), "major"));
}

#[test]
fn shape_lst_style_beats_the_font_ref() {
    let c = colors();
    let mut e = env(&c);
    e.font_ref.font = Some(FontRef::Major);
    let mut raw = one_run(0, ParaProps::default(), RunProps::default());
    raw.lst[0].run.font = Some(FontRef::Named("Georgia".into()));
    assert_that!(resolve_textbox(&raw, &no_chain(), &e).paragraphs[0].runs[0].style.font.clone()).is_equal_to(Styled::explicit("Georgia".to_owned()));
}

#[test]
fn explicit_false_overrides_inherited_bold() {
    let c = colors();
    let m = levels_with(0, ParaProps { run: RunProps { bold: Some(true), ..RunProps::default() }, ..ParaProps::default() });
    let raw = one_run(0, ParaProps::default(), RunProps { bold: Some(false), ..RunProps::default() });
    assert_that!(resolve_textbox(&raw, &TextChain { levels: vec![&m], bodies: vec![] }, &env(&c)).paragraphs[0].runs[0].style.bold).is_false();
}

#[test]
fn inherited_scheme_color_resolves_with_the_consumer_color_map() {
    let mut map = default_map();
    map.insert("tx1".into(), "lt1".into());
    let dark = ColorCtx { map: Rc::new(map), ..colors() };
    let tx1 = ColorSpec { base: ColorBase::Scheme("tx1".into()), mods: vec![] };
    let m = levels_with(0, ParaProps { run: RunProps { color: Some(tx1), ..RunProps::default() }, ..ParaProps::default() });
    let tb = resolve_textbox(&one_run(0, ParaProps::default(), RunProps::default()), &TextChain { levels: vec![&m], bodies: vec![] }, &env(&dark));
    assert_that!(tb.paragraphs[0].runs[0].style.color.clone()).is_equal_to(Styled::mapped(Rgba::new(255, 255, 255), "tx1", "lt1", vec![]));
}

#[test]
fn bullet_parts_inherit_separately() {
    let c = colors();
    let mut m = ParaProps::default();
    m.bullet.kind = Some(BulletKind::Char("•".into()));
    m.bullet.size_pct = Some(0.8);
    let master = levels_with(0, m);
    let mut p = ParaProps::default();
    p.bullet.color = Some(Some(ColorSpec { base: ColorBase::Fixed(Rgba::new(255, 0, 0)), mods: vec![] }));
    let tb = resolve_textbox(&one_run(0, p, RunProps::default()), &TextChain { levels: vec![&master], bodies: vec![] }, &env(&c));
    let Bullet::Char { char, color, size_pct, font } = &tb.paragraphs[0].style.bullet else { panic!("puce attendue") };
    assert_that!((char.as_str(), *size_pct, font.clone(), color.as_ref().map(|c| c.value))).is_equal_to(("•", 0.8, None, Some(Rgba::new(255, 0, 0))));
}

#[test]
fn body_props_cascade_through_bodies_then_own() {
    let c = colors();
    let layout_body = BodyProps { anchor: Some(Anchor::Bottom), insets: [Some(0.0), None, None, None], ..BodyProps::default() };
    let mut raw = one_run(0, ParaProps::default(), RunProps::default());
    raw.body.anchor = Some(Anchor::Middle);
    let tb = resolve_textbox(&raw, &TextChain { levels: vec![], bodies: vec![&layout_body] }, &env(&c));
    assert_that!((tb.anchor, tb.insets.l, tb.insets.t)).is_equal_to((Anchor::Middle, 0.0, 3.6));
}

#[test]
fn resolve_levels_gives_nine_level_styles() {
    let c = colors();
    let m = levels_with(2, ParaProps { align: Some(Align::Center), ..size(20.0) });
    let lv = resolve_levels(&TextChain { levels: vec![&m], bodies: vec![] }, &empty_levels(), &env(&c));
    assert_that!((lv.len(), lv[2].para.align, lv[2].run.size, lv[0].run.size)).is_equal_to((9, Align::Center, 20.0, 18.0));
}
```

- [ ] **Step 2 : lancer, constater l'échec** — FAIL.

- [ ] **Step 3 : implémentation**

`src/pptx/cascade.rs` :

```rust
//! La cascade des styles texte : la seule fonction qui résout l'héritage OOXML.
//!
//! Couches, du plus faible au plus fort : `TextChain` (défauts de la
//! présentation, `txStyles` du master, `lstStyle` des placeholders du master
//! puis du layout) ; le `fontRef` de `p:style` ; le `lstStyle` de la forme ;
//! `pPr` ; `rPr`. Les couleurs sont résolues à la fin, avec le `ColorCtx` de
//! la part qui affiche le texte. Ce qui reste vide prend les défauts du lecteur.

use crate::pptx::color::ColorCtx;
use crate::pptx::text::{BodyProps, BulletKind, FontRef, LEVELS, LevelProps, ParaProps, RawText, RunProps};
use crate::source::{
    Align, Anchor, Autofit, Bullet, Caps, Color, Insets, LevelStyle, ParaStyle, Paragraph, Rgba, Run, RunStyle, Spacing, Styled,
    TextBox,
};

const DEFAULT_SIZE_PT: f64 = 18.0;

/// Couches héritées, de la plus faible à la plus forte.
#[derive(Debug)]
pub(crate) struct TextChain<'a> {
    pub levels: Vec<&'a LevelProps>,
    pub bodies: Vec<&'a BodyProps>,
}

/// Ce que la cascade ne trouve pas dans les couches.
#[derive(Debug)]
pub(crate) struct TextEnv<'a> {
    pub major: &'a str,
    pub minor: &'a str,
    pub colors: &'a ColorCtx,
    pub font_ref: RunProps,
}

impl TextEnv<'_> {
    fn default_color(&self) -> Color {
        self.colors
            .slot("tx1")
            .map_or_else(|| Styled::default_value(Rgba::new(0, 0, 0)), |(slot, v)| Styled::mapped(v, "tx1", &slot, Vec::new()))
    }
}

fn level_props(chain: &TextChain<'_>, env: &TextEnv<'_>, lst: &LevelProps, level: usize) -> ParaProps {
    let mut acc = ParaProps::default();
    for l in &chain.levels {
        if let Some(p) = l.get(level) {
            acc.overlay(p);
        }
    }
    acc.run.overlay(&env.font_ref);
    if let Some(p) = lst.get(level) {
        acc.overlay(p);
    }
    acc
}

fn run_style(p: &RunProps, env: &TextEnv<'_>) -> RunStyle {
    let font = match &p.font {
        Some(FontRef::Major) => Styled::scheme(env.major.to_owned(), "major"),
        Some(FontRef::Named(n)) => Styled::explicit(n.clone()),
        Some(FontRef::Minor) | None => Styled::scheme(env.minor.to_owned(), "minor"),
    };
    RunStyle {
        font,
        size: p.size.unwrap_or(DEFAULT_SIZE_PT),
        bold: p.bold.unwrap_or(false),
        italic: p.italic.unwrap_or(false),
        underline: p.underline.unwrap_or(false),
        strike: p.strike.unwrap_or(false),
        color: p.color.as_ref().and_then(|c| c.resolve(env.colors)).unwrap_or_else(|| env.default_color()),
        highlight: p.highlight.as_ref().and_then(|c| c.resolve(env.colors)),
        baseline: p.baseline.unwrap_or(0),
        caps: p.caps.unwrap_or(Caps::None),
        char_spacing: p.char_spacing.unwrap_or(0.0),
        link: p.link.clone(),
    }
}

fn para_style(p: &ParaProps, env: &TextEnv<'_>) -> ParaStyle {
    let bullet = match p.bullet.kind.clone() {
        Some(BulletKind::Char(char)) => Bullet::Char {
            char,
            color: p.bullet.color.clone().flatten().and_then(|c| c.resolve(env.colors)),
            size_pct: p.bullet.size_pct.unwrap_or(1.0),
            font: p.bullet.font.clone().flatten(),
        },
        Some(BulletKind::AutoNum { scheme, start }) => Bullet::AutoNum { scheme, start },
        Some(BulletKind::None) | None => Bullet::None,
    };
    ParaStyle {
        align: p.align.unwrap_or(Align::Left),
        indent: p.indent.unwrap_or(0.0),
        margin_left: p.margin_left.unwrap_or(0.0),
        line_spacing: p.line_spacing.unwrap_or(Spacing::Percent(1.0)),
        space_before: p.space_before.unwrap_or(Spacing::Points(0.0)),
        space_after: p.space_after.unwrap_or(Spacing::Points(0.0)),
        bullet,
    }
}

/// Résout un corps de texte complet.
pub(crate) fn resolve_textbox(raw: &RawText, chain: &TextChain<'_>, env: &TextEnv<'_>) -> TextBox {
    let paragraphs = raw
        .paragraphs
        .iter()
        .map(|rp| {
            let mut p = level_props(chain, env, &raw.lst, usize::from(rp.level));
            p.overlay(&rp.props);
            let runs = rp
                .runs
                .iter()
                .map(|r| {
                    let mut props = p.run.clone();
                    props.overlay(&r.props);
                    Run { text: r.text.clone(), field: r.field.clone(), style: run_style(&props, env) }
                })
                .collect();
            Paragraph { level: rp.level, style: para_style(&p, env), runs }
        })
        .collect();
    let mut body = BodyProps::default();
    for b in &chain.bodies {
        body.overlay(b);
    }
    body.overlay(&raw.body);
    let d = TextBox::default();
    TextBox {
        paragraphs,
        insets: Insets {
            l: body.insets[0].unwrap_or(d.insets.l),
            t: body.insets[1].unwrap_or(d.insets.t),
            r: body.insets[2].unwrap_or(d.insets.r),
            b: body.insets[3].unwrap_or(d.insets.b),
        },
        anchor: body.anchor.unwrap_or(Anchor::Top),
        autofit: body.autofit.unwrap_or(Autofit::None),
        wrap: body.wrap.unwrap_or(d.wrap),
        columns: body.columns.unwrap_or(d.columns),
    }
}

/// Style de chaque niveau 0..8 d'un placeholder.
pub(crate) fn resolve_levels(chain: &TextChain<'_>, lst: &LevelProps, env: &TextEnv<'_>) -> Vec<LevelStyle> {
    (0..LEVELS)
        .map(|l| {
            let p = level_props(chain, env, lst, l);
            LevelStyle { para: para_style(&p, env), run: run_style(&p.run, env) }
        })
        .collect()
}

#[cfg(test)]
#[path = "cascade_tests.rs"]
mod cascade_tests;
```

- [ ] **Step 4 : lancer, constater le succès** — PASS.

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : cascade des styles texte, couleurs résolues par la part qui affiche (t-692)
oku cli task done --project slider --id t-712
```

---

### Task 8: Placeholders et contexte de lecture

**Files:**
- Create: `src/pptx/placeholder.rs` (+`_tests`), `src/pptx/context.rs` (+`_tests`)
- Modify: `src/pptx/mod.rs` (`pub(crate) mod placeholder; pub(crate) mod context;`), `src/pptx/test_support.rs` (`PNG`, contextes de test)

**Interfaces:**
- Consumes : `Package`, `Rels` (2) ; `Sink`, `clip`, `clip_to` (2) ; `ColorCtx` (3) ; `FillSpec`, `LineSpec` (4) ; `ThemeData` (5) ; `LevelProps`, `BodyProps`, `RunProps` (6) ; `TextEnv` (7).
- Produces (`placeholder.rs`) :

```rust
pub(crate) struct PhKey { pub ty: String, pub idx: Option<u32> }   // "obj" par défaut
impl PhKey { fn from_ph(ph: Node) -> PhKey; fn role(&self) -> Role; fn master_type(&self) -> &str; }
pub(crate) struct PhInfo { pub id: String, pub key: PhKey, pub frame: Frame, pub lst: LevelProps, pub body: BodyProps,
                           pub fill: Option<FillSpec>, pub line: LineSpec, pub geom: Option<Geometry> }
#[derive(Default)] pub(crate) struct PhTable { pub entries: Vec<PhInfo> }
impl PhTable { fn find(&self, key) -> Option<&PhInfo>; fn find_master(&self, key) -> Option<&PhInfo>; }
pub(crate) const MAX_PLACEHOLDERS: usize = 256;              // par part
pub(crate) struct TextStyles { pub pres: LevelProps, pub title: LevelProps, pub body: LevelProps, pub other: LevelProps }
impl TextStyles { fn for_key(&self, key: Option<&PhKey>) -> &LevelProps; }
#[derive(Default)] pub(crate) struct Inherited { pub levels: Vec<LevelProps>, pub bodies: Vec<BodyProps>, pub frame: Option<Frame>,
                           pub fill: Option<FillSpec>, pub line: LineSpec, pub geom: Option<Geometry>, pub link: Option<PlaceholderLink> }
pub(crate) struct PhInheritance<'a> { pub styles: &'a TextStyles, pub master: Option<&'a PhTable>, pub layout: Option<&'a PhTable> }
impl PhInheritance<'_> { fn lookup(&self, ph: Option<&PhKey>) -> Inherited; }
```

  `find` : même `idx`, sinon même type, sinon même `master_type`. `find_master` : même `master_type` (`ctrTitle`→`title` ; `dt`/`ftr`/`sldNum` tels quels ; le reste → `body`). `for_key` : titres → `title` ; `dt`/`ftr`/`sldNum` et forme ordinaire → `other` ; reste → `body`. `lookup(None)` → `[pres, other]`. `lookup(Some(k))` → `levels = [pres, for_key(k), master.lst?, layout.lst?]`, `bodies = [master.body?, layout.body?]`, `frame`/`fill`/`geom` du layout sinon du master, `line = layout.line.over(master.line)`, `link`.

- Produces (`context.rs`) :

```rust
pub(crate) struct Shared<'x> { pub pkg: &'x Package<'x>, pub table_styles: Option<Document<'x>>, pub pres: LevelProps, pub sink: RefCell<Sink> }
impl<'x> Shared<'x> { pub(crate) fn new(pkg: &'x Package<'x>, table_styles: Option<Document<'x>>, pres: LevelProps) -> Self; }
pub(crate) struct PartSpec<'p> { pub owner: String, pub slide: Option<u32>, pub colors: ColorCtx, pub theme: &'p ThemeData<'p>, pub inherit: Option<&'p PhInheritance<'p>> }
pub(crate) struct PartCtx<'p> { pub shared, pub rels: Rc<Rels>, pub owner, pub slide, pub colors, pub theme, pub inherit, /* ids (privé) */ }
impl<'p> PartCtx<'p> {
    pub(crate) fn new(shared: &'p Shared<'p>, part: &str, spec: PartSpec<'p>) -> Result<Self, ImportError>;
    pub(crate) fn pkg(&self) -> &'p Package<'p>;
    pub(crate) fn nodes(&self) -> u32;
    pub(crate) fn lookup(&self, ph: Option<&PhKey>) -> Inherited;   // sans héritage : [pres]
    pub(crate) fn item_id(&self, cnv_id: &str) -> String;           // entier, sinon "x" ; ~2, ~3… jusqu'à un id libre
    pub(crate) fn item_id_raw(&self, key: &str) -> String;          // modelId : [A-Za-z0-9{}-], 64 car.
    pub(crate) fn warn(&self, item: Option<&str>, kind: WarningKind);
    pub(crate) fn fail(&self, e: ImportError);
    pub(crate) fn asset(&self, rid: &str) -> Option<String>;
    pub(crate) fn link(&self, rid: &str) -> Option<String>;
    pub(crate) fn part_text(&self, part: &str, item: &str) -> Option<String>;
    pub(crate) fn env(&self, font_ref: RunProps) -> TextEnv<'_>;
}
```

- Produces (tests, `test_support.rs`) : `PNG` ; `run_ctx(pkg, part, table_styles: Option<&str>, f) -> Result<(R, Vec<Warning>, BTreeMap<String, Asset>), ImportError>` ; `with_ctx(pkg, part, f) -> (R, Vec<Warning>, BTreeMap)` ; `with_table_styles(pkg, styles, f) -> (R, Vec<Warning>)` — contexte de slide 1 (`owner` `s1`, thème `THEME`, `clrMap` par défaut, sans héritage).

- [ ] **Step 1 : tests qui échouent**

`src/pptx/placeholder_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::pptx::color::{ColorBase, ColorSpec};
use crate::pptx::text::{ParaProps, RunProps, empty_levels};
use crate::source::{Frame, Rgba};

fn sized(s: f64) -> LevelProps {
    let mut l = empty_levels();
    l[0] = ParaProps { run: RunProps { size: Some(s), ..RunProps::default() }, ..ParaProps::default() };
    l
}

fn solid(r: u8) -> Option<FillSpec> {
    Some(FillSpec::Solid(ColorSpec { base: ColorBase::Fixed(Rgba::new(r, 0, 0)), mods: vec![] }))
}

fn rect(name: &str) -> Option<Geometry> {
    Some(Geometry::Preset { name: name.into(), adj: Default::default() })
}

fn info(id: &str, ty: &str, idx: Option<u32>, x: f64, lst_size: f64) -> PhInfo {
    PhInfo { id: id.into(), key: PhKey { ty: ty.into(), idx }, frame: Frame::new(x, 0.0, 1.0, 1.0), lst: sized(lst_size), body: Default::default(), fill: None, line: Default::default(), geom: None }
}

fn master() -> PhTable {
    PhTable { entries: vec![info("m1/2", "title", None, 1.0, 30.0), info("m1/3", "body", Some(1), 2.0, 31.0), info("m1/4", "dt", Some(10), 4.0, 32.0)] }
}

fn styles() -> TextStyles {
    TextStyles { pres: sized(10.0), title: sized(20.0), body: sized(21.0), other: sized(22.0) }
}

fn top_size(levels: &[LevelProps]) -> Vec<Option<f64>> {
    levels.iter().map(|l| l[0].run.size).collect()
}

#[test]
fn find_prefers_index_then_exact_type_then_master_type() {
    let t = PhTable { entries: vec![info("l1/2", "ctrTitle", None, 9.0, 0.0), info("l1/3", "body", Some(2), 3.0, 0.0)] };
    assert_that!(t.find(&PhKey { ty: "obj".into(), idx: Some(2) }).map(|i| i.frame.x)).is_equal_to(Some(3.0));
    assert_that!(t.find(&PhKey { ty: "ctrTitle".into(), idx: None }).map(|i| i.frame.x)).is_equal_to(Some(9.0));
    assert_that!(t.find(&PhKey { ty: "title".into(), idx: None }).map(|i| i.frame.x)).is_equal_to(Some(9.0));
}

#[test]
fn find_master_uses_normalized_type_only() {
    let t = master();
    assert_that!(t.find_master(&PhKey { ty: "subTitle".into(), idx: Some(7) }).map(|i| i.frame.x)).is_equal_to(Some(2.0));
    assert_that!(t.find_master(&PhKey { ty: "ftr".into(), idx: Some(11) })).is_none();
}

#[test]
fn lookup_stacks_layers_in_order_and_links_both_parents() {
    let s = styles();
    let m = master();
    let layout = PhTable { entries: vec![info("l1/2", "ctrTitle", None, 9.0, 40.0)] };
    let got = PhInheritance { styles: &s, master: Some(&m), layout: Some(&layout) }.lookup(Some(&PhKey { ty: "ctrTitle".into(), idx: None }));
    assert_that!(top_size(&got.levels)).is_equal_to(vec![Some(10.0), Some(20.0), Some(30.0), Some(40.0)]);
    assert_that!((got.bodies.len(), got.frame.map(|f| f.x))).is_equal_to((2, Some(9.0)));
    assert_that!(got.link).is_equal_to(Some(PlaceholderLink { idx: None, layout: Some("l1/2".into()), master: Some("m1/2".into()) }));
}

#[test]
fn lookup_for_plain_shape_is_presentation_then_other() {
    let s = styles();
    let got = PhInheritance { styles: &s, master: None, layout: None }.lookup(None);
    assert_that!((top_size(&got.levels), got.link)).is_equal_to((vec![Some(10.0), Some(22.0)], None));
}

#[test]
fn lookup_for_body_uses_body_style_not_other() {
    let s = styles();
    let got = PhInheritance { styles: &s, master: None, layout: None }.lookup(Some(&PhKey { ty: "obj".into(), idx: Some(1) }));
    assert_that!(top_size(&got.levels)).is_equal_to(vec![Some(10.0), Some(21.0)]);
}

#[test]
fn lookup_layout_fill_and_geometry_beat_the_master() {
    let s = styles();
    let mut m = master();
    m.entries[0].fill = solid(1);
    m.entries[0].geom = rect("rect");
    let mut layout = PhTable { entries: vec![info("l1/2", "title", None, 0.0, 0.0)] };
    layout.entries[0].fill = solid(2);
    layout.entries[0].geom = rect("roundRect");
    let got = PhInheritance { styles: &s, master: Some(&m), layout: Some(&layout) }.lookup(Some(&PhKey { ty: "title".into(), idx: None }));
    assert_that!((got.fill, got.geom)).is_equal_to((solid(2), rect("roundRect")));
}

#[test]
fn lookup_falls_back_to_master_fill_when_layout_has_none() {
    let s = styles();
    let mut m = master();
    m.entries[0].fill = solid(1);
    let layout = PhTable { entries: vec![info("l1/2", "title", None, 0.0, 0.0)] };
    let got = PhInheritance { styles: &s, master: Some(&m), layout: Some(&layout) }.lookup(Some(&PhKey { ty: "title".into(), idx: None }));
    assert_that!(got.fill).is_equal_to(solid(1));
}

#[test]
fn ph_key_defaults_to_obj_and_maps_roles() {
    let d = crate::pptx::xml::parse(r#"<p:ph xmlns:p="urn:p" idx="1"/>"#, "t", 10).unwrap();
    let k = PhKey::from_ph(d.root_element());
    assert_that!((k.ty.as_str(), k.idx, k.role(), k.master_type())).is_equal_to(("obj", Some(1), Role::Object, "body"));
}
```

`src/pptx/context_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::ImportErrorKind;
use crate::pptx::test_support::{PNG, TestPkg, run_ctx, with_ctx};
use crate::source::WarningKind;

const SRELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/missing.png"/>
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/pic.emf"/>
<Relationship Id="rId8" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="javascript:alert(1)" TargetMode="External"/>
<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://oku.example/doc" TargetMode="External"/>
<Relationship Id="rId10" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://oku.example/a&#10;b" TargetMode="External"/>
</Relationships>"#;

fn pkg() -> TestPkg {
    TestPkg::with(&[("ppt/slides/_rels/slide1.xml.rels", SRELS.as_bytes()), ("ppt/media/image1.png", PNG), ("ppt/media/pic.emf", b"emf")])
}

#[test]
fn asset_read_twice_is_one_asset_with_mime() {
    let t = pkg();
    let ((a, b), _, assets) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (cx.asset("rId2"), cx.asset("rId2")));
    assert_that!(a.clone()).is_equal_to(b);
    assert_that!(assets.get(&a.unwrap()).map(|x| x.mime.as_str())).is_equal_to(Some("image/png"));
}

#[test]
fn missing_asset_warns_missing_part() {
    let t = pkg();
    let (a, w, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| cx.asset("rId3"));
    assert_that!(a).is_none();
    assert_that!(w.iter().any(|w| matches!(&w.kind, WarningKind::MissingPart(p) if p == "ppt/media/missing.png"))).is_true();
}

#[test]
fn emf_asset_is_kept_and_reported_unsupported() {
    let t = pkg();
    let (a, w, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| cx.asset("rId4"));
    assert_that!(a.is_some()).is_true();
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::UnsupportedImage(_)))).is_true();
}

#[test]
fn https_link_is_kept_and_unsafe_links_are_refused() {
    let t = pkg();
    let ((good, script, spaced), w, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (cx.link("rId9"), cx.link("rId8"), cx.link("rId10")));
    assert_that!((good.as_deref(), script, spaced)).is_equal_to((Some("https://oku.example/doc"), None, None));
    assert_that!(w.iter().filter(|w| matches!(w.kind, WarningKind::UnsafeLink(_))).count()).is_equal_to(2);
}

#[test]
fn item_ids_are_integers_made_unique_and_unforgeable() {
    let t = pkg();
    let (ids, _, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| [cx.item_id("4"), cx.item_id("4"), cx.item_id("4~2"), cx.item_id("../s2/1")]);
    assert_that!(ids).is_equal_to(["s1/4".to_owned(), "s1/4~2".to_owned(), "s1/x".to_owned(), "s1/x~2".to_owned()]);
}

#[test]
fn item_ids_many_duplicates_get_consecutive_suffixes() {
    // Fichier hostile : des milliers de `cNvPr id="4"` ; le suffixe suivant est mémorisé, pas recherché depuis ~2.
    let t = pkg();
    let (ids, _, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (0..5000).map(|_| cx.item_id("4")).collect::<Vec<_>>());
    assert_that!((ids[2].as_str(), ids[4999].as_str())).is_equal_to(("s1/4~3", "s1/4~5000"));
}

#[test]
fn raw_item_ids_keep_only_safe_characters() {
    let t = pkg();
    let (id, _, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| cx.item_id_raw("{A1/../B2}"));
    assert_that!(id).is_equal_to("s1/{A1B2}".to_owned());
}

#[test]
fn asset_over_part_limit_becomes_a_fatal_too_large() {
    let big = vec![0u8; 2 << 20];
    let t = TestPkg::with(&[("ppt/slides/_rels/slide1.xml.rels", SRELS.as_bytes()), ("ppt/media/image1.png", big.as_slice())]);
    let p = t.pkg();
    let err = run_ctx(&p, "ppt/slides/slide1.xml", None, |cx| cx.asset("rId2")).map(|_| ()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}
```

- [ ] **Step 2 : lancer, constater l'échec** — FAIL.

- [ ] **Step 3 : `placeholder.rs`**

```rust
//! Placeholders : clés, tables par part, styles du master, héritage slide → layout → master.

use roxmltree::Node;

use crate::pptx::fill::{FillSpec, LineSpec};
use crate::pptx::text::{BodyProps, LevelProps};
use crate::pptx::xml::{attr_str, attr_u32};
use crate::source::{Frame, Geometry, PlaceholderLink, Role};

/// Placeholders gardés par part (au-delà, ignorés).
pub(crate) const MAX_PLACEHOLDERS: usize = 256;

/// Clé d'un placeholder : type brut (`obj` par défaut) et index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PhKey {
    pub ty: String,
    pub idx: Option<u32>,
}

impl PhKey {
    /// Lit un `p:ph`.
    pub(crate) fn from_ph(ph: Node<'_, '_>) -> PhKey {
        PhKey { ty: crate::collect::clip_to(attr_str(ph, "type").unwrap_or("obj"), 16), idx: attr_u32(ph, "idx") }
    }

    /// Rôle du placeholder.
    pub(crate) fn role(&self) -> Role {
        match self.ty.as_str() {
            "title" | "ctrTitle" => Role::Title,
            "subTitle" => Role::Subtitle,
            "body" => Role::Body,
            "pic" | "clipArt" => Role::Picture,
            "chart" => Role::Chart,
            "tbl" => Role::Table,
            "media" => Role::Media,
            "dt" => Role::Date,
            "ftr" => Role::Footer,
            "sldNum" => Role::SlideNum,
            _ => Role::Object,
        }
    }

    /// Type du placeholder de master correspondant : `title`, `dt`, `ftr`, `sldNum` ou `body`.
    pub(crate) fn master_type(&self) -> &str {
        match self.ty.as_str() {
            "title" | "ctrTitle" => "title",
            "dt" | "ftr" | "sldNum" => self.ty.as_str(),
            _ => "body",
        }
    }
}

/// Un placeholder déclaré dans un master ou un layout, avec ses propriétés propres (non résolues).
#[derive(Debug, Clone)]
pub(crate) struct PhInfo {
    pub id: String,
    pub key: PhKey,
    pub frame: Frame,
    pub lst: LevelProps,
    pub body: BodyProps,
    pub fill: Option<FillSpec>,
    pub line: LineSpec,
    pub geom: Option<Geometry>,
}

/// Placeholders d'une part.
#[derive(Debug, Clone, Default)]
pub(crate) struct PhTable {
    pub entries: Vec<PhInfo>,
}

impl PhTable {
    /// Placeholder de layout correspondant à une forme de slide : index, puis type exact, puis type de master.
    pub(crate) fn find(&self, key: &PhKey) -> Option<&PhInfo> {
        key.idx
            .and_then(|i| self.entries.iter().find(|e| e.key.idx == Some(i)))
            .or_else(|| self.entries.iter().find(|e| e.key.ty == key.ty))
            .or_else(|| self.entries.iter().find(|e| e.key.master_type() == key.master_type()))
    }

    /// Placeholder de master correspondant (par type de master).
    pub(crate) fn find_master(&self, key: &PhKey) -> Option<&PhInfo> {
        self.entries.iter().find(|e| e.key.master_type() == key.master_type())
    }
}

/// Styles texte du master (`txStyles`) et défauts de la présentation (`defaultTextStyle`).
#[derive(Debug, Clone)]
pub(crate) struct TextStyles {
    pub pres: LevelProps,
    pub title: LevelProps,
    pub body: LevelProps,
    pub other: LevelProps,
}

impl TextStyles {
    /// Style de `txStyles` qui s'applique à une forme.
    pub(crate) fn for_key(&self, key: Option<&PhKey>) -> &LevelProps {
        match key.map(|k| k.ty.as_str()) {
            Some("title" | "ctrTitle") => &self.title,
            Some("dt" | "ftr" | "sldNum") | None => &self.other,
            Some(_) => &self.body,
        }
    }
}

/// Ce qu'une forme hérite de ses placeholders parents (non résolu).
#[derive(Debug, Clone, Default)]
pub(crate) struct Inherited {
    /// Couches de styles texte, de la plus faible à la plus forte.
    pub levels: Vec<LevelProps>,
    pub bodies: Vec<BodyProps>,
    pub frame: Option<Frame>,
    pub fill: Option<FillSpec>,
    pub line: LineSpec,
    pub geom: Option<Geometry>,
    pub link: Option<PlaceholderLink>,
}

/// Héritage dans une part : styles du master, placeholders du master et du layout.
#[derive(Debug)]
pub(crate) struct PhInheritance<'a> {
    pub styles: &'a TextStyles,
    pub master: Option<&'a PhTable>,
    pub layout: Option<&'a PhTable>,
}

impl PhInheritance<'_> {
    /// Héritage d'une forme ; `None` = forme ordinaire.
    pub(crate) fn lookup(&self, ph: Option<&PhKey>) -> Inherited {
        let mut levels = vec![self.styles.pres.clone(), self.styles.for_key(ph).clone()];
        let Some(key) = ph else { return Inherited { levels, ..Inherited::default() } };
        let m = self.master.and_then(|t| t.find_master(key));
        let l = self.layout.and_then(|t| t.find(key));
        let mut bodies = Vec::new();
        for info in [m, l].into_iter().flatten() {
            levels.push(info.lst.clone());
            bodies.push(info.body.clone());
        }
        let base_line = m.map(|i| i.line.clone()).unwrap_or_default();
        Inherited {
            levels,
            bodies,
            frame: l.or(m).map(|i| i.frame),
            fill: l.and_then(|i| i.fill.clone()).or_else(|| m.and_then(|i| i.fill.clone())),
            line: l.map(|i| i.line.clone()).unwrap_or_default().over(&base_line),
            geom: l.and_then(|i| i.geom.clone()).or_else(|| m.and_then(|i| i.geom.clone())),
            link: Some(PlaceholderLink { idx: key.idx, layout: l.map(|i| i.id.clone()), master: m.map(|i| i.id.clone()) }),
        }
    }
}

#[cfg(test)]
#[path = "placeholder_tests.rs"]
mod placeholder_tests;
```

- [ ] **Step 4 : `context.rs`**

```rust
//! Contexte de lecture : ressources partagées par tout le deck, et contexte d'une part.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use roxmltree::Document;

use crate::ImportErrorKind;
use crate::collect::{Sink, clip, clip_to};
use crate::error::ImportError;
use crate::pptx::cascade::TextEnv;
use crate::pptx::color::ColorCtx;
use crate::pptx::package::{Package, Rels};
use crate::pptx::placeholder::{Inherited, PhInheritance, PhKey};
use crate::pptx::text::{LevelProps, RunProps};
use crate::pptx::theme::ThemeData;
use crate::source::WarningKind;

/// Formats d'image que Typst ne dessine pas : indication pour les projecteurs.
const UNSUPPORTED_IMAGES: [&str; 5] = ["emf", "wmf", "tif", "tiff", "wdp"];
const LINK_SCHEMES: [&str; 3] = ["http://", "https://", "mailto:"];
const MAX_LINK_CHARS: usize = 2048;
const MAX_RAW_ID: usize = 64;

/// Ressources lues une fois pour tout le deck.
#[derive(Debug)]
pub(crate) struct Shared<'x> {
    pub pkg: &'x Package<'x>,
    /// `tableStyles.xml`, analysé une fois.
    pub table_styles: Option<Document<'x>>,
    /// `defaultTextStyle` de `presentation.xml` (couleurs non résolues).
    pub pres: LevelProps,
    pub sink: RefCell<Sink>,
}

impl<'x> Shared<'x> {
    pub(crate) fn new(pkg: &'x Package<'x>, table_styles: Option<Document<'x>>, pres: LevelProps) -> Self {
        Self { pkg, table_styles, pres, sink: RefCell::new(Sink::new()) }
    }
}

/// Ce qui distingue une part : propriétaire des ids, slide, couleurs, thème, héritage.
#[derive(Debug)]
pub(crate) struct PartSpec<'p> {
    pub owner: String,
    pub slide: Option<u32>,
    pub colors: ColorCtx,
    pub theme: &'p ThemeData<'p>,
    pub inherit: Option<&'p PhInheritance<'p>>,
}

/// Contexte de lecture d'une part.
#[derive(Debug)]
pub(crate) struct PartCtx<'p> {
    pub shared: &'p Shared<'p>,
    pub rels: Rc<Rels>,
    pub owner: String,
    pub slide: Option<u32>,
    pub colors: ColorCtx,
    pub theme: &'p ThemeData<'p>,
    pub inherit: Option<&'p PhInheritance<'p>>,
    ids: RefCell<BTreeSet<String>>,
    /// Prochain suffixe à essayer par id de base : `unique` reste linéaire face à des milliers de doublons.
    next: RefCell<BTreeMap<String, usize>>,
}

fn mime(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("bmp") => "image/bmp",
        Some("emf") => "image/emf",
        Some("wmf") => "image/wmf",
        Some("tif" | "tiff") => "image/tiff",
        Some("wdp") => "image/vnd.ms-photo",
        _ => "application/octet-stream",
    }
}

impl<'p> PartCtx<'p> {
    /// Contexte de la part `part` (ses relations sont lues ici).
    pub(crate) fn new(shared: &'p Shared<'p>, part: &str, spec: PartSpec<'p>) -> Result<Self, ImportError> {
        Ok(Self::with_rels(shared, shared.pkg.rels(part)?, spec))
    }

    fn with_rels(shared: &'p Shared<'p>, rels: Rc<Rels>, spec: PartSpec<'p>) -> Self {
        Self {
            shared,
            rels,
            owner: spec.owner,
            slide: spec.slide,
            colors: spec.colors,
            theme: spec.theme,
            inherit: spec.inherit,
            ids: RefCell::new(BTreeSet::new()),
            next: RefCell::new(BTreeMap::new()),
        }
    }

    pub(crate) fn pkg(&self) -> &'p Package<'p> {
        self.shared.pkg
    }

    /// Limite de nœuds d'une part XML.
    pub(crate) fn nodes(&self) -> u32 {
        self.shared.pkg.limits().max_xml_nodes
    }

    /// Héritage d'une forme ; sans héritage de placeholder : les défauts de la présentation.
    pub(crate) fn lookup(&self, ph: Option<&PhKey>) -> Inherited {
        match self.inherit {
            Some(i) => i.lookup(ph),
            None => Inherited { levels: vec![self.shared.pres.clone()], ..Inherited::default() },
        }
    }

    fn unique(&self, base: String) -> String {
        let mut ids = self.ids.borrow_mut();
        if ids.insert(base.clone()) {
            return base;
        }
        let mut next = self.next.borrow_mut();
        let n = next.entry(base.clone()).or_insert(2);
        loop {
            let candidate = format!("{base}~{n}");
            *n += 1;
            if ids.insert(candidate.clone()) {
                return candidate;
            }
        }
    }

    /// Identifiant d'item `"<owner>/<id>"` : `cNvPr@id` entier (sinon `x`), rendu unique (`~2`, `~3`…).
    pub(crate) fn item_id(&self, cnv_id: &str) -> String {
        let id = cnv_id.trim().parse::<u32>().map_or_else(|_| "x".to_owned(), |n| n.to_string());
        self.unique(format!("{}/{id}", self.owner))
    }

    /// Identifiant d'item depuis une clé libre (`modelId` de SmartArt) : `[A-Za-z0-9{}-]`, 64 caractères.
    pub(crate) fn item_id_raw(&self, key: &str) -> String {
        let safe: String = key.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '{' | '}' | '-')).take(MAX_RAW_ID).collect();
        self.unique(format!("{}/{}", self.owner, if safe.is_empty() { "x" } else { safe.as_str() }))
    }

    /// Note un avertissement.
    pub(crate) fn warn(&self, item: Option<&str>, kind: WarningKind) {
        self.shared.sink.borrow_mut().warn(self.slide, item, kind);
    }

    /// Note une erreur fatale (limite dépassée), rendue après la part.
    pub(crate) fn fail(&self, e: ImportError) {
        self.shared.sink.borrow_mut().set_fatal(e);
    }

    /// Asset de la relation `rid` (lu une fois par chemin) ; `None` si absent.
    pub(crate) fn asset(&self, rid: &str) -> Option<String> {
        let target = self.rels.target(rid)?.to_owned();
        if let Some(id) = self.shared.sink.borrow().cached_asset(&target) {
            return Some(id);
        }
        let bytes = match self.pkg().bytes(&target) {
            Ok(b) => b,
            Err(e) if e.kind() == ImportErrorKind::TooLarge => {
                self.fail(e);
                return None;
            }
            Err(_) => {
                self.warn(None, WarningKind::MissingPart(clip(&target)));
                return None;
            }
        };
        let ext = target.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
        if UNSUPPORTED_IMAGES.contains(&ext.as_str()) {
            self.warn(None, WarningKind::UnsupportedImage(clip(&target)));
        }
        Some(self.shared.sink.borrow_mut().intern(&target, bytes, mime(&target)))
    }

    /// Cible d'un lien externe, si elle est sûre : `http`, `https`, `mailto`, ≤ 2048 caractères, sans contrôle ni espace.
    pub(crate) fn link(&self, rid: &str) -> Option<String> {
        let target = self.rels.external(rid)?.trim();
        let lower = target.to_ascii_lowercase();
        let clean = !target.chars().any(|c| c.is_control() || c.is_whitespace());
        if clean && target.chars().count() <= MAX_LINK_CHARS && LINK_SCHEMES.iter().any(|s| lower.starts_with(s)) {
            return Some(target.to_owned());
        }
        self.warn(None, WarningKind::UnsafeLink(clip_to(target, 80)));
        None
    }

    /// Texte d'une part liée : `None` (et `MissingPart`) si elle manque ; une limite dépassée devient fatale.
    pub(crate) fn part_text(&self, part: &str, item: &str) -> Option<String> {
        match self.pkg().text(part) {
            Ok(t) => Some(t),
            Err(e) if e.kind() == ImportErrorKind::TooLarge => {
                self.fail(e);
                None
            }
            Err(_) => {
                self.warn(Some(item), WarningKind::MissingPart(clip(part)));
                None
            }
        }
    }

    /// Environnement de la cascade pour cette part.
    pub(crate) fn env(&self, font_ref: RunProps) -> TextEnv<'_> {
        TextEnv { major: &self.theme.theme.fonts.major, minor: &self.theme.theme.fonts.minor, colors: &self.colors, font_ref }
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
```

> `with_rels` sert aussi à `PartCtx::sub` (tâche 14) : une seule liste de champs.

Ajouter à `test_support.rs` (fusionner les `use` en tête) :

```rust
use std::collections::BTreeMap;

use crate::error::ImportError;
use crate::pptx::context::{PartCtx, PartSpec, Shared};
use crate::pptx::text::empty_levels;
use crate::pptx::theme::ThemeData;
use crate::source::{Asset, Warning};

/// Image PNG factice.
pub(crate) const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-fake";

/// Résultat de `f`, avertissements et assets d'un contexte de test.
pub(crate) type CtxOut<R> = (R, Vec<Warning>, BTreeMap<String, Asset>);

/// Exécute `f` dans le contexte de la slide 1 de `pkg` (avec `tableStyles.xml` s'il est donné) ; rend aussi avertissements et assets, ou l'erreur fatale notée.
pub(crate) fn run_ctx<R>(
    pkg: &Package<'_>,
    part: &str,
    table_styles: Option<&str>,
    f: impl FnOnce(&PartCtx<'_>) -> R,
) -> Result<CtxOut<R>, ImportError> {
    let theme = ThemeData::parse(THEME, "ppt/theme/theme1.xml", LIMITS.max_xml_nodes).unwrap();
    let styles = table_styles.map(|s| crate::pptx::xml::parse(s, "ppt/tableStyles.xml", LIMITS.max_xml_nodes).unwrap());
    let shared = Shared::new(pkg, styles, empty_levels());
    let r = {
        let spec = PartSpec { owner: "s1".into(), slide: Some(1), colors: theme.default_ctx(), theme: &theme, inherit: None };
        let cx = PartCtx::new(&shared, part, spec).unwrap();
        f(&cx)
    };
    let (assets, warnings) = shared.sink.into_inner().finish()?;
    Ok((r, warnings, assets))
}

/// `run_ctx` sans styles de tableau, qui panique sur une erreur fatale.
pub(crate) fn with_ctx<R>(pkg: &Package<'_>, part: &str, f: impl FnOnce(&PartCtx<'_>) -> R) -> CtxOut<R> {
    run_ctx(pkg, part, None, f).unwrap()
}

/// `run_ctx` avec `tableStyles.xml` sur la slide 1 (tâche 12).
pub(crate) fn with_table_styles<R>(pkg: &Package<'_>, styles: &str, f: impl FnOnce(&PartCtx<'_>) -> R) -> (R, Vec<Warning>) {
    let (r, w, _) = run_ctx(pkg, "ppt/slides/slide1.xml", Some(styles), f).unwrap();
    (r, w)
}
```

- [ ] **Step 5 : lancer, constater le succès** — PASS.

- [ ] **Step 6 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : placeholders et contexte de lecture partagé (t-692)
oku cli task done --project slider --id t-713
```

---

### Task 9: Arbre de formes (profondeur bornée, branches choisies avant lecture)

**Files:**
- Create: `src/pptx/shapes.rs`, `src/pptx/shapes_tests.rs`, `src/pptx/frame.rs`
- Modify: `src/pptx/mod.rs` (`pub(crate) mod shapes; pub(crate) mod frame;`), `src/pptx/test_support.rs` (paquet et arbre de slide de test)

**Interfaces:**
- Consumes : `PartCtx`, `PhKey`, `Inherited` (8) ; `FillSpec`, `LineSpec`, `parse_fill`, `approximation`, `parse_line` (4) ; `parse_geometry` (4) ; `parse_txbody`, `font_ref`, `RunProps`, `empty_levels` (6) ; `resolve_textbox`, `TextChain` (7) ; `color_spec_in` (3).
- Produces :

```rust
// shapes.rs
pub(crate) struct PhDecl { pub key: PhKey, pub lst: LevelProps, pub body: BodyProps, pub fill: Option<FillSpec>, pub line: LineSpec, pub geom: Option<Geometry> }
pub(crate) struct ParsedItem { pub item: Item, pub ph: Option<PhDecl> }
#[derive(Clone)] pub(crate) struct GroupCtx { pub sx, pub sy, pub tx, pub ty: f64, pub fill: Option<FillSpec>, pub depth: u8, pub in_drawing: bool }
impl GroupCtx { pub(crate) const ROOT: GroupCtx; pub(crate) fn frame(&self, xfrm: Node) -> Frame; pub(crate) fn nested(&self, xfrm: Node, fill: Option<FillSpec>) -> GroupCtx; }
pub(crate) const MAX_NESTING: u8 = 32;
pub(crate) fn parse_tree(tree: Node, cx: &PartCtx, g: &GroupCtx) -> Vec<ParsedItem>;
pub(crate) fn identity(n: Node, cx: &PartCtx) -> Option<(String, String, Option<String>)>;  // (id, nom, alt) ; modelId prioritaire
pub(crate) fn blip_asset(blip: Node, cx: &PartCtx) -> Option<String>;
pub(crate) fn opaque(reason: OpaqueReason, fallback: Option<String>) -> ItemKind;
// frame.rs
pub(crate) fn parse_graphic_frame(n: Node, cx: &PartCtx, g: &GroupCtx) -> Option<ParsedItem>;
// test_support.rs
pub(crate) const SLIDE_RELS: &str; pub(crate) fn slide_pkg(extra: &[(&str, &[u8])]) -> TestPkg;
pub(crate) fn tree(inner: &str) -> (Vec<ParsedItem>, Vec<Warning>, BTreeMap<String, Asset>); pub(crate) fn tree_in(t: &TestPkg, inner: &str) -> (…);
```

Règles :
- Enfants de `tree`, dans l'ordre : `sp` ; `cxnSp` (sans texte) ; `pic` (ou `Opaque{Media}` si `nvPr` contient `videoFile`, `audioFile`, `quickTimeFile` ou un descendant `media`) ; `grpSp` ; `graphicFrame` ; `AlternateContent` (décision 11) ; le reste est ignoré. `z` = rang dans la sortie.
- Profondeur : `nested()`/`deeper()` incrémentent `depth` ; `grpSp` ou `AlternateContent` à `depth >= MAX_NESTING` → `Opaque{Unknown("nesting")}` + `NestingTooDeep` (le second ne produit qu'un avertissement).
- Identité : `cNvPr hidden="1"` → ignoré ; `modelId` de l'élément (SmartArt) sinon `cNvPr@id` ; `descr` → `alt`.
- Cadre : `xfrm` via `GroupCtx::frame` (bornes de la décision 10, rotation ramenée à `[0, 360)`) ; sinon le cadre hérité ; sinon `(0,0,0,0)`.
- `sp` : géométrie `spPr` > héritée > `rect` ; remplissage `spPr` (`grpFill` → celui du groupe) > hérité > `style/fillRef` > rien, **résolu avec `cx.colors`** ; trait `spPr/ln` par-dessus l'hérité par-dessus `style/lnRef`, résolu de même ; `fontRef` → `TextEnv.font_ref`. `Text` si rien n'est peint, sans trait, géométrie `rect` : gardé s'il a du texte non blanc **ou** s'il est un placeholder ; sinon omis. Sinon `Shape` (texte gardé s'il est non blanc). `txXfrm` présent → `Approximated("txXfrm")`.
- `spPr` : `effectLst` non vide, `scene3d`, `sp3d` → `EffectIgnored` ; `fill::approximation` → `Approximated`. Groupe tourné → `EffectIgnored("group rotation")`, en miroir → `EffectIgnored("group flip")`.
- `pic` : `blip` (préférer son `svgBlip`) → asset ; `srcRect` → `Crop` (chaque bord dans `[-1, 1]` ; si `l + r >= 1` ou `t + b >= 1`, la paire est remise à 0).

- [ ] **Step 1 : support de test**

Ajouter à `test_support.rs` :

```rust
/// Relations de `ppt/slides/slide1.xml` des tests d'arbre de formes (les tâches 13 et 14 y ajoutent leurs lignes).
pub(crate) const SLIDE_RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/missing.png"/>
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.svg"/>
</Relationships>"#;

/// Paquet d'une slide de test : `SLIDE_RELS` et ses parts, plus `extra`.
pub(crate) fn slide_pkg(extra: &[(&str, &[u8])]) -> TestPkg {
    let mut parts: Vec<(&str, &[u8])> = vec![
        ("ppt/slides/_rels/slide1.xml.rels", SLIDE_RELS.as_bytes()),
        ("ppt/media/image1.png", PNG),
        ("ppt/media/image1.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
    ];
    parts.extend_from_slice(extra);
    TestPkg::with(&parts)
}

/// Lit `inner` comme un `spTree` de la slide 1 de `slide_pkg(&[])`.
pub(crate) fn tree(inner: &str) -> (Vec<crate::pptx::shapes::ParsedItem>, Vec<Warning>, BTreeMap<String, Asset>) {
    tree_in(&slide_pkg(&[]), inner)
}

/// Comme `tree`, dans un paquet donné.
pub(crate) fn tree_in(t: &TestPkg, inner: &str) -> (Vec<crate::pptx::shapes::ParsedItem>, Vec<Warning>, BTreeMap<String, Asset>) {
    let xml = format!(
        r#"<p:spTree xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="urn:mc" xmlns:m="urn:m" xmlns:asvg="urn:asvg"><p:nvGrpSpPr/><p:grpSpPr/>{inner}</p:spTree>"#
    );
    let doc = crate::pptx::xml::parse(&xml, "t", LIMITS.max_xml_nodes).unwrap();
    let pkg = t.pkg();
    with_ctx(&pkg, "ppt/slides/slide1.xml", |cx| crate::pptx::shapes::parse_tree(doc.root_element(), cx, &crate::pptx::shapes::GroupCtx::ROOT))
}
```

- [ ] **Step 2 : tests qui échouent**

`src/pptx/shapes_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::pptx::test_support::tree;
use crate::source::{Fill, ItemKind, OpaqueReason, Origin, WarningKind};

const TEXTBOX: &str = r#"<p:sp><p:nvSpPr><p:cNvPr id="4" name="ZoneTexte 3" descr="Légende"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="1270000" y="635000"/><a:ext cx="2540000" cy="508000"/></a:xfrm><a:prstGeom prst="rect"/><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:t>Bonjour</a:t></a:r></a:p></p:txBody></p:sp>"#;
const PT: i64 = 12_700;
const P14: &str = r#"xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main""#;

fn textbox_at(id: u32, off: (i64, i64), ext: (i64, i64)) -> String {
    TEXTBOX.replace(r#"id="4""#, &format!(r#"id="{id}""#)).replace(
        r#"<a:off x="1270000" y="635000"/><a:ext cx="2540000" cy="508000"/>"#,
        &format!(r#"<a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/>"#, off.0, off.1, ext.0, ext.1),
    )
}

fn group(id: u32, off: (i64, i64), ext: (i64, i64), ch_off: (i64, i64), ch_ext: (i64, i64), inner: &str) -> String {
    format!(
        r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="{id}" name="Groupe {id}"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/><a:chOff x="{}" y="{}"/><a:chExt cx="{}" cy="{}"/></a:xfrm></p:grpSpPr>{inner}</p:grpSp>"#,
        off.0, off.1, ext.0, ext.1, ch_off.0, ch_off.1, ch_ext.0, ch_ext.1
    )
}

fn kids(items: &[crate::pptx::shapes::ParsedItem]) -> Vec<crate::source::Item> {
    match &items[0].item.kind {
        ItemKind::Group { items } => items.clone(),
        k => panic!("groupe attendu, trouvé {k:?}"),
    }
}

#[test]
fn textbox_becomes_text_item_with_frame_alt_and_id() {
    let (items, _, _) = tree(TEXTBOX);
    let it = &items[0].item;
    assert_that!((it.id.as_str(), it.name.as_str(), it.alt.as_deref())).is_equal_to(("s1/4", "ZoneTexte 3", Some("Légende")));
    assert_that!((it.frame.x, it.frame.y, it.frame.w, it.frame.h)).is_equal_to((100.0, 50.0, 200.0, 40.0));
    let ItemKind::Text(tb) = &it.kind else { panic!("texte attendu") };
    assert_that!(tb.paragraphs[0].runs[0].text.as_str()).is_equal_to("Bonjour");
}

#[test]
fn rotation_is_normalized_and_flips_are_kept() {
    let (items, _, _) = tree(&TEXTBOX.replace("<a:xfrm>", r#"<a:xfrm rot="-5400000" flipH="1">"#));
    let f = items[0].item.frame;
    assert_that!((f.rot, f.flip_h, f.flip_v)).is_equal_to((270.0, true, false));
}

#[test]
fn styled_rectangle_takes_fill_and_line_from_style_refs() {
    let (items, _, _) = tree(r#"<p:sp><p:nvSpPr><p:cNvPr id="5" name="Rectangle 4"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="127000" cy="127000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:style><a:lnRef idx="2"><a:schemeClr val="accent1"><a:shade val="50000"/></a:schemeClr></a:lnRef><a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef><a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef><a:fontRef idx="minor"><a:schemeClr val="lt1"/></a:fontRef></p:style></p:sp>"#);
    let ItemKind::Shape(s) = &items[0].item.kind else { panic!("forme attendue") };
    assert_that!(matches!(&s.fill, Fill::Solid { color } if color.origin == Origin::scheme("accent1"))).is_true();
    let stroke = s.stroke.clone().unwrap();
    assert_that!((stroke.width, stroke.color.value.to_hex())).is_equal_to((1.0, "#2f528f".to_owned()));
}

#[test]
fn own_fill_beats_the_style_fill() {
    let (items, _, _) = tree(r#"<p:sp><p:nvSpPr><p:cNvPr id="5" name="R"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></p:spPr><p:style><a:lnRef idx="0"><a:schemeClr val="accent1"/></a:lnRef><a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef><a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef><a:fontRef idx="minor"/></p:style></p:sp>"#);
    let ItemKind::Shape(s) = &items[0].item.kind else { panic!("forme attendue") };
    assert_that!(matches!(&s.fill, Fill::Solid { color } if color.value.to_hex() == "#ff0000")).is_true();
}

#[test]
fn empty_non_placeholder_rect_and_blank_text_are_dropped() {
    let (items, _, _) = tree(r#"<p:sp><p:nvSpPr><p:cNvPr id="6" name="Vide"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:t>   </a:t></a:r></a:p></p:txBody></p:sp>"#);
    assert_that!(items).is_empty();
}

#[test]
fn empty_placeholder_is_kept_as_text() {
    let (items, _, _) = tree(r#"<p:sp><p:nvSpPr><p:cNvPr id="7" name="Titre"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp>"#);
    assert_that!((matches!(items[0].item.kind, ItemKind::Text(_)), items[0].ph.as_ref().map(|d| d.key.ty.as_str()))).is_equal_to((true, Some("title")));
}

#[test]
fn hidden_shape_is_skipped() {
    let (items, _, _) = tree(&TEXTBOX.replace(r#"name="ZoneTexte 3""#, r#"name="ZoneTexte 3" hidden="1""#));
    assert_that!(items).is_empty();
}

#[test]
fn connector_is_a_shape_with_its_line() {
    let (items, _, _) = tree(r#"<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="8" name="Connecteur"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:prstGeom prst="line"/><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln></p:spPr></p:cxnSp>"#);
    let ItemKind::Shape(s) = &items[0].item.kind else { panic!("forme attendue") };
    assert_that!(s.stroke.as_ref().map(|st| st.width)).is_equal_to(Some(2.0));
}

#[test]
fn z_follows_document_order() {
    let (items, _, _) = tree(&format!("{}{}", textbox_at(4, (0, 0), (PT, PT)), textbox_at(5, (0, 0), (PT, PT))));
    assert_that!(items.iter().map(|p| p.item.z).collect::<Vec<_>>()).is_equal_to(vec![0, 1]);
}

#[test]
fn picture_reads_asset_and_bounded_crop() {
    let (items, _, assets) = tree(r#"<p:pic><p:nvPicPr><p:cNvPr id="7" name="Image 6"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/><a:srcRect l="10000" r="20000" t="60000" b="60000"/><a:stretch/></p:blipFill><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm></p:spPr></p:pic>"#);
    let ItemKind::Picture(p) = &items[0].item.kind else { panic!("image attendue") };
    // l + r = 0,3 : gardés ; t + b = 1,2 : impossible, remis à 0.
    assert_that!((p.crop.l, p.crop.r, p.crop.t, p.crop.b)).is_equal_to((0.1, 0.2, 0.0, 0.0));
    assert_that!(assets.get(&p.asset).map(|a| a.mime.as_str())).is_equal_to(Some("image/png"));
}

#[test]
fn picture_prefers_its_svg_version() {
    let (items, _, assets) = tree(r#"<p:pic><p:nvPicPr><p:cNvPr id="7" name="Icône"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"><a:extLst><a:ext uri="{96DAC541-7B7A-43D3-8B79-37D633B846F1}"><asvg:svgBlip r:embed="rId4"/></a:ext></a:extLst></a:blip></p:blipFill><p:spPr/></p:pic>"#);
    let ItemKind::Picture(p) = &items[0].item.kind else { panic!("image attendue") };
    assert_that!(assets.get(&p.asset).map(|a| a.mime.as_str())).is_equal_to(Some("image/svg+xml"));
}

#[test]
fn video_picture_is_opaque_media_with_its_poster() {
    let (items, _, assets) = tree(r#"<p:pic><p:nvPicPr><p:cNvPr id="9" name="Vidéo"/><p:cNvPicPr/><p:nvPr><a:videoFile r:link="rIdV"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/></p:blipFill><p:spPr/></p:pic>"#);
    let ItemKind::Opaque(o) = &items[0].item.kind else { panic!("opaque attendu") };
    assert_that!((o.reason.clone(), o.fallback.as_ref().is_some_and(|a| assets.contains_key(a)))).is_equal_to((OpaqueReason::Media, true));
}

#[test]
fn picture_with_missing_part_warns_and_is_dropped() {
    let (items, w, _) = tree(r#"<p:pic><p:nvPicPr><p:cNvPr id="8" name="X"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId3"/></p:blipFill><p:spPr/></p:pic>"#);
    assert_that!(items).is_empty();
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::MissingPart(_)))).is_true();
}

#[test]
fn group_with_zero_child_offset_scales_children() {
    // Groupe en (100, 100) pt, 200×100 pt ; espace enfant 100×50 pt depuis 0 : échelle 2.
    let child = textbox_at(4, (50 * PT, 25 * PT), (50 * PT, 25 * PT));
    let (items, _, _) = tree(&group(10, (100 * PT, 100 * PT), (200 * PT, 100 * PT), (0, 0), (100 * PT, 50 * PT), &child));
    let f = kids(&items)[0].frame;
    assert_that!((f.x, f.y, f.w, f.h)).is_equal_to((200.0, 150.0, 100.0, 50.0));
}

#[test]
fn group_with_offset_child_space_and_scale_maps_children() {
    // off (100,100), ext 200×100, chOff (50,50), chExt 100×50 : échelle 2 ; enfant en (60,60) → 100 + 2×(60−50) = 120.
    let child = textbox_at(4, (60 * PT, 60 * PT), (10 * PT, 10 * PT));
    let (items, _, _) = tree(&group(10, (100 * PT, 100 * PT), (200 * PT, 100 * PT), (50 * PT, 50 * PT), (100 * PT, 50 * PT), &child));
    let f = kids(&items)[0].frame;
    assert_that!((f.x, f.y, f.w, f.h)).is_equal_to((120.0, 120.0, 20.0, 20.0));
}

#[test]
fn nested_groups_compose_their_transforms() {
    // Extérieur : échelle 2 depuis (0,0). Intérieur, posé en (10,10) pt dans l'extérieur, chOff (5,5), chExt 20×20 pour 40×40 : échelle 2.
    let leaf = textbox_at(4, (10 * PT, 10 * PT), (10 * PT, 10 * PT));
    let inner = group(11, (10 * PT, 10 * PT), (40 * PT, 40 * PT), (5 * PT, 5 * PT), (20 * PT, 20 * PT), &leaf);
    let outer = group(10, (0, 0), (200 * PT, 200 * PT), (0, 0), (100 * PT, 100 * PT), &inner);
    let (items, _, _) = tree(&outer);
    let ItemKind::Group { items: leaves } = &kids(&items)[0].kind else { panic!("groupe attendu") };
    // feuille : dans l'extérieur 10 + 2×(10 − 5) = 20, puis ×2 → 40 ; taille 10 × 2 × 2 = 40.
    let f = leaves[0].frame;
    assert_that!((f.x, f.y, f.w, f.h)).is_equal_to((40.0, 40.0, 40.0, 40.0));
}

#[test]
fn absurd_group_scale_stays_finite_and_bounded() {
    let child = textbox_at(4, (1, 1), (1, 1));
    let (items, _, _) = tree(&group(10, (0, 0), (9_000_000_000_000, 9_000_000_000_000), (0, 0), (1, 1), &child));
    let f = kids(&items)[0].frame;
    assert_that!(f.x.is_finite() && f.w.is_finite() && f.w <= 1e6 && f.x <= 1e6).is_true();
}

#[test]
fn deep_group_nesting_is_cut_with_a_warning() {
    // 100 groupes : au-delà de MAX_NESTING (32), en deçà de la profondeur XML admise (256).
    let open = group(100, (0, 0), (PT, PT), (0, 0), (PT, PT), "").replace("</p:grpSp>", "");
    let xml = format!("{}{}{}", open.repeat(100), textbox_at(4, (0, 0), (PT, PT)), "</p:grpSp>".repeat(100));
    let (items, w, _) = tree(&xml);
    assert_that!(items.len()).is_equal_to(1);
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::NestingTooDeep(_)))).is_true();
}

#[test]
fn deep_alternate_content_nesting_is_cut_with_a_warning() {
    let open = format!(r#"<mc:AlternateContent {P14}><mc:Choice Requires="p14">"#).repeat(100);
    let close = "</mc:Choice></mc:AlternateContent>".repeat(100);
    let (_, w, _) = tree(&format!("{open}{}{close}", textbox_at(4, (0, 0), (PT, PT))));
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::NestingTooDeep(_)))).is_true();
}

#[test]
fn group_rotation_and_flip_are_reported() {
    let g = group(10, (0, 0), (PT, PT), (0, 0), (PT, PT), &textbox_at(4, (0, 0), (PT, PT))).replace("<a:xfrm>", r#"<a:xfrm rot="60000" flipV="1">"#);
    let (_, w, _) = tree(&g);
    let k: Vec<_> = w.iter().map(|w| w.kind.clone()).collect();
    assert_that!(k.contains(&WarningKind::EffectIgnored("group rotation".into())) && k.contains(&WarningKind::EffectIgnored("group flip".into()))).is_true();
}

#[test]
fn group_fill_is_used_by_children_with_grp_fill() {
    let child = r#"<p:sp><p:nvSpPr><p:cNvPr id="4" name="R"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="ellipse"/><a:grpFill/></p:spPr></p:sp>"#;
    let g = group(10, (0, 0), (PT, PT), (0, 0), (PT, PT), child).replace("</a:xfrm></p:grpSpPr>", r#"</a:xfrm><a:solidFill><a:srgbClr val="00FF00"/></a:solidFill></p:grpSpPr>"#);
    let (items, _, _) = tree(&g);
    let ItemKind::Shape(s) = &kids(&items)[0].kind else { panic!("forme attendue") };
    assert_that!(matches!(&s.fill, Fill::Solid { color } if color.value.to_hex() == "#00ff00")).is_true();
}

#[test]
fn effects_and_approximations_are_reported() {
    let (_, w, _) = tree(&TEXTBOX.replace("<a:noFill/>", r#"<a:pattFill prst="pct5"><a:fgClr><a:srgbClr val="000000"/></a:fgClr></a:pattFill><a:effectLst><a:outerShdw blurRad="1"/></a:effectLst>"#));
    let k: Vec<_> = w.iter().map(|w| w.kind.clone()).collect();
    assert_that!(k.contains(&WarningKind::EffectIgnored("effectLst".into())) && k.contains(&WarningKind::Approximated("pattFill".into()))).is_true();
}

#[test]
fn math_alternate_content_becomes_opaque_with_its_fallback_picture() {
    let (items, _, assets) = tree(r#"<mc:AlternateContent><mc:Choice Requires="a14" xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"><p:sp><p:nvSpPr><p:cNvPr id="11" name="Eq"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><m:oMathPara/></a:p></p:txBody></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="11" name="Eq"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="12700" y="25400"/><a:ext cx="12700" cy="12700"/></a:xfrm><a:blipFill><a:blip r:embed="rId2"/></a:blipFill></p:spPr></p:sp></mc:Fallback></mc:AlternateContent>"#);
    let it = &items[0].item;
    let ItemKind::Opaque(o) = &it.kind else { panic!("opaque attendu") };
    assert_that!((o.reason.clone(), it.id.as_str(), it.frame.x, it.frame.y)).is_equal_to((OpaqueReason::Math, "s1/11", 1.0, 2.0));
    assert_that!(o.fallback.as_ref().is_some_and(|a| assets.get(a).is_some_and(|x| x.mime == "image/png"))).is_true();
}

#[test]
fn alternate_content_with_understood_choice_reads_only_the_choice() {
    let (items, _, _) = tree(&format!(r#"<mc:AlternateContent {P14}><mc:Choice Requires="p14">{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent>"#, textbox_at(4, (0, 0), (PT, PT)), textbox_at(5, (0, 0), (PT, PT))));
    assert_that!(items.iter().map(|p| p.item.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["s1/4"]);
}

#[test]
fn alternate_content_with_unknown_requirement_reads_only_the_fallback() {
    let choice = r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="12" name="Chartex"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/drawing/2014/chartex"><cx:chart xmlns:cx="urn:cx" r:id="rId3"/></a:graphicData></a:graphic></p:graphicFrame>"#;
    let fallback = r#"<p:pic><p:nvPicPr><p:cNvPr id="12" name="Chartex"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/></p:blipFill><p:spPr/></p:pic>"#;
    let (items, w, assets) = tree(&format!(r#"<mc:AlternateContent xmlns:cx1="http://schemas.microsoft.com/office/drawing/2015/9/8/chartex"><mc:Choice Requires="cx1">{choice}</mc:Choice><mc:Fallback>{fallback}</mc:Fallback></mc:AlternateContent>"#));
    // Le Choice n'est pas lu : l'id du repli n'a pas de suffixe, aucun avertissement ni asset du Choice.
    assert_that!((matches!(items[0].item.kind, ItemKind::Picture(_)), items[0].item.id.as_str(), w.len(), assets.len())).is_equal_to((true, "s1/12", 0, 1));
}

#[test]
fn unknown_graphic_frame_is_opaque_with_its_uri() {
    let (items, _, _) = tree(r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="12" name="Objet"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="urn:custom"/></a:graphic></p:graphicFrame>"#);
    let ItemKind::Opaque(o) = &items[0].item.kind else { panic!("opaque attendu") };
    assert_that!(o.reason.clone()).is_equal_to(OpaqueReason::Unknown("urn:custom".into()));
}
```

- [ ] **Step 3 : lancer, constater l'échec** — FAIL.

- [ ] **Step 4 : implémentation**

`src/pptx/shapes.rs` :

```rust
//! Arbre de formes (`spTree`, `grpSp`, `dsp:spTree`) → items.

use roxmltree::Node;

use crate::pptx::cascade::{TextChain, resolve_textbox};
use crate::pptx::color::color_spec_in;
use crate::pptx::context::PartCtx;
use crate::pptx::fill::{FillSpec, LineSpec, approximation, parse_fill, parse_line};
use crate::pptx::frame::parse_graphic_frame;
use crate::pptx::geometry::parse_geometry;
use crate::pptx::placeholder::PhKey;
use crate::pptx::text::{BodyProps, LevelProps, RunProps, empty_levels, font_ref, parse_txbody};
use crate::pptx::units::{angle, emu, pct};
use crate::pptx::xml::{attr_bool, attr_i64, attr_r, attr_str, attr_u32, child, descendant, elements};
use crate::source::{Crop, Fill, Frame, Geometry, Item, ItemKind, Opaque, OpaqueReason, Picture, Shape, TextBox, WarningKind};

/// Profondeur d'imbrication (groupes, `AlternateContent`) au-delà de laquelle on coupe.
pub(crate) const MAX_NESTING: u8 = 32;
const MAX_COORD: f64 = 1e6;
const MIN_SCALE: f64 = 1e-6;
const MAX_SCALE: f64 = 1e6;
/// Espaces de noms d'extension dont on sait lire le `mc:Choice` (a14, p14, p15, a16, asvg).
const KNOWN_NS: [&str; 5] = [
    "http://schemas.microsoft.com/office/drawing/2010/main",
    "http://schemas.microsoft.com/office/powerpoint/2010/main",
    "http://schemas.microsoft.com/office/powerpoint/2012/main",
    "http://schemas.microsoft.com/office/drawing/2014/main",
    "http://schemas.microsoft.com/office/drawing/2016/SVG/main",
];

/// Déclaration de placeholder d'un item : clé et propriétés propres (non résolues).
#[derive(Debug, Clone)]
pub(crate) struct PhDecl {
    pub key: PhKey,
    pub lst: LevelProps,
    pub body: BodyProps,
    pub fill: Option<FillSpec>,
    pub line: LineSpec,
    pub geom: Option<Geometry>,
}

/// Item lu et, s'il en est un, sa déclaration de placeholder.
#[derive(Debug, Clone)]
pub(crate) struct ParsedItem {
    pub item: Item,
    pub ph: Option<PhDecl>,
}

/// Transformation de groupe (pt), remplissage de groupe, profondeur, « dans un dessin de SmartArt ».
#[derive(Debug, Clone)]
pub(crate) struct GroupCtx {
    pub sx: f64,
    pub sy: f64,
    pub tx: f64,
    pub ty: f64,
    pub fill: Option<FillSpec>,
    pub depth: u8,
    pub in_drawing: bool,
}

fn bound(v: f64) -> f64 {
    if v.is_finite() { v.clamp(-MAX_COORD, MAX_COORD) } else { 0.0 }
}

fn pt_pair(n: Option<Node<'_, '_>>, a: &str, b: &str) -> (f64, f64) {
    n.map_or((0.0, 0.0), |n| (emu(attr_i64(n, a).unwrap_or(0)), emu(attr_i64(n, b).unwrap_or(0))))
}

impl GroupCtx {
    /// Pas de groupe.
    pub(crate) const ROOT: GroupCtx = GroupCtx { sx: 1.0, sy: 1.0, tx: 0.0, ty: 0.0, fill: None, depth: 0, in_drawing: false };

    /// Cadre de `xfrm` dans l'espace de la page.
    pub(crate) fn frame(&self, xfrm: Node<'_, '_>) -> Frame {
        let (x, y) = pt_pair(child(xfrm, "off"), "x", "y");
        let (w, h) = pt_pair(child(xfrm, "ext"), "cx", "cy");
        Frame {
            x: bound(self.tx + x * self.sx),
            y: bound(self.ty + y * self.sy),
            w: bound(w * self.sx),
            h: bound(h * self.sy),
            rot: attr_i64(xfrm, "rot").map_or(0.0, angle).rem_euclid(360.0),
            flip_h: attr_bool(xfrm, "flipH").unwrap_or(false),
            flip_v: attr_bool(xfrm, "flipV").unwrap_or(false),
        }
    }

    /// Contexte des enfants d'un groupe de `xfrm` donné.
    pub(crate) fn nested(&self, xfrm: Node<'_, '_>, fill: Option<FillSpec>) -> GroupCtx {
        let (ox, oy) = pt_pair(child(xfrm, "off"), "x", "y");
        let (ew, eh) = pt_pair(child(xfrm, "ext"), "cx", "cy");
        let (cx, cy) = pt_pair(child(xfrm, "chOff"), "x", "y");
        let (cw, ch) = pt_pair(child(xfrm, "chExt"), "cx", "cy");
        let scale = |e: f64, c: f64| if c > 0.0 && e > 0.0 { (e / c).clamp(MIN_SCALE, MAX_SCALE) } else { 1.0 };
        let (sx, sy) = (scale(ew, cw), scale(eh, ch));
        GroupCtx {
            sx: (self.sx * sx).clamp(MIN_SCALE, MAX_SCALE),
            sy: (self.sy * sy).clamp(MIN_SCALE, MAX_SCALE),
            tx: bound(self.tx + self.sx * (ox - cx * sx)),
            ty: bound(self.ty + self.sy * (oy - cy * sy)),
            fill: fill.or_else(|| self.fill.clone()),
            depth: self.depth.saturating_add(1),
            in_drawing: self.in_drawing,
        }
    }

    fn deeper(&self) -> GroupCtx {
        GroupCtx { depth: self.depth.saturating_add(1), ..self.clone() }
    }
}

fn nv<'a, 'i>(n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    elements(n).find(|c| c.tag_name().name().starts_with("nv"))
}

fn ph_key(n: Node<'_, '_>) -> Option<PhKey> {
    nv(n).and_then(|v| child(v, "nvPr")).and_then(|p| child(p, "ph")).map(PhKey::from_ph)
}

/// (id, nom, texte alternatif) d'un item, ou `None` s'il est caché ; `modelId` (SmartArt) prime sur `cNvPr@id`.
pub(crate) fn identity(n: Node<'_, '_>, cx: &PartCtx<'_>) -> Option<(String, String, Option<String>)> {
    let c = nv(n).and_then(|v| child(v, "cNvPr"))?;
    if attr_bool(c, "hidden") == Some(true) {
        return None;
    }
    let id = match attr_str(n, "modelId") {
        Some(m) => cx.item_id_raw(m),
        None => cx.item_id(attr_str(c, "id").unwrap_or("0")),
    };
    let alt = attr_str(c, "descr").filter(|d| !d.is_empty()).map(|d| crate::collect::clip_to(d, 2048));
    Some((id, crate::collect::clip(attr_str(c, "name").unwrap_or_default()), alt))
}

/// Item opaque.
pub(crate) fn opaque(reason: OpaqueReason, fallback: Option<String>) -> ItemKind {
    ItemKind::Opaque(Opaque { reason, fallback })
}

fn has_text(tb: &TextBox) -> bool {
    tb.paragraphs.iter().any(|p| p.runs.iter().any(|r| !r.text.trim().is_empty()))
}

fn report_sp_pr(sp_pr: Node<'_, '_>, id: &str, cx: &PartCtx<'_>) {
    if child(sp_pr, "effectLst").is_some_and(|e| elements(e).next().is_some()) {
        cx.warn(Some(id), WarningKind::EffectIgnored("effectLst".into()));
    }
    for e in ["scene3d", "sp3d"] {
        if child(sp_pr, e).is_some() {
            cx.warn(Some(id), WarningKind::EffectIgnored(e.into()));
        }
    }
    if let Some(a) = approximation(sp_pr) {
        cx.warn(Some(id), WarningKind::Approximated(a.into()));
    }
}

/// Références de style (`p:style`) : remplissage (si nécessaire), trait, police et couleur du texte.
fn style_refs(n: Node<'_, '_>, cx: &PartCtx<'_>, need_fill: bool) -> (Option<FillSpec>, LineSpec, RunProps) {
    let Some(style) = child(n, "style") else { return (None, LineSpec::default(), RunProps::default()) };
    let idx = |r: Node<'_, '_>| attr_u32(r, "idx").unwrap_or(0);
    let fill = if need_fill {
        child(style, "fillRef").and_then(|r| cx.theme.fill_style(idx(r), color_spec_in(r).as_ref(), &mut |id| cx.asset(id)))
    } else {
        None
    };
    let line = child(style, "lnRef").map(|r| cx.theme.line_style(idx(r), color_spec_in(r).as_ref())).unwrap_or_default();
    let font = child(style, "fontRef").map(|r| RunProps { font: font_ref(r), color: color_spec_in(r), ..RunProps::default() }).unwrap_or_default();
    (fill, line, font)
}

fn parse_sp(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx, connector: bool) -> Option<ParsedItem> {
    let (id, name, alt) = identity(n, cx)?;
    let ph = ph_key(n);
    let inh = cx.lookup(ph.as_ref());
    let sp_pr = child(n, "spPr");
    let frame = sp_pr.and_then(|s| child(s, "xfrm")).map(|x| g.frame(x)).or(inh.frame).unwrap_or(Frame::new(0.0, 0.0, 0.0, 0.0));
    if let Some(s) = sp_pr {
        report_sp_pr(s, &id, cx);
    }
    if child(n, "txXfrm").is_some() {
        cx.warn(Some(&id), WarningKind::Approximated("txXfrm".into()));
    }
    let own_fill = match sp_pr {
        Some(s) if child(s, "grpFill").is_some() => g.fill.clone(),
        Some(s) => parse_fill(s, &mut |r| cx.asset(r)),
        None => None,
    };
    let own_line = sp_pr.and_then(|s| child(s, "ln")).map(parse_line).unwrap_or_default();
    let own_geom = sp_pr.and_then(parse_geometry);
    let (fill_ref, line_ref, font) = style_refs(n, cx, own_fill.is_none() && inh.fill.is_none());
    let fill: Fill = own_fill.clone().or_else(|| inh.fill.clone()).or(fill_ref).map_or(Fill::None, |f| f.resolve(&cx.colors));
    let stroke = own_line.clone().over(&inh.line.clone().over(&line_ref)).finish(&cx.colors);
    let geom = own_geom.clone().or_else(|| inh.geom.clone()).unwrap_or(Geometry::Preset { name: "rect".into(), adj: Default::default() });
    let raw = if connector { None } else { child(n, "txBody").map(|tx| parse_txbody(tx, &mut |r| cx.link(r))) };
    let tb = raw.as_ref().map(|raw| {
        let chain = TextChain { levels: inh.levels.iter().collect(), bodies: inh.bodies.iter().collect() };
        resolve_textbox(raw, &chain, &cx.env(font))
    });
    let texty = tb.as_ref().is_some_and(has_text);
    let is_rect = matches!(&geom, Geometry::Preset { name, .. } if name == "rect");
    let kind = if !connector && fill.is_none() && stroke.is_none() && is_rect {
        if !texty && ph.is_none() {
            return None;
        }
        ItemKind::Text(tb.unwrap_or_default())
    } else {
        ItemKind::Shape(Box::new(Shape { geom, fill, stroke, text: tb.filter(has_text) }))
    };
    let decl = ph.clone().map(|key| PhDecl {
        key,
        lst: raw.as_ref().map_or_else(empty_levels, |r| r.lst.clone()),
        body: raw.map(|r| r.body).unwrap_or_default(),
        fill: own_fill,
        line: own_line,
        geom: own_geom,
    });
    let item = Item { id, name, alt, frame, z: 0, role: ph.map(|k| k.role()), placeholder: inh.link, kind };
    Some(ParsedItem { item, ph: decl })
}

/// Asset d'un `blip` (préfère sa version SVG).
pub(crate) fn blip_asset(blip: Node<'_, '_>, cx: &PartCtx<'_>) -> Option<String> {
    let svg = descendant(blip, "svgBlip").and_then(|s| attr_r(s, "embed"));
    svg.or_else(|| attr_r(blip, "embed")).and_then(|rid| cx.asset(rid))
}

/// Paire de bords de rognage : chacun dans `[-1, 1]`, remise à 0 si leur somme laisse une image vide.
fn crop_pair(a: f64, b: f64) -> (f64, f64) {
    let (a, b) = (a.clamp(-1.0, 1.0), b.clamp(-1.0, 1.0));
    if a + b >= 1.0 { (0.0, 0.0) } else { (a, b) }
}

fn parse_pic(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    let (id, name, alt) = identity(n, cx)?;
    let ph = ph_key(n);
    let inh = cx.lookup(ph.as_ref());
    let sp_pr = child(n, "spPr");
    let frame = sp_pr.and_then(|s| child(s, "xfrm")).map(|x| g.frame(x)).or(inh.frame).unwrap_or(Frame::new(0.0, 0.0, 0.0, 0.0));
    if let Some(s) = sp_pr {
        report_sp_pr(s, &id, cx);
    }
    let blip_fill = child(n, "blipFill");
    let asset = blip_fill.and_then(|b| child(b, "blip")).and_then(|b| blip_asset(b, cx));
    let nv_pr = nv(n).and_then(|v| child(v, "nvPr"));
    let media = nv_pr.is_some_and(|p| ["videoFile", "audioFile", "quickTimeFile"].iter().any(|m| child(p, m).is_some()) || descendant(p, "media").is_some());
    let kind = if media {
        opaque(OpaqueReason::Media, asset)
    } else {
        let crop = blip_fill.and_then(|b| child(b, "srcRect")).map_or_else(Crop::default, |r| {
            let side = |a: &str| attr_i64(r, a).map_or(0.0, pct);
            let ((l, rr), (t, b)) = (crop_pair(side("l"), side("r")), crop_pair(side("t"), side("b")));
            Crop { l, t, r: rr, b }
        });
        ItemKind::Picture(Picture { asset: asset?, crop })
    };
    let decl = ph.clone().map(|key| PhDecl { key, lst: empty_levels(), body: BodyProps::default(), fill: None, line: LineSpec::default(), geom: None });
    let item = Item { id, name, alt, frame, z: 0, role: ph.map(|k| k.role()), placeholder: inh.link, kind };
    Some(ParsedItem { item, ph: decl })
}

fn too_deep(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    let (id, name, alt) = identity(n, cx).unwrap_or_else(|| (cx.item_id("x"), String::new(), None));
    cx.warn(Some(&id), WarningKind::NestingTooDeep(id.clone()));
    let frame = child(n, "grpSpPr").and_then(|p| child(p, "xfrm")).map_or(Frame::new(0.0, 0.0, 0.0, 0.0), |x| g.frame(x));
    let kind = opaque(OpaqueReason::Unknown("nesting".into()), None);
    Some(ParsedItem { item: Item { id, name, alt, frame, z: 0, role: None, placeholder: None, kind }, ph: None })
}

fn parse_group(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    if g.depth >= MAX_NESTING {
        return too_deep(n, cx, g);
    }
    let (id, name, alt) = identity(n, cx)?;
    let gp = child(n, "grpSpPr");
    let xfrm = gp.and_then(|p| child(p, "xfrm"));
    let frame = xfrm.map_or(Frame::new(0.0, 0.0, 0.0, 0.0), |x| g.frame(x));
    if frame.rot != 0.0 {
        cx.warn(Some(&id), WarningKind::EffectIgnored("group rotation".into()));
    }
    if frame.flip_h || frame.flip_v {
        cx.warn(Some(&id), WarningKind::EffectIgnored("group flip".into()));
    }
    let fill = gp.and_then(|p| parse_fill(p, &mut |r| cx.asset(r)));
    let inner = xfrm.map_or_else(|| g.deeper(), |x| g.nested(x, fill));
    let items: Vec<Item> = parse_tree(n, cx, &inner).into_iter().map(|p| p.item).collect();
    if items.is_empty() {
        return None;
    }
    Some(ParsedItem { item: Item { id, name, alt, frame, z: 0, role: None, placeholder: None, kind: ItemKind::Group { items } }, ph: None })
}

/// `mc:Choice` lisible : chaque préfixe de `Requires` désigne un espace de noms connu.
fn understood(choice: Node<'_, '_>) -> bool {
    attr_str(choice, "Requires").is_some_and(|req| {
        req.split_whitespace().all(|p| choice.lookup_namespace_uri(Some(p)).is_some_and(|ns| KNOWN_NS.contains(&ns)))
    })
}

fn parse_alternate(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Vec<ParsedItem> {
    if g.depth >= MAX_NESTING {
        cx.warn(None, WarningKind::NestingTooDeep("AlternateContent".into()));
        return Vec::new();
    }
    let inner = g.deeper();
    let choice = child(n, "Choice");
    let fallback = child(n, "Fallback");
    let is_math = choice.is_some_and(|c| c.descendants().take(10_000).any(|d| matches!(d.tag_name().name(), "oMath" | "oMathPara")));
    if is_math {
        let Some(fb) = fallback else { return Vec::new() };
        let asset = descendant(fb, "blip").and_then(|b| blip_asset(b, cx));
        let mut items = parse_tree(fb, cx, &inner);
        for p in &mut items {
            p.item.kind = opaque(OpaqueReason::Math, asset.clone());
        }
        return items;
    }
    // La branche est choisie avant d'être lue : l'autre ne consomme ni id, ni asset, ni erreur.
    let branch = match (choice, fallback) {
        (Some(c), _) if understood(c) => Some(c),
        (_, Some(f)) => Some(f),
        (c, None) => c,
    };
    branch.map(|b| parse_tree(b, cx, &inner)).unwrap_or_default()
}

/// Items d'un arbre de formes, dans l'ordre de dessin.
pub(crate) fn parse_tree(tree: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Vec<ParsedItem> {
    let mut out: Vec<ParsedItem> = Vec::new();
    for n in elements(tree) {
        match n.tag_name().name() {
            "sp" => out.extend(parse_sp(n, cx, g, false)),
            "cxnSp" => out.extend(parse_sp(n, cx, g, true)),
            "pic" => out.extend(parse_pic(n, cx, g)),
            "grpSp" => out.extend(parse_group(n, cx, g)),
            "graphicFrame" => out.extend(parse_graphic_frame(n, cx, g)),
            "AlternateContent" => out.extend(parse_alternate(n, cx, g)),
            _ => {}
        }
    }
    for (z, p) in out.iter_mut().enumerate() {
        p.item.z = u32::try_from(z).unwrap_or(u32::MAX);
    }
    out
}

#[cfg(test)]
#[path = "shapes_tests.rs"]
mod shapes_tests;
```

> `rotation_is_normalized…` : `rot="-5400000"` = −90° → `rem_euclid(360)` = 270°. `deep_*` : la récursion s'arrête à 32 niveaux ; 100 niveaux de XML restent sous `MAX_XML_DEPTH` (256), la tâche 2 couvre le reste.

`src/pptx/frame.rs` (état de la tâche 9 ; les tâches 12, 13 et 14 le réécrivent en entier) :

```rust
//! `graphicFrame` : tableau, graphique, SmartArt, objet OLE, sinon opaque.

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::context::PartCtx;
use crate::pptx::shapes::{GroupCtx, ParsedItem, identity, opaque};
use crate::pptx::xml::{attr_str, child, descendant};
use crate::source::{Frame, Item, OpaqueReason};

/// Lit un `graphicFrame`.
pub(crate) fn parse_graphic_frame(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    let (id, name, alt) = identity(n, cx)?;
    let frame = child(n, "xfrm").map_or(Frame::new(0.0, 0.0, 0.0, 0.0), |x| g.frame(x));
    let data = descendant(n, "graphicData")?;
    let kind = opaque(OpaqueReason::Unknown(clip(attr_str(data, "uri").unwrap_or_default())), None);
    Some(ParsedItem { item: Item { id, name, alt, frame, z: 0, role: None, placeholder: None, kind }, ph: None })
}
```

- [ ] **Step 5 : lancer, constater le succès** — PASS.

- [ ] **Step 6 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : arbre de formes — textes, formes, images, groupes, replis, profondeur bornée (t-692)
oku cli task done --project slider --id t-714
```

---

### Task 10: Masters et layouts

**Files:**
- Create: `src/pptx/master.rs`, `src/pptx/master_tests.rs`, `fixtures/import/minimal/ppt/slideMasters/slideMaster1.xml`, `fixtures/import/minimal/ppt/slideLayouts/slideLayout1.xml`
- Modify: `src/pptx/mod.rs` (`pub(crate) mod master;`), `src/pptx/test_support.rs` (`MASTER`, `LAYOUT`, `master_and_layout`)

**Interfaces:**
- Consumes : `Shared`, `PartCtx`, `PartSpec` (8) ; `PhTable`, `PhInfo`, `TextStyles`, `PhInheritance`, `MAX_PLACEHOLDERS` (8) ; `ParsedItem`, `GroupCtx`, `parse_tree` (9) ; `parse_lst_style`, `empty_levels`, `RunProps` (6) ; `resolve_levels`, `TextChain` (7) ; `ThemeData` (5) ; `FillSpec`, `parse_fill` (4) ; `clean_map`, `default_map`, `color_spec_in` (3).
- Produces :

```rust
pub(crate) struct MasterInfo { pub master: Master, pub styles: TextStyles, pub phs: PhTable, pub clr_map: BTreeMap<String, String>, pub bg: Option<FillSpec> }
pub(crate) struct LayoutInfo { pub layout: Layout, pub phs: PhTable, pub clr_map: BTreeMap<String, String>, pub bg: Option<FillSpec> }
pub(crate) fn clr_map_override(root: Node, base: &BTreeMap<String, String>) -> BTreeMap<String, String>;
pub(crate) fn background(c_sld: Node, cx: &PartCtx) -> Option<FillSpec>;   // bgPr, ou bgRef via le thème (non résolu)
pub(crate) fn table_of(items: &[ParsedItem]) -> PhTable;                   // MAX_PLACEHOLDERS au plus
pub(crate) fn placeholders(items: Vec<ParsedItem>, cx: &PartCtx) -> (Vec<Item>, Vec<Placeholder>);
pub(crate) fn parse_master(shared: &Shared<'_>, part: &str, n: usize, theme_index: usize, theme: &ThemeData<'_>) -> Result<MasterInfo, ImportError>;
pub(crate) fn parse_layout(shared: &Shared<'_>, part: &str, n: usize, master: &MasterInfo, theme: &ThemeData<'_>) -> Result<LayoutInfo, ImportError>;
// test_support.rs
pub(crate) fn master_and_layout(layout_xml: &str) -> (MasterInfo, LayoutInfo);
```

Ids : master `m<n>`, layout `l<n>`. Les fonds restent aussi non résolus (`bg`) : la slide les résout avec **ses** couleurs. `Master.background`/`Layout.background` sont la version résolue avec les couleurs de la part qui les déclare.

- [ ] **Step 1 : fixtures master et layout de titre**

`fixtures/import/minimal/ppt/slideMasters/slideMaster1.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title Placeholder 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="838200" y="365125"/><a:ext cx="10515600" cy="1325563"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/><a:p><a:r><a:t>Titre</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Text Placeholder 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="838200" y="1825625"/><a:ext cx="10515600" cy="4351338"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Texte</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="7" name="Bandeau"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="6604000"/><a:ext cx="12192000" cy="254000"/></a:xfrm><a:prstGeom prst="rect"/><a:solidFill><a:schemeClr val="accent1"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>
</p:spTree></p:cSld>
<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/><p:sldLayoutId id="2147483650" r:id="rId2"/></p:sldLayoutIdLst>
<p:txStyles>
<p:titleStyle><a:lvl1pPr algn="l"><a:defRPr sz="4400"><a:solidFill><a:schemeClr val="tx1"/></a:solidFill><a:latin typeface="+mj-lt"/></a:defRPr></a:lvl1pPr></p:titleStyle>
<p:bodyStyle><a:lvl1pPr marL="228600" indent="-228600"><a:buFont typeface="Arial"/><a:buChar char="•"/><a:defRPr sz="2800"><a:solidFill><a:schemeClr val="tx1"/></a:solidFill><a:latin typeface="+mn-lt"/></a:defRPr></a:lvl1pPr><a:lvl2pPr marL="685800" indent="-228600"><a:buChar char="–"/><a:defRPr sz="2400"/></a:lvl2pPr></p:bodyStyle>
<p:otherStyle><a:lvl1pPr><a:defRPr sz="1600"/></a:lvl1pPr></p:otherStyle>
</p:txStyles>
</p:sldMaster>
```

`fixtures/import/minimal/ppt/slideLayouts/slideLayout1.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="title" preserve="1">
<p:cSld name="Diapositive de titre"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="ctrTitle"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="1524000" y="1122363"/><a:ext cx="9144000" cy="2387600"/></a:xfrm></p:spPr><p:txBody><a:bodyPr anchor="b"/><a:lstStyle><a:lvl1pPr algn="ctr"><a:defRPr sz="6000"/></a:lvl1pPr></a:lstStyle><a:p><a:r><a:t>Titre</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Subtitle 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="subTitle" idx="1"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="1524000" y="3602038"/><a:ext cx="9144000" cy="1655762"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr marL="0" indent="0" algn="ctr"><a:buNone/><a:defRPr sz="2400"/></a:lvl1pPr></a:lstStyle><a:p><a:r><a:t>Sous-titre</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>
```

- [ ] **Step 2 : tests qui échouent**

`src/pptx/master_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::pptx::test_support::{LAYOUT, master_and_layout};
use crate::source::{Align, Anchor, Bullet, Fill, ItemKind, Origin, Role};

#[test]
fn master_splits_placeholders_from_decor() {
    let (m, _) = master_and_layout(LAYOUT);
    assert_that!((m.master.id.as_str(), m.master.theme)).is_equal_to(("m1", 0));
    assert_that!(m.master.placeholders.iter().map(|p| p.role).collect::<Vec<_>>()).is_equal_to(vec![Role::Title, Role::Body]);
    assert_that!((m.master.decor.len(), matches!(m.master.decor[0].kind, ItemKind::Shape(_)))).is_equal_to((1, true));
}

#[test]
fn master_background_comes_from_bg_ref_written_bg1() {
    let (m, _) = master_and_layout(LAYOUT);
    let Fill::Solid { color } = &m.master.background else { panic!("fond plein attendu") };
    assert_that!(color.origin.clone()).is_equal_to(Origin::Scheme { name: "bg1".into(), slot: "lt1".into(), mods: vec![] });
    assert_that!(m.bg.is_some()).is_true();
}

#[test]
fn master_title_level_has_title_style() {
    let (m, _) = master_and_layout(LAYOUT);
    let st = &m.master.placeholders[0].levels[0].run;
    assert_that!((st.size, st.font.origin.clone())).is_equal_to((44.0, Origin::scheme("major")));
}

#[test]
fn master_body_level_two_has_its_bullet() {
    let (m, _) = master_and_layout(LAYOUT);
    let body = &m.master.placeholders[1];
    assert_that!(body.levels[1].run.size).is_equal_to(24.0);
    assert_that!(matches!(&body.levels[1].para.bullet, Bullet::Char { char, .. } if char == "–")).is_true();
}

#[test]
fn layout_reads_kind_name_master_and_inherited_title_style() {
    let (_, l) = master_and_layout(LAYOUT);
    assert_that!((l.layout.id.as_str(), l.layout.kind.as_str(), l.layout.name.as_str(), l.layout.master.as_str())).is_equal_to(("l1", "title", "Diapositive de titre", "m1"));
    let title = &l.layout.placeholders[0];
    assert_that!((title.id.as_str(), title.role, title.levels[0].run.size, title.levels[0].para.align)).is_equal_to(("l1/2", Role::Title, 60.0, Align::Center));
    assert_that!((title.text.anchor, l.layout.show_master_decor)).is_equal_to((Anchor::Bottom, true));
}

#[test]
fn layout_subtitle_drops_the_master_bullet() {
    let (_, l) = master_and_layout(LAYOUT);
    let sub = &l.layout.placeholders[1];
    assert_that!((sub.levels[0].para.bullet.clone(), sub.levels[0].run.size)).is_equal_to((Bullet::None, 24.0));
}

#[test]
fn layout_color_map_override_reaches_inherited_text() {
    let dark = LAYOUT.replace(
        "<a:masterClrMapping/>",
        r#"<a:overrideClrMapping bg1="dk1" tx1="lt1" bg2="dk2" tx2="lt2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>"#,
    );
    let (_, l) = master_and_layout(&dark);
    assert_that!(l.layout.placeholders[0].levels[0].run.color.origin.clone()).is_equal_to(Origin::Scheme { name: "tx1".into(), slot: "lt1".into(), mods: vec![] });
}
```

Ajouter à `test_support.rs` :

```rust
pub(crate) const MASTER: &str = include_str!("../../../../fixtures/import/minimal/ppt/slideMasters/slideMaster1.xml");
pub(crate) const LAYOUT: &str = include_str!("../../../../fixtures/import/minimal/ppt/slideLayouts/slideLayout1.xml");

/// Master de `minimal` et le layout donné, lus avec le thème `THEME`.
pub(crate) fn master_and_layout(layout_xml: &str) -> (crate::pptx::master::MasterInfo, crate::pptx::master::LayoutInfo) {
    const M: &str = "ppt/slideMasters/slideMaster1.xml";
    const L: &str = "ppt/slideLayouts/slideLayout1.xml";
    let t = TestPkg::with(&[(M, MASTER.as_bytes()), (L, layout_xml.as_bytes())]);
    let pkg = t.pkg();
    let theme = ThemeData::parse(THEME, "theme", LIMITS.max_xml_nodes).unwrap();
    let shared = Shared::new(&pkg, None, empty_levels());
    let m = crate::pptx::master::parse_master(&shared, M, 1, 0, &theme).unwrap();
    let l = crate::pptx::master::parse_layout(&shared, L, 1, &m, &theme).unwrap();
    shared.sink.into_inner().finish().unwrap();
    (m, l)
}
```

- [ ] **Step 3 : lancer, constater l'échec** — FAIL.

- [ ] **Step 4 : implémentation**

`src/pptx/master.rs` :

```rust
//! Masters et layouts : décor, placeholders, styles, fond, `clrMap`.

use std::collections::BTreeMap;

use roxmltree::Node;

use crate::error::ImportError;
use crate::pptx::cascade::{TextChain, resolve_levels};
use crate::pptx::color::{clean_map, color_spec_in, default_map};
use crate::pptx::context::{PartCtx, PartSpec, Shared};
use crate::pptx::fill::{FillSpec, parse_fill};
use crate::pptx::placeholder::{MAX_PLACEHOLDERS, PhInfo, PhInheritance, PhTable, TextStyles};
use crate::pptx::shapes::{GroupCtx, ParsedItem, parse_tree};
use crate::pptx::text::{LevelProps, RunProps, empty_levels, parse_lst_style};
use crate::pptx::theme::ThemeData;
use crate::pptx::xml::{attr_bool, attr_str, attr_u32, child, parse};
use crate::source::{Fill, Item, ItemKind, Layout, Master, Placeholder, TextBox};

/// Master lu et ce que ses layouts et slides en héritent.
#[derive(Debug)]
pub(crate) struct MasterInfo {
    pub master: Master,
    pub styles: TextStyles,
    pub phs: PhTable,
    pub clr_map: BTreeMap<String, String>,
    /// Fond non résolu (résolu par chaque slide).
    pub bg: Option<FillSpec>,
}

/// Layout lu et ce que ses slides en héritent.
#[derive(Debug)]
pub(crate) struct LayoutInfo {
    pub layout: Layout,
    pub phs: PhTable,
    pub clr_map: BTreeMap<String, String>,
    pub bg: Option<FillSpec>,
}

/// `clrMapOvr/overrideClrMapping` d'un layout ou d'une slide, sinon `base`.
pub(crate) fn clr_map_override(root: Node<'_, '_>, base: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    match child(root, "clrMapOvr").and_then(|o| child(o, "overrideClrMapping")) {
        Some(o) => clean_map(o.attributes().map(|a| (a.name(), a.value())), base),
        None => base.clone(),
    }
}

/// Fond d'un `cSld`, non résolu : `bgPr`, ou `bgRef` via le thème.
pub(crate) fn background(c_sld: Node<'_, '_>, cx: &PartCtx<'_>) -> Option<FillSpec> {
    let bg = child(c_sld, "bg")?;
    if let Some(pr) = child(bg, "bgPr") {
        return parse_fill(pr, &mut |r| cx.asset(r));
    }
    let r = child(bg, "bgRef")?;
    cx.theme.fill_style(attr_u32(r, "idx").unwrap_or(0), color_spec_in(r).as_ref(), &mut |id| cx.asset(id))
}

/// Table des placeholders déclarés parmi `items`.
pub(crate) fn table_of(items: &[ParsedItem]) -> PhTable {
    let entries = items
        .iter()
        .filter_map(|p| {
            let d = p.ph.as_ref()?;
            Some(PhInfo {
                id: p.item.id.clone(),
                key: d.key.clone(),
                frame: p.item.frame,
                lst: d.lst.clone(),
                body: d.body.clone(),
                fill: d.fill.clone(),
                line: d.line.clone(),
                geom: d.geom.clone(),
            })
        })
        .take(MAX_PLACEHOLDERS)
        .collect();
    PhTable { entries }
}

/// Sépare décor et placeholders ; résout le style de chaque niveau des placeholders.
pub(crate) fn placeholders(items: Vec<ParsedItem>, cx: &PartCtx<'_>) -> (Vec<Item>, Vec<Placeholder>) {
    let mut decor = Vec::new();
    let mut phs = Vec::new();
    for p in items {
        let Some(decl) = p.ph else {
            decor.push(p.item);
            continue;
        };
        let inh = cx.lookup(Some(&decl.key));
        let chain = TextChain { levels: inh.levels.iter().collect(), bodies: inh.bodies.iter().collect() };
        let levels = resolve_levels(&chain, &decl.lst, &cx.env(RunProps::default()));
        let text = match p.item.kind {
            ItemKind::Text(tb) => tb,
            ItemKind::Shape(s) => s.text.unwrap_or_default(),
            _ => TextBox::default(),
        };
        phs.push(Placeholder { id: p.item.id, role: decl.key.role(), idx: decl.key.idx, frame: p.item.frame, levels, text });
    }
    (decor, phs)
}

fn style(root: Node<'_, '_>, name: &str) -> LevelProps {
    child(root, "txStyles").and_then(|t| child(t, name)).map_or_else(empty_levels, parse_lst_style)
}

/// Lit le master `part` (n-ième, 1..), dont le thème est `themes[theme_index]`.
pub(crate) fn parse_master(shared: &Shared<'_>, part: &str, n: usize, theme_index: usize, theme: &ThemeData<'_>) -> Result<MasterInfo, ImportError> {
    let xml = shared.pkg.text(part)?;
    let doc = parse(&xml, part, shared.pkg.limits().max_xml_nodes)?;
    let root = doc.root_element();
    let map = match child(root, "clrMap") {
        Some(m) => clean_map(m.attributes().map(|a| (a.name(), a.value())), &default_map()),
        None => default_map(),
    };
    let styles = TextStyles { pres: shared.pres.clone(), title: style(root, "titleStyle"), body: style(root, "bodyStyle"), other: style(root, "otherStyle") };
    let inherit = PhInheritance { styles: &styles, master: None, layout: None };
    let id = format!("m{n}");
    let spec = PartSpec { owner: id.clone(), slide: None, colors: theme.color_ctx(map.clone()), theme, inherit: Some(&inherit) };
    let cx = PartCtx::new(shared, part, spec)?;
    let c_sld = child(root, "cSld");
    let items = c_sld.and_then(|c| child(c, "spTree")).map(|t| parse_tree(t, &cx, &GroupCtx::ROOT)).unwrap_or_default();
    let phs = table_of(&items);
    let bg = c_sld.and_then(|c| background(c, &cx));
    let background: Fill = bg.as_ref().map_or(Fill::None, |b| b.resolve(&cx.colors));
    let name = crate::collect::clip(c_sld.and_then(|c| attr_str(c, "name")).unwrap_or_default());
    let (decor, placeholders) = placeholders(items, &cx);
    let master = Master { id, name, theme: theme_index, background, decor, placeholders };
    Ok(MasterInfo { master, styles, phs, clr_map: map, bg })
}

/// Lit le layout `part` (n-ième, 1..) de `master`.
pub(crate) fn parse_layout(shared: &Shared<'_>, part: &str, n: usize, master: &MasterInfo, theme: &ThemeData<'_>) -> Result<LayoutInfo, ImportError> {
    let xml = shared.pkg.text(part)?;
    let doc = parse(&xml, part, shared.pkg.limits().max_xml_nodes)?;
    let root = doc.root_element();
    let map = clr_map_override(root, &master.clr_map);
    let inherit = PhInheritance { styles: &master.styles, master: Some(&master.phs), layout: None };
    let id = format!("l{n}");
    let spec = PartSpec { owner: id.clone(), slide: None, colors: theme.color_ctx(map.clone()), theme, inherit: Some(&inherit) };
    let cx = PartCtx::new(shared, part, spec)?;
    let c_sld = child(root, "cSld");
    let items = c_sld.and_then(|c| child(c, "spTree")).map(|t| parse_tree(t, &cx, &GroupCtx::ROOT)).unwrap_or_default();
    let phs = table_of(&items);
    let bg = c_sld.and_then(|c| background(c, &cx));
    let name = crate::collect::clip(c_sld.and_then(|c| attr_str(c, "name")).unwrap_or_default());
    let (decor, placeholders) = placeholders(items, &cx);
    let layout = Layout {
        id,
        name,
        kind: crate::collect::clip_to(attr_str(root, "type").unwrap_or("cust"), 32),
        master: master.master.id.clone(),
        background: bg.as_ref().map(|b| b.resolve(&cx.colors)),
        show_master_decor: attr_bool(root, "showMasterSp").unwrap_or(true),
        decor,
        placeholders,
    };
    Ok(LayoutInfo { layout, phs, clr_map: map, bg })
}

#[cfg(test)]
#[path = "master_tests.rs"]
mod master_tests;
```

> Pas de `drop(cx)` : `cx` n'est plus utilisé après `placeholders`, l'emprunt de `styles` se termine avant son déplacement.

- [ ] **Step 5 : lancer, constater le succès** — PASS.

- [ ] **Step 6 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import fixtures/import/minimal && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : masters et layouts, fonds et styles non résolus (t-692)
oku cli task done --project slider --id t-715
```

---

### Task 11: Slides, notes et `read_pptx` — de bout en bout, vrai deck compris

> *Alignement sur l'exécution :* régression supplémentaire `tests/regressions/issue_692_xml_nodes_limit.rs` et test `oversized_structure_part_is_too_large_not_a_warning`.

**Files:**
- Create: `src/pptx/slide.rs`, `tests/read.rs`, `tests/alto.rs`, `tests/regressions/main.rs`, `tests/regressions/issue_692_xml_depth.rs`, fichiers restants de `fixtures/import/minimal/`, `fixtures/import/README.md`, `fixtures/import/alto-wealth.pptx`, `fixtures/import/alto-wealth.pdf` (LFS)
- Modify: `src/pptx/mod.rs` (réécrit), `src/lib.rs` (forme finale), `.gitattributes`

**Interfaces:**
- Produces : `pub fn read_pptx(bytes: &[u8], limits: &ReadLimits) -> Result<SourceDeck, ImportError>` ; `slide::parse_slide(shared, part, n: u32, layout: Option<&LayoutInfo>, master: &MasterInfo, theme: &ThemeData) -> Result<SourceSlide, ImportError>` ; `slide::notes_text(cx) -> String`.

Règles (décision 8) :
- Masters selon `sldMasterIdLst`, **dédupliqués par chemin** ; layouts de chaque master selon ses relations `/slideLayout` ; slides selon `sldIdLst` (plus de `max_slides` → `TooLarge`).
- Thème : relation `/theme` de chaque master ; absente → `UnreadablePart("<master>: theme")` ; illisible → `UnreadablePart(<part du thème>)` ; dans les deux cas `ThemeData::office()`.
- Erreur non fatale sur une part de structure → `UnreadablePart(part)`, part omise ; `TooLarge` toujours rendu. Aucun master utilisable → `Unreadable`.
- Slide sans layout lisible → `layout: None`, premier master utilisable, `UnreadablePart("<slide>: layout")`.
- Fond : celui de la slide, sinon du layout, sinon du master, **résolu avec les couleurs de la slide**. `show_layout_decor` = `showMasterSp` (vrai par défaut), `hidden` = `show="0"`.
- Notes : relation `/notesSlide` → forme `ph type="body"` → paragraphes (`r`, `fld` → texte ; `br` → `\n`) joints par `\n`.
- Après chaque part, une erreur fatale notée par `Sink` est rendue (`check`). `fonts` reste vide ici (tâche 15).

- [ ] **Step 1 : le reste de la fixture `minimal`, sa provenance, le vrai deck**

`[Content_Types].xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
<Override PartName="/ppt/slides/slide2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
<Override PartName="/ppt/slides/slide3.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
<Override PartName="/ppt/notesSlides/notesSlide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>
```

`_rels/.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>
```

`ppt/presentation.xml` (l'`id` est écrit avant `r:id`, comme PowerPoint et PptxGenJS ; `spc="100"` n'existe que dans `defaultTextStyle` pour le rendre observable) :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
<p:sldIdLst><p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId3"/><p:sldId id="258" r:id="rId5"/></p:sldIdLst>
<p:sldSz cx="12192000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/>
<p:defaultTextStyle><a:defPPr><a:defRPr lang="fr-FR"/></a:defPPr><a:lvl1pPr><a:defRPr sz="2000" spc="100"><a:solidFill><a:schemeClr val="tx1"/></a:solidFill><a:latin typeface="+mn-lt"/></a:defRPr></a:lvl1pPr></p:defaultTextStyle>
</p:presentation>
```

`ppt/_rels/presentation.xml.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
<Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide3.xml"/>
</Relationships>
```

`ppt/slideMasters/_rels/slideMaster1.xml.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>
```

`ppt/slideLayouts/slideLayout2.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="obj" preserve="1">
<p:cSld name="Titre et contenu"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Titre</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Content Placeholder 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Texte</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>
```

`ppt/slideLayouts/_rels/slideLayout1.xml.rels` et `ppt/slideLayouts/_rels/slideLayout2.xml.rels` (identiques) :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>
```

`ppt/slides/slide1.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Titre 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="ctrTitle"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="fr-FR"/><a:t>Rapport annuel</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Sous-titre 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="subTitle" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="fr-FR"/><a:t>Exercice 2026</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>
```

`ppt/slides/_rels/slide1.xml.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>
```

`ppt/slides/slide2.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Titre 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="fr-FR"/><a:t>Résultats</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Espace réservé du contenu 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="fr-FR"/><a:t>Chiffre d'affaires en hausse</a:t></a:r></a:p><a:p><a:pPr lvl="1"/><a:r><a:rPr lang="fr-FR"/><a:t>+12 % sur un an</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="4" name="ZoneTexte 3"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="838200" y="6096000"/><a:ext cx="3048000" cy="369332"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr wrap="square"><a:spAutoFit/></a:bodyPr><a:lstStyle/><a:p><a:r><a:rPr lang="fr-FR"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:rPr><a:t>Source : comptes 2026</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="5" name="Rectangle 4"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="8382000" y="1825625"/><a:ext cx="2971800" cy="1371600"/></a:xfrm><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom></p:spPr><p:style><a:lnRef idx="2"><a:schemeClr val="accent1"><a:shade val="50000"/></a:schemeClr></a:lnRef><a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef><a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef><a:fontRef idx="minor"><a:schemeClr val="lt1"/></a:fontRef></p:style><p:txBody><a:bodyPr rtlCol="0" anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="fr-FR" sz="3200" b="1"/><a:t>+12 %</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>
```

`ppt/slides/_rels/slide2.xml.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide1.xml"/>
</Relationships>
```

`ppt/slides/slide3.xml` (section sombre : texte **et** fond suivent le `clrMapOvr`) :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Titre 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="fr-FR"/><a:t>Section sombre</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="ZoneTexte 2"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="838200" y="3048000"/><a:ext cx="3048000" cy="369332"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="fr-FR"/><a:t>Texte sans couleur</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld><p:clrMapOvr><a:overrideClrMapping bg1="dk1" tx1="lt1" bg2="dk2" tx2="lt2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:clrMapOvr></p:sld>
```

`ppt/slides/_rels/slide3.xml.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/>
</Relationships>
```

`ppt/notesSlides/notesSlide1.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Image de la diapositive 1"/><p:cNvSpPr/><p:nvPr><p:ph type="sldImg"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Notes 2"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Dire bonjour.</a:t></a:r></a:p><a:p><a:r><a:t>Puis</a:t></a:r><a:br/><a:r><a:t>enchaîner.</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld></p:notes>
```

`fixtures/import/README.md` :

```markdown
# Fixtures d'import (t-692)

- `minimal/`, `kitchen/` : OOXML décompressé écrit à la main, zippé à la
  volée par `crates/oku-slides-import/tests/common`.
- `libreoffice.pptx` : `kitchen` réécrit par LibreOffice (aller-retour).
- `alto-wealth.pptx` : deck d'exemple fourni par Matthieu le 2026-10-09,
  généré par PptxGenJS, versionné avec son accord pour les tests.
  `alto-wealth.pdf` (LFS) : son rendu de référence, produit séparément
  (« appifact kit ») ; cible visuelle des lots 4 et 6, lu par aucun test du lot 1.
```

**Le vrai deck** (si `git log -- fixtures/import/alto-wealth.pptx` est vide) :

```bash
cp "$HOME/Downloads/Alto Wealth — Wealth Bank as an API.pptx" fixtures/import/alto-wealth.pptx
cp "$HOME/Downloads/Alto Wealth — Wealth Bank as an API.pdf" fixtures/import/alto-wealth.pdf
```

`.gitattributes` : ajouter `fixtures/import/*.pdf filter=lfs diff=lfs merge=lfs -text`.

- [ ] **Step 2 : tests d'intégration (qui échouent)**

`tests/read.rs` :

```rust
//! Lecture de bout en bout de la fixture `minimal`, et ses dégradations.

mod common;

use oku_slides_import::source::{
    Align, Bullet, Fill, Item, ItemKind, Origin, PlaceholderLink, Role, SourceDeck, SourceSlide, Spacing, TextBox, WarningKind,
};
use oku_slides_import::{ImportErrorKind, ReadLimits, read_pptx};
use speculoos::prelude::*;

fn minimal() -> SourceDeck {
    read_pptx(&common::pptx("minimal"), &common::LIMITS).unwrap_or_else(|e| panic!("lecture de minimal : {e}"))
}

fn minimal_with(overrides: &[(&str, Option<&str>)]) -> Result<SourceDeck, oku_slides_import::ImportError> {
    read_pptx(&common::pptx_with("minimal", overrides), &common::LIMITS)
}

fn item<'a>(s: &'a SourceSlide, name: &str) -> &'a Item {
    s.items.iter().find(|i| i.name == name).unwrap_or_else(|| panic!("pas d'item {name}"))
}

fn text(i: &Item) -> &TextBox {
    match &i.kind {
        ItemKind::Text(tb) => tb,
        ItemKind::Shape(s) => s.text.as_ref().unwrap_or_else(|| panic!("forme sans texte")),
        k => panic!("texte attendu, trouvé {k:?}"),
    }
}

fn unreadable_parts(d: &SourceDeck) -> Vec<String> {
    d.warnings.iter().filter_map(|w| match &w.kind { WarningKind::UnreadablePart(p) => Some(p.clone()), _ => None }).collect()
}

#[test]
fn minimal_has_its_structure_and_no_warning() {
    let d = minimal();
    assert_that!((d.size.w, d.size.h, d.masters.len(), d.themes.len())).is_equal_to((960.0, 540.0, 1, 1));
    assert_that!(d.layouts.iter().map(|l| l.kind.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["title", "obj"]);
    assert_that!(d.slides.iter().map(|s| s.layout.as_deref()).collect::<Vec<_>>()).is_equal_to(vec![Some("l1"), Some("l2"), Some("l2")]);
    assert_that!(d.warnings.clone()).is_empty();
}

#[test]
fn title_slide_inherits_frame_and_style_from_its_layout() {
    let d = minimal();
    let t = item(&d.slides[0], "Titre 1");
    assert_that!((t.role, t.frame.x)).is_equal_to((Some(Role::Title), 120.0));
    let p = &text(t).paragraphs[0];
    assert_that!((p.runs[0].text.as_str(), p.runs[0].style.size, p.style.align)).is_equal_to(("Rapport annuel", 60.0, Align::Center));
}

#[test]
fn content_slide_inherits_master_frame_and_bullets() {
    let d = minimal();
    let s = &d.slides[1];
    let title = item(s, "Titre 1");
    assert_that!((title.frame.x, text(title).paragraphs[0].runs[0].style.size)).is_equal_to((66.0, 44.0));
    assert_that!(text(title).paragraphs[0].runs[0].style.font.origin.clone()).is_equal_to(Origin::scheme("major"));
    let body = text(item(s, "Espace réservé du contenu 2"));
    let bullets: Vec<String> = body.paragraphs.iter().map(|p| match &p.style.bullet { Bullet::Char { char, .. } => char.clone(), _ => String::new() }).collect();
    assert_that!(bullets).is_equal_to(vec!["•".to_owned(), "–".to_owned()]);
    assert_that!(body.paragraphs.iter().map(|p| p.runs[0].style.size).collect::<Vec<_>>()).is_equal_to(vec![28.0, 24.0]);
    assert_that!(body.paragraphs[0].style.line_spacing).is_equal_to(Spacing::Percent(1.0));
}

#[test]
fn slide_placeholder_links_to_layout_and_master_placeholders() {
    let d = minimal();
    assert_that!(item(&d.slides[1], "Espace réservé du contenu 2").placeholder.clone())
        .is_equal_to(Some(PlaceholderLink { idx: Some(1), layout: Some("l2/3".into()), master: Some("m1/3".into()) }));
}

#[test]
fn plain_textbox_takes_other_style_size_and_presentation_spacing() {
    let d = minimal();
    // otherStyle (16 pt) l'emporte sur defaultTextStyle (20 pt) ; spc n'existe que dans defaultTextStyle.
    let r = &text(item(&d.slides[1], "ZoneTexte 3")).paragraphs[0].runs[0];
    assert_that!((r.style.size, r.style.char_spacing, r.style.color.value.to_hex(), r.style.color.origin.clone()))
        .is_equal_to((16.0, 1.0, "#ff0000".to_owned(), Origin::Explicit));
}

#[test]
fn styled_shape_gets_theme_fill_and_font_ref_color() {
    let d = minimal();
    let ItemKind::Shape(s) = &item(&d.slides[1], "Rectangle 4").kind else { panic!("forme attendue") };
    assert_that!(matches!(&s.fill, Fill::Solid { color } if color.origin == Origin::scheme("accent1"))).is_true();
    let r = &s.text.as_ref().unwrap_or_else(|| panic!("texte attendu")).paragraphs[0].runs[0];
    assert_that!((r.style.color.origin.clone(), r.style.bold)).is_equal_to((Origin::scheme("lt1"), true));
}

#[test]
fn dark_slide_color_map_reaches_text_and_inherited_background() {
    let d = minimal();
    let s = &d.slides[2];
    let title = &text(item(s, "Titre 1")).paragraphs[0].runs[0];
    let plain = &text(item(s, "ZoneTexte 2")).paragraphs[0].runs[0];
    assert_that!((title.style.color.origin.slot(), plain.style.color.origin.slot())).is_equal_to((Some("lt1"), Some("lt1")));
    // Fond hérité du master (bgRef bg1) : bg1 → dk1 sur cette slide, donc noir sous un texte clair.
    let Fill::Solid { color } = &s.background else { panic!("fond plein attendu") };
    assert_that!((color.origin.clone(), color.value.to_hex())).is_equal_to((Origin::Scheme { name: "bg1".into(), slot: "dk1".into(), mods: vec![] }, "#000000".to_owned()));
}

#[test]
fn slide_background_and_notes_are_resolved() {
    let d = minimal();
    assert_that!(matches!(&d.slides[1].background, Fill::Solid { color } if color.origin.slot() == Some("lt1"))).is_true();
    assert_that!((d.slides[1].notes.as_str(), d.slides[0].notes.as_str())).is_equal_to(("Dire bonjour.\nPuis\nenchaîner.", ""));
}

#[test]
fn missing_theme_falls_back_to_office_with_a_warning() {
    let d = minimal_with(&[("ppt/theme/theme1.xml", None)]).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(d.themes[0].name.as_str()).is_equal_to("Office");
    assert_that!(unreadable_parts(&d)).contains("ppt/theme/theme1.xml".to_owned());
}

#[test]
fn unreadable_slide_is_omitted_with_a_warning() {
    let d = minimal_with(&[("ppt/slides/slide2.xml", Some("<p:sld"))]).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(d.slides.iter().map(|s| s.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["s1", "s3"]);
    assert_that!(unreadable_parts(&d)).contains("ppt/slides/slide2.xml".to_owned());
}

#[test]
fn unreadable_layout_leaves_its_slides_without_layout() {
    let d = minimal_with(&[("ppt/slideLayouts/slideLayout2.xml", Some("<p:sldLayout"))]).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(d.slides.iter().map(|s| s.layout.as_deref()).collect::<Vec<_>>()).is_equal_to(vec![Some("l1"), None, None]);
    assert_that!(unreadable_parts(&d)).contains("ppt/slideLayouts/slideLayout2.xml".to_owned());
}

#[test]
fn missing_master_is_fatal_unreadable() {
    let err = minimal_with(&[("ppt/slideMasters/slideMaster1.xml", None)]).map(|_| ()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::Unreadable);
}

#[test]
fn too_many_slides_is_too_large() {
    let err = read_pptx(&common::pptx("minimal"), &ReadLimits { max_slides: 2, ..common::LIMITS }).map(|_| ()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn minimal_snapshot_matches() {
    insta::assert_json_snapshot!("minimal", minimal());
}
```

`tests/alto.rs` (vrai deck PptxGenJS ; valeurs relevées dans `ppt/slides/slide3.xml`) :

```rust
//! Le vrai deck d'exemple (`fixtures/import/alto-wealth.pptx`, généré par PptxGenJS).

mod common;

use oku_slides_import::read_pptx;
use oku_slides_import::source::{Fill, ItemKind, Origin, SourceDeck, Spacing, WarningKind};
use speculoos::prelude::*;

fn alto() -> SourceDeck {
    read_pptx(&common::file("alto-wealth.pptx"), &common::LIMITS).unwrap_or_else(|e| panic!("lecture d'alto-wealth : {e}"))
}

#[test]
fn alto_reads_its_eight_slides_on_its_single_layout() {
    let d = alto();
    assert_that!(d.slides.len()).is_equal_to(8);
    assert_that!(d.slides.iter().all(|s| s.layout.as_deref() == Some("l1"))).is_true();
}

#[test]
fn alto_has_no_structural_warning() {
    let d = alto();
    let structural = d.warnings.iter().filter(|w| matches!(w.kind, WarningKind::UnreadablePart(_) | WarningKind::MissingPart(_) | WarningKind::UnreadableItem(_)));
    assert_that!(structural.count()).is_equal_to(0);
}

#[test]
fn alto_kicker_keeps_letter_spacing_line_spacing_and_explicit_color() {
    let d = alto();
    let it = d.slides[2].items.iter().find(|i| i.name == "Text 0").unwrap_or_else(|| panic!("pas de « Text 0 » en slide 3"));
    let ItemKind::Text(tb) = &it.kind else { panic!("texte attendu") };
    let p = &tb.paragraphs[0];
    let r = &p.runs[0].style;
    assert_that!(p.runs[0].text.as_str()).is_equal_to("THE SHIFT");
    assert_that!((r.size, r.bold, r.char_spacing, r.font.value.as_str())).is_equal_to((9.5, true, 0.5, "Inter"));
    assert_that!((r.color.value.to_hex(), r.color.origin.clone(), p.style.line_spacing)).is_equal_to(("#00bef4".to_owned(), Origin::Explicit, Spacing::Points(13.3)));
}

#[test]
fn alto_third_slide_background_is_explicit_navy() {
    let d = alto();
    assert_that!(matches!(&d.slides[2].background, Fill::Solid { color } if color.value.to_hex() == "#00154e")).is_true();
}

#[test]
fn alto_repeated_images_are_stored_once() {
    let d = alto();
    // image-2-1, image-3-5, image-4-1 … image-8-1 : le même fichier de 6 334 octets.
    assert_that!(d.assets.values().filter(|a| a.bytes.len() == 6334).count()).is_equal_to(1);
}
```

Régression de la profondeur XML (décision 2 ; créée ici, avec `read_pptx` et la fixture `minimal` — un test d'intégration ignoré ne compilerait pas avant). `tests/regressions/main.rs` :

```rust
//! Tests de régression de l'import (un fichier par bug).

#[path = "../common/mod.rs"]
mod common;
mod issue_692_xml_depth;
```

`tests/regressions/issue_692_xml_depth.rs` :

```rust
//! Régression (t-692) : un XML imbriqué sur des milliers de niveaux débordait la pile de roxmltree.

use oku_slides_import::{ImportErrorKind, ReadLimits, read_pptx};
use speculoos::prelude::*;

use crate::common;

#[test]
fn deeply_nested_part_is_refused_not_crashed() {
    let deep = format!("{}{}", "<a>".repeat(20_000), "</a>".repeat(20_000));
    let bytes = common::pptx_with("minimal", &[("ppt/presentation.xml", Some(&deep))]);
    match read_pptx(&bytes, &ReadLimits::default()) {
        Err(e) => assert_that!(e.kind()).is_equal_to(ImportErrorKind::Unreadable),
        Ok(_) => panic!("refus attendu"),
    }
}
```

- [ ] **Step 3 : lancer, constater l'échec** — `cargo nextest run -p oku-slides-import --test read --test alto --test regressions` : FAIL à la compilation.

- [ ] **Step 4 : `slide.rs`**

```rust
//! Slides et notes.

use crate::collect::clip;
use crate::error::ImportError;
use crate::pptx::context::{PartCtx, PartSpec, Shared};
use crate::pptx::master::{LayoutInfo, MasterInfo, background, clr_map_override};
use crate::pptx::placeholder::PhInheritance;
use crate::pptx::shapes::{GroupCtx, parse_tree};
use crate::pptx::theme::ThemeData;
use crate::pptx::xml::{attr_bool, attr_str, child, descendant, elements, parse};
use crate::source::{Fill, SourceSlide, WarningKind};

/// Lit la slide `part`, n-ième (1..), sur `layout` (s'il est lisible) de `master`.
pub(crate) fn parse_slide(
    shared: &Shared<'_>,
    part: &str,
    n: u32,
    layout: Option<&LayoutInfo>,
    master: &MasterInfo,
    theme: &ThemeData<'_>,
) -> Result<SourceSlide, ImportError> {
    let xml = shared.pkg.text(part)?;
    let doc = parse(&xml, part, shared.pkg.limits().max_xml_nodes)?;
    let root = doc.root_element();
    let base = layout.map_or(&master.clr_map, |l| &l.clr_map);
    let inherit = PhInheritance { styles: &master.styles, master: Some(&master.phs), layout: layout.map(|l| &l.phs) };
    let spec = PartSpec { owner: format!("s{n}"), slide: Some(n), colors: theme.color_ctx(clr_map_override(root, base)), theme, inherit: Some(&inherit) };
    let cx = PartCtx::new(shared, part, spec)?;
    let c_sld = child(root, "cSld");
    let items = c_sld.and_then(|c| child(c, "spTree")).map(|t| parse_tree(t, &cx, &GroupCtx::ROOT)).unwrap_or_default();
    // Fond propre, sinon hérité ; résolu avec les couleurs de la slide (son clrMapOvr compris).
    let bg: Fill = c_sld
        .and_then(|c| background(c, &cx))
        .or_else(|| layout.and_then(|l| l.bg.clone()))
        .or_else(|| master.bg.clone())
        .map_or(Fill::None, |b| b.resolve(&cx.colors));
    Ok(SourceSlide {
        id: format!("s{n}"),
        number: n,
        layout: layout.map(|l| l.layout.id.clone()),
        background: bg,
        show_layout_decor: attr_bool(root, "showMasterSp").unwrap_or(true),
        hidden: attr_bool(root, "show") == Some(false),
        items: items.into_iter().map(|p| p.item).collect(),
        notes: notes_text(&cx),
    })
}

/// Texte des notes de la slide (paragraphes joints par `\n`, `a:br` → `\n`), vide sans notes.
pub(crate) fn notes_text(cx: &PartCtx<'_>) -> String {
    let Some((_, part)) = cx.rels.by_type("/notesSlide").first().copied() else { return String::new() };
    let Some(xml) = cx.part_text(part, "notes") else { return String::new() };
    let Ok(doc) = parse(&xml, part, cx.nodes()) else {
        cx.warn(None, WarningKind::UnreadablePart(clip(part)));
        return String::new();
    };
    let body = doc
        .root_element()
        .descendants()
        .find(|n| n.tag_name().name() == "sp" && descendant(*n, "ph").is_some_and(|ph| attr_str(ph, "type") == Some("body")));
    let Some(tx) = body.and_then(|b| child(b, "txBody")) else { return String::new() };
    elements(tx)
        .filter(|p| p.tag_name().name() == "p")
        .map(|p| {
            elements(p)
                .filter_map(|r| match r.tag_name().name() {
                    "r" | "fld" => child(r, "t").and_then(|t| t.text()).map(str::to_owned),
                    "br" => Some("\n".to_owned()),
                    _ => None,
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

- [ ] **Step 5 : `src/pptx/mod.rs` réécrit (forme de la tâche 11)**

```rust
//! Lecteur PPTX : du paquet OOXML à la [`crate::source::SourceDeck`].

pub(crate) mod cascade;
pub(crate) mod color;
pub(crate) mod context;
pub(crate) mod fill;
pub(crate) mod frame;
pub(crate) mod geometry;
pub(crate) mod master;
pub(crate) mod package;
pub(crate) mod placeholder;
pub(crate) mod shapes;
pub(crate) mod slide;
#[cfg(test)]
pub(crate) mod test_support;
pub(crate) mod text;
pub(crate) mod theme;
pub(crate) mod units;
pub(crate) mod xml;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use roxmltree::Node;

use crate::collect::{Sink, clip};
use crate::error::{ImportError, structure, too_large};
use crate::pptx::context::Shared;
use crate::pptx::master::{LayoutInfo, MasterInfo, parse_layout, parse_master};
use crate::pptx::package::{Package, Rels};
use crate::pptx::slide::parse_slide;
use crate::pptx::text::{empty_levels, parse_lst_style};
use crate::pptx::theme::ThemeData;
use crate::pptx::units::emu;
use crate::pptx::xml::{attr_i64, attr_r, child, children, parse};
use crate::source::{Size, SourceDeck, SourceSlide, WarningKind};
use crate::{ImportErrorKind, ReadLimits};

/// `Ok(Some)` si lu ; `Ok(None)` + `UnreadablePart` pour une erreur non fatale ; `Err` pour une limite dépassée.
fn soft<T>(r: Result<T, ImportError>, sink: &RefCell<Sink>, part: &str) -> Result<Option<T>, ImportError> {
    match r {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.kind() == ImportErrorKind::TooLarge => Err(e),
        Err(_) => {
            sink.borrow_mut().warn(None, None, WarningKind::UnreadablePart(clip(part)));
            Ok(None)
        }
    }
}

/// Rend l'erreur fatale notée pendant la lecture d'une part.
fn check(shared: &Shared<'_>) -> Result<(), ImportError> {
    if !shared.sink.borrow().has_fatal() {
        return Ok(());
    }
    std::mem::take(&mut *shared.sink.borrow_mut()).finish().map(|_| ())
}

/// Cibles des relations listées par `list/item@r:id`, sans doublon.
fn listed(root: Node<'_, '_>, list: &str, item: &str, rels: &Rels) -> Vec<String> {
    let mut seen = BTreeSet::new();
    child(root, list)
        .into_iter()
        .flat_map(|l| children(l, item))
        .filter_map(|m| rels.target(attr_r(m, "id")?).map(str::to_owned))
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

/// Texte du thème de chaque master (même index) ; `None` si absent ou illisible (déjà signalé).
fn theme_texts(shared: &Shared<'_>, masters: &[String]) -> Result<Vec<(String, Option<String>)>, ImportError> {
    let mut out = Vec::new();
    for mp in masters {
        let tp = soft(shared.pkg.rels(mp), &shared.sink, mp)?.and_then(|r| r.by_type("/theme").first().map(|(_, t)| (*t).to_owned()));
        let Some(tp) = tp else {
            shared.sink.borrow_mut().warn(None, None, WarningKind::UnreadablePart(clip(&format!("{mp}: theme"))));
            out.push((String::new(), None));
            continue;
        };
        let text = soft(shared.pkg.text(&tp), &shared.sink, &tp)?;
        out.push((tp, text));
    }
    Ok(out)
}

/// Thèmes analysés (thème Office si absent ou illisible).
fn themes<'t>(shared: &Shared<'_>, texts: &'t [(String, Option<String>)]) -> Result<Vec<ThemeData<'t>>, ImportError> {
    let mut out = Vec::new();
    for (part, text) in texts {
        let parsed = match text {
            Some(x) => soft(ThemeData::parse(x, part, shared.pkg.limits().max_xml_nodes), &shared.sink, part)?,
            None => None,
        };
        out.push(match parsed {
            Some(t) => t,
            None => ThemeData::office()?,
        });
    }
    Ok(out)
}

type Layouts = Vec<(String, usize, LayoutInfo)>; // (part, index dans les masters, layout)

/// Masters utilisables et leurs layouts.
fn masters(shared: &Shared<'_>, parts: &[String], themes: &[ThemeData<'_>]) -> Result<(Vec<MasterInfo>, Layouts), ImportError> {
    let mut masters = Vec::new();
    let mut layouts: Layouts = Vec::new();
    for (i, (mp, theme)) in parts.iter().zip(themes).enumerate() {
        let Some(m) = soft(parse_master(shared, mp, i + 1, i, theme), &shared.sink, mp)? else { continue };
        check(shared)?;
        let layout_parts: Vec<String> = soft(shared.pkg.rels(mp), &shared.sink, mp)?
            .map(|r| r.by_type("/slideLayout").into_iter().map(|(_, t)| t.to_owned()).collect())
            .unwrap_or_default();
        for lp in layout_parts {
            if let Some(l) = soft(parse_layout(shared, &lp, layouts.len() + 1, &m, theme), &shared.sink, &lp)? {
                layouts.push((lp, masters.len(), l));
            }
            check(shared)?;
        }
        masters.push(m);
    }
    if masters.is_empty() {
        return Err(structure("no usable slide master"));
    }
    Ok((masters, layouts))
}

/// Slides lisibles, dans l'ordre de la présentation.
fn slides(shared: &Shared<'_>, parts: &[String], masters: &[MasterInfo], layouts: &Layouts, themes: &[ThemeData<'_>]) -> Result<Vec<SourceSlide>, ImportError> {
    let by_part: BTreeMap<&str, usize> = layouts.iter().enumerate().map(|(k, (p, _, _))| (p.as_str(), k)).collect();
    let mut out = Vec::new();
    for (i, sp) in parts.iter().enumerate() {
        let n = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let lp = soft(shared.pkg.rels(sp), &shared.sink, sp)?.and_then(|r| r.by_type("/slideLayout").first().map(|(_, t)| (*t).to_owned()));
        let entry = lp.as_deref().and_then(|p| by_part.get(p)).and_then(|k| layouts.get(*k));
        if entry.is_none() {
            shared.sink.borrow_mut().warn(Some(n), None, WarningKind::UnreadablePart(clip(&format!("{sp}: layout"))));
        }
        let (layout, mi) = entry.map_or((None, 0), |(_, mi, l)| (Some(l), *mi));
        let (Some(master), Some(theme)) = (masters.get(mi), masters.get(mi).and_then(|m| themes.get(m.master.theme))) else { continue };
        if let Some(s) = soft(parse_slide(shared, sp, n, layout, master, theme), &shared.sink, sp)? {
            out.push(s);
        }
        check(shared)?;
    }
    Ok(out)
}

/// Lit un `.pptx` et rend sa représentation intermédiaire.
///
/// Échoue seulement si le paquet est illisible, n'est pas une présentation,
/// est protégé, n'a aucun master utilisable, ou dépasse `limits`.
pub fn read_pptx(bytes: &[u8], limits: &ReadLimits) -> Result<SourceDeck, ImportError> {
    let pkg = Package::open(bytes, limits)?;
    let nodes = limits.max_xml_nodes;
    let main = pkg.main_part().to_owned();
    let pres_xml = pkg.text(&main)?;
    let pres = parse(&pres_xml, &main, nodes)?;
    let root = pres.root_element();
    let rels = pkg.rels(&main)?;

    // `tableStyles.xml` doit être analysé avant `Shared` (qui le garde) : ses avertissements attendent dans `early`.
    let early = RefCell::new(Sink::new());
    let ts_part = rels.by_type("/tableStyles").first().map(|(_, t)| (*t).to_owned());
    let ts_xml = match &ts_part {
        Some(p) => soft(pkg.text(p), &early, p)?,
        None => None,
    };
    let table_styles = match (&ts_xml, &ts_part) {
        (Some(x), Some(p)) => soft(parse(x, p, nodes), &early, p)?,
        _ => None,
    };
    let shared = Shared { sink: early, ..Shared::new(&pkg, table_styles, child(root, "defaultTextStyle").map_or_else(empty_levels, parse_lst_style)) };

    let size = child(root, "sldSz").map_or(Size { w: 960.0, h: 540.0 }, |s| Size {
        w: emu(attr_i64(s, "cx").unwrap_or(12_192_000)).clamp(1.0, 1e6),
        h: emu(attr_i64(s, "cy").unwrap_or(6_858_000)).clamp(1.0, 1e6),
    });
    let master_parts = listed(root, "sldMasterIdLst", "sldMasterId", &rels);
    let theme_xml = theme_texts(&shared, &master_parts)?;
    let themes = themes(&shared, &theme_xml)?;
    let (masters, layouts) = masters(&shared, &master_parts, &themes)?;
    let slide_parts = listed(root, "sldIdLst", "sldId", &rels);
    if slide_parts.len() > limits.max_slides {
        return Err(too_large("slides"));
    }
    let slides = slides(&shared, &slide_parts, &masters, &layouts, &themes)?;

    let themes_out = themes.iter().map(|t| t.theme.clone()).collect();
    let masters_out = masters.into_iter().map(|m| m.master).collect();
    let layouts_out = layouts.into_iter().map(|(_, _, l)| l.layout).collect();
    let (assets, warnings) = shared.sink.into_inner().finish()?;
    Ok(SourceDeck { size, themes: themes_out, masters: masters_out, layouts: layouts_out, slides, assets, fonts: Vec::new(), warnings })
}
```

> Doublons de `sldIdLst` : `listed` dédoublonne aussi les slides (une slide listée deux fois n'est lue qu'une fois).

`src/lib.rs` — **forme finale** :

```rust
//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! [`read_pptx`] lit les octets d'un `.pptx` et rend une [`source::SourceDeck`] :
//! géométrie en pt, styles texte résolus, couleurs avec leur provenance,
//! tableaux, graphiques, SmartArt, replis, polices. Bornes : [`ReadLimits`] ;
//! erreurs : [`ImportError`]. Les projecteurs vers un deck Oku viennent aux lots suivants.

pub mod source;

mod collect;
mod error;
mod limits;
// Retiré à la tâche 13 : `xml::text_of` et les styles de tableau n'ont d'appelant qu'aux tâches 12 et 13.
#[allow(dead_code, reason = "dernières fonctions branchées aux tâches 12 et 13")]
mod pptx;

pub use error::{ImportError, ImportErrorKind};
pub use limits::ReadLimits;
pub use pptx::read_pptx;
```

- [ ] **Step 6 : lancer ; relire puis accepter le snapshot**

Run: `cargo nextest run -p oku-slides-import --test read --test alto --test regressions` → tout PASS sauf `minimal_snapshot_matches`.

Run: `INSTA_UPDATE=new cargo nextest run -p oku-slides-import --test read minimal_snapshot_matches`, puis relire `tests/snapshots/read__minimal.snap.new` : trois slides, `layout` `l1`/`l2`/`l2`, aucune clé de contenu binaire, `placeholder` renseignés, slide 3 : textes en `slot: lt1` (`name: tx1`) **et** fond `name: bg1, slot: dk1, #000000`. Puis `cargo insta accept --manifest-path crates/oku-slides-import/Cargo.toml`. Corriger le code, jamais le snapshot (G14).

Run: `cargo nextest run -p oku-slides-import` → PASS.

- [ ] **Step 7 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add .gitattributes crates/oku-slides-import fixtures/import/minimal fixtures/import/README.md fixtures/import/alto-wealth.pptx fixtures/import/alto-wealth.pdf
git diff --cached --name-only
git cat-file -p :fixtures/import/alto-wealth.pdf | head -1   # attendu : « version https://git-lfs.github.com/spec/v1 » (pointeur LFS)
git commit -F <scratchpad>/msg.txt   # feat(import) : read_pptx de bout en bout — slides, notes, fixture minimal, vrai deck (t-692)
oku cli task done --project slider --id t-716
```

---

### Task 12: Tableaux

**Files:**
- Create: `src/pptx/table.rs`, `src/pptx/table_tests.rs`, `fixtures/import/kitchen/ppt/tableStyles.xml`
- Modify: `src/pptx/mod.rs` (`pub(crate) mod table;`), `src/pptx/frame.rs` (réécrit), `src/pptx/test_support.rs` (`TABLE_STYLES`)

**Interfaces:**
- Consumes : `PartCtx`, `Shared.table_styles` (8) ; `parse_txbody`, `font_ref`, `anchor`, `RunProps`, `LevelProps` (6) ; `resolve_textbox`, `TextChain` (7) ; `FillSpec`, `LineSpec`, `parse_fill`, `parse_line` (4) ; `color_spec`, `color_spec_in` (3).
- Produces : `table::parse_table(tbl: Node, cx: &PartCtx) -> Table`.
- Règles : colonnes `tblGrid/gridCol@w`, lignes `tr@h` ; `gridSpan`/`rowSpan` bornés à la grille ; `hMerge`/`vMerge` → `covered`. `tblPr` → `Table.flags`. `tableStyleId` → `tblStyle[@styleId]` de `Shared.table_styles` ; absent → `UnknownTableStyle`. Ordre : `wholeTbl` ; `band1V`/`band2V` si `bandCol` (jamais la colonne 0 quand `firstCol`, parité comptée après elle) ; `band1H`/`band2H` si `bandRow` (même règle) ; `firstCol` ; `lastCol` ; `firstRow` ; `lastRow` ; `tcPr`. Partie : `tcStyle/fill` ou `fillRef`, `tcStyle/tcBdr/*` (`ln` ou `lnRef`), `tcTxStyle@b`/`@i` (`attr_bool`), sa couleur (premier enfant couleur), son `fontRef`. Bordures : `left`/`right`/`top`/`bottom` sur les bords, `insideV`/`insideH` entre cellules ; `tcPr/lnL lnR lnT lnB` l'emportent. Texte : `cx.lookup(None).levels`, `TextEnv.font_ref` = texte du style ; marges `tcPr@mar*` et `tcPr@anchor`. Tout est résolu avec `cx.colors`.

- [ ] **Step 1 : la fixture des styles de tableau**

`fixtures/import/kitchen/ppt/tableStyles.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{S}">
<a:tblStyle styleId="{S}" styleName="Moyen 2 - Accent 1">
<a:wholeTbl><a:tcTxStyle><a:fontRef idx="minor"/><a:schemeClr val="dk1"/></a:tcTxStyle><a:tcStyle><a:tcBdr><a:insideH><a:ln w="12700"><a:solidFill><a:schemeClr val="lt1"/></a:solidFill></a:ln></a:insideH></a:tcBdr><a:fill><a:solidFill><a:schemeClr val="accent1"><a:tint val="20000"/></a:schemeClr></a:solidFill></a:fill></a:tcStyle></a:wholeTbl>
<a:band1H><a:tcStyle><a:fill><a:solidFill><a:schemeClr val="accent1"><a:tint val="40000"/></a:schemeClr></a:solidFill></a:fill></a:tcStyle></a:band1H>
<a:band1V><a:tcStyle><a:fill><a:solidFill><a:schemeClr val="accent3"/></a:solidFill></a:fill></a:tcStyle></a:band1V>
<a:firstCol><a:tcTxStyle b="on"/><a:tcStyle><a:fill><a:solidFill><a:schemeClr val="accent2"/></a:solidFill></a:fill></a:tcStyle></a:firstCol>
<a:firstRow><a:tcTxStyle b="on"><a:fontRef idx="minor"/><a:schemeClr val="lt1"/></a:tcTxStyle><a:tcStyle><a:fill><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></a:fill></a:tcStyle></a:firstRow>
</a:tblStyle>
</a:tblStyleLst>
```

`test_support.rs` : `pub(crate) const TABLE_STYLES: &str = include_str!("../../../../fixtures/import/kitchen/ppt/tableStyles.xml");`

- [ ] **Step 2 : tests qui échouent**

`src/pptx/table_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::pptx::test_support::{TABLE_STYLES, TestPkg, with_table_styles};
use crate::pptx::xml::parse;
use crate::source::{Anchor, Fill, Origin, Warning, WarningKind};

/// Tableau 3×2 : en-tête fusionné, bandes de lignes.
const TBL: &str = r#"<a:tbl xmlns:a="urn:a"><a:tblPr firstRow="1" bandRow="1"><a:tableStyleId>{S}</a:tableStyleId></a:tblPr>
<a:tblGrid><a:gridCol w="1270000"/><a:gridCol w="2540000"/></a:tblGrid>
<a:tr h="381000"><a:tc gridSpan="2"><a:txBody><a:bodyPr/><a:p><a:r><a:t>Titre</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc hMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr>
<a:tr h="381000"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>A</a:t></a:r></a:p></a:txBody><a:tcPr marL="0" anchor="ctr"/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>1</a:t></a:r></a:p></a:txBody><a:tcPr><a:lnL w="38100"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:lnL><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:tcPr></a:tc></a:tr>
<a:tr h="381000"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>B</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>2</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
</a:tbl>"#;

fn table_of(xml: &str) -> (Table, Vec<Warning>) {
    let t = TestPkg::with(&[]);
    let pkg = t.pkg();
    let doc = parse(xml, "t", 10_000).unwrap();
    with_table_styles(&pkg, TABLE_STYLES, |cx| parse_table(doc.root_element(), cx))
}

fn fill_hex(t: &Table, r: usize, c: usize) -> String {
    match &t.rows[r][c].fill {
        Fill::Solid { color } => color.value.to_hex(),
        f => panic!("plein attendu en ({r},{c}), trouvé {f:?}"),
    }
}

#[test]
fn grid_rows_and_flags_are_read() {
    let (t, _) = table_of(TBL);
    assert_that!((t.col_widths.clone(), t.row_heights.clone())).is_equal_to((vec![100.0, 200.0], vec![30.0, 30.0, 30.0]));
    assert_that!((t.flags.first_row, t.flags.band_row, t.flags.first_col)).is_equal_to((true, true, false));
}

#[test]
fn merged_cells_are_spanned_and_covered() {
    let (t, _) = table_of(TBL);
    assert_that!((t.rows[0][0].col_span, t.rows[0][1].covered)).is_equal_to((2, true));
}

#[test]
fn first_row_takes_its_style_part() {
    let (t, _) = table_of(TBL);
    assert_that!(fill_hex(&t, 0, 0)).is_equal_to("#4472c4".to_owned());
    let run = &t.rows[0][0].text.paragraphs[0].runs[0].style;
    assert_that!((run.bold, run.color.origin.slot())).is_equal_to((true, Some("lt1")));
}

#[test]
fn bands_alternate_after_the_header_row_with_exact_colors() {
    let (t, _) = table_of(TBL);
    // ligne 1 = band1H (accent1 tint 40 %) ; ligne 2 = wholeTbl seul (accent1 tint 20 %).
    assert_that!((fill_hex(&t, 1, 0), fill_hex(&t, 2, 0), fill_hex(&t, 2, 1))).is_equal_to(("#cfd5ea".to_owned(), "#e9ebf5".to_owned(), "#e9ebf5".to_owned()));
}

#[test]
fn body_cell_text_takes_the_whole_table_color() {
    let (t, _) = table_of(TBL);
    assert_that!(t.rows[2][0].text.paragraphs[0].runs[0].style.color.origin.clone()).is_equal_to(Origin::scheme("dk1"));
}

#[test]
fn cell_properties_override_the_style() {
    let (t, _) = table_of(TBL);
    assert_that!(fill_hex(&t, 1, 1)).is_equal_to("#ff0000".to_owned());
    assert_that!((t.rows[1][0].text.insets.l, t.rows[1][0].text.anchor, t.rows[1][1].borders.l.as_ref().map(|s| s.width))).is_equal_to((0.0, Anchor::Middle, Some(3.0)));
}

#[test]
fn inside_horizontal_border_applies_between_rows_only() {
    let (t, _) = table_of(TBL);
    assert_that!((t.rows[1][0].borders.t.as_ref().map(|s| s.width), t.rows[0][0].borders.t.clone())).is_equal_to((Some(1.0), None));
}

#[test]
fn band_columns_skip_the_first_column_and_count_after_it() {
    let xml = TBL.replace(r#"<a:tblPr firstRow="1" bandRow="1">"#, r#"<a:tblPr firstRow="1" bandRow="1" firstCol="1" bandCol="1">"#);
    let (t, _) = table_of(&xml);
    // (2,0) : firstCol (accent2) ; (2,1) : première colonne en bande → band1V (accent3).
    assert_that!((fill_hex(&t, 2, 0), fill_hex(&t, 2, 1))).is_equal_to(("#ed7d31".to_owned(), "#a5a5a5".to_owned()));
}

#[test]
fn vertical_merge_and_row_span_are_read_and_bounded() {
    let xml = r#"<a:tbl xmlns:a="urn:a"><a:tblGrid><a:gridCol w="12700"/></a:tblGrid><a:tr h="12700"><a:tc rowSpan="99"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr><a:tr h="12700"><a:tc vMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl>"#;
    let (t, _) = table_of(xml);
    assert_that!((t.rows[0][0].row_span, t.rows[1][0].covered)).is_equal_to((2, true));
}

#[test]
fn table_without_style_has_no_fill_and_no_warning() {
    let (t, w) = table_of(&TBL.replace("<a:tableStyleId>{S}</a:tableStyleId>", ""));
    assert_that!((t.rows[2][0].fill.clone(), w.len())).is_equal_to((Fill::None, 0));
}

#[test]
fn unknown_style_id_warns() {
    let (_, w) = table_of(&TBL.replace("{S}", "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"));
    assert_that!(w.iter().any(|w| w.kind == WarningKind::UnknownTableStyle("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}".into()))).is_true();
}
```

Ajouter à `src/pptx/shapes_tests.rs` :

```rust
#[test]
fn table_graphic_frame_becomes_table_item() {
    let (items, _, _) = tree(r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="13" name="Tableau"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblGrid><a:gridCol w="12700"/></a:tblGrid><a:tr h="12700"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>x</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#);
    assert_that!(matches!(items[0].item.kind, ItemKind::Table(_))).is_true();
}
```

- [ ] **Step 3 : lancer, constater l'échec** — FAIL.

- [ ] **Step 4 : implémentation**

`src/pptx/table.rs` :

```rust
//! Tableaux (`a:tbl`) : grille, fusions, style de tableau, propriétés de cellule.

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::cascade::{TextChain, resolve_textbox};
use crate::pptx::color::{color_spec, color_spec_in};
use crate::pptx::context::PartCtx;
use crate::pptx::fill::{FillSpec, LineSpec, parse_fill, parse_line};
use crate::pptx::text::{LevelProps, RunProps, anchor, font_ref, parse_txbody};
use crate::pptx::units::emu;
use crate::pptx::xml::{attr_bool, attr_i64, attr_str, attr_u32, child, children, elements};
use crate::source::{Borders, Cell, Fill, Stroke, Table, TableFlags, TextBox, WarningKind};

const COLORS: [&str; 5] = ["srgbClr", "schemeClr", "sysClr", "prstClr", "scrgbClr"];

/// Une partie de style de tableau, lue (non résolue).
#[derive(Debug, Clone, Default)]
struct Part {
    fill: Option<FillSpec>,
    left: Option<LineSpec>,
    right: Option<LineSpec>,
    top: Option<LineSpec>,
    bottom: Option<LineSpec>,
    inside_h: Option<LineSpec>,
    inside_v: Option<LineSpec>,
    text: RunProps,
}

fn border(bdr: Node<'_, '_>, name: &str, cx: &PartCtx<'_>) -> Option<LineSpec> {
    let side = child(bdr, name)?;
    if let Some(ln) = child(side, "ln") {
        return Some(parse_line(ln));
    }
    let r = child(side, "lnRef")?;
    Some(cx.theme.line_style(attr_u32(r, "idx").unwrap_or(0), color_spec_in(r).as_ref()))
}

fn read_part(p: Node<'_, '_>, cx: &PartCtx<'_>) -> Part {
    let mut part = Part::default();
    if let Some(tc) = child(p, "tcStyle") {
        part.fill = child(tc, "fill").and_then(|f| parse_fill(f, &mut |r| cx.asset(r))).or_else(|| {
            let r = child(tc, "fillRef")?;
            cx.theme.fill_style(attr_u32(r, "idx").unwrap_or(0), color_spec_in(r).as_ref(), &mut |id| cx.asset(id))
        });
        if let Some(b) = child(tc, "tcBdr") {
            part.left = border(b, "left", cx);
            part.right = border(b, "right", cx);
            part.top = border(b, "top", cx);
            part.bottom = border(b, "bottom", cx);
            part.inside_h = border(b, "insideH", cx);
            part.inside_v = border(b, "insideV", cx);
        }
    }
    if let Some(tx) = child(p, "tcTxStyle") {
        part.text.bold = attr_bool(tx, "b");
        part.text.italic = attr_bool(tx, "i");
        part.text.color = elements(tx).find(|c| COLORS.contains(&c.tag_name().name())).and_then(color_spec);
        part.text.font = child(tx, "fontRef").and_then(font_ref);
    }
    part
}

fn overlay(acc: &mut Part, top: &Part) {
    if top.fill.is_some() {
        acc.fill.clone_from(&top.fill);
    }
    for (a, t) in [
        (&mut acc.left, &top.left),
        (&mut acc.right, &top.right),
        (&mut acc.top, &top.top),
        (&mut acc.bottom, &top.bottom),
        (&mut acc.inside_h, &top.inside_h),
        (&mut acc.inside_v, &top.inside_v),
    ] {
        if t.is_some() {
            a.clone_from(t);
        }
    }
    acc.text.overlay(&top.text);
}

/// Parties du style `id` (`wholeTbl`, `band1H`…), ou `None` s'il n'existe pas.
fn style_parts(id: &str, cx: &PartCtx<'_>) -> Option<Vec<(String, Part)>> {
    let doc = cx.shared.table_styles.as_ref()?;
    let style = elements(doc.root_element()).find(|n| n.tag_name().name() == "tblStyle" && attr_str(*n, "styleId") == Some(id))?;
    Some(elements(style).take(16).map(|p| (p.tag_name().name().to_owned(), read_part(p, cx))).collect())
}

fn flags(pr: Option<Node<'_, '_>>) -> TableFlags {
    let f = |name: &str| pr.and_then(|p| attr_bool(p, name)).unwrap_or(false);
    TableFlags { first_row: f("firstRow"), last_row: f("lastRow"), first_col: f("firstCol"), last_col: f("lastCol"), band_row: f("bandRow"), band_col: f("bandCol") }
}

/// Parties qui s'appliquent à la cellule (r, c), du plus faible au plus fort.
fn parts_for(fl: TableFlags, (r, c): (usize, usize), (nr, nc): (usize, usize)) -> Vec<&'static str> {
    let mut out = vec!["wholeTbl"];
    if fl.band_col && !(fl.first_col && c == 0) {
        let k = c - usize::from(fl.first_col);
        out.push(if k % 2 == 0 { "band1V" } else { "band2V" });
    }
    if fl.band_row && !(fl.first_row && r == 0) {
        let k = r - usize::from(fl.first_row);
        out.push(if k % 2 == 0 { "band1H" } else { "band2H" });
    }
    if fl.first_col && c == 0 {
        out.push("firstCol");
    }
    if fl.last_col && c + 1 == nc {
        out.push("lastCol");
    }
    if fl.first_row && r == 0 {
        out.push("firstRow");
    }
    if fl.last_row && r + 1 == nr {
        out.push("lastRow");
    }
    out
}

/// Lit un `a:tbl`.
pub(crate) fn parse_table(tbl: Node<'_, '_>, cx: &PartCtx<'_>) -> Table {
    let pr = child(tbl, "tblPr");
    let fl = flags(pr);
    let style_id = pr.and_then(|p| child(p, "tableStyleId")).and_then(|n| n.text()).map(str::trim).unwrap_or_default();
    let parts = if style_id.is_empty() {
        Vec::new()
    } else {
        style_parts(style_id, cx).unwrap_or_else(|| {
            cx.warn(None, WarningKind::UnknownTableStyle(clip(style_id)));
            Vec::new()
        })
    };
    let col_widths: Vec<f64> = child(tbl, "tblGrid").into_iter().flat_map(|g| children(g, "gridCol")).map(|c| emu(attr_i64(c, "w").unwrap_or(0))).collect();
    let trs: Vec<Node<'_, '_>> = children(tbl, "tr").collect();
    let row_heights = trs.iter().map(|r| emu(attr_i64(*r, "h").unwrap_or(0))).collect();
    let (nr, nc) = (trs.len(), col_widths.len().max(1));
    let levels = cx.lookup(None).levels;
    let rows = trs
        .iter()
        .enumerate()
        .map(|(r, tr)| {
            children(*tr, "tc")
                .enumerate()
                .map(|(c, tc)| {
                    let mut acc = Part::default();
                    for name in parts_for(fl, (r, c), (nr, nc)) {
                        if let Some((_, p)) = parts.iter().find(|(n, _)| n == name) {
                            overlay(&mut acc, p);
                        }
                    }
                    cell(tc, &acc, (r, c), (nr, nc), cx, &levels)
                })
                .collect()
        })
        .collect();
    Table { col_widths, row_heights, flags: fl, rows }
}

fn cell(tc: Node<'_, '_>, style: &Part, (r, c): (usize, usize), (nr, nc): (usize, usize), cx: &PartCtx<'_>, levels: &[LevelProps]) -> Cell {
    let tc_pr = child(tc, "tcPr");
    let covered = attr_bool(tc, "hMerge") == Some(true) || attr_bool(tc, "vMerge") == Some(true);
    let fill = tc_pr.and_then(|p| parse_fill(p, &mut |id| cx.asset(id))).or_else(|| style.fill.clone()).map_or(Fill::None, |f| f.resolve(&cx.colors));
    let pick = |edge: Option<&LineSpec>, own: &str| -> Option<Stroke> {
        let own = tc_pr.and_then(|p| child(p, own)).map(parse_line);
        own.or_else(|| edge.cloned()).and_then(|l| l.finish(&cx.colors))
    };
    let borders = Borders {
        l: pick(if c == 0 { style.left.as_ref() } else { style.inside_v.as_ref() }, "lnL"),
        r: pick(if c + 1 == nc { style.right.as_ref() } else { style.inside_v.as_ref() }, "lnR"),
        t: pick(if r == 0 { style.top.as_ref() } else { style.inside_h.as_ref() }, "lnT"),
        b: pick(if r + 1 == nr { style.bottom.as_ref() } else { style.inside_h.as_ref() }, "lnB"),
    };
    let text = if covered {
        TextBox::default()
    } else {
        let mut tb = child(tc, "txBody").map_or_else(TextBox::default, |tx| {
            let raw = parse_txbody(tx, &mut |id| cx.link(id));
            let chain = TextChain { levels: levels.iter().collect(), bodies: Vec::new() };
            resolve_textbox(&raw, &chain, &cx.env(style.text.clone()))
        });
        if let Some(p) = tc_pr {
            let m = |a: &str, d: f64| attr_i64(p, a).map_or(d, emu);
            tb.insets.l = m("marL", 7.2);
            tb.insets.r = m("marR", 7.2);
            tb.insets.t = m("marT", 3.6);
            tb.insets.b = m("marB", 3.6);
            if let Some(a) = attr_str(p, "anchor") {
                tb.anchor = anchor(a);
            }
        }
        tb
    };
    let span = |a: &str, room: usize| attr_u32(tc, a).unwrap_or(1).clamp(1, u32::try_from(room.max(1)).unwrap_or(u32::MAX));
    Cell { text, fill, borders, row_span: span("rowSpan", nr - r), col_span: span("gridSpan", nc.saturating_sub(c)), covered }
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
```

> `parts_for` : `c - usize::from(first_col)` est gardé par `!(first_col && c == 0)` (correctif de revue ; `band_columns_skip…` le couvre). `nr - r` ne déborde pas : `r < nr`.

`src/pptx/frame.rs` (réécrit) :

```rust
//! `graphicFrame` : tableau, graphique, SmartArt, objet OLE, sinon opaque.

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::context::PartCtx;
use crate::pptx::shapes::{GroupCtx, ParsedItem, identity, opaque};
use crate::pptx::table::parse_table;
use crate::pptx::xml::{attr_str, child, descendant};
use crate::source::{Frame, Item, ItemKind, OpaqueReason};

/// Lit un `graphicFrame`.
pub(crate) fn parse_graphic_frame(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    let (id, name, alt) = identity(n, cx)?;
    let frame = child(n, "xfrm").map_or(Frame::new(0.0, 0.0, 0.0, 0.0), |x| g.frame(x));
    let data = descendant(n, "graphicData")?;
    let uri = attr_str(data, "uri").unwrap_or_default();
    let unknown = || opaque(OpaqueReason::Unknown(clip(uri)), None);
    let kind = if uri.ends_with("/table") { child(data, "tbl").map_or_else(unknown, |t| ItemKind::Table(parse_table(t, cx))) } else { unknown() };
    Some(ParsedItem { item: Item { id, name, alt, frame, z: 0, role: None, placeholder: None, kind }, ph: None })
}
```

- [ ] **Step 5 : lancer, constater le succès** — PASS (snapshot `minimal` inchangé).

- [ ] **Step 6 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import fixtures/import/kitchen/ppt/tableStyles.xml && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : tableaux — fusions, styles de tableau, propriétés de cellule (t-692)
oku cli task done --project slider --id t-717
```

---

### Task 13: Graphiques

> *Alignement sur l'exécution :* `chart_item` route `TooLarge` vers `cx.fail` (voir décision 8) ; test ajouté `chart_part_over_the_node_limit_is_fatal_not_swallowed`.

**Files:**
- Create: `src/pptx/chart.rs`, `src/pptx/chart_tests.rs`, `fixtures/import/kitchen/ppt/charts/chart1.xml`
- Modify: `src/pptx/mod.rs` (`pub(crate) mod chart;`), `src/pptx/frame.rs` (réécrit), `src/pptx/test_support.rs` (`BAR`, relation et part de graphique), `src/lib.rs` (retirer l'`#[allow(dead_code)]` de `mod pptx;` et son commentaire)

**Interfaces:**
- Consumes : `PartCtx` (8) ; `parse_color` (3) ; `xml::*` (dont `text_of`).
- Produces : `chart::ChartSkip { Unsupported(String), Malformed(&'static str) }` ; `chart::parse_chart(root, cx, item: &str) -> Result<Chart, ChartSkip>` ; `MAX_CHART_POINTS = 10_000`, `MAX_SERIES = 256`, `MAX_TRAILING_GAP = 64`.
- Règles : premier type (`*Chart`) de `plotArea` ; plusieurs → `UnsupportedChart("combo: <nom>")` sur l'`item`. Types et `grouping` comme décrit (défaut : `Clustered` pour les barres, `Standard` sinon). Séries (256 au plus) : nom (`tx/v`, sinon cache de `tx`) ; catégories de la première série (`cat` ou `xVal`) ; valeurs (`val` ou `yVal`) ; couleur `spPr/solidFill`. Cache : longueur `min(max(ptCount, idx max + 1), nombre de points + 64, 10 000)`, un `pt` au-delà est ignoré ; valeur non numérique → `None`. Titre : `a:t` de `chart/title`. `frame.rs` : relation introuvable → `MissingPart` ; `Unsupported(n)` → `UnsupportedChart(n)` + `Opaque{Unknown(n)}` ; `Malformed` → `UnreadableItem`.

- [ ] **Step 1 : la fixture du graphique**

`fixtures/import/kitchen/ppt/charts/chart1.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<c:chart><c:title><c:tx><c:rich><a:bodyPr/><a:p><a:r><a:t>Ventes</a:t></a:r><a:r><a:t> 2026</a:t></a:r></a:p></c:rich></c:tx></c:title>
<c:plotArea><c:barChart><c:barDir val="col"/><c:grouping val="stacked"/>
<c:ser><c:idx val="0"/><c:tx><c:strRef><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>Nord</c:v></c:pt></c:strCache></c:strRef></c:tx>
<c:spPr><a:solidFill><a:srgbClr val="4472C4"/></a:solidFill></c:spPr>
<c:cat><c:strRef><c:strCache><c:ptCount val="3"/><c:pt idx="0"><c:v>T1</c:v></c:pt><c:pt idx="1"><c:v>T2</c:v></c:pt><c:pt idx="2"><c:v>T3</c:v></c:pt></c:strCache></c:strRef></c:cat>
<c:val><c:numRef><c:numCache><c:ptCount val="3"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="2"><c:v>12.5</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser>
<c:ser><c:idx val="1"/><c:tx><c:v>Sud</c:v></c:tx>
<c:val><c:numLit><c:ptCount val="3"/><c:pt idx="0"><c:v>4</c:v></c:pt><c:pt idx="1"><c:v>#N/A</c:v></c:pt><c:pt idx="2"><c:v>6</c:v></c:pt></c:numLit></c:val></c:ser>
</c:barChart><c:lineChart><c:grouping val="standard"/></c:lineChart></c:plotArea></c:chart></c:chartSpace>
```

`test_support.rs` : `pub(crate) const BAR: &str = include_str!("../../../../fixtures/import/kitchen/ppt/charts/chart1.xml");` ; ajouter à `SLIDE_RELS` la ligne `<Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>` et à la liste de `slide_pkg` la part `("ppt/charts/chart1.xml", BAR.as_bytes())`.

- [ ] **Step 2 : tests qui échouent**

`src/pptx/chart_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::pptx::test_support::{BAR, slide_pkg, with_ctx};
use crate::pptx::xml::parse;
use crate::source::{ChartKind, Grouping, Warning, WarningKind};

fn read(xml: &str) -> (Result<Chart, ChartSkip>, Vec<Warning>) {
    let t = slide_pkg(&[]);
    let pkg = t.pkg();
    let doc = parse(xml, "t", 100_000).unwrap();
    let (r, w, _) = with_ctx(&pkg, "ppt/slides/slide1.xml", |cx| parse_chart(doc.root_element(), cx, "s1/14"));
    (r, w)
}

fn chart(xml: &str) -> Chart {
    read(xml).0.unwrap_or_else(|e| panic!("graphique attendu : {e:?}"))
}

fn space(inner: &str) -> String {
    format!(r#"<c:chartSpace xmlns:c="urn:c" xmlns:a="urn:a"><c:chart><c:plotArea>{inner}</c:plotArea></c:chart></c:chartSpace>"#)
}

fn line_values(pt_count: &str, pts: &str) -> Vec<Option<f64>> {
    chart(&space(&format!(r#"<c:lineChart><c:ser><c:val><c:numLit><c:ptCount val="{pt_count}"/>{pts}</c:numLit></c:val></c:ser></c:lineChart>"#))).series[0].values.clone()
}

#[test]
fn bar_chart_reads_kind_grouping_and_title() {
    let c = chart(BAR);
    assert_that!((c.kind, c.grouping, c.title.as_deref())).is_equal_to((ChartKind::Column, Grouping::Stacked, Some("Ventes 2026")));
}

#[test]
fn series_read_cached_values_with_gaps_and_na() {
    let c = chart(BAR);
    assert_that!(c.categories.clone()).is_equal_to(vec!["T1".to_owned(), "T2".to_owned(), "T3".to_owned()]);
    assert_that!((c.series[0].name.as_str(), c.series[0].values.clone())).is_equal_to(("Nord", vec![Some(10.0), None, Some(12.5)]));
    assert_that!(c.series[0].color.as_ref().map(|c| c.value.to_hex())).is_equal_to(Some("#4472c4".to_owned()));
    assert_that!((c.series[1].name.as_str(), c.series[1].values.clone())).is_equal_to(("Sud", vec![Some(4.0), None, Some(6.0)]));
}

#[test]
fn combo_chart_keeps_first_type_and_warns_on_the_item() {
    let (_, w) = read(BAR);
    assert_that!(w.iter().any(|w| w.kind == WarningKind::UnsupportedChart("combo: barChart".into()) && w.item.as_deref() == Some("s1/14"))).is_true();
}

#[test]
fn horizontal_bars_and_default_grouping() {
    let c = chart(&space(r#"<c:barChart><c:barDir val="bar"/></c:barChart>"#));
    assert_that!((c.kind, c.grouping, chart(&space("<c:lineChart/>")).grouping)).is_equal_to((ChartKind::Bar, Grouping::Clustered, Grouping::Standard));
}

#[test]
fn scatter_reads_x_and_y_values() {
    let c = chart(&space(r#"<c:scatterChart><c:ser><c:tx><c:v>S</c:v></c:tx><c:xVal><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit></c:xVal><c:yVal><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>5</c:v></c:pt><c:pt idx="1"><c:v>7</c:v></c:pt></c:numLit></c:yVal></c:ser></c:scatterChart>"#));
    assert_that!((c.kind, c.categories.clone(), c.series[0].values.clone())).is_equal_to((ChartKind::Scatter, vec!["1".to_owned(), "2".to_owned()], vec![Some(5.0), Some(7.0)]));
}

#[test]
fn unreadable_point_count_follows_the_points_present() {
    assert_that!(line_values("9223372036854775807", r#"<c:pt idx="0"><c:v>1</c:v></c:pt>"#).len()).is_equal_to(1);
}

#[test]
fn huge_point_count_is_bounded_by_the_points_present() {
    // 1 point présent : au plus 1 + 64 cases, quel que soit ptCount.
    assert_that!(line_values("900000", r#"<c:pt idx="0"><c:v>1</c:v></c:pt>"#).len()).is_equal_to(1 + MAX_TRAILING_GAP);
}

#[test]
fn point_beyond_the_bounded_length_is_ignored() {
    let v = line_values("3", r#"<c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="20000"><c:v>9</c:v></c:pt>"#);
    assert_that!((v.len(), v[0], v.iter().flatten().count())).is_equal_to((2 + MAX_TRAILING_GAP, Some(1.0), 1));
}

#[test]
fn series_beyond_the_cap_are_dropped() {
    let sers = r#"<c:ser><c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>"#.repeat(MAX_SERIES + 10);
    assert_that!(chart(&space(&format!("<c:lineChart>{sers}</c:lineChart>"))).series.len()).is_equal_to(MAX_SERIES);
}

#[test]
fn unsupported_type_is_skipped_with_its_name() {
    assert_that!(read(&space("<c:bubbleChart/>")).0).is_equal_to(Err(ChartSkip::Unsupported("bubbleChart".to_owned())));
}

#[test]
fn chart_without_plot_area_is_malformed() {
    assert_that!(read(r#"<c:chartSpace xmlns:c="urn:c"/>"#).0).is_equal_to(Err(ChartSkip::Malformed("plotArea")));
}
```

Ajouter à `shapes_tests.rs` :

```rust
#[test]
fn chart_graphic_frame_reads_its_part() {
    let (items, _, _) = tree(r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="14" name="Graphique"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="urn:c" r:id="rId5"/></a:graphicData></a:graphic></p:graphicFrame>"#);
    let ItemKind::Chart(c) = &items[0].item.kind else { panic!("graphique attendu") };
    assert_that!(c.series.len()).is_equal_to(2);
}

#[test]
fn chart_with_unknown_relationship_warns_missing_part() {
    let (items, w, _) = tree(r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="14" name="Graphique"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="urn:c" r:id="rIdNone"/></a:graphicData></a:graphic></p:graphicFrame>"#);
    assert_that!((matches!(items[0].item.kind, ItemKind::Opaque(_)), w.iter().any(|w| matches!(w.kind, WarningKind::MissingPart(_))))).is_equal_to((true, true));
}
```

- [ ] **Step 3 : lancer, constater l'échec** — FAIL.

- [ ] **Step 4 : implémentation**

`src/pptx/chart.rs` :

```rust
//! Graphiques (`c:chartSpace`) : type, empilement, séries lues dans le cache (bornées).

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::color::parse_color;
use crate::pptx::context::PartCtx;
use crate::pptx::xml::{attr_str, attr_u32, child, children, descendant, elements, text_of};
use crate::source::{Chart, ChartKind, Grouping, Series, WarningKind};

/// Nombre maximal de points lus par cache.
pub(crate) const MAX_CHART_POINTS: usize = 10_000;
/// Nombre maximal de séries par graphique.
pub(crate) const MAX_SERIES: usize = 256;
/// Cases vides admises au-delà des points présents (fin de série sans valeur).
pub(crate) const MAX_TRAILING_GAP: usize = 64;

/// Pourquoi un graphique n'est pas lu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChartSkip {
    /// Type non géré (`bubbleChart`, `stockChart`, `surfaceChart`…).
    Unsupported(String),
    /// Structure absente (`plotArea`, aucun type).
    Malformed(&'static str),
}

/// Valeurs d'un cache, de longueur proportionnelle aux points présents.
fn cache(n: Node<'_, '_>) -> Vec<Option<String>> {
    let Some(c) = n.descendants().find(|d| matches!(d.tag_name().name(), "strCache" | "numCache" | "strLit" | "numLit")) else {
        return Vec::new();
    };
    let declared = child(c, "ptCount").and_then(|p| attr_u32(p, "val")).map_or(0, |v| usize::try_from(v).unwrap_or(usize::MAX));
    let pts: Vec<(usize, Option<String>)> = children(c, "pt")
        .take(MAX_CHART_POINTS)
        .filter_map(|pt| Some((usize::try_from(attr_u32(pt, "idx")?).ok()?, child(pt, "v").and_then(|v| v.text()).map(clip))))
        .collect();
    let present = pts.iter().map(|(i, _)| i.saturating_add(1)).max().unwrap_or(0);
    let len = declared.max(present).min(pts.len().saturating_add(MAX_TRAILING_GAP)).min(MAX_CHART_POINTS);
    let mut out = vec![None; len];
    for (i, v) in pts {
        if let Some(slot) = out.get_mut(i) {
            *slot = v;
        }
    }
    out
}

fn series_name(ser: Node<'_, '_>) -> String {
    let Some(tx) = child(ser, "tx") else { return String::new() };
    if let Some(v) = child(tx, "v") {
        return clip(v.text().unwrap_or_default());
    }
    clip(&cache(tx).into_iter().flatten().collect::<Vec<_>>().join(" "))
}

/// Lit un `c:chartSpace` ; `item` désigne le cadre dans les avertissements.
pub(crate) fn parse_chart(root: Node<'_, '_>, cx: &PartCtx<'_>, item: &str) -> Result<Chart, ChartSkip> {
    let plot = descendant(root, "plotArea").ok_or(ChartSkip::Malformed("plotArea"))?;
    let types: Vec<Node<'_, '_>> = elements(plot).filter(|n| n.tag_name().name().ends_with("Chart")).take(8).collect();
    let first = *types.first().ok_or(ChartSkip::Malformed("chart type"))?;
    let name = first.tag_name().name();
    if types.len() > 1 {
        cx.warn(Some(item), WarningKind::UnsupportedChart(clip(&format!("combo: {name}"))));
    }
    let kind = match name {
        "barChart" | "bar3DChart" => {
            if child(first, "barDir").and_then(|d| attr_str(d, "val")) == Some("bar") { ChartKind::Bar } else { ChartKind::Column }
        }
        "lineChart" | "line3DChart" => ChartKind::Line,
        "pieChart" | "pie3DChart" | "ofPieChart" => ChartKind::Pie,
        "doughnutChart" => ChartKind::Doughnut,
        "areaChart" | "area3DChart" => ChartKind::Area,
        "scatterChart" => ChartKind::Scatter,
        "radarChart" => ChartKind::Radar,
        other => return Err(ChartSkip::Unsupported(clip(other))),
    };
    let grouping = match child(first, "grouping").and_then(|g| attr_str(g, "val")) {
        Some("stacked") => Grouping::Stacked,
        Some("percentStacked") => Grouping::PercentStacked,
        Some("standard") => Grouping::Standard,
        Some("clustered") => Grouping::Clustered,
        _ if matches!(kind, ChartKind::Bar | ChartKind::Column) => Grouping::Clustered,
        _ => Grouping::Standard,
    };
    let sers: Vec<Node<'_, '_>> = children(first, "ser").take(MAX_SERIES).collect();
    let categories = sers
        .first()
        .and_then(|s| child(*s, "cat").or_else(|| child(*s, "xVal")))
        .map(|c| cache(c).into_iter().map(Option::unwrap_or_default).collect())
        .unwrap_or_default();
    let series = sers
        .iter()
        .map(|s| Series {
            name: series_name(*s),
            values: child(*s, "val")
                .or_else(|| child(*s, "yVal"))
                .map(|v| cache(v).into_iter().map(|x| x.and_then(|t| t.trim().parse::<f64>().ok().filter(|f| f.is_finite()))).collect())
                .unwrap_or_default(),
            color: child(*s, "spPr").and_then(|p| child(p, "solidFill")).and_then(|f| parse_color(f, &cx.colors)),
        })
        .collect();
    let title = descendant(root, "chart").and_then(|c| child(c, "title")).map(|t| clip(&text_of(t))).filter(|t| !t.is_empty());
    Ok(Chart { kind, grouping, title, categories, series })
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod chart_tests;
```

> `point_beyond_the_bounded_length_is_ignored` : 2 points présents (idx 0 et 20 000) → longueur `min(max(3, 20 001), 2 + 64, 10 000)` = 66 ; le point 20 000 tombe hors de la table.

`src/pptx/frame.rs` (réécrit ; ajout de la branche graphique) :

```rust
//! `graphicFrame` : tableau, graphique, SmartArt, objet OLE, sinon opaque.

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::chart::{ChartSkip, parse_chart};
use crate::pptx::context::PartCtx;
use crate::pptx::shapes::{GroupCtx, ParsedItem, identity, opaque};
use crate::pptx::table::parse_table;
use crate::pptx::xml::{attr_r, attr_str, child, descendant, parse};
use crate::source::{Frame, Item, ItemKind, OpaqueReason, WarningKind};

/// Lit un `graphicFrame`.
pub(crate) fn parse_graphic_frame(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    let (id, name, alt) = identity(n, cx)?;
    let frame = child(n, "xfrm").map_or(Frame::new(0.0, 0.0, 0.0, 0.0), |x| g.frame(x));
    let data = descendant(n, "graphicData")?;
    let uri = attr_str(data, "uri").unwrap_or_default();
    let unknown = || opaque(OpaqueReason::Unknown(clip(uri)), None);
    let kind = if uri.ends_with("/table") {
        child(data, "tbl").map_or_else(unknown, |t| ItemKind::Table(parse_table(t, cx)))
    } else if uri.ends_with("/chart") {
        chart_item(data, cx, &id)
    } else {
        unknown()
    };
    Some(ParsedItem { item: Item { id, name, alt, frame, z: 0, role: None, placeholder: None, kind }, ph: None })
}

fn chart_item(data: Node<'_, '_>, cx: &PartCtx<'_>, id: &str) -> ItemKind {
    let unknown = |what: &str| opaque(OpaqueReason::Unknown(clip(what)), None);
    let Some(rid) = child(data, "chart").and_then(|c| attr_r(c, "id")) else { return unknown("chart") };
    let Some(part) = cx.rels.target(rid).map(str::to_owned) else {
        cx.warn(Some(id), WarningKind::MissingPart(clip(rid)));
        return unknown("chart");
    };
    let Some(xml) = cx.part_text(&part, id) else { return unknown("chart") };
    let doc = match parse(&xml, &part, cx.nodes()) {
        Ok(d) => d,
        Err(e) if e.kind() == ImportErrorKind::TooLarge => {
            cx.fail(e);
            return unknown("chart");
        }
        Err(_) => {
            cx.warn(Some(id), WarningKind::UnreadableItem(clip(&part)));
            return unknown("chart");
        }
    };
    match parse_chart(doc.root_element(), cx, id) {
        Ok(c) => ItemKind::Chart(c),
        Err(ChartSkip::Unsupported(name)) => {
            cx.warn(Some(id), WarningKind::UnsupportedChart(name.clone()));
            unknown(&name)
        }
        Err(ChartSkip::Malformed(what)) => {
            cx.warn(Some(id), WarningKind::UnreadableItem(clip(&format!("{part}: {what}"))));
            unknown("chart")
        }
    }
}
```

`src/lib.rs` : retirer `// Retiré à la tâche 13 : …` et `#[allow(dead_code, …)]` au-dessus de `mod pptx;` (tout est désormais branché).

- [ ] **Step 5 : lancer, constater le succès** — PASS.

- [ ] **Step 6 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import fixtures/import/kitchen/ppt/charts && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : graphiques — type, empilement, séries du cache bornées (t-692)
oku cli task done --project slider --id t-718
```

---

### Task 14: SmartArt et objets OLE

> *Alignement sur l'exécution :* `smartart.rs` définit `parse_part(xml, part, cx, id) -> Option<Document>` (limite dépassée : `cx.fail`, sinon `UnreadableItem` et `None`), utilisée par `smartart` et `drawing_part`. Tests ajoutés : `chart_part_over_the_node_limit_is_fatal_not_swallowed` (Task 13, `shapes_tests.rs`) et `smartart_drawing_over_the_node_limit_is_fatal_not_swallowed` (Task 14).

**Files:**
- Create: `src/pptx/smartart.rs`, `src/pptx/smartart_tests.rs`, `fixtures/import/kitchen/ppt/diagrams/data1.xml`, `fixtures/import/kitchen/ppt/diagrams/drawing1.xml`
- Modify: `src/pptx/mod.rs` (`pub(crate) mod smartart;`), `src/pptx/context.rs` (`PartCtx::sub`), `src/pptx/frame.rs` (forme finale), `src/pptx/test_support.rs` (`DATA`, `DRAWING`, `SLIDE_RELS` et `slide_pkg` finaux)

**Interfaces:**
- Consumes : `PartCtx` (8), `GroupCtx`, `parse_tree`, `MAX_NESTING`, `blip_asset`, `opaque` (9), `Rels` (2), `chart_item`/`parse_table` (12, 13).
- Produces : `PartCtx::sub(&self, rels: Rc<Rels>, owner: String) -> PartCtx<'p>` ; `smartart::smartart(data: Node, cx: &PartCtx, id: &str, frame: Frame, g: &GroupCtx) -> ItemKind`.
- Règles : `uri` `/diagram` : si `g.in_drawing` ou `g.depth >= MAX_NESTING` → `Opaque{SmartArt}` + `NestingTooDeep(id)` **sans suivre de relation**. Sinon `relIds@r:dm` → données → `dataModelExt@relId` → relation de la slide vers le dessin → `spTree` lu avec `cx.sub(relations du dessin, id)` et `GroupCtx { sx, sy de g ; tx, ty = coin du cadre ; depth + 1 ; in_drawing: true }` → `Group`. Dessin introuvable → `Opaque{SmartArt}` + `MissingPart`. Ids des enfants : `<id du cadre>/<modelId>`. `uri` `/ole` : premier `pic` descendant → `blip` → `Opaque{Ole, fallback}`.

- [ ] **Step 1 : fixtures du SmartArt et support de test final**

`fixtures/import/kitchen/ppt/diagrams/data1.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:dsp="http://schemas.microsoft.com/office/drawing/2008/diagram"><dgm:ptLst/><dgm:extLst><a:ext uri="http://schemas.microsoft.com/office/drawing/2008/diagram"><dsp:dataModelExt relId="rId7" minVer="http://schemas.openxmlformats.org/drawingml/2006/diagram"/></a:ext></dgm:extLst></dgm:dataModel>
```

`fixtures/import/kitchen/ppt/diagrams/drawing1.xml` (comme PowerPoint : `cNvPr id="0"` partout, `modelId` distinct) :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<dsp:drawing xmlns:dsp="http://schemas.microsoft.com/office/drawing/2008/diagram" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><dsp:spTree><dsp:nvGrpSpPr><dsp:cNvPr id="0" name=""/><dsp:cNvGrpSpPr/></dsp:nvGrpSpPr><dsp:grpSpPr/>
<dsp:sp modelId="{A}"><dsp:nvSpPr><dsp:cNvPr id="0" name=""/><dsp:cNvSpPr/></dsp:nvSpPr><dsp:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1270000" cy="635000"/></a:xfrm><a:prstGeom prst="roundRect"/><a:solidFill><a:srgbClr val="4472C4"/></a:solidFill></dsp:spPr><dsp:txBody><a:bodyPr/><a:p><a:r><a:t>Étape 1</a:t></a:r></a:p></dsp:txBody></dsp:sp>
<dsp:sp modelId="{B}"><dsp:nvSpPr><dsp:cNvPr id="0" name=""/><dsp:cNvSpPr/></dsp:nvSpPr><dsp:spPr><a:xfrm><a:off x="1905000" y="0"/><a:ext cx="1270000" cy="635000"/></a:xfrm><a:prstGeom prst="roundRect"/><a:solidFill><a:srgbClr val="4472C4"/></a:solidFill></dsp:spPr><dsp:txBody><a:bodyPr/><a:p><a:r><a:t>Étape 2</a:t></a:r></a:p></dsp:txBody></dsp:sp>
</dsp:spTree></dsp:drawing>
```

`test_support.rs` — constantes et **forme finale** de `SLIDE_RELS` et `slide_pkg` (remplacent ceux des tâches 9 et 13) :

```rust
pub(crate) const DATA: &str = include_str!("../../../../fixtures/import/kitchen/ppt/diagrams/data1.xml");
pub(crate) const DRAWING: &str = include_str!("../../../../fixtures/import/kitchen/ppt/diagrams/drawing1.xml");

/// Relations de `ppt/slides/slide1.xml` des tests d'arbre de formes.
pub(crate) const SLIDE_RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/missing.png"/>
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.svg"/>
<Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
<Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData" Target="../diagrams/data1.xml"/>
<Relationship Id="rId7" Type="http://schemas.microsoft.com/office/2007/relationships/diagramDrawing" Target="../diagrams/drawing1.xml"/>
</Relationships>"#;

/// Paquet d'une slide de test : `SLIDE_RELS`, image PNG et SVG, graphique, SmartArt, plus `extra`.
pub(crate) fn slide_pkg(extra: &[(&str, &[u8])]) -> TestPkg {
    let mut parts: Vec<(&str, &[u8])> = vec![
        ("ppt/slides/_rels/slide1.xml.rels", SLIDE_RELS.as_bytes()),
        ("ppt/media/image1.png", PNG),
        ("ppt/media/image1.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
        ("ppt/charts/chart1.xml", BAR.as_bytes()),
        ("ppt/diagrams/data1.xml", DATA.as_bytes()),
        ("ppt/diagrams/drawing1.xml", DRAWING.as_bytes()),
    ];
    parts.extend_from_slice(extra);
    TestPkg::with(&parts)
}
```

- [ ] **Step 2 : tests qui échouent**

`src/pptx/smartart_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::pptx::test_support::{DATA, DRAWING, SLIDE_RELS, TestPkg, tree, tree_in};
use crate::source::{ItemKind, OpaqueReason, WarningKind};

/// Cadre SmartArt en (100, 100) pt, 250 × 50 pt.
const FRAME: &str = r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="15" name="Diagramme"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="1270000" y="1270000"/><a:ext cx="3175000" cy="635000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:relIds xmlns:dgm="urn:dgm" r:dm="rId6" r:lo="rId6" r:qs="rId6" r:cs="rId6"/></a:graphicData></a:graphic></p:graphicFrame>"#;

fn group_kids(items: &[crate::pptx::shapes::ParsedItem]) -> Vec<crate::source::Item> {
    match &items[0].item.kind {
        ItemKind::Group { items } => items.clone(),
        k => panic!("groupe attendu, trouvé {k:?}"),
    }
}

#[test]
fn smartart_becomes_a_group_identified_by_model_ids() {
    let (items, _, _) = tree(FRAME);
    assert_that!(group_kids(&items).iter().map(|k| k.id.clone()).collect::<Vec<_>>()).is_equal_to(vec!["s1/15/{A}".to_owned(), "s1/15/{B}".to_owned()]);
}

#[test]
fn smartart_shapes_are_placed_relative_to_the_frame() {
    let (items, _, _) = tree(FRAME);
    // 2ᵉ forme à 1 905 000 EMU = 150 pt du cadre (100, 100) → (250, 100).
    let k = &group_kids(&items)[1];
    assert_that!((k.frame.x, k.frame.y)).is_equal_to((250.0, 100.0));
}

#[test]
fn smartart_inside_a_scaled_group_keeps_the_group_scale() {
    // Groupe d'échelle 2 posé en (0,0) ; cadre à (50,50) pt dans le groupe → (100,100) ; la 2ᵉ forme à 150 pt × 2 = 300 pt du cadre.
    let frame = FRAME.replace(r#"<a:off x="1270000" y="1270000"/>"#, r#"<a:off x="635000" y="635000"/>"#);
    let g = format!(r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="30" name="G"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="25400000" cy="25400000"/><a:chOff x="0" y="0"/><a:chExt cx="12700000" cy="12700000"/></a:xfrm></p:grpSpPr>{frame}</p:grpSp>"#);
    let (items, _, _) = tree(&g);
    let ItemKind::Group { items: shapes } = &group_kids(&items)[0].kind else { panic!("SmartArt attendu") };
    assert_that!((shapes[1].frame.x, shapes[1].frame.w)).is_equal_to((400.0, 200.0));
}

#[test]
fn smartart_with_unknown_relationship_is_opaque_and_warns() {
    let (items, w, _) = tree(&FRAME.replace(r#"r:dm="rId6""#, r#"r:dm="rIdNone""#));
    assert_that!(matches!(&items[0].item.kind, ItemKind::Opaque(o) if o.reason == OpaqueReason::SmartArt)).is_true();
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::MissingPart(_)))).is_true();
}

#[test]
fn smartart_whose_drawing_file_is_absent_warns_missing_part() {
    let t = TestPkg::with(&[("ppt/slides/_rels/slide1.xml.rels", SLIDE_RELS.as_bytes()), ("ppt/diagrams/data1.xml", DATA.as_bytes())]);
    let (items, w, _) = tree_in(&t, FRAME);
    assert_that!(matches!(&items[0].item.kind, ItemKind::Opaque(o) if o.reason == OpaqueReason::SmartArt)).is_true();
    assert_that!(w.iter().any(|w| matches!(&w.kind, WarningKind::MissingPart(p) if p.ends_with("drawing1.xml")))).is_true();
}

#[test]
fn smartart_inside_its_own_drawing_is_not_followed() {
    // Fichier hostile : le dessin contient lui-même un cadre SmartArt qui pointe vers ce même dessin.
    let looping = DRAWING.replace("</dsp:spTree>", &format!("{}</dsp:spTree>", FRAME.replace("<p:", "<dsp:").replace("</p:", "</dsp:")));
    let t = TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", SLIDE_RELS.as_bytes()),
        ("ppt/diagrams/data1.xml", DATA.as_bytes()),
        ("ppt/diagrams/drawing1.xml", looping.as_bytes()),
    ]);
    let (items, w, _) = tree_in(&t, FRAME);
    assert_that!(group_kids(&items).iter().any(|k| matches!(&k.kind, ItemKind::Opaque(o) if o.reason == OpaqueReason::SmartArt))).is_true();
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::NestingTooDeep(_)))).is_true();
}

#[test]
fn ole_object_is_opaque_with_its_picture() {
    let (items, _, assets) = tree(r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="16" name="Objet"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/ole"><p:oleObj r:id="rIdX" progId="Excel.Sheet.12"><p:embed/><p:pic><p:nvPicPr><p:cNvPr id="0" name=""/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/></p:blipFill><p:spPr/></p:pic></p:oleObj></a:graphicData></a:graphic></p:graphicFrame>"#);
    let ItemKind::Opaque(o) = &items[0].item.kind else { panic!("opaque attendu") };
    assert_that!((o.reason.clone(), o.fallback.as_ref().is_some_and(|a| assets.get(a).is_some_and(|x| x.mime == "image/png")))).is_equal_to((OpaqueReason::Ole, true));
}
```

> `smartart_inside_a_scaled_group…` : groupe 2×2 (`ext` = 2 × `chExt`) ; cadre en (50, 50) pt dans le groupe → page (100, 100) ; dessin à l'échelle 2 : x = 100 + 150 × 2 = 400, w = 100 × 2 = 200.

- [ ] **Step 3 : lancer, constater l'échec** — FAIL.

- [ ] **Step 4 : implémentation**

Dans `context.rs`, ajouter à `impl<'p> PartCtx<'p>` :

```rust
    /// Contexte d'une part rattachée (dessin de SmartArt) : même deck, slide, couleurs et thème ; ids neufs, sans héritage.
    pub(crate) fn sub(&self, rels: Rc<Rels>, owner: String) -> PartCtx<'p> {
        let spec = PartSpec { owner, slide: self.slide, colors: self.colors.clone(), theme: self.theme, inherit: None };
        Self::with_rels(self.shared, rels, spec)
    }
```

`src/pptx/smartart.rs` :

```rust
//! SmartArt : sa version déjà dessinée (`diagramDrawing`), lue comme un groupe de formes.

use std::rc::Rc;

use roxmltree::{Document, Node};

use crate::ImportErrorKind;
use crate::collect::clip;
use crate::pptx::context::PartCtx;
use crate::pptx::package::Rels;
use crate::pptx::shapes::{GroupCtx, MAX_NESTING, opaque, parse_tree};
use crate::pptx::xml::{attr_r, attr_str, child, descendant, parse};
use crate::source::{Frame, Item, ItemKind, OpaqueReason, WarningKind};

/// Analyse une part ; une limite dépassée est fatale, toute autre erreur rend `None` avec `UnreadableItem`.
fn parse_part<'a>(xml: &'a str, part: &str, cx: &PartCtx<'_>, id: &str) -> Option<Document<'a>> {
    match parse(xml, part, cx.nodes()) {
        Ok(d) => Some(d),
        Err(e) if e.kind() == ImportErrorKind::TooLarge => {
            cx.fail(e);
            None
        }
        Err(_) => {
            cx.warn(Some(id), WarningKind::UnreadableItem(clip(part)));
            None
        }
    }
}

/// Part du dessin d'un SmartArt, via sa part de données ; `None` si une étape manque.
fn drawing_part(data: Node<'_, '_>, cx: &PartCtx<'_>, id: &str) -> Option<String> {
    let dm = child(data, "relIds").and_then(|r| attr_r(r, "dm")).and_then(|r| cx.rels.target(r))?.to_owned();
    let xml = cx.part_text(&dm, id)?;
    let doc = parse_part(&xml, &dm, cx, id)?;
    let rel = descendant(doc.root_element(), "dataModelExt").and_then(|e| attr_str(e, "relId"))?;
    cx.rels.target(rel).map(str::to_owned)
}

/// Le SmartArt du cadre `id` : un groupe de formes, ou un opaque.
pub(crate) fn smartart(data: Node<'_, '_>, cx: &PartCtx<'_>, id: &str, frame: Frame, g: &GroupCtx) -> ItemKind {
    let not_drawn = || opaque(OpaqueReason::SmartArt, None);
    if g.in_drawing || g.depth >= MAX_NESTING {
        cx.warn(Some(id), WarningKind::NestingTooDeep(clip(id)));
        return not_drawn();
    }
    let Some(part) = drawing_part(data, cx, id) else {
        if !cx.shared.sink.borrow().has_fatal() {
            cx.warn(Some(id), WarningKind::MissingPart("diagramDrawing".into()));
        }
        return not_drawn();
    };
    let Some(xml) = cx.part_text(&part, id) else { return not_drawn() };
    let Some(doc) = parse_part(&xml, &part, cx, id) else { return not_drawn() };
    let Some(tree) = descendant(doc.root_element(), "spTree") else { return not_drawn() };
    let rels = match cx.pkg().rels(&part) {
        Ok(r) => r,
        Err(e) if e.kind() == ImportErrorKind::TooLarge => {
            cx.fail(e);
            return not_drawn();
        }
        Err(_) => Rc::new(Rels::default()),
    };
    let sub = cx.sub(rels, id.to_owned());
    let inner = GroupCtx {
        sx: g.sx,
        sy: g.sy,
        tx: frame.x,
        ty: frame.y,
        fill: None,
        depth: g.depth.saturating_add(1),
        in_drawing: true,
    };
    let items: Vec<Item> = parse_tree(tree, &sub, &inner).into_iter().map(|p| p.item).collect();
    if items.is_empty() { not_drawn() } else { ItemKind::Group { items } }
}

#[cfg(test)]
#[path = "smartart_tests.rs"]
mod smartart_tests;
```

`src/pptx/frame.rs` — **forme finale** :

```rust
//! `graphicFrame` : tableau, graphique, SmartArt, objet OLE, sinon opaque.

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::chart::{ChartSkip, parse_chart};
use crate::pptx::context::PartCtx;
use crate::pptx::shapes::{GroupCtx, ParsedItem, blip_asset, identity, opaque};
use crate::pptx::smartart::smartart;
use crate::pptx::table::parse_table;
use crate::pptx::xml::{attr_r, attr_str, child, descendant, parse};
use crate::source::{Frame, Item, ItemKind, OpaqueReason, WarningKind};

/// Lit un `graphicFrame`.
pub(crate) fn parse_graphic_frame(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    let (id, name, alt) = identity(n, cx)?;
    let frame = child(n, "xfrm").map_or(Frame::new(0.0, 0.0, 0.0, 0.0), |x| g.frame(x));
    let data = descendant(n, "graphicData")?;
    let uri = attr_str(data, "uri").unwrap_or_default();
    let unknown = || opaque(OpaqueReason::Unknown(clip(uri)), None);
    let kind = if uri.ends_with("/table") {
        child(data, "tbl").map_or_else(unknown, |t| ItemKind::Table(parse_table(t, cx)))
    } else if uri.ends_with("/chart") {
        chart_item(data, cx, &id)
    } else if uri.ends_with("/diagram") {
        smartart(data, cx, &id, frame, g)
    } else if uri.ends_with("/ole") {
        let fallback = descendant(data, "pic").and_then(|p| descendant(p, "blip")).and_then(|b| blip_asset(b, cx));
        opaque(OpaqueReason::Ole, fallback)
    } else {
        unknown()
    };
    Some(ParsedItem { item: Item { id, name, alt, frame, z: 0, role: None, placeholder: None, kind }, ph: None })
}

fn chart_item(data: Node<'_, '_>, cx: &PartCtx<'_>, id: &str) -> ItemKind {
    let unknown = |what: &str| opaque(OpaqueReason::Unknown(clip(what)), None);
    let Some(rid) = child(data, "chart").and_then(|c| attr_r(c, "id")) else { return unknown("chart") };
    let Some(part) = cx.rels.target(rid).map(str::to_owned) else {
        cx.warn(Some(id), WarningKind::MissingPart(clip(rid)));
        return unknown("chart");
    };
    let Some(xml) = cx.part_text(&part, id) else { return unknown("chart") };
    let doc = match parse(&xml, &part, cx.nodes()) {
        Ok(d) => d,
        Err(e) if e.kind() == ImportErrorKind::TooLarge => {
            cx.fail(e);
            return unknown("chart");
        }
        Err(_) => {
            cx.warn(Some(id), WarningKind::UnreadableItem(clip(&part)));
            return unknown("chart");
        }
    };
    match parse_chart(doc.root_element(), cx, id) {
        Ok(c) => ItemKind::Chart(c),
        Err(ChartSkip::Unsupported(name)) => {
            cx.warn(Some(id), WarningKind::UnsupportedChart(name.clone()));
            unknown(&name)
        }
        Err(ChartSkip::Malformed(what)) => {
            cx.warn(Some(id), WarningKind::UnreadableItem(clip(&format!("{part}: {what}"))));
            unknown("chart")
        }
    }
}
```

- [ ] **Step 5 : lancer, constater le succès** — PASS.

- [ ] **Step 6 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import fixtures/import/kitchen/ppt/diagrams && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : SmartArt dessiné en groupe (sans cycle), objets OLE opaques avec repli (t-692)
oku cli task done --project slider --id t-719
```

---

### Task 15: Polices utilisées et embarquées (sfnt, EOT, ODTTF, droits)

**Files:**
- Create: `src/pptx/fonts.rs` (+`_tests`), `src/pptx/eot.rs` (+`_tests`)
- Modify: `src/pptx/mod.rs` (`pub(crate) mod fonts; pub(crate) mod eot;` ; `read_pptx` remplit `fonts`), `tests/read.rs`, `tests/alto.rs`, `tests/snapshots/read__minimal.snap`

**Interfaces:**
- Produces :
  - `eot::decode(data) -> Option<Vec<u8>>` (en-tête : `EOTSize` u32 LE à 0, `FontDataSize` à 4, `Flags` à 12, `MagicNumber` 0x504C u16 LE à 34 ; données = les `FontDataSize` derniers des `EOTSize` premiers octets ; `0x4` MicroType → `None` ; `0x1000_0000` → XOR 0x50) ; `eot::{COMPRESSED, XOR}`.
  - `fonts::is_sfnt(data) -> bool` ; `fonts::fs_type(sfnt) -> Option<u16>` (table `OS/2`, champ `fsType`) ; `fonts::deobfuscate(data: &mut [u8], part) -> bool` (ODTTF) ; `fonts::used(masters, layouts, slides) -> BTreeSet<String>` ; `fonts::collect(pres: Node, rels: &Rels, pkg: &Package, used: BTreeSet<String>, sink: &RefCell<Sink>) -> Result<Vec<SourceFont>, ImportError>`.
- Règles de `collect` : familles utilisées **ou** embarquées, triées ; faces `regular/bold/italic/boldItalic` ; chaque part lue une fois (cache par chemin) ; sfnt direct, sinon EOT puis sfnt, sinon ODTTF puis sfnt, sinon `FontEmbeddedUnreadable(famille)` ; part absente → `MissingPart` ; `TooLarge` → erreur ; `subset` = `presentation@saveSubsetFonts` ; `fs_type` lu dans la face.

- [ ] **Step 1 : tests qui échouent**

`src/pptx/eot_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;

const HEADER: usize = 82;

fn sfnt() -> Vec<u8> {
    let mut v = vec![0x00, 0x01, 0x00, 0x00];
    v.extend(0u8..60);
    v
}

/// Conteneur EOT minimal autour de `font`.
fn eot(font: &[u8], flags: u32) -> Vec<u8> {
    let mut v = vec![0u8; HEADER];
    v[0..4].copy_from_slice(&u32::try_from(HEADER + font.len()).unwrap().to_le_bytes());
    v[4..8].copy_from_slice(&u32::try_from(font.len()).unwrap().to_le_bytes());
    v[8..12].copy_from_slice(&0x0002_0001u32.to_le_bytes());
    v[12..16].copy_from_slice(&flags.to_le_bytes());
    v[34..36].copy_from_slice(&0x504Cu16.to_le_bytes());
    if flags & XOR != 0 { v.extend(font.iter().map(|b| b ^ 0x50)) } else { v.extend_from_slice(font) }
    v
}

#[test]
fn decode_plain_eot_returns_the_font() {
    assert_that!(decode(&eot(&sfnt(), 0))).is_equal_to(Some(sfnt()));
}

#[test]
fn decode_xor_eot_undoes_the_xor() {
    assert_that!(decode(&eot(&sfnt(), XOR))).is_equal_to(Some(sfnt()));
}

#[test]
fn decode_compressed_eot_is_none() {
    assert_that!(decode(&eot(&sfnt(), COMPRESSED))).is_none();
}

#[test]
fn decode_without_magic_is_none() {
    let mut v = eot(&sfnt(), 0);
    v[34] = 0;
    assert_that!(decode(&v)).is_none();
}

#[test]
fn decode_with_inconsistent_sizes_is_none() {
    let mut v = eot(&sfnt(), 0);
    v[0..4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_that!((decode(&v), decode(&v[..20]))).is_equal_to((None, None));
}
```

`src/pptx/fonts_tests.rs` :

```rust
use std::cell::RefCell;

use speculoos::prelude::*;

use super::*;
use crate::collect::Sink;
use crate::pptx::test_support::TestPkg;
use crate::pptx::xml::parse;
use crate::source::{FaceStyle, WarningKind};

const GUID_PART: &str = "ppt/fonts/{2C7F3E6A-1B2D-4C5E-8F90-A1B2C3D4E5F6}.odttf";

fn sfnt() -> Vec<u8> {
    let mut v = vec![0x00, 0x01, 0x00, 0x00];
    v.extend(0u8..60);
    v
}

/// sfnt minimal avec une table `OS/2` dont `fsType` = `fs`.
fn sfnt_with_os2(fs: u16) -> Vec<u8> {
    let mut v = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    v.extend_from_slice(b"OS/2");
    v.extend_from_slice(&[0, 0, 0, 0]); // somme de contrôle
    v.extend_from_slice(&28u32.to_be_bytes());
    v.extend_from_slice(&10u32.to_be_bytes());
    v.extend_from_slice(&[0; 8]);
    v.extend_from_slice(&fs.to_be_bytes());
    v
}

#[test]
fn is_sfnt_recognizes_truetype_and_opentype_only() {
    assert_that!((is_sfnt(&sfnt()), is_sfnt(b"OTTO...."), is_sfnt(b"LP\x00\x00eot"))).is_equal_to((true, true, false));
}

#[test]
fn fs_type_reads_the_os2_embedding_rights() {
    assert_that!((fs_type(&sfnt_with_os2(0x0004)), fs_type(&sfnt()))).is_equal_to((Some(0x0004), None));
}

#[test]
fn deobfuscate_uses_the_reversed_guid_bytes_as_key() {
    // Clé : GUID à l'envers → F6 E5 D4 C3 B2 A1 90 8F 5E 4C 2D 1B 6A 3E 7F 2C, répétée.
    let mut data = sfnt();
    assert_that!(deobfuscate(&mut data, GUID_PART)).is_true();
    // octets 0..4 : 00 01 00 00 ⊕ F6 E5 D4 C3 ; octet 16 (= 12) ⊕ F6 (la clé recommence).
    assert_that!((data[..4].to_vec(), data[16])).is_equal_to((vec![0xF6, 0xE4, 0xD4, 0xC3], 12 ^ 0xF6));
}

#[test]
fn deobfuscate_only_touches_the_first_32_bytes() {
    let clear = sfnt();
    let mut data = clear.clone();
    deobfuscate(&mut data, GUID_PART);
    assert_that!(data[32..].to_vec()).is_equal_to(clear[32..].to_vec());
}

#[test]
fn deobfuscate_without_guid_is_false() {
    assert_that!(deobfuscate(&mut sfnt(), "ppt/fonts/font1.fntdata")).is_false();
}

const PRES: &str = r#"<p:presentation xmlns:p="urn:p" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" saveSubsetFonts="1"><p:embeddedFontLst>
<p:embeddedFont><p:font typeface="Inter"/><p:regular r:id="rId1"/><p:bold r:id="rId2"/><p:italic r:id="rId3"/><p:boldItalic r:id="rId9"/></p:embeddedFont>
<p:embeddedFont><p:font typeface="Inter Copy"/><p:regular r:id="rId1"/></p:embeddedFont>
</p:embeddedFontLst></p:presentation>"#;
const RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/font" Target="fonts/font1.fntdata"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/font" Target="fonts/font2.fntdata"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/font" Target="fonts/font3.fntdata"/>
<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/font" Target="fonts/missing.fntdata"/>
</Relationships>"#;

fn eot_xor(font: &[u8], flags: u32) -> Vec<u8> {
    let mut v = vec![0u8; 82];
    v[0..4].copy_from_slice(&u32::try_from(82 + font.len()).unwrap().to_le_bytes());
    v[4..8].copy_from_slice(&u32::try_from(font.len()).unwrap().to_le_bytes());
    v[12..16].copy_from_slice(&flags.to_le_bytes());
    v[34..36].copy_from_slice(&0x504Cu16.to_le_bytes());
    v.extend(font.iter().map(|b| b ^ 0x50));
    v
}

#[test]
fn collect_reads_eot_and_plain_faces_and_reports_unreadable_and_missing_ones() {
    let regular = eot_xor(&sfnt_with_os2(0x0008), crate::pptx::eot::XOR);
    let compressed = eot_xor(&sfnt(), crate::pptx::eot::XOR | crate::pptx::eot::COMPRESSED);
    let t = TestPkg::with(&[
        ("ppt/_rels/presentation.xml.rels", RELS.as_bytes()),
        ("ppt/fonts/font1.fntdata", regular.as_slice()),
        ("ppt/fonts/font2.fntdata", sfnt().as_slice()),
        ("ppt/fonts/font3.fntdata", compressed.as_slice()),
    ]);
    let pkg = t.pkg();
    let rels = pkg.rels("ppt/presentation.xml").unwrap();
    let doc = parse(PRES, "p", 1000).unwrap();
    let sink = RefCell::new(Sink::new());
    let fonts = collect(doc.root_element(), &rels, &pkg, BTreeSet::from(["Calibri".to_owned()]), &sink).unwrap();
    assert_that!(fonts.iter().map(|f| f.family.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["Calibri", "Inter", "Inter Copy"]);
    let inter = &fonts[1];
    assert_that!(inter.faces.iter().map(|f| (f.style, f.fs_type, f.subset)).collect::<Vec<_>>())
        .is_equal_to(vec![(FaceStyle::Regular, Some(0x0008), true), (FaceStyle::Bold, None, true)]);
    let (_, w) = sink.into_inner().finish().unwrap();
    let kinds: Vec<_> = w.into_iter().map(|w| w.kind).collect();
    assert_that!(kinds.contains(&WarningKind::FontEmbeddedUnreadable("Inter".into()))).is_true();
    assert_that!(kinds.iter().filter(|k| matches!(k, WarningKind::MissingPart(p) if p.ends_with("missing.fntdata"))).count()).is_equal_to(1);
}

#[test]
fn collect_counts_copies_of_a_shared_part_against_the_total_budget() {
    // `Inter` et `Inter Copy` partagent font1 : lue une fois (dans le budget), copiée deux fois (hors budget).
    let mut big = sfnt();
    big.resize(4000, 0);
    let t = TestPkg::with(&[("ppt/_rels/presentation.xml.rels", RELS.as_bytes()), ("ppt/fonts/font1.fntdata", big.as_slice())]);
    let budget = (crate::pptx::test_support::CT.len() + crate::pptx::test_support::ROOT_RELS.len() + RELS.len() + 4100) as u64;
    let pkg = Package::open(t.bytes(), &crate::ReadLimits { max_total_bytes: budget, ..crate::pptx::test_support::LIMITS }).unwrap();
    let rels = pkg.rels("ppt/presentation.xml").unwrap();
    let doc = parse(PRES, "p", 1000).unwrap();
    let sink = RefCell::new(Sink::new());
    let r = collect(doc.root_element(), &rels, &pkg, BTreeSet::new(), &sink);
    assert_that!(r.map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
}
```

Ajouter à `tests/read.rs` :

```rust
#[test]
fn minimal_lists_its_font_families() {
    let fams: Vec<String> = minimal().fonts.into_iter().map(|f| f.family).collect();
    // Arial : police de la puce « • » du bodyStyle du master.
    assert_that!(fams).is_equal_to(vec!["Arial".to_owned(), "Calibri".to_owned(), "Calibri Light".to_owned()]);
}
```

et à `tests/alto.rs` :

```rust
#[test]
fn alto_uses_inter_without_embedding_it() {
    let d = alto();
    let inter = d.fonts.iter().find(|f| f.family == "Inter").unwrap_or_else(|| panic!("Inter attendue"));
    assert_that!(inter.faces.is_empty()).is_true();
}
```

- [ ] **Step 2 : lancer, constater l'échec** — FAIL.

- [ ] **Step 3 : implémentation**

`src/pptx/eot.rs` :

```rust
//! Conteneur Embedded OpenType (EOT), format des `.fntdata` de PowerPoint.
//!
//! Seules les données non compressées sont lues ; le MicroType (compression
//! MTX) n'est pas décodé : la police est alors signalée illisible.

const MAGIC: u16 = 0x504C;
/// `TTEMBED_TTCOMPRESSED` : données compressées en MicroType.
pub(crate) const COMPRESSED: u32 = 0x0000_0004;
/// `TTEMBED_XORENCRYPTDATA` : données XOR 0x50.
pub(crate) const XOR: u32 = 0x1000_0000;

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..at.checked_add(4)?)?.try_into().ok().map(u32::from_le_bytes)
}

/// Police (sfnt) d'un conteneur EOT, ou `None` s'il est compressé ou incohérent.
pub(crate) fn decode(data: &[u8]) -> Option<Vec<u8>> {
    let eot_size = usize::try_from(u32_at(data, 0)?).ok()?;
    let font_size = usize::try_from(u32_at(data, 4)?).ok()?;
    let flags = u32_at(data, 12)?;
    let magic = data.get(34..36)?.try_into().ok().map(u16::from_le_bytes)?;
    if magic != MAGIC || eot_size > data.len() || font_size > eot_size || flags & COMPRESSED != 0 {
        return None;
    }
    let font = data.get(eot_size - font_size..eot_size)?;
    Some(if flags & XOR != 0 { font.iter().map(|b| b ^ 0x50).collect() } else { font.to_vec() })
}

#[cfg(test)]
#[path = "eot_tests.rs"]
mod eot_tests;
```

`src/pptx/fonts.rs` :

```rust
//! Polices : familles utilisées par le texte, faces embarquées lisibles et leurs droits.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use roxmltree::Node;

use crate::ImportErrorKind;
use crate::collect::{Sink, clip};
use crate::error::ImportError;
use crate::pptx::eot;
use crate::pptx::package::{Package, Rels};
use crate::pptx::theme::MAX_FONT_NAME;
use crate::pptx::xml::{attr_bool, attr_r, attr_str, child, children};
use crate::source::{Bullet, FaceStyle, FontFace, Item, ItemKind, Layout, Master, ParaStyle, Paragraph, Placeholder, SourceFont, SourceSlide, TextBox, WarningKind};

const FACES: [(&str, FaceStyle); 4] =
    [("regular", FaceStyle::Regular), ("bold", FaceStyle::Bold), ("italic", FaceStyle::Italic), ("boldItalic", FaceStyle::BoldItalic)];

fn bullet_font(p: &ParaStyle, out: &mut BTreeSet<String>) {
    if let Bullet::Char { font: Some(f), .. } = &p.bullet {
        out.insert(f.clone());
    }
}

fn paragraphs(ps: &[Paragraph], out: &mut BTreeSet<String>) {
    for p in ps {
        out.extend(p.runs.iter().map(|r| r.style.font.value.clone()));
        bullet_font(&p.style, out);
    }
}

fn textbox(tb: &TextBox, out: &mut BTreeSet<String>) {
    paragraphs(&tb.paragraphs, out);
}

fn items(list: &[Item], out: &mut BTreeSet<String>) {
    for it in list {
        match &it.kind {
            ItemKind::Text(tb) => textbox(tb, out),
            ItemKind::Shape(s) => s.text.iter().for_each(|tb| textbox(tb, out)),
            ItemKind::Table(t) => t.rows.iter().flatten().for_each(|c| textbox(&c.text, out)),
            ItemKind::Group { items: kids } => items(kids, out),
            ItemKind::Picture(_) | ItemKind::Chart(_) | ItemKind::Opaque(_) => {}
        }
    }
}

fn placeholders(phs: &[Placeholder], out: &mut BTreeSet<String>) {
    for p in phs {
        for l in &p.levels {
            out.insert(l.run.font.value.clone());
            bullet_font(&l.para, out);
        }
        textbox(&p.text, out);
    }
}

/// Familles utilisées par tout le texte du deck.
pub(crate) fn used(masters: &[Master], layouts: &[Layout], slides: &[SourceSlide]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for m in masters {
        items(&m.decor, &mut out);
        placeholders(&m.placeholders, &mut out);
    }
    for l in layouts {
        items(&l.decor, &mut out);
        placeholders(&l.placeholders, &mut out);
    }
    for s in slides {
        items(&s.items, &mut out);
    }
    out.retain(|f| !f.is_empty());
    out
}

/// Vrai pour une police sfnt (TrueType, OpenType, collection).
pub(crate) fn is_sfnt(data: &[u8]) -> bool {
    matches!(data.get(..4), Some(b"\x00\x01\x00\x00" | b"OTTO" | b"true" | b"ttcf"))
}

/// Droits d'incorporation (`fsType` de la table `OS/2`) d'une police sfnt.
pub(crate) fn fs_type(sfnt: &[u8]) -> Option<u16> {
    let be16 = |o: usize| sfnt.get(o..o.checked_add(2)?).and_then(|b| b.try_into().ok()).map(u16::from_be_bytes);
    let be32 = |o: usize| sfnt.get(o..o.checked_add(4)?).and_then(|b| b.try_into().ok()).map(u32::from_be_bytes);
    let tables = usize::from(be16(4)?);
    (0..tables.min(256)).find_map(|i| {
        let rec = 12 + 16 * i;
        (sfnt.get(rec..rec + 4)? == b"OS/2").then(|| be16(usize::try_from(be32(rec + 8)?).ok()?.checked_add(8)?)).flatten()
    })
}

/// Obfuscation ODTTF : XOR des 32 premiers octets avec les 16 octets du GUID du nom de part, dans l'ordre inverse.
pub(crate) fn deobfuscate(data: &mut [u8], part: &str) -> bool {
    let name = part.rsplit('/').next().unwrap_or(part);
    let digits: Vec<u8> =
        name.split('.').next().unwrap_or_default().chars().filter_map(|c| c.to_digit(16)).filter_map(|d| u8::try_from(d).ok()).collect();
    let Ok(digits32) = <[u8; 32]>::try_from(digits.as_slice()) else { return false };
    let key: Vec<u8> = digits32.as_chunks::<2>().0.iter().rev().map(|[hi, lo]| (hi << 4) | lo).collect();
    for (b, k) in data.iter_mut().take(32).zip(key.iter().cycle()) {
        *b ^= k;
    }
    true
}

fn readable(mut data: Vec<u8>, part: &str) -> Option<Vec<u8>> {
    if is_sfnt(&data) {
        return Some(data);
    }
    if let Some(font) = eot::decode(&data).filter(|f| is_sfnt(f)) {
        return Some(font);
    }
    (deobfuscate(&mut data, part) && is_sfnt(&data)).then_some(data)
}

/// Familles utilisées et faces embarquées, triées par famille.
pub(crate) fn collect(pres: Node<'_, '_>, rels: &Rels, pkg: &Package<'_>, used: BTreeSet<String>, sink: &RefCell<Sink>) -> Result<Vec<SourceFont>, ImportError> {
    let subset = attr_bool(pres, "saveSubsetFonts").unwrap_or(false);
    let mut faces: BTreeMap<String, Vec<FontFace>> = used.into_iter().map(|f| (f, Vec::new())).collect();
    let mut read: BTreeMap<String, Option<Vec<u8>>> = BTreeMap::new();
    // Une part partagée par plusieurs faces est copiée dans chacune : les copies comptent dans le budget total.
    let mut copied: u64 = 0;
    for ef in child(pres, "embeddedFontLst").into_iter().flat_map(|l| children(l, "embeddedFont")).take(64) {
        let Some(family) = child(ef, "font").and_then(|f| attr_str(f, "typeface")).map(|t| crate::collect::clip_to(t, MAX_FONT_NAME)) else { continue };
        for (tag, style) in FACES {
            let Some(part) = child(ef, tag).and_then(|n| attr_r(n, "id")).and_then(|r| rels.target(r)) else { continue };
            if !read.contains_key(part) {
                let decoded = match pkg.bytes(part) {
                    Ok(d) => readable(d, part),
                    Err(e) if e.kind() == ImportErrorKind::TooLarge => return Err(e),
                    Err(_) => {
                        sink.borrow_mut().warn(None, None, WarningKind::MissingPart(clip(part)));
                        read.insert(part.to_owned(), None);
                        continue;
                    }
                };
                if decoded.is_none() {
                    sink.borrow_mut().warn(None, None, WarningKind::FontEmbeddedUnreadable(family.clone()));
                }
                read.insert(part.to_owned(), decoded);
            }
            if let Some(Some(data)) = read.get(part) {
                copied = copied.saturating_add(u64::try_from(data.len()).unwrap_or(u64::MAX));
                if copied > pkg.limits().max_total_bytes {
                    return Err(crate::error::too_large("fonts"));
                }
                faces.entry(family.clone()).or_default().push(FontFace { style, fs_type: fs_type(data), subset, data: data.clone() });
            }
        }
        faces.entry(family).or_default();
    }
    Ok(faces.into_iter().map(|(family, faces)| SourceFont { family, faces }).collect())
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod fonts_tests;
```

> `collect_reads…` : `Inter` et `Inter Copy` partagent `font1.fntdata` : la part n'est lue qu'une fois (cache `read`), une part manquante n'est signalée qu'une fois. Chaque face en garde sa copie : la somme des copies est bornée par `max_total_bytes` (`collect_counts_copies…`), sans quoi 64 polices × 4 faces sur une part de 64 Mio pèseraient 16 Gio.

Dans `read_pptx`, remplacer la fin à partir de `let themes_out = …` par :

```rust
    let themes_out = themes.iter().map(|t| t.theme.clone()).collect();
    let masters_out: Vec<_> = masters.into_iter().map(|m| m.master).collect();
    let layouts_out: Vec<_> = layouts.into_iter().map(|(_, _, l)| l.layout).collect();
    let used = fonts::used(&masters_out, &layouts_out, &slides);
    let fonts = fonts::collect(root, &rels, &pkg, used, &shared.sink)?;
    let (assets, warnings) = shared.sink.into_inner().finish()?;
    Ok(SourceDeck { size, themes: themes_out, masters: masters_out, layouts: layouts_out, slides, assets, fonts, warnings })
```

- [ ] **Step 4 : lancer ; relire puis accepter le snapshot**

Run: `cargo nextest run -p oku-slides-import` → tout PASS sauf `minimal_snapshot_matches`.
Run: `INSTA_UPDATE=new cargo nextest run -p oku-slides-import --test read minimal_snapshot_matches` ; `diff` entre `.snap` et `.snap.new` : **seule la liste `fonts` change** (trois familles, `faces` vides). Puis `cargo insta accept --manifest-path crates/oku-slides-import/Cargo.toml`.

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings
git add crates/oku-slides-import && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : polices utilisées et embarquées — sfnt, EOT, ODTTF, droits d'incorporation (t-692)
oku cli task done --project slider --id t-720
```

---

### Task 16: Fixture « kitchen », aller-retour LibreOffice, résumé du vrai deck et consolidation

> *Alignement sur l'exécution :* le dépôt ignore `*.png` ; `fixtures/import/kitchen/ppt/media/image1.png` est ajouté par `git add -f`. `libreoffice_round_trip_reads_the_same_deck` vérifie aussi `lo.warnings.is_empty()`.

**Files:**
- Create: fichiers restants de `fixtures/import/kitchen/`, `fixtures/import/libreoffice.pptx`, `tests/kitchen.rs`, snapshots
- Modify: `tests/alto.rs`, `ARCHITECTURE.md` (si besoin), `docs/superpowers/specs/2026-10-09-import-pptx-design.md`

- [ ] **Step 1 : la fixture `kitchen`**

Copier depuis `fixtures/import/minimal/` : `_rels/.rels`, `ppt/theme/theme1.xml`, `ppt/slideMasters/` (avec `_rels`), `ppt/slideLayouts/` (avec `_rels`). Les tâches 12 à 14 ont créé `ppt/tableStyles.xml`, `ppt/charts/chart1.xml`, `ppt/diagrams/{data1,drawing1}.xml`. Puis :

`ppt/media/image1.png` — vraie image PNG 2×2 :

```bash
python3 -I -c "import zlib,struct;c=lambda t,d:struct.pack('>I',len(d))+t+d+struct.pack('>I',zlib.crc32(t+d));raw=b''.join(b'\x00'+b'\xff\x00\x00\xff'*2 for _ in range(2));open('fixtures/import/kitchen/ppt/media/image1.png','wb').write(b'\x89PNG\r\n\x1a\n'+c(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+c(b'IDAT',zlib.compress(raw))+c(b'IEND',b''))"
```

`[Content_Types].xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="png" ContentType="image/png"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
<Override PartName="/ppt/slides/slide2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
<Override PartName="/ppt/slides/slide3.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
<Override PartName="/ppt/tableStyles.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"/>
<Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
<Override PartName="/ppt/diagrams/data1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml"/>
<Override PartName="/ppt/diagrams/drawing1.xml" ContentType="application/vnd.ms-office.drawingml.diagramDrawing+xml"/>
</Types>
```

`ppt/presentation.xml` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
<p:sldIdLst><p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId3"/><p:sldId id="258" r:id="rId5"/></p:sldIdLst>
<p:sldSz cx="12192000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/>
<p:defaultTextStyle><a:lvl1pPr><a:defRPr sz="1800"><a:solidFill><a:schemeClr val="tx1"/></a:solidFill><a:latin typeface="+mn-lt"/></a:defRPr></a:lvl1pPr></p:defaultTextStyle>
</p:presentation>
```

`ppt/_rels/presentation.xml.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
<Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide3.xml"/>
<Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles" Target="tableStyles.xml"/>
</Relationships>
```

`ppt/slides/slide1.xml` (« Tableau ») :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Titre 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Tableau</a:t></a:r></a:p></p:txBody></p:sp>
<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="4" name="Tableau 3"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp="1"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="838200" y="1825625"/><a:ext cx="3810000" cy="1143000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstRow="1" bandRow="1"><a:tableStyleId>{S}</a:tableStyleId></a:tblPr>
<a:tblGrid><a:gridCol w="1270000"/><a:gridCol w="2540000"/></a:tblGrid>
<a:tr h="381000"><a:tc gridSpan="2"><a:txBody><a:bodyPr/><a:p><a:r><a:t>Titre</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc hMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr>
<a:tr h="381000"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>A</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>1</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
<a:tr h="381000"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>B</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>2</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
</a:tbl></a:graphicData></a:graphic></p:graphicFrame>
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>
```

`ppt/slides/_rels/slide1.xml.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/>
</Relationships>
```

`ppt/slides/slide2.xml` (« Graphique et image ») :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Titre 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Graphique et image</a:t></a:r></a:p></p:txBody></p:sp>
<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="4" name="Graphique 3"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="838200" y="1825625"/><a:ext cx="6096000" cy="3429000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart r:id="rId5"/></a:graphicData></a:graphic></p:graphicFrame>
<p:pic><p:nvPicPr><p:cNvPr id="5" name="Image 4" descr="Carré rouge"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/><a:stretch/></p:blipFill><p:spPr><a:xfrm><a:off x="7315200" y="1825625"/><a:ext cx="3048000" cy="3048000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr></p:pic>
<p:grpSp><p:nvGrpSpPr><p:cNvPr id="6" name="Groupe 5"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="838200" y="5486400"/><a:ext cx="6096000" cy="508000"/><a:chOff x="838200" y="5486400"/><a:chExt cx="6096000" cy="508000"/></a:xfrm></p:grpSpPr>
<p:sp><p:nvSpPr><p:cNvPr id="7" name="ZoneTexte 6"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="838200" y="5486400"/><a:ext cx="3048000" cy="508000"/></a:xfrm><a:prstGeom prst="rect"/><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:t>Gauche</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="8" name="ZoneTexte 7"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="3886200" y="5486400"/><a:ext cx="3048000" cy="508000"/></a:xfrm><a:prstGeom prst="rect"/><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:t>Droite</a:t></a:r></a:p></p:txBody></p:sp>
</p:grpSp>
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>
```

`ppt/slides/_rels/slide2.xml.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
<Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>
```

`ppt/slides/slide3.xml` (« Processus et équation ») :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Titre 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Processus et équation</a:t></a:r></a:p></p:txBody></p:sp>
<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="4" name="Diagramme 3"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="1270000" y="1270000"/><a:ext cx="3175000" cy="635000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:relIds r:dm="rId6" r:lo="rId6" r:qs="rId6" r:cs="rId6"/></a:graphicData></a:graphic></p:graphicFrame>
<mc:AlternateContent><mc:Choice Requires="a14"><p:sp><p:nvSpPr><p:cNvPr id="5" name="Équation 4"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="1270000" y="3175000"/><a:ext cx="2540000" cy="508000"/></a:xfrm><a:prstGeom prst="rect"/><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:p><a14:m><m:oMathPara><m:oMath><m:r><m:t>E=mc²</m:t></m:r></m:oMath></m:oMathPara></a14:m></a:p></p:txBody></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="5" name="Équation 4"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="1270000" y="3175000"/><a:ext cx="2540000" cy="508000"/></a:xfrm><a:prstGeom prst="rect"/><a:blipFill><a:blip r:embed="rId2"/><a:stretch/></a:blipFill></p:spPr></p:sp></mc:Fallback></mc:AlternateContent>
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>
```

`ppt/slides/_rels/slide3.xml.rels` :

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
<Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData" Target="../diagrams/data1.xml"/>
<Relationship Id="rId7" Type="http://schemas.microsoft.com/office/2007/relationships/diagramDrawing" Target="../diagrams/drawing1.xml"/>
</Relationships>
```

- [ ] **Step 2 : `tests/kitchen.rs` et résumé du vrai deck**

```rust
//! Toutes les sortes d'items, et le même deck réécrit par LibreOffice.

mod common;

use oku_slides_import::read_pptx;
use oku_slides_import::source::{Item, ItemKind, Role, SourceDeck};
use speculoos::prelude::*;

fn kind(i: &Item) -> String {
    match &i.kind {
        ItemKind::Text(_) => "text".into(),
        ItemKind::Shape(_) => "shape".into(),
        ItemKind::Picture(_) => "picture".into(),
        ItemKind::Table(_) => "table".into(),
        ItemKind::Chart(_) => "chart".into(),
        ItemKind::Group { items } => format!("group[{}]", items.iter().map(kind).collect::<Vec<_>>().join(",")),
        ItemKind::Opaque(o) => format!("opaque({:?})", o.reason),
    }
}

/// Une ligne par slide (sortes d'items) et par avertissement.
fn summary(d: &SourceDeck) -> Vec<String> {
    let mut out: Vec<String> = d.slides.iter().map(|s| format!("{}: {}", s.id, s.items.iter().map(kind).collect::<Vec<_>>().join(" "))).collect();
    out.extend(d.warnings.iter().map(|w| format!("warning: {:?}", w.kind)));
    out
}

fn titles(d: &SourceDeck) -> Vec<String> {
    d.slides
        .iter()
        .map(|s| {
            s.items
                .iter()
                .find(|i| i.role == Some(Role::Title))
                .and_then(|i| match &i.kind {
                    ItemKind::Text(tb) => tb.paragraphs.first().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect()),
                    _ => None,
                })
                .unwrap_or_default()
        })
        .collect()
}

fn kitchen() -> SourceDeck {
    read_pptx(&common::pptx("kitchen"), &common::LIMITS).unwrap_or_else(|e| panic!("lecture de kitchen : {e}"))
}

#[test]
fn kitchen_reads_every_kind_of_item() {
    assert_that!(summary(&kitchen())).is_equal_to(vec![
        "s1: text table".to_owned(),
        "s2: text chart picture group[text,text]".to_owned(),
        "s3: text group[shape,shape] opaque(Math)".to_owned(),
        r#"warning: UnsupportedChart("combo: barChart")"#.to_owned(),
    ]);
}

#[test]
fn kitchen_titles_are_read() {
    assert_that!(titles(&kitchen())).is_equal_to(vec!["Tableau".to_owned(), "Graphique et image".to_owned(), "Processus et équation".to_owned()]);
}

#[test]
fn kitchen_snapshot_matches() {
    insta::assert_json_snapshot!("kitchen", kitchen());
}

#[test]
fn libreoffice_round_trip_reads_the_same_deck() {
    let lo = read_pptx(&common::file("libreoffice.pptx"), &common::LIMITS).unwrap_or_else(|e| panic!("lecture de libreoffice.pptx : {e}"));
    assert_that!(titles(&lo)).is_equal_to(titles(&kitchen()));
    let kinds: Vec<String> = lo.slides.iter().flat_map(|s| s.items.iter().map(kind)).collect();
    assert_that!(kinds.iter().any(|k| k == "table") && kinds.iter().any(|k| k == "chart")).is_true();
    insta::assert_json_snapshot!("libreoffice_summary", summary(&lo));
}
```

Ajouter à `tests/alto.rs` :

```rust
#[test]
fn alto_summary_matches() {
    let d = alto();
    let lines: Vec<String> = d
        .slides
        .iter()
        .map(|s| {
            let kinds: Vec<&str> = s
                .items
                .iter()
                .map(|i| match &i.kind {
                    ItemKind::Text(_) => "text",
                    ItemKind::Shape(_) => "shape",
                    ItemKind::Picture(_) => "picture",
                    _ => "other",
                })
                .collect();
            format!("{}: {}", s.id, kinds.join(" "))
        })
        .chain(d.warnings.iter().map(|w| format!("warning: {:?}", w.kind)))
        .collect();
    insta::assert_json_snapshot!("alto_summary", lines);
}
```

Run: `cargo nextest run -p oku-slides-import --test kitchen kitchen_reads kitchen_titles` → PASS. Sinon : fixture (relire les XML) ou défaut du lecteur → régression dans `tests/regressions/` d'abord (G13).

- [ ] **Step 3 : produire `libreoffice.pptx`**

LibreOffice tourne avec un profil jetable du scratchpad ; `zip` et `soffice` sont requis (s'arrêter et le signaler s'ils manquent) :

```bash
S=<scratchpad>
command -v zip && command -v soffice
rm -f "$S/kitchen.pptx" && (cd fixtures/import/kitchen && zip -X -r "$S/kitchen.pptx" '[Content_Types].xml' _rels ppt -x '*.DS_Store')
soffice -env:UserInstallation="file://$S/lo-profile" --headless --convert-to 'pptx:Impress MS PowerPoint 2007 XML' --outdir "$S/lo" "$S/kitchen.pptx"
cp "$S/lo/kitchen.pptx" fixtures/import/libreoffice.pptx
```

- [ ] **Step 4 : relire puis accepter les snapshots**

Run: `INSTA_UPDATE=new cargo nextest run -p oku-slides-import --test kitchen --test alto`, puis lire chaque `.snap.new` :
- `kitchen__kitchen` : tableau (spans, `covered`, `#4472c4` / `#cfd5ea` / `#e9ebf5`, `flags`), graphique (deux séries, `None` aux bons endroits), SmartArt (ids `s3/4/{A}`, `s3/4/{B}`, cadres en (100,100) et (250,100)), image (`alt` « Carré rouge »), équation (`opaque` `math` avec `fallback`).
- `kitchen__libreoffice_summary` : chaque écart avec `kitchen_reads_every_kind_of_item` doit s'expliquer par la réécriture de LibreOffice (SmartArt aplati, équation en image…) ; un `opaque(Unknown(...))`, `UnreadableItem` ou `MissingPart` révèle un défaut → régression, correction, puis acceptation.
- `alto__alto_summary` : 8 slides de `text`/`shape`/`picture`, aucun avertissement structurel.

Puis `cargo insta accept --manifest-path crates/oku-slides-import/Cargo.toml` ; `cargo nextest run -p oku-slides-import` → PASS.

- [ ] **Step 5 : consolidation (G10)**

- `rg -n "fn (zip_of|run_ctx|with_ctx|slide_pkg|tree_in|master_and_layout)\b" crates/oku-slides-import/src` : chacun défini une seule fois, dans `test_support.rs`.
- `rg -n "Document::parse" crates/oku-slides-import/src` : seulement dans `xml.rs` et `error_tests.rs`.
- `rg -n "pkg\(\)\.text\(|pkg\.text\(" crates/oku-slides-import/src/pptx/{shapes,frame,smartart,chart,table}.rs` : aucune occurrence (parts liées via `PartCtx::part_text`).
- `rg -n "\.unwrap\(\)|\.expect\(|unreachable!" crates/oku-slides-import/src --glob '!*_tests.rs' --glob '!test_support.rs'` : aucune occurrence.
- `wc -l crates/oku-slides-import/src/**/*.rs` : aucun fichier > 500 lignes (au-delà, découper par concept).
- `ARCHITECTURE.md` : couches et modules conformes au code.
- Spec (`docs/superpowers/specs/2026-10-09-import-pptx-design.md`) :
  - « Représentation intermédiaire » : remplacer le croquis par un renvoi à `crates/oku-slides-import/src/source/mod.rs` et aux « Décisions de la revue » de ce plan ;
  - « Lecteur PPTX » : `mc:AlternateContent` suit la décision 11 (branche choisie d'après `Requires` ; seule l'équation devient `Opaque`) ;
  - tableau des modules : ajouter `collect.rs`, `pptx/units.rs`, `pptx/frame.rs`, `pptx/eot.rs`, `pptx/context.rs`, `pptx/placeholder.rs` ; la conversion preset → path appartient au lot 3 ;
  - « Tests » : mentionner `alto-wealth.pptx` (vrai deck PptxGenJS) et son PDF de référence ;
  - « Lots » : marquer le lot 1 « livré (t-692) ».
- Oku : `t-726` (remplissages hérités sous `clrMapOvr`) est résolu par la décision 3 — le clore avec un commentaire qui renvoie à ce plan.

Run: `cargo fmt --all --check && cargo clippy -p oku-slides-import -p oku-slides-i18n --all-targets -- -D warnings && cargo nextest run -p oku-slides-import -p oku-slides-i18n`

- [ ] **Step 6 : commit, sous-tâche**

```bash
git add fixtures/import crates/oku-slides-import docs/superpowers/specs/2026-10-09-import-pptx-design.md && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # test(import) : fixture kitchen, aller-retour LibreOffice, vrai deck, consolidation du lot 1 (t-692)
oku cli task done --project slider --id t-739
oku cli task done --project slider --id t-726
```

---

## Hors de ce lot

Projecteurs retrofit (lot 2, avec la CLI et le câblage de `ReadLimits` dans la configuration `[import]`), hi-fi (lot 3), rendu de référence et métriques (lot 4 — `alto-wealth.pdf` sert de cible), app et fine-tuning (lot 5), lecteur PDF (lot 6). Backlog issu des revues : t-721 à t-725, t-727 à t-732, t-752 à t-755. `read_pptx` n'a aucun appelant hors tests jusqu'au lot 2.
