//! Générateur Typst du décor : les items de décor d'un master ou d'un layout
//! en code Typst qui les dessine à leur place exacte sur la page Oku.
//!
//! Le code produit est en mode code (sans `#`), une instruction `place(…)`
//! par dessin, à poser dans le corps d'une fonction de rendu d'un fichier de
//! pack. Il suppose en portée `t` (les jetons, `ctx.tokens`), `ctx` (pour
//! `ctx.load`, le chargeur d'image de l'hôte), `color-of` et `font-of`
//! (`#import "/oku-slides/lib.typ": color-of, font-of`). Les cadres sont en
//! pt page (marges du thème généré nulles : `place` part du coin de la page).
//!
//! | Item | Typst |
//! |---|---|
//! | `rect`, `roundRect`, `ellipse`, `line` | `rect` (`radius`), `ellipse`, `line` |
//! | autre géométrie | `curve` depuis [`super::geometry::outline`], étiré au cadre |
//! | texte (zone ou forme) | `block` (marges, ancrage) d'une `stack` de paragraphes, runs en `text` |
//! | image, repli d'un opaque | `(ctx.load)(asset: …, fit: "stretch")`, rognage par `clip` |
//! | groupe | ses enfants, à leur place absolue |
//! | tableau, graphique, opaque sans repli | rien, perte |
//!
//! Rotation et miroirs se dessinent ici (`rotate`, `scale`) : c'est du Typst
//! libre. Un texte suit la rotation de sa forme mais pas ses miroirs, comme
//! dans PowerPoint (un miroir vertical le retourne de 180°).
//!
//! Déterministe : nombres en notation fixe arrondis au centième ([`num`]),
//! jamais `-0`. Aucun texte source n'entre dans le code autrement que par
//! un littéral de chaîne échappé ([`string`]) : il ne peut pas injecter de
//! Typst.

// `write!` into a `String` never fails: its result is ignored.
use std::fmt::Write as _;

mod placeholder;
mod shape;
mod text;

use std::collections::{BTreeMap, BTreeSet};

pub(crate) use placeholder::{background, placeholder};
pub(crate) use shape::round_rect_radius;
pub(crate) use text::{DEFAULT_SIZE, Fit, LineBox, spacing};

use super::color::ThemeColors;
use super::page::{Page, round2};
use super::theme::role_families;
use crate::fonts::{FontPlan, LineMetrics};
use crate::report::{Loss, LossKind};
use crate::source::{Asset, Crop, Frame, Item, ItemKind, OpaqueReason, Shape, ThemeFonts};

/// Tampon d'écriture de code Typst, indenté de deux espaces par niveau.
#[derive(Debug, Default)]
pub(crate) struct Typst {
    out: String,
    depth: usize,
}

impl Typst {
    /// Un tampon vide.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Une ligne au niveau courant.
    pub(crate) fn line(&mut self, s: &str) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    /// Une ligne qui ouvre un niveau (`f(`, `{`).
    pub(crate) fn open(&mut self, s: &str) {
        self.line(s);
        self.depth += 1;
    }

    /// Une ligne qui ferme le niveau ouvert (`)`, `}`).
    pub(crate) fn close(&mut self, s: &str) {
        self.depth = self.depth.saturating_sub(1);
        self.line(s);
    }

    /// Le niveau courant.
    pub(crate) fn depth(&self) -> usize {
        self.depth
    }

    /// Le code écrit.
    pub(crate) fn finish(self) -> String {
        self.out
    }
}

/// Un nombre en notation fixe, arrondi au centième, sans `-0` ; 0 s'il n'est pas fini.
pub(crate) fn num(v: f64) -> String {
    let v = if v.is_finite() { round2(v) } else { 0.0 };
    format!("{v}")
}

/// Une longueur en pt (`12.5pt`).
pub(crate) fn pt(v: f64) -> String {
    format!("{}pt", num(v))
}

/// Un littéral de chaîne Typst : guillemets, barres obliques inverses et
/// caractères de contrôle échappés.
pub(crate) fn string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Ce dont le générateur a besoin.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GenCtx<'a> {
    /// La page source sur la page Oku.
    pub(crate) page: &'a Page,
    /// Les couleurs du thème généré.
    pub(crate) colors: &'a ThemeColors,
    /// Les polices retenues.
    pub(crate) fonts: &'a FontPlan,
    /// Les assets de la source.
    pub(crate) assets: &'a BTreeMap<String, Asset>,
    /// Les polices du thème source du master retenu, celles du thème généré
    /// (`font.display`, `font.body`) : leurs familles s'écrivent par rôle.
    pub(crate) theme_fonts: Option<&'a ThemeFonts>,
    /// La langue du deck ([`crate::source::SourceDeck::language`]) : un texte
    /// d'une autre langue ou d'un autre sens d'écriture le dit.
    pub(crate) lang: Option<&'a str>,
}

/// La police Oku d'une famille source ([`GenCtx::font_name`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FontName<'a> {
    /// Un rôle du thème généré (`display`, `body`) : sa police suit le thème.
    Role(&'static str),
    /// Une famille retenue hors du thème, en dur.
    Family(&'a str),
}

impl<'a> GenCtx<'a> {
    /// La police Oku de la famille source `family` : le rôle `body` ou
    /// `display` d'une police du thème (la même famille, sinon la même
    /// famille retenue), sinon la famille retenue ; `None` si aucune ne l'est
    /// (le corps du thème). Une seule décision pour les éléments et le Typst
    /// généré (décor, widgets, `_commun.typ`).
    pub(crate) fn font_name(&self, family: &str) -> Option<FontName<'a>> {
        let fonts: &'a FontPlan = self.fonts;
        let used = fonts.used(family)?;
        let role = self.theme_fonts.and_then(|tf| {
            let roles = [("body", &tf.minor), ("display", &tf.major)];
            let named = roles.iter().find(|(_, f)| f.eq_ignore_ascii_case(family));
            named.or_else(|| roles.iter().find(|(_, f)| fonts.used(f) == Some(used))).map(|(role, _)| *role)
        });
        Some(role.map_or(FontName::Family(used), FontName::Role))
    }

    /// Les familles que la police `name` donne au rendu : celles du rôle
    /// ([`role_families`] : latine puis des autres écritures), sinon la famille.
    pub(crate) fn families<'n>(&self, name: FontName<'n>) -> Vec<&'n str>
    where
        'a: 'n,
    {
        match name {
            FontName::Role(role) => {
                let fonts: &'a FontPlan = self.fonts;
                self.theme_fonts.map(|tf| role_families(tf, role == "display", fonts)).unwrap_or_default()
            }
            FontName::Family(f) => vec![f],
        }
    }

    /// Les métriques verticales de la police employée pour la famille source
    /// `family` ; sans famille, ou si sa police ne les donne pas, celles de la
    /// police du corps (la mineure du thème, sinon le repli) ; à défaut
    /// [`LineMetrics::DEFAULT`]. Une seule règle pour le décor, les
    /// placeholders et les éléments.
    pub(crate) fn line_metrics(&self, family: Option<&str>) -> LineMetrics {
        let fonts = self.fonts;
        let body = || {
            let minor = self.theme_fonts.map(|tf| tf.minor.as_str());
            minor.or(fonts.fallback.as_deref()).and_then(|f| fonts.metrics(f))
        };
        family.and_then(|f| fonts.metrics(f)).or_else(body).unwrap_or(LineMetrics::DEFAULT)
    }

    /// Le sens d'écriture de la langue du deck : de droite à gauche ?
    pub(crate) fn deck_rtl(&self) -> bool {
        self.lang.is_some_and(rtl_lang)
    }
}

/// Les langues que Typst écrit de droite à gauche (`text.dir` à `auto`).
const RTL_LANGS: [&str; 11] = ["ar", "dv", "fa", "he", "ks", "pa", "ps", "sd", "ug", "ur", "yi"];

/// La langue de l'étiquette `tag` (`ar-SA`) s'écrit-elle de droite à gauche ?
pub(crate) fn rtl_lang(tag: &str) -> bool {
    RTL_LANGS.contains(&tag.split('-').next().unwrap_or_default())
}

/// Les arguments `lang` et `region` de `text` pour l'étiquette `tag`
/// ([`crate::source::lang_tag`] : `zh-Hant-TW` → `lang: "zh", region: "TW"`).
pub(crate) fn lang_args(tag: &str) -> String {
    let mut parts = tag.split('-');
    let lang = parts.next().unwrap_or_default();
    match parts.find(|p| p.len() == 2) {
        Some(region) => format!("lang: {}, region: {}", string(lang), string(region)),
        None => format!("lang: {}", string(lang)),
    }
}

/// Le décor généré.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Decor {
    /// Le code Typst, en mode code (vide si rien n'est dessiné).
    pub(crate) code: String,
    /// Les assets dessinés : le deck doit les porter.
    pub(crate) assets: BTreeSet<String>,
    /// Ce qui n'a pas pu être dessiné.
    pub(crate) losses: Vec<Loss>,
}

/// Le code Typst qui dessine `items` (décor d'un master ou d'un layout, dans
/// l'ordre de dessin).
#[cfg(test)]
pub(crate) fn decor(items: &[Item], ctx: &GenCtx<'_>) -> Decor {
    decor_within(items, ctx, 0, usize::MAX)
}

/// Les octets que prennent les lignes de `code` réécrites au niveau `depth`
/// d'un [`Typst`] (deux espaces par niveau, une fin de ligne chacune).
pub(crate) fn indented_len(code: &str, depth: usize) -> usize {
    code.lines().map(|l| l.len().saturating_add(2 * depth + 1)).fold(0, usize::saturating_add)
}

/// [`decor`] en `max_bytes` octets au plus une fois réécrit au niveau
/// `depth` ([`indented_len`]) : les items qui ne tiennent plus (le premier
/// qui déborde et tous les suivants) sont omis, une perte
/// [`LossKind::DecorTruncated`] sur le premier omis. Chaque item est généré
/// une fois au plus : le coût est linéaire. Le fichier de pack qui porte le
/// décor tient ainsi sous la taille que la sonde charge.
pub(crate) fn decor_within(items: &[Item], ctx: &GenCtx<'_>, depth: usize, max_bytes: usize) -> Decor {
    let mut out = Decor::default();
    let mut spent = 0usize;
    for (i, item) in items.iter().enumerate() {
        let mut g = Gen { w: Typst::new(), ctx: *ctx, assets: BTreeSet::new(), losses: Vec::new() };
        g.item(item);
        let code = g.w.finish();
        let cost = indented_len(&code, depth);
        if spent.saturating_add(cost) > max_bytes {
            let omitted = u32::try_from(items.len() - i).unwrap_or(u32::MAX);
            out.losses.push(Loss::on(&item.id, LossKind::DecorTruncated { omitted }));
            break;
        }
        spent = spent.saturating_add(cost);
        out.code.push_str(&code);
        out.assets.extend(g.assets);
        out.losses.extend(g.losses);
    }
    out
}

/// Boîte d'un dessin sur la page Oku et sa transformation.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    /// Cadre en pt page.
    frame: oku_slides_core::Frame,
    /// Rotation en degrés, sens horaire.
    rot: f64,
    /// Miroir horizontal.
    flip_h: bool,
    /// Miroir vertical.
    flip_v: bool,
}

impl Placement {
    /// Le dessin d'une boîte source, rotation et miroirs compris.
    fn of(page: &Page, f: &Frame) -> Self {
        Self { frame: page.frame(f), rot: f.rot, flip_h: f.flip_h, flip_v: f.flip_v }
    }

    /// Le dessin du texte d'une boîte source, sur la part `rect` (gauche,
    /// haut, droite, bas, fractions) de sa géométrie : la part suit les
    /// miroirs et tourne avec la forme autour du centre de la boîte, mais le
    /// texte n'est pas mis en miroir (un miroir vertical le retourne).
    #[expect(clippy::many_single_char_names, reason = "sides l, t, r, b and size w, h of a geometric formula")]
    fn text(page: &Page, f: &Frame, [l, t, r, b]: [f64; 4]) -> Self {
        let (l, r) = if f.flip_h { (1.0 - r, 1.0 - l) } else { (l, r) };
        let (t, b) = if f.flip_v { (1.0 - b, 1.0 - t) } else { (t, b) };
        let (w, h) = ((r - l) * f.w, (b - t) * f.h);
        // Centre de la part, relatif au centre de la boîte, tourné avec elle (sens horaire, y vers le bas).
        let (dx, dy) = ((f64::midpoint(l, r) - 0.5) * f.w, (f64::midpoint(t, b) - 0.5) * f.h);
        let (sin, cos) = f.rot.to_radians().sin_cos();
        let (cx, cy) = (f.x + f.w / 2.0 + dx * cos - dy * sin, f.y + f.h / 2.0 + dx * sin + dy * cos);
        let sub = Frame::new(cx - w / 2.0, cy - h / 2.0, w, h);
        let rot = if f.flip_v { (f.rot + 180.0) % 360.0 } else { f.rot };
        Self { frame: page.frame(&sub), rot, flip_h: false, flip_v: false }
    }

    /// Écrit `place(…)` et les transformations autour du dessin que `body`
    /// écrit, de la taille du cadre.
    fn emit(&self, w: &mut Typst, body: impl FnOnce(&mut Typst)) {
        let opened = self.open(w);
        body(w);
        Self::close(w, opened);
    }

    /// Ouvre `place(…)` et les transformations ; renvoie le nombre de niveaux ouverts.
    fn open(&self, w: &mut Typst) -> usize {
        let f = &self.frame;
        w.open(&format!("place(top + left, dx: {}, dy: {},", pt(f.x), pt(f.y)));
        let mut opened = 1;
        let rot = round2(self.rot);
        if rot != 0.0 {
            w.open(&format!("rotate({}deg, reflow: false,", num(rot)));
            opened += 1;
        }
        if self.flip_h || self.flip_v {
            let sx = if self.flip_h { "-100%" } else { "100%" };
            let sy = if self.flip_v { "-100%" } else { "100%" };
            w.open(&format!("scale(x: {sx}, y: {sy}, reflow: false,"));
            opened += 1;
        }
        if opened > 1 {
            // Le centre de la transformation est celui du cadre, même pour un contour qui déborde.
            w.open(&format!("box(width: {}, height: {},", pt(f.w), pt(f.h)));
            opened += 1;
        }
        opened
    }

    /// Referme les `opened` niveaux de [`Self::open`].
    fn close(w: &mut Typst, opened: usize) {
        for _ in 1..opened {
            w.close("),");
        }
        // `place` est une instruction du bloc de code : sans virgule.
        w.close(")");
    }
}

/// L'état d'une génération.
struct Gen<'a> {
    w: Typst,
    ctx: GenCtx<'a>,
    assets: BTreeSet<String>,
    losses: Vec<Loss>,
}

impl Gen<'_> {
    fn item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Text(tb) => {
                self.text_box(&Placement::text(self.ctx.page, &item.frame, [0.0, 0.0, 1.0, 1.0]), tb);
            }
            ItemKind::Shape(s) => self.shape_item(item, s),
            ItemKind::Picture(p) => self.picture(item, &p.asset, p.crop),
            ItemKind::Group { items, .. } => items.iter().for_each(|child| self.item(child)),
            ItemKind::Opaque(o) => match &o.fallback {
                Some(asset) => self.picture(item, asset, Crop::default()),
                None => self.losses.push(Loss::on(&item.id, LossKind::Opaque(o.reason.clone()))),
            },
            ItemKind::SmartArt(_) => self.losses.push(Loss::on(&item.id, LossKind::Opaque(OpaqueReason::SmartArt))),
            ItemKind::Table(_) => self.lose_kind(item, "table"),
            ItemKind::Chart(_) => self.lose_kind(item, "chart"),
        }
    }

    /// Un tableau ou un graphique dans un décor : rare, non dessiné.
    fn lose_kind(&mut self, item: &Item, kind: &str) {
        self.losses.push(Loss::on(&item.id, LossKind::Opaque(OpaqueReason::Unknown(kind.to_owned()))));
    }

    /// Une forme, puis son texte dans la zone de texte de sa géométrie.
    fn shape_item(&mut self, item: &Item, s: &Shape) {
        let at = Placement::of(self.ctx.page, &item.frame);
        self.shape(&item.id, &at, s);
        if let Some(tb) = &s.text {
            let rect = shape::text_rect(s, at.frame.w, at.frame.h);
            self.text_box(&Placement::text(self.ctx.page, &item.frame, rect), tb);
        }
    }

    /// L'image `asset`, étirée au cadre et rognée de `crop` ; rien si l'asset
    /// manque (le lecteur l'a dit), perte s'il ne se dessine pas.
    fn picture(&mut self, item: &Item, asset: &str, crop: Crop) {
        let Some(a) = self.ctx.assets.get(asset) else { return };
        if !a.is_drawable() {
            self.losses.push(Loss::on(&item.id, LossKind::UnsupportedImage(a.mime.clone())));
            return;
        }
        self.assets.insert(asset.to_owned());
        let at = Placement::of(self.ctx.page, &item.frame);
        let f = at.frame;
        at.emit(&mut self.w, |w| {
            let (sw, sh) = (1.0 - crop.l - crop.r, 1.0 - crop.t - crop.b);
            if crop == Crop::default() || sw <= 0.0 || sh <= 0.0 {
                w.line(&format!("box(width: {}, height: {}, {}),", pt(f.w), pt(f.h), load(asset)));
                return;
            }
            // L'image entière, plus grande que le cadre, décalée du rognage haut et gauche.
            let (iw, ih) = (f.w / sw, f.h / sh);
            w.open(&format!("box(width: {}, height: {}, clip: true,", pt(f.w), pt(f.h)));
            w.open(&format!("place(top + left, dx: {}, dy: {},", pt(-crop.l * iw), pt(-crop.t * ih)));
            w.line(&format!("box(width: {}, height: {}, {}),", pt(iw), pt(ih), load(asset)));
            w.close("),");
            w.close("),");
        });
    }
}

/// L'image `asset` par le chargeur de l'hôte, étirée à son conteneur ; rien sans chargeur.
fn load(asset: &str) -> String {
    format!("if ctx.load != none {{ (ctx.load)(asset: {}, fit: \"stretch\") }}", string(asset))
}

#[cfg(test)]
#[path = "mod_tests.rs"]
pub(super) mod mod_tests;
