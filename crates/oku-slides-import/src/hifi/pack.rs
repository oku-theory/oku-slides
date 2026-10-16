//! Le pack généré : `_commun.typ` (le champ d'un placeholder, le décor de
//! chaque master appelé), un widget de slide par layout utilisé
//! ([`super::widget`]), leurs schémas déclarés et le `pack.json`.
//!
//! Un layout est utilisé si une slide source le désigne ; les widgets
//! suivent l'ordre des layouts de la source, les fonctions de master celui
//! des masters (`master-<rang>`, rang 1.. dans la source). Le pack vit dans
//! le deck seulement : la bibliothèque n'est pas touchée.
//!
//! Le projecteur applique et valide ses ops sans rendu : il lit les schémas
//! déclarés ([`generated_registry`]). La CLI sonde aussi le pack par le rendu
//! et refuse tout écart, un bogue du générateur.
//!
//! Le pack tient dans les bornes de la sonde ([`PackLimits`]), qui ne charge
//! ni les fichiers au-delà du nombre maximal (dans l'ordre des noms,
//! `_commun` en tête), ni un fichier trop gros :
//! - au plus `max_files − 1` widgets : les layouts les plus employés (par
//!   nombre de slides, à égalité dans l'ordre de la source) ; les autres
//!   ([`GeneratedPack::dropped`]) n'ont pas de widget, leurs slides prennent
//!   `blank` et y reçoivent fond et décor en éléments ([`super::slide`]) :
//!   garder les plus employés minimise les slides sans modèle ;
//! - chaque fichier sous `max_file_bytes` : le décor qui ne tient plus est
//!   omis, en partant de la fin de l'ordre de dessin (perte `DecorTruncated`).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};
use uuid::Uuid;

use oku_slides_core::{
    PackManifest, ParamValue, Slide, Template, WidgetError, WidgetFile, WidgetRegistry, WidgetSchema,
};

use super::typst::{GenCtx, Typst};
use super::widget::{Drawn, LayoutWidget, layout_widget, master_fn};
use crate::report::Loss;
use crate::source::{Master, SourceDeck};

/// Le champ d'un placeholder, partagé par les widgets générés (`champ`).
const CHAMP: &str = r#"/// Le champ `name` d'un placeholder : `body` dans sa boîte (pt page), à ses
/// marges `inset`, ancré en `anchor`. Vide, il garde sa boîte (l'éditeur la
/// montre) ; dans un texte, `{n}` est le numéro de la page.
#let champ(ctx, name, body, x, y, w, h, inset, anchor) = {
  let body = if body == none or body == "" { [] } else if type(body) == str and body.contains("{n}") {
    context body.replace("{n}", str(counter(page).get().first()))
  } else { body }
  place(top + left, dx: x, dy: y, block(width: w, height: h, inset: inset, align(anchor, field(ctx, name, body))))
}
"#;

/// Les bornes de la sonde d'un pack (`[render]` de la configuration) : le
/// pack généré s'y tient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackLimits {
    /// Nombre maximal de fichiers de widgets, `_commun` compris (`max_widgets`).
    pub max_files: usize,
    /// Taille maximale d'un fichier de widget, en octets (`max_widget_kib` × 1024).
    pub max_file_bytes: usize,
}

impl Default for PackLimits {
    /// Celles de la configuration par défaut : 64 fichiers, 256 Kio.
    fn default() -> Self {
        Self { max_files: 64, max_file_bytes: 256 * 1024 }
    }
}

/// Le pack généré pour une source.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct GeneratedPack {
    /// Les fichiers de widgets : `_commun` et un par layout utilisé.
    pub(crate) files: BTreeMap<WidgetFile, String>,
    /// Les widgets, dans l'ordre des layouts de la source.
    pub(crate) widgets: Vec<LayoutWidget>,
    /// Les assets que dessinent les widgets (fonds, décors) : le deck doit les porter.
    pub(crate) assets: BTreeSet<String>,
    /// Ce que les fonds et les décors n'ont pas pu dessiner.
    pub(crate) losses: Vec<Loss>,
    /// Les layouts utilisés laissés sans widget (bornes de la sonde), dans l'ordre de la source.
    pub(crate) dropped: Vec<String>,
}

impl GeneratedPack {
    /// Le widget du layout `layout`, s'il est utilisé.
    pub(crate) fn widget(&self, layout: &str) -> Option<&LayoutWidget> {
        self.widgets.iter().find(|w| w.layout == layout)
    }

    /// Les schémas déclarés, dans l'ordre des widgets.
    pub(crate) fn schemas(&self) -> impl Iterator<Item = &WidgetSchema> {
        self.widgets.iter().map(|w| &w.schema)
    }
}

/// Le pack généré pour `deck`, dans les bornes `limits`.
pub(crate) fn generate_pack(deck: &SourceDeck, ctx: &GenCtx<'_>, limits: PackLimits) -> GeneratedPack {
    let (kept, dropped) = kept_layouts(deck, limits.max_files.saturating_sub(1));
    let hiding: BTreeSet<&str> =
        deck.slides.iter().filter(|s| !s.show_layout_decor).filter_map(|s| s.layout.as_deref()).collect();
    let mut drawn = Drawn::default();
    let mut widgets = Vec::new();
    let mut called = BTreeSet::new();
    for (i, layout) in deck.layouts.iter().enumerate().filter(|(_, l)| kept.contains(l.id.as_str())) {
        let master = deck.masters.iter().enumerate().find(|(_, m)| m.id == layout.master).map(|(r, m)| (r + 1, m));
        let hideable = hiding.contains(layout.id.as_str());
        let Some((widget, d)) = layout_widget(i + 1, layout, master, hideable, ctx, limits.max_file_bytes) else {
            continue;
        };
        if layout.show_master_decor {
            called.extend(master.map(|(r, _)| r));
        }
        drawn.extend(d);
        widgets.push(widget);
    }
    let (common, d) = common_file(deck, &called, ctx, limits.max_file_bytes);
    drawn.extend(d);

    let mut files = BTreeMap::from([(WidgetFile::common(), common)]);
    files.extend(widgets.iter().map(|w| (w.name.clone(), w.source.clone())));
    GeneratedPack { files, widgets, assets: drawn.assets, losses: drawn.losses, dropped }
}

/// Les layouts utilisés qui ont un widget, au plus `room` : les plus
/// employés, à égalité les premiers de la source ; puis les autres
/// utilisés, dans l'ordre de la source.
fn kept_layouts(deck: &SourceDeck, room: usize) -> (BTreeSet<&str>, Vec<String>) {
    let mut uses: BTreeMap<&str, usize> = BTreeMap::new();
    for l in deck.slides.iter().filter_map(|s| s.layout.as_deref()) {
        *uses.entry(l).or_default() += 1;
    }
    let mut used: Vec<(usize, usize, &str)> = deck
        .layouts
        .iter()
        .enumerate()
        .filter_map(|(i, l)| uses.get(l.id.as_str()).map(|&n| (i, n, l.id.as_str())))
        .collect();
    used.sort_by_key(|&(i, n, _)| (std::cmp::Reverse(n), i));
    let kept: BTreeSet<&str> = used.iter().take(room).map(|&(_, _, id)| id).collect();
    let dropped = used.iter().skip(room).map(|&(i, _, id)| (i, id.to_owned())).collect::<BTreeMap<_, _>>();
    (kept, dropped.into_values().collect())
}

/// `_commun.typ` : le champ d'un placeholder, puis le décor de chaque master
/// de `called` (rangs 1..). En `max_bytes` octets si le décor des masters y
/// suffit à couper : il est réduit (dans l'ordre des masters) à ce qui tient,
/// en une passe ; le fichier sans décor donne la place.
fn common_file(deck: &SourceDeck, called: &BTreeSet<usize>, ctx: &GenCtx<'_>, max_bytes: usize) -> (String, Drawn) {
    let mut head = Typst::new();
    head.line("// Code partagé du pack généré par l'import : le champ d'un placeholder et");
    head.line("// le décor de chaque master.");
    head.line("#import \"/oku-slides/lib.typ\": field, color-of, font-of");
    head.line("");
    let mut common = head.finish();
    common.push_str(CHAMP);
    let masters: Vec<(usize, &Master)> =
        deck.masters.iter().enumerate().map(|(i, m)| (i + 1, m)).filter(|(r, _)| called.contains(r)).collect();
    // Sans place, `master_fn` n'écrit que la coquille (il ne génère que le premier item, écarté).
    let shell = masters.iter().fold(common.len(), |len, &(rank, m)| {
        let mut w = Typst::new();
        w.line("");
        master_fn(&mut w, rank, m, ctx, 0);
        len.saturating_add(w.finish().len())
    });
    let mut room = max_bytes.saturating_sub(shell);
    let mut drawn = Drawn::default();
    for (rank, master) in masters {
        let mut w = Typst::new();
        w.line("");
        let (d, len) = master_fn(&mut w, rank, master, ctx, room);
        common.push_str(&w.finish());
        drawn.extend(d);
        room = room.saturating_sub(len);
    }
    (common, drawn)
}

/// Le registre du deck importé : les widgets de `base` (ceux de la
/// bibliothèque, sans pack) et les widgets déclarés de `pack`, contrôlés
/// comme le serait la sonde (`WidgetRegistry::from_probe`).
pub(crate) fn generated_registry(base: &WidgetRegistry, pack: &GeneratedPack) -> Result<WidgetRegistry, WidgetError> {
    let base: Vec<&WidgetSchema> = base.iter().collect();
    let pack: Vec<&WidgetSchema> = pack.schemas().collect();
    WidgetRegistry::from_probe(json!({ "base": base, "pack": pack }))
}

/// Le `pack.json` du pack généré (première version, sans parent) : un
/// modèle par widget, nommé comme son layout, dont la slide est
/// `examples[layout]` (la première slide source sur ce layout, projetée) ou,
/// à défaut, une slide nue du widget. Les textes de ces slides sont aussi
/// les valeurs d'exemple des widgets (aperçu, réglages).
pub(crate) fn pack_manifest(
    pack: &GeneratedPack,
    id: Uuid,
    name: &str,
    created: &str,
    examples: &BTreeMap<String, Slide>,
) -> PackManifest {
    let templates: Vec<Template> = pack
        .widgets
        .iter()
        .map(|w| Template {
            id: w.name.as_str().to_owned(),
            name: w.schema.label.clone().unwrap_or_else(|| w.name.as_str().to_owned()),
            description: String::new(),
            slide: examples.get(&w.layout).cloned().unwrap_or_else(|| Slide::new(w.name.as_str(), w.name.as_str())),
        })
        .collect();
    let values = templates
        .iter()
        .zip(&pack.widgets)
        .map(|(t, w)| (w.name.clone(), text_values(&t.slide)))
        .filter(|(_, v)| !v.is_empty())
        .collect();
    PackManifest {
        id,
        name: name.to_owned(),
        description: String::new(),
        parent: None,
        created: created.to_owned(),
        templates,
        examples: values,
    }
}

/// Les textes non vides de `slide` (paramètres `plain` et `rich`), en valeurs d'exemple.
fn text_values(slide: &Slide) -> Map<String, Value> {
    slide
        .params
        .iter()
        .filter(|(_, v)| match v {
            ParamValue::Text(s) => !s.is_empty(),
            ParamValue::Rich(p) => !p.is_empty(),
            _ => false,
        })
        .filter_map(|(k, v)| Some((k.to_owned(), serde_json::to_value(v).ok()?)))
        .collect()
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod pack_tests;
