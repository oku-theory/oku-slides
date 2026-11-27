//! DOCX: the main document's headings, paragraphs, lists, tables, links, images and text
//! boxes, walked without recursion; elements nested past [`MAX_DEPTH`] are cut and stated.
//! Hidden text (set on the run or by its style), tracked deletions and comments are left out
//! and stated.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use oku_slides_core::knowledge::{Cut, CutKind, ExtractLimits, ExtractRefusal};
use oku_slides_core::num::count_u64;
use oku_slides_core::num::size_usize;
use oku_slides_import::bounded::BoundedZip;
use oku_slides_import::{ImportErrorKind, ReadLimits};
use roxmltree::Node;

use crate::archive::{self, Rel, attr, children};
use crate::doc::{Doc, Fetch, Inline, Kind, MAX_TABLE_CELLS};
use crate::images::Skip;
use crate::markdown::Bound;
use oku_slides_core::knowledge::Extracted;

/// The `WordprocessingML` namespace.
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// Deepest element read below `w:body`; deeper ones are cut.
const MAX_DEPTH: usize = 64;

/// Elements whose children are read in their place (content controls, revisions kept, the
/// chosen branch of alternate content…).
const CONTAINERS: [&str; 10] = [
    "sdt",
    "sdtContent",
    "customXml",
    "ins",
    "moveTo",
    "smartTag",
    "fldSimple",
    "AlternateContent",
    "Choice",
    "txbxContent",
];

/// Extracts a DOCX document.
pub(crate) fn extract(
    bytes: &[u8],
    limits: &ExtractLimits,
    read: &ReadLimits,
    token: &str,
) -> Result<Extracted, ExtractRefusal> {
    let zip = archive::open(bytes, read)?;
    let main = archive::main_part(&zip)?;
    let text = archive::text(&zip, &main)?;
    let doc = archive::xml(&zip, &text, &main)?;
    let root = doc.root_element();
    if root.tag_name().name() != "document" || root.tag_name().namespace() != Some(W) {
        return Err(ExtractRefusal::TypeMismatch);
    }
    let (rels, links) = archive::rels_and_links(&zip, &main)?;
    let fetch = |key: &str| fetch(&zip, key);
    let Styles { headings, hidden: hidden_styles } = styles(&zip, &rels)?;
    let mut walk = Walk {
        headings,
        hidden_styles,
        lists: lists(&zip, &rels)?,
        images: rels.iter().filter(|r| r.kind.ends_with("/image")).map(|r| (r.id.clone(), r.target.clone())).collect(),
        links,
        doc: Doc::new(limits, limits.max_image_pixels.min(read.max_image_pixels), token),
        fetch: &fetch,
        depth_cut: 0,
        left: BTreeSet::new(),
    };
    if let Some(body) = children(root, "body").next() {
        walk.blocks(body);
    }
    let skipped: Vec<&str> = walk.left.iter().map(|l| l.label()).collect();
    walk.doc.finish(Cut { what: CutKind::Depth, count: walk.depth_cut }, Bound::Limit(MAX_DEPTH), &skipped, limits)
}

/// The bytes of the image part `key`; a part past the read limits (its own, or the total
/// the archive has left) is stated as such.
fn fetch<'b>(zip: &BoundedZip<'_>, key: &str) -> Result<Cow<'b, [u8]>, Skip> {
    zip.read(key).map(Cow::Owned).map_err(|e| match e.kind() {
        ImportErrorKind::TooLarge => Skip::ReadLimit,
        _ => Skip::Unreadable,
    })
}

/// The XML of the part behind the first relationship of `kind` (its type's last segment),
/// read by `f`; `T::default()` when there is none.
fn related<T: Default>(
    zip: &BoundedZip<'_>,
    rels: &[Rel],
    kind: &str,
    f: impl FnOnce(Node<'_, '_>) -> T,
) -> Result<T, ExtractRefusal> {
    let Some(rel) = rels.iter().find(|r| r.kind.rsplit('/').next() == Some(kind)) else {
        return Ok(T::default());
    };
    let text = archive::text(zip, &rel.target)?;
    let doc = archive::xml(zip, &text, &rel.target)?;
    Ok(f(doc.root_element()))
}

/// What the paragraph and character styles say.
#[derive(Debug, Default)]
struct Styles {
    /// The heading level of each style that is a title (1) or a heading (1 to 6), by id;
    /// known by its id or by its name.
    headings: BTreeMap<String, usize>,
    /// The styles whose text is hidden (`w:vanish` or `w:specVanish`, own or inherited).
    hidden: BTreeSet<String>,
}

/// The styles of the document.
fn styles(zip: &BoundedZip<'_>, rels: &[Rel]) -> Result<Styles, ExtractRefusal> {
    related(zip, rels, "styles", |root| {
        let mut headings = BTreeMap::new();
        let mut own: BTreeMap<&str, (Option<bool>, Option<&str>)> = BTreeMap::new();
        for s in children(root, "style") {
            let Some(id) = attr(s, "styleId") else { continue };
            let name = children(s, "name").next().and_then(|n| attr(n, "val"));
            if let Some(level) = level_of(id).or_else(|| name.and_then(level_of)) {
                headings.insert(id.to_owned(), level);
            }
            let props = children(s, "rPr").next();
            let hidden = match (toggle(props, "vanish"), toggle(props, "specVanish")) {
                (None, None) => None,
                (vanish, spec) => Some(vanish == Some(true) || spec == Some(true)),
            };
            let based = children(s, "basedOn").next().and_then(|b| attr(b, "val"));
            own.insert(id, (hidden, based));
        }
        let hidden = own.keys().filter(|id| inherits_hidden(&own, id)).map(|id| (*id).to_owned()).collect();
        Styles { headings, hidden }
    })
}

/// Whether style `id` hides its text: its own setting, else its base's, followed for at
/// most [`MAX_DEPTH`] styles.
fn inherits_hidden(own: &BTreeMap<&str, (Option<bool>, Option<&str>)>, id: &str) -> bool {
    let mut at = id;
    for _ in 0..MAX_DEPTH {
        let Some((hidden, based)) = own.get(at) else { return false };
        if let Some(hidden) = hidden {
            return *hidden;
        }
        let Some(based) = based else { return false };
        at = based;
    }
    false
}

/// The heading level a style name or id says: `Title` 1, `Heading N` / `heading N` N.
fn level_of(name: &str) -> Option<usize> {
    let name: String = name.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
    if name == "title" {
        return Some(1);
    }
    let n: usize = name.strip_prefix("heading")?.parse().ok()?;
    (1..=6).contains(&n).then_some(n)
}

/// Whether each list level is numbered (`decimal`), by numbering id then level.
fn lists(zip: &BoundedZip<'_>, rels: &[Rel]) -> Result<BTreeMap<String, BTreeMap<String, bool>>, ExtractRefusal> {
    related(zip, rels, "numbering", |root| {
        let levels: BTreeMap<&str, BTreeMap<String, bool>> = children(root, "abstractNum")
            .filter_map(|a| {
                let levels = children(a, "lvl")
                    .filter_map(|l| {
                        let format = children(l, "numFmt").next().and_then(|f| attr(f, "val"));
                        Some((attr(l, "ilvl")?.to_owned(), format == Some("decimal")))
                    })
                    .collect();
                Some((attr(a, "abstractNumId")?, levels))
            })
            .collect();
        children(root, "num")
            .filter_map(|n| {
                let abstract_id = children(n, "abstractNumId").next().and_then(|a| attr(a, "val"))?;
                Some((attr(n, "numId")?.to_owned(), levels.get(abstract_id)?.clone()))
            })
            .collect()
    })
}

/// The walk of the document body.
struct Walk<'a, 'l> {
    headings: BTreeMap<String, usize>,
    /// Styles whose text is hidden.
    hidden_styles: BTreeSet<String>,
    lists: BTreeMap<String, BTreeMap<String, bool>>,
    /// Image relationships: id to part.
    images: BTreeMap<String, String>,
    /// External hyperlinks: id to URL.
    links: BTreeMap<String, String>,
    doc: Doc<'l>,
    fetch: Fetch<'a, 'a>,
    /// Elements cut past [`MAX_DEPTH`].
    depth_cut: u64,
    /// What the document holds that is left out on purpose.
    left: BTreeSet<Left>,
}

/// Content left out on purpose, stated in the document in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Left {
    Hidden,
    Deleted,
    Comments,
    /// Headers and footers (`w:headerReference`, `w:footerReference` in a section).
    Margins,
    Footnotes,
    Endnotes,
}

impl Left {
    /// What the notice line says.
    fn label(self) -> &'static str {
        match self {
            Left::Hidden => "hidden text",
            Left::Deleted => "tracked deletions",
            Left::Comments => "comments",
            Left::Margins => "headers and footers",
            Left::Footnotes => "footnotes",
            Left::Endnotes => "endnotes",
        }
    }
}

/// True when section properties `sect` give a header or a footer.
fn has_margins(sect: Node<'_, '_>) -> bool {
    sect.children().any(|c| matches!(c.tag_name().name(), "headerReference" | "footerReference"))
}

/// The element children of `node`, last first (a stack pops them in order), at `depth`.
fn pushed<'a, 'i, T: Copy>(
    node: Node<'a, 'i>,
    depth: usize,
    with: T,
) -> impl Iterator<Item = (Node<'a, 'i>, usize, T)> {
    node.children().filter(Node::is_element).collect::<Vec<_>>().into_iter().rev().map(move |n| (n, depth, with))
}

/// True when the element `name` toggles on in the run or paragraph properties `props`.
fn on(props: Option<Node<'_, '_>>, name: &'static str) -> bool {
    toggle(props, name) == Some(true)
}

/// Whether the element `name` toggles on or off in the properties `props`; `None` when it
/// is not set there.
fn toggle(props: Option<Node<'_, '_>>, name: &'static str) -> Option<bool> {
    props
        .and_then(|p| children(p, name).next())
        .map(|n| !matches!(attr(n, "val"), Some("0" | "false" | "off" | "none")))
}

/// The text boxes found in a paragraph, with their depth, read after it.
type Boxes<'a, 'i> = Vec<(Node<'a, 'i>, usize)>;

/// What a run inherits from where it sits.
#[derive(Debug, Clone, Copy)]
struct Run {
    /// The link it is in, an index of the paragraph's links.
    link: Option<usize>,
    /// True when its paragraph's style hides its text.
    para_hidden: bool,
}

impl Walk<'_, '_> {
    /// True, the subtree counted as cut, when `node` is past [`MAX_DEPTH`].
    fn too_deep(&mut self, node: Node<'_, '_>, depth: usize) -> bool {
        if depth <= MAX_DEPTH {
            return false;
        }
        let count = node.descendants().filter(Node::is_element).count();
        self.depth_cut = self.depth_cut.saturating_add(count_u64(count));
        true
    }

    /// The paragraphs and tables of the body, in order.
    fn blocks(&mut self, body: Node<'_, '_>) {
        let mut stack: Vec<_> = pushed(body, 1, ()).collect();
        while let Some((node, depth, ())) = stack.pop() {
            if self.too_deep(node, depth) {
                continue;
            }
            match node.tag_name().name() {
                "p" => {
                    let boxes = self.paragraph(node, depth);
                    for (b, depth) in boxes.into_iter().rev() {
                        stack.extend(pushed(b, depth + 1, ()));
                    }
                }
                "tbl" => self.table(node, depth),
                "sectPr" if has_margins(node) => {
                    self.left.insert(Left::Margins);
                }
                "del" | "moveFrom" => {
                    self.left.insert(Left::Deleted);
                }
                name if CONTAINERS.contains(&name) => stack.extend(pushed(node, depth + 1, ())),
                _ => {}
            }
        }
    }

    /// Paragraph `p` (at `depth`) into the document; returns its text boxes.
    fn paragraph<'a, 'i>(&mut self, p: Node<'a, 'i>, depth: usize) -> Boxes<'a, 'i> {
        let mut inline = Inline::new(self.doc.room());
        let mut boxes = Vec::new();
        self.inline(p, depth, &mut inline, &mut boxes);
        let props = children(p, "pPr").next();
        if props.and_then(|pr| children(pr, "sectPr").next()).is_some_and(has_margins) {
            self.left.insert(Left::Margins);
        }
        let style = props.and_then(|pr| children(pr, "pStyle").next()).and_then(|s| attr(s, "val"));
        let numbering = props.and_then(|pr| children(pr, "numPr").next());
        let num_id = numbering.and_then(|n| children(n, "numId").next()).and_then(|n| attr(n, "val"));
        let kind = match (style.and_then(|s| self.headings.get(s)), num_id) {
            (Some(level), _) => Kind::Heading(*level),
            (None, Some(id)) if id != "0" => {
                let ilvl =
                    numbering.and_then(|n| children(n, "ilvl").next()).and_then(|l| attr(l, "val")).unwrap_or("0");
                let ordered = self.lists.get(id).and_then(|l| l.get(ilvl)).copied().unwrap_or(false);
                Kind::Item { ordered, level: ilvl.parse::<usize>().unwrap_or(0).min(8) }
            }
            _ => Kind::Para,
        };
        self.doc.paragraph(&inline, kind, self.fetch);
        boxes
    }

    /// The runs of paragraph `p` (at `depth`) into `out`, its text boxes into `boxes`.
    fn inline<'a, 'i>(&mut self, p: Node<'a, 'i>, depth: usize, out: &mut Inline, boxes: &mut Boxes<'a, 'i>) {
        let para_style =
            children(p, "pPr").next().and_then(|pr| children(pr, "pStyle").next()).and_then(|s| attr(s, "val"));
        let para_hidden = para_style.is_some_and(|s| self.hidden_styles.contains(s));
        let mut stack: Vec<_> = pushed(p, depth + 1, None).collect();
        while let Some((node, depth, link)) = stack.pop() {
            if self.too_deep(node, depth) {
                continue;
            }
            match node.tag_name().name() {
                "r" => self.run(node, depth, Run { link, para_hidden }, out, boxes),
                "hyperlink" => {
                    let url = attr(node, "id").and_then(|id| self.links.get(id));
                    let link = url.map(|u| out.link(u)).or(link);
                    stack.extend(pushed(node, depth + 1, link));
                }
                "del" | "moveFrom" => {
                    self.left.insert(Left::Deleted);
                }
                "commentRangeStart" => {
                    self.left.insert(Left::Comments);
                }
                name if CONTAINERS.contains(&name) => stack.extend(pushed(node, depth + 1, link)),
                _ => {}
            }
        }
    }

    fn run<'a, 'i>(&mut self, r: Node<'a, 'i>, depth: usize, ctx: Run, out: &mut Inline, boxes: &mut Boxes<'a, 'i>) {
        let props = children(r, "rPr").next();
        let style = props.and_then(|p| children(p, "rStyle").next()).and_then(|s| attr(s, "val"));
        let hidden = match toggle(props, "vanish") {
            Some(direct) => direct,
            None => ctx.para_hidden || style.is_some_and(|s| self.hidden_styles.contains(s)),
        };
        if hidden || on(props, "specVanish") {
            self.left.insert(Left::Hidden);
            return;
        }
        let link = ctx.link;
        let (bold, italic) = (on(props, "b"), on(props, "i"));
        for c in r.children().filter(Node::is_element) {
            match c.tag_name().name() {
                "t" => out.text(c.text().unwrap_or_default(), bold, italic, link),
                "tab" | "ptab" => out.text(" ", bold, italic, link),
                "br" | "cr" => out.line_break(),
                "delText" => {
                    self.left.insert(Left::Deleted);
                }
                "commentReference" => {
                    self.left.insert(Left::Comments);
                }
                "footnoteReference" => {
                    self.left.insert(Left::Footnotes);
                }
                "endnoteReference" => {
                    self.left.insert(Left::Endnotes);
                }
                "drawing" | "pict" | "object" | "AlternateContent" => self.pictures(c, depth + 1, out, boxes),
                _ => {}
            }
        }
    }

    /// The images of a drawing (at `depth`): each `a:blip` (or VML `v:imagedata`), described
    /// by the drawing's `descr`; its text boxes (`w:txbxContent`) go to `boxes`, their own
    /// images read with their text. Of alternate content, at any level, only the chosen branch:
    /// a `Fallback` is not entered.
    fn pictures<'a, 'i>(&mut self, drawing: Node<'a, 'i>, depth: usize, out: &mut Inline, boxes: &mut Boxes<'a, 'i>) {
        // One pass in document order; a text box is not entered: what it holds, its own
        // `docPr` included, is read with its text.
        let mut alt = None;
        let mut ids = Vec::new();
        let mut stack: Vec<_> = pushed(drawing, depth + 1, ()).collect();
        while let Some((n, at, ())) = stack.pop() {
            match n.tag_name().name() {
                "txbxContent" => boxes.push((n, at)),
                "docPr" => alt = alt.or_else(|| attr(n, "descr")),
                "blip" => ids.extend(attr(n, "embed").or_else(|| attr(n, "link"))),
                "imagedata" => ids.extend(attr(n, "id")),
                "Fallback" => {}
                _ => stack.extend(pushed(n, at + 1, ())),
            }
        }
        for id in ids {
            match self.images.get(id) {
                Some(part) => out.image(part, alt.unwrap_or("")),
                None => self.doc.skip_image(&format!("#{id}"), Skip::Unreadable),
            }
        }
    }

    /// A table: one row per `w:tr`, one cell per `w:tc`, nested tables flattened in their cell;
    /// past [`MAX_TABLE_CELLS`] cells, the rest are left out.
    fn table(&mut self, tbl: Node<'_, '_>, depth: usize) {
        let mut rows: Vec<Vec<Inline>> = Vec::new();
        let (mut used, mut cells) = (0usize, 0usize);
        let mut stack: Vec<_> = pushed(tbl, depth + 1, ()).collect();
        while let Some((node, depth, ())) = stack.pop() {
            if self.too_deep(node, depth) {
                continue;
            }
            match node.tag_name().name() {
                "tr" => {
                    if cells < MAX_TABLE_CELLS {
                        rows.push(Vec::new());
                    }
                    stack.extend(pushed(node, depth + 1, ()));
                }
                "tc" => {
                    let cell = self.cell(node, depth, self.doc.room().saturating_sub(used));
                    used = used.saturating_add(size_usize(cell.read_bytes()));
                    if cells == MAX_TABLE_CELLS {
                        self.doc.left_out(&[&cell]);
                        continue;
                    }
                    cells += 1;
                    match rows.last_mut() {
                        Some(row) => row.push(cell),
                        None => rows.push(vec![cell]),
                    }
                }
                name if CONTAINERS.contains(&name) => stack.extend(pushed(node, depth + 1, ())),
                _ => {}
            }
        }
        self.doc.table(&rows, self.fetch);
    }

    /// The text of cell `tc`, its paragraphs (nested tables' too) joined by spaces.
    fn cell(&mut self, tc: Node<'_, '_>, depth: usize, cap: usize) -> Inline {
        let mut out = Inline::new(cap);
        let mut stack: Vec<_> = pushed(tc, depth + 1, ()).collect();
        while let Some((node, depth, ())) = stack.pop() {
            if self.too_deep(node, depth) {
                continue;
            }
            match node.tag_name().name() {
                "p" => {
                    out.text(" ", false, false, None);
                    let mut boxes = Vec::new();
                    self.inline(node, depth, &mut out, &mut boxes);
                    for (b, depth) in boxes.into_iter().rev() {
                        stack.extend(pushed(b, depth + 1, ()));
                    }
                }
                "tbl" | "tr" | "tc" => stack.extend(pushed(node, depth + 1, ())),
                "del" | "moveFrom" => {
                    self.left.insert(Left::Deleted);
                }
                name if CONTAINERS.contains(&name) => stack.extend(pushed(node, depth + 1, ())),
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
#[path = "docx_tests.rs"]
mod docx_tests;
