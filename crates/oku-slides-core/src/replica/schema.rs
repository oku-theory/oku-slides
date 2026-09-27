//! Catégorie de chaque champ du deck dans le replica, selon son contexte.

/// Où l'on se trouve dans le deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ctx {
    Deck,
    Params,
    Slide,
    Element,
    /// Chaque variable de `vars` est une feuille LWW, quelle que soit sa forme
    /// JSON (spec : chaque entrée de `vars` est LWW, jamais fusionnée champ à
    /// champ même si elle contient un objet).
    Vars,
    /// `tokens`, `style`, `frame`, `cell`, `grid`, `layout` de groupe, extras
    /// de `params` : cartes à feuilles LWW.
    Opaque,
}

/// Ce que devient un champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// `LoroText`, fusion au caractère.
    Prose,
    /// `LoroText` à marques (paragraphes).
    Rich,
    /// `LoroMovableList` de `LoroMap` à `id`.
    Collection(Ctx),
    /// Une valeur JSON entière, LWW.
    Atom,
    /// Objet → `LoroMap` ; autre valeur → LWW.
    Nested(Ctx),
}

impl Ctx {
    pub(super) fn kind(self, key: &str) -> Kind {
        use Ctx::*;
        match (self, key) {
            (Deck, "slides") => Kind::Collection(Slide),
            (Deck, "params") => Kind::Nested(Params),
            (Deck, "vars") => Kind::Nested(Vars),
            (Vars, _) => Kind::Atom,
            (Params, "title" | "subtitle" | "author" | "date" | "organization") => Kind::Prose,
            (Slide, "name" | "description" | "title" | "subtitle" | "kicker" | "text" | "notes") => Kind::Prose,
            (Slide, "blocks" | "elements") => Kind::Collection(Element),
            (Element, "title" | "kicker") => Kind::Prose,
            (Element, "text") => Kind::Rich,
            (Element, "children") => Kind::Collection(Element),
            // Invariant global (`consistent_chart`) : un graphique est un atome.
            (Element, "option") => Kind::Atom,
            _ => Kind::Nested(Opaque),
        }
    }

    /// Textes créés dès la création de l'objet (spec : « toujours présents »).
    /// Prose simple ([`Kind::Prose`]) comme prose riche ([`Kind::Rich`]).
    pub(super) fn texts(self) -> &'static [&'static str] {
        match self {
            Ctx::Params => &["title", "subtitle", "author", "date", "organization"],
            Ctx::Slide => &["name", "description", "title", "subtitle", "kicker", "text", "notes"],
            Ctx::Element => &["title", "kicker", "text"],
            Ctx::Deck | Ctx::Vars | Ctx::Opaque => &[],
        }
    }

    /// Collections créées dès la création de l'objet.
    pub(super) fn lists(self) -> &'static [&'static str] {
        match self {
            Ctx::Deck => &["slides"],
            Ctx::Slide => &["blocks", "elements"],
            Ctx::Element => &["children"],
            Ctx::Params | Ctx::Vars | Ctx::Opaque => &[],
        }
    }
}
