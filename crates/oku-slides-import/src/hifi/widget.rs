//! Le widget de slide généré pour un layout source : son nom, ses
//! paramètres, son schéma déclaré et sa source Typst.
//!
//! | Layout | Widget |
//! |---|---|
//! | rang `r` (1..) dans la source, nom | `l<r>-<slug du nom>` (`[a-z][a-z0-9-]*`, 64 au plus) ; slug vide → `l<r>` |
//! | placeholder titre, sous-titre, corps ou objet | paramètre `rich` : `titre`, `sous-titre`, `corps` (objet aussi) |
//! | placeholder date, pied, numéro | paramètre `plain` : `date`, `pied`, `numero` (défaut `{n}`) |
//! | placeholder image, graphique, tableau, média | rien : le contenu de la slide va dans `extras`, à sa place |
//! | une slide du layout masque son décor (`showMasterSp="0"`) | `decor` (`flag`, défaut vrai) : le décor du master et du layout est dessiné |
//! | | `grid` (grille de `blank`) et `extras` (slot `free`) |
//!
//! Un nom répété dans un layout prend un suffixe : `corps`, `corps-2`…
//!
//! Rendu, de bas en haut : fond du layout (sinon du master), décor du master
//! (si le layout le montre, `master-<rang>` de `_commun.typ`), décor du
//! layout, placeholders (champs à leur boîte, au style de leur premier
//! niveau), puis `extras` sur toute la page (marges du thème généré nulles :
//! les cadres y sont en pt page). Dans un texte `plain`, `{n}` est le numéro
//! de la page, comme dans les gabarits d'en-tête et de pied de la
//! bibliothèque (`interp` ne connaît que les variables du deck).
//!
//! Le schéma déclaré ([`LayoutWidget::schema`]) est celui que la sonde lit
//! dans la source : `oku-slides-render/tests/import_pack.rs` le vérifie.

use std::collections::BTreeSet;

use serde_json::json;

use oku_slides_core::{Category, ParamSchema, ParamType, SlotMode, WidgetFile, WidgetSchema};

use super::typst::{Decor, GenCtx, Typst, background, decor_within, indented_len, placeholder, string};
use crate::report::Loss;
use crate::source::{Layout, Master, Role};

/// Paramètre du décor : faux, la slide masque le décor du master et du layout
/// (pas leur fond), comme une slide PowerPoint `showMasterSp="0"`.
pub(crate) const DECOR_PARAM: &str = "decor";
/// Paramètre de la grille de la zone libre (celle du widget `blank`).
pub(crate) const GRID_PARAM: &str = "grid";
/// Paramètre du slot libre : ce que la slide pose sur son layout.
pub(crate) const EXTRAS_PARAM: &str = "extras";
/// Défaut du numéro de slide : le numéro de la page.
pub(crate) const PAGE_NUMBER: &str = "{n}";
/// Longueur maximale d'un nom de widget (celle d'un [`WidgetFile`]).
const NAME_MAX: usize = 64;

/// Le paramètre d'un placeholder de texte du layout.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlaceholderParam {
    /// Id du placeholder de layout.
    pub(crate) placeholder: String,
    /// Son index (`ph@idx`), que les placeholders des slides reprennent.
    pub(crate) idx: Option<u32>,
    /// Son rôle.
    pub(crate) role: Role,
    /// Le nom du paramètre.
    pub(crate) param: String,
}

/// Le widget généré d'un layout.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutWidget {
    /// Id du layout.
    pub(crate) layout: String,
    /// Nom du widget (et de son fichier).
    pub(crate) name: WidgetFile,
    /// Paramètres des placeholders de texte, dans l'ordre du layout.
    pub(crate) params: Vec<PlaceholderParam>,
    /// Schéma déclaré.
    pub(crate) schema: WidgetSchema,
    /// Source Typst (`widgets/<nom>.typ`).
    pub(crate) source: String,
}

/// Ce que dessine un code généré : les assets que le deck doit porter, les pertes.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Drawn {
    /// Assets dessinés.
    pub(crate) assets: BTreeSet<String>,
    /// Ce qui n'a pas pu être dessiné.
    pub(crate) losses: Vec<Loss>,
}

impl Drawn {
    /// Ajoute les assets et les pertes de `other`.
    pub(crate) fn extend(&mut self, other: Drawn) {
        self.assets.extend(other.assets);
        self.losses.extend(other.losses);
    }

    /// Ajoute les assets et les pertes de `d` ; rend son code.
    fn take(&mut self, d: Decor) -> String {
        self.assets.extend(d.assets);
        self.losses.extend(d.losses);
        d.code
    }
}

/// Le nom du widget du layout de rang `rank` (1..) nommé `name` (toujours
/// `Some` : `l`, des chiffres, puis des minuscules, des chiffres et des tirets).
pub(crate) fn widget_name(rank: usize, name: &str) -> Option<WidgetFile> {
    let mut out = format!("l{rank}");
    let slug = slug(name);
    if !slug.is_empty() {
        out.push('-');
        out.push_str(&slug);
    }
    out.truncate(NAME_MAX);
    WidgetFile::parse(out.trim_end_matches('-'))
}

/// `name` en minuscules ASCII et tirets : lettres latines sans leurs
/// accents, chiffres ; tout le reste sépare.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        let folded = match c {
            'a'..='z' | '0'..='9' => Some(c.to_string()),
            c => fold(c).map(str::to_owned),
        };
        match folded {
            Some(s) => out.push_str(&s),
            None if !out.is_empty() && !out.ends_with('-') => out.push('-'),
            None => {}
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// Une lettre latine accentuée (minuscule) sans son accent.
fn fold(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'ç' | 'ć' | 'č' => "c",
        'ď' | 'đ' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'ī' => "i",
        'ł' => "l",
        'ñ' | 'ń' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => "o",
        'ŕ' | 'ř' => "r",
        'ś' | 'š' | 'ş' => "s",
        'ť' | 'ţ' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => "u",
        'ý' | 'ÿ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        'æ' => "ae",
        'œ' => "oe",
        'ß' => "ss",
        _ => return None,
    })
}

/// Le nom de base du paramètre d'un placeholder de rôle `role`, et s'il est
/// `plain` ; `None` pour un placeholder sans texte.
fn param_base(role: Role) -> Option<(&'static str, bool)> {
    match role {
        Role::Title => Some(("titre", false)),
        Role::Subtitle => Some(("sous-titre", false)),
        Role::Body | Role::Object => Some(("corps", false)),
        Role::Date => Some(("date", true)),
        Role::Footer => Some(("pied", true)),
        Role::SlideNum => Some(("numero", true)),
        Role::Picture | Role::Chart | Role::Table | Role::Media => None,
    }
}

/// Le paramètre `plain` d'un placeholder de rôle `role` ?
pub(crate) fn is_plain(role: Role) -> bool {
    param_base(role).is_some_and(|(_, plain)| plain)
}

/// Les paramètres des placeholders de texte de `layout`, dans son ordre.
fn placeholder_params(layout: &Layout) -> Vec<PlaceholderParam> {
    let mut used = BTreeSet::new();
    layout
        .placeholders
        .iter()
        .filter_map(|ph| {
            let (base, _) = param_base(ph.role)?;
            let param = (1..)
                .map(|n| if n == 1 { base.to_owned() } else { format!("{base}-{n}") })
                .find(|p| !used.contains(p))
                .unwrap_or_default();
            used.insert(param.clone());
            Some(PlaceholderParam { placeholder: ph.id.clone(), idx: ph.idx, role: ph.role, param })
        })
        .collect()
}

/// Un paramètre déclaré sans libellé ni lien.
fn param(name: &str, ty: ParamType, default: Option<serde_json::Value>) -> ParamSchema {
    ParamSchema { name: name.to_owned(), label: None, ty, default, bind: None }
}

/// Le schéma du widget `name` (libellé `label`) aux paramètres de
/// placeholders `params`, avec le paramètre du décor si `decor`.
fn schema(name: &WidgetFile, label: Option<&str>, params: &[PlaceholderParam], decor: bool) -> WidgetSchema {
    let mut out: Vec<ParamSchema> = params
        .iter()
        .map(|p| match p.role {
            Role::SlideNum => param(&p.param, ParamType::Plain, Some(json!(PAGE_NUMBER))),
            r if is_plain(r) => param(&p.param, ParamType::Plain, None),
            _ => param(&p.param, ParamType::Rich, None),
        })
        .collect();
    if decor {
        out.push(param(DECOR_PARAM, ParamType::Flag, Some(json!(true))));
    }
    out.push(param(GRID_PARAM, ParamType::Grid, Some(json!({ "columns": 12, "rows": 6 }))));
    out.push(param(EXTRAS_PARAM, ParamType::Slot { mode: SlotMode::Free, accepts: None }, None));
    WidgetSchema {
        name: name.as_str().to_owned(),
        label: label.map(str::to_owned),
        category: Category::Slide,
        params: out,
    }
}

/// Le widget de `layout`, de rang `rank` (1..), sur son master `master` et
/// son rang (1..) s'il est connu. Le décor du master n'est qu'appelé
/// (`master-<rang>`, défini dans `_commun.typ`) ; ce que dessine le widget
/// lui-même (fond, décor du layout) est rendu avec lui. `hideable` : une
/// slide du layout masque son décor ; s'il y en a un, le paramètre `decor`
/// le commande.
///
/// La source tient en `max_bytes` octets si le décor du layout y suffit à
/// couper : il est réduit à ce qui tient (perte
/// [`crate::report::LossKind::DecorTruncated`]), en une passe : la source
/// sans décor propre donne la place qui reste et le niveau du décor.
pub(crate) fn layout_widget(
    rank: usize,
    layout: &Layout,
    master: Option<(usize, &Master)>,
    hideable: bool,
    ctx: &GenCtx<'_>,
    max_bytes: usize,
) -> Option<(LayoutWidget, Drawn)> {
    let name = widget_name(rank, &layout.name)?;
    let label = Some(layout.name.trim()).filter(|l| !l.is_empty());
    let params = placeholder_params(layout);
    let fill = layout.background.as_ref().or(master.map(|(_, m)| &m.background));
    let back = fill.map(|f| background(&layout.id, f, ctx)).unwrap_or_default();
    let called = master.filter(|_| layout.show_master_decor).map(|(r, _)| r);
    let source = |switch: bool, own: &str| {
        let schema = schema(&name, label, &params, switch);
        let parts = Parts { layout, master, name: &name, label, params: &params, schema: &schema, called, switch };
        let (source, depth) = widget_source(&parts, &back.code, own, ctx);
        (source, depth, schema)
    };
    // Avec un décor propre, l'interrupteur suit `hideable` : la coquille le compte déjà.
    let (shell, depth, _) = source(hideable, "");
    let own = decor_within(&layout.decor, ctx, depth, max_bytes.saturating_sub(shell.len()));
    let switch = hideable && (called.is_some() || !own.code.is_empty());
    let (source, _, schema) = source(switch, &own.code);
    let mut drawn = Drawn::default();
    drawn.take(back);
    drawn.take(own);
    let widget = LayoutWidget { layout: layout.id.clone(), name, params, schema, source };
    Some((widget, drawn))
}

/// Ce que la source d'un widget de layout assemble.
struct Parts<'a> {
    layout: &'a Layout,
    master: Option<(usize, &'a Master)>,
    name: &'a WidgetFile,
    label: Option<&'a str>,
    params: &'a [PlaceholderParam],
    schema: &'a WidgetSchema,
    /// Rang du master dont le décor est appelé.
    called: Option<usize>,
    /// Le paramètre `decor` commande le décor.
    switch: bool,
}

/// La source Typst d'un widget de layout : fond `back`, décor du master
/// appelé, décor propre `own`, champs des placeholders, puis `extras` ; et le
/// niveau où `own` est écrit.
fn widget_source(parts: &Parts<'_>, back: &str, own: &str, ctx: &GenCtx<'_>) -> (String, usize) {
    let Parts { layout, master, name, label, params, schema, called, switch } = *parts;
    let mut w = Typst::new();
    let on = master.map_or_else(String::new, |(_, m)| format!(" (master {})", m.id));
    w.line(&format!("// Widget de slide généré par l'import : le layout {}{on}.", layout.id));
    w.line(
        "#import \"/oku-slides/lib.typ\": widget, params as p, blank-slide, free-area, default-grid, color-of, font-of",
    );
    match called {
        Some(r) => w.line(&format!("#import \"_commun.typ\": champ, master-{r}")),
        None => w.line("#import \"_commun.typ\": champ"),
    }
    w.line("");
    let label = label.map_or_else(|| "none".to_owned(), string);
    w.open(&format!("#let widget = widget({}, label: {label}, category: \"slide\",", string(name.as_str())));
    w.open("params: (");
    for p in &schema.params {
        let decl = match &p.ty {
            ParamType::Plain if p.default.is_some() => format!("p.plain(default: {})", string(PAGE_NUMBER)),
            ParamType::Plain => "p.plain()".to_owned(),
            ParamType::Flag => "p.flag(default: true)".to_owned(),
            ParamType::Grid => "p.grid(default: default-grid)".to_owned(),
            ParamType::Slot { .. } => "p.slot(mode: \"free\")".to_owned(),
            _ => "p.rich()".to_owned(),
        };
        w.line(&format!("{}: {decl},", string(&p.name)));
    }
    w.close("),");
    w.open("(args, ctx) => blank-slide(id: ctx.id, {");
    w.line("let t = ctx.tokens");
    lines(&mut w, back);
    if switch {
        w.open(&format!("if args.at({}) {{", string(DECOR_PARAM)));
    }
    if let Some(r) = called {
        w.line(&format!("master-{r}(t, ctx)"));
    }
    let depth = w.depth();
    lines(&mut w, own);
    if switch {
        w.close("}");
    }
    for p in params {
        if let Some(ph) = layout.placeholders.iter().find(|ph| ph.id == p.placeholder) {
            lines(&mut w, &placeholder(ph, &p.param, ctx));
        }
    }
    w.line(&format!("free-area(t, args.at({}), grid: args.at({}))", string(EXTRAS_PARAM), string(GRID_PARAM)));
    w.close("}),");
    w.close(")");
    (w.finish(), depth)
}

/// Le code `master-<rank>(t, ctx)` de `_commun.typ` : le décor du master,
/// en `max_bytes` octets au plus une fois écrit ([`decor_within`]). Rend
/// aussi les octets de ce décor dans `w`.
pub(crate) fn master_fn(
    w: &mut Typst,
    rank: usize,
    master: &Master,
    ctx: &GenCtx<'_>,
    max_bytes: usize,
) -> (Drawn, usize) {
    let mut drawn = Drawn::default();
    w.line(&format!("/// Décor du master {}.", master.id));
    w.open(&format!("#let master-{rank}(t, ctx) = {{"));
    let depth = w.depth();
    let code = drawn.take(decor_within(&master.decor, ctx, depth, max_bytes));
    lines(w, &code);
    w.close("}");
    (drawn, indented_len(&code, depth))
}

/// Les lignes de `code` au niveau courant de `w`.
fn lines(w: &mut Typst, code: &str) {
    code.lines().for_each(|l| w.line(l));
}

#[cfg(test)]
#[path = "widget_tests.rs"]
mod widget_tests;
