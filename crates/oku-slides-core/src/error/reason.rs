//! Les raisons d'un refus de valeur : [`ValueReason`] (`op-value-*`), et la forme attendue d'un
//! paramètre, [`ExpectedType`] et [`Shape`] (`op-param-shape-*`).

/// Pourquoi une valeur est refusée ([`OpErrorKind::InvalidValue`](super::OpErrorKind::InvalidValue)) : dite par
/// `op-value-*` (Fluent), en anglais par `Display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValueReason {
    /// Le paramètre n'a pas de valeur.
    NoValue,
    /// Le graphique n'a pas de jeu de données.
    NoDataset,
    /// Une grille a de 1 à 24 colonnes et lignes.
    GridTracks,
    /// Un id d'item est un texte non vide, sans `/`.
    BadItemId,
    /// Chaque item a un id.
    MissingItemId,
    /// Les éléments s'imbriquent trop profondément.
    TooDeep,
    /// Chemin vide.
    EmptyPath,
    /// Un texte ou `null` attendu.
    NotText,
    /// Largeur et hauteur d'un cadre nulles ou négatives.
    EmptyFrame,
    /// Une grille cacherait les éléments d'un slot `flow`.
    BlocksOffGrid,
    /// Cellule hors de la grille.
    OutsideGrid,
    /// Jetons : un objet ou `null` attendu.
    NotTokens,
    /// Le paramètre n'est pas un slot.
    NotASlot,
    /// Nombre hors de ses bornes.
    OutOfBounds,
    /// La valeur n'a pas la forme attendue (illisible).
    Unreadable,
    /// Ancrage vertical d'un style : `top`, `middle` ou `bottom`.
    Valign,
    /// Chemin de forme mal formé (voir [`ShapePath`](crate::ShapePath)).
    Path,
}

impl ValueReason {
    /// Le nom de la raison, sa clé sans `op-value-` : le sélecteur `$reason`
    /// des messages qui la citent (`issue-invalid-style`).
    pub(crate) fn name(self) -> &'static str {
        let key = self.key();
        key.strip_prefix("op-value-").unwrap_or(key)
    }

    /// La clé Fluent de la raison (`op-value-*`).
    pub(super) fn key(self) -> &'static str {
        match self {
            ValueReason::NoValue => "op-value-no-value",
            ValueReason::NoDataset => "op-value-no-dataset",
            ValueReason::GridTracks => "op-value-grid-tracks",
            ValueReason::BadItemId => "op-value-bad-item-id",
            ValueReason::MissingItemId => "op-value-missing-item-id",
            ValueReason::TooDeep => "op-value-too-deep",
            ValueReason::EmptyPath => "op-value-empty-path",
            ValueReason::NotText => "op-value-not-text",
            ValueReason::EmptyFrame => "op-value-empty-frame",
            ValueReason::BlocksOffGrid => "op-value-blocks-off-grid",
            ValueReason::OutsideGrid => "op-value-outside-grid",
            ValueReason::NotTokens => "op-value-not-tokens",
            ValueReason::NotASlot => "op-value-not-a-slot",
            ValueReason::OutOfBounds => "op-value-out-of-bounds",
            ValueReason::Unreadable => "op-value-unreadable",
            ValueReason::Valign => "op-value-valign",
            ValueReason::Path => "op-value-path",
        }
    }
}

impl std::fmt::Display for ValueReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ValueReason::NoValue => "the parameter has no value",
            ValueReason::NoDataset => "the chart has no dataset",
            ValueReason::GridTracks => "1 to 24 columns and rows",
            ValueReason::BadItemId => "an item id is a non-empty string without /",
            ValueReason::MissingItemId => "every item needs an id",
            ValueReason::TooDeep => "elements nest too deeply",
            ValueReason::EmptyPath => "empty path",
            ValueReason::NotText => "expected a string or null",
            ValueReason::EmptyFrame => "w and h must be > 0",
            ValueReason::BlocksOffGrid => {
                "flow slots are laid out by the slide and a grid hides them; move each element to a free slot first"
            }
            ValueReason::OutsideGrid => "outside the grid",
            ValueReason::NotTokens => "an object or null",
            ValueReason::NotASlot => "not a slot",
            ValueReason::OutOfBounds => "out of bounds",
            ValueReason::Unreadable => "unreadable value",
            ValueReason::Valign => "top, middle or bottom",
            ValueReason::Path => "M, L, C, Q and Z commands with numbers, starting with M",
        })
    }
}

/// La forme qu'un paramètre attend ([`ExpectedType`]) : dite par
/// `op-param-shape-*` (Fluent), en anglais par `Display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// Une chaîne (`plain`, `asset`).
    Text,
    /// L'une des valeurs d'un `choice`.
    Choice,
    /// Une liste de paragraphes (`rich`).
    Paragraphs,
    /// Une liste d'éléments (un slot).
    Elements,
    /// Un booléen.
    Flag,
    /// Un nombre.
    Number,
    /// Un nombre dans les bornes du paramètre.
    BoundedNumber,
    /// Une grille.
    Grid,
    /// Une option de graphique.
    ChartOption,
    /// Une option de tableau.
    TableOption,
    /// Une option de graphique ou de tableau (hôte inconnu).
    ChartOrTableOption,
    /// Un chemin de forme : objet à clé `d`.
    PathOption,
    /// Une liste d'items.
    Items,
    /// Une liste (`edit_list` sur un paramètre qui n'en est pas une).
    List,
    /// Des éléments, ajoutés par `add_element` (jamais par `set_param`).
    AddElementOnly,
    /// Une valeur, donnée par `set_param` (jamais par `add_element`).
    SetParamOnly,
}

impl Shape {
    /// Chaque forme, dans l'ordre de [`Shape::position`] (tests : le `match` de `position`
    /// oblige à placer une forme ajoutée, et un test vérifie que cette liste le suit).
    #[cfg(test)]
    pub(crate) const ALL: [Shape; 16] = [
        Shape::Text,
        Shape::Choice,
        Shape::Paragraphs,
        Shape::Elements,
        Shape::Flag,
        Shape::Number,
        Shape::BoundedNumber,
        Shape::Grid,
        Shape::ChartOption,
        Shape::TableOption,
        Shape::ChartOrTableOption,
        Shape::Items,
        Shape::List,
        Shape::AddElementOnly,
        Shape::SetParamOnly,
        Shape::PathOption,
    ];

    /// Rang de la forme dans [`Shape::ALL`] ; `match` exhaustif.
    #[cfg(test)]
    pub(super) const fn position(self) -> usize {
        match self {
            Shape::Text => 0,
            Shape::Choice => 1,
            Shape::Paragraphs => 2,
            Shape::Elements => 3,
            Shape::Flag => 4,
            Shape::Number => 5,
            Shape::BoundedNumber => 6,
            Shape::Grid => 7,
            Shape::ChartOption => 8,
            Shape::TableOption => 9,
            Shape::ChartOrTableOption => 10,
            Shape::Items => 11,
            Shape::List => 12,
            Shape::AddElementOnly => 13,
            Shape::SetParamOnly => 14,
            Shape::PathOption => 15,
        }
    }

    /// Cette forme, `null` refusé.
    pub(crate) const fn required(self) -> ExpectedType {
        ExpectedType { shape: self, nullable: false }
    }

    /// Cette forme, ou `null` (le paramètre se retire).
    pub(crate) const fn or_null(self) -> ExpectedType {
        ExpectedType { shape: self, nullable: true }
    }

    /// La clé Fluent de la forme (`op-param-shape-*`).
    pub(super) fn key(self) -> &'static str {
        match self {
            Shape::Text => "op-param-shape-text",
            Shape::Choice => "op-param-shape-choice",
            Shape::Paragraphs => "op-param-shape-paragraphs",
            Shape::Elements => "op-param-shape-elements",
            Shape::Flag => "op-param-shape-flag",
            Shape::Number => "op-param-shape-number",
            Shape::BoundedNumber => "op-param-shape-bounded-number",
            Shape::Grid => "op-param-shape-grid",
            Shape::ChartOption => "op-param-shape-chart-option",
            Shape::TableOption => "op-param-shape-table-option",
            Shape::ChartOrTableOption => "op-param-shape-chart-or-table-option",
            Shape::Items => "op-param-shape-items",
            Shape::List => "op-param-shape-list",
            Shape::AddElementOnly => "op-param-shape-add-element-only",
            Shape::SetParamOnly => "op-param-shape-set-param-only",
            Shape::PathOption => "op-param-shape-path-option",
        }
    }
}

/// Ce qu'un paramètre attendait quand une valeur d'un autre type est refusée
/// ([`OpErrorKind::ParamType`](super::OpErrorKind::ParamType)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpectedType {
    /// La forme attendue.
    pub(crate) shape: Shape,
    /// `null` est aussi accepté (il retire le paramètre).
    pub(crate) nullable: bool,
}

impl std::fmt::Display for ExpectedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.shape {
            Shape::Text => "a string",
            Shape::Choice => "one of its values",
            Shape::Paragraphs => "a list of paragraphs",
            Shape::Elements => "a list of elements",
            Shape::Flag => "a boolean",
            Shape::Number => "a number",
            Shape::BoundedNumber => "a number within its bounds",
            Shape::Grid => "a grid",
            Shape::ChartOption => "a chart option",
            Shape::TableOption => "a table option",
            Shape::ChartOrTableOption => "a chart or table option",
            Shape::Items => "a list of items",
            Shape::List => "a list",
            Shape::AddElementOnly => "elements, through add_element",
            Shape::SetParamOnly => "a value, through set_param",
            Shape::PathOption => "a shape path (an object with d)",
        })?;
        if self.nullable {
            f.write_str(" or null")?;
        }
        Ok(())
    }
}

/// Why a knowledge path is refused: said by `knowledge-path-*` (Fluent), in English by `Display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathReason {
    /// The path is empty or only `/`.
    Empty,
    /// The path does not start with `/`.
    NotAbsolute,
    /// A segment is empty, has a character outside `[a-z0-9._-]`, or ends with a dot.
    BadSegment,
    /// A segment starts with a dot (`.`, `..`, `.hidden`).
    DotSegment,
    /// A segment is a Windows reserved device name.
    Reserved,
    /// More segments than allowed.
    TooDeep,
    /// More bytes than allowed.
    TooLong,
    /// A written file must end in `.md`.
    NotMarkdown,
}

impl PathReason {
    /// The Fluent key of the reason (`knowledge-path-*`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            PathReason::Empty => "knowledge-path-empty",
            PathReason::NotAbsolute => "knowledge-path-not-absolute",
            PathReason::BadSegment => "knowledge-path-bad-segment",
            PathReason::DotSegment => "knowledge-path-dot-segment",
            PathReason::Reserved => "knowledge-path-reserved",
            PathReason::TooDeep => "knowledge-path-too-deep",
            PathReason::TooLong => "knowledge-path-too-long",
            PathReason::NotMarkdown => "knowledge-path-not-markdown",
        }
    }
}

impl std::fmt::Display for PathReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PathReason::Empty => "the path is empty",
            PathReason::NotAbsolute => "the path must start with /",
            PathReason::BadSegment => "a segment is empty, ends with a dot or has a character outside a-z 0-9 . _ -",
            PathReason::DotSegment => "a segment must not start with a dot",
            PathReason::Reserved => "a segment is a reserved name",
            PathReason::TooDeep => "the path has too many segments",
            PathReason::TooLong => "the path is too long",
            PathReason::NotMarkdown => "the file must end in .md",
        })
    }
}

/// Why a knowledge file's frontmatter is refused: said by `knowledge-frontmatter-*` (Fluent), in
/// English by `Display`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrontmatterReason {
    /// The text does not open with a `---` line.
    Missing,
    /// There is no closing `---` line.
    Unterminated,
    /// The YAML block is over the byte ceiling.
    TooLarge,
    /// The YAML does not parse.
    NotYaml,
    /// The root is not a mapping, or a key is not text.
    NotMapping,
    /// A mapping repeats a key.
    DuplicateKey {
        /// The repeated key.
        key: String,
    },
    /// The block uses an anchor or an alias.
    Alias,
    /// The block uses an explicit tag.
    Tag,
    /// Mappings and sequences nest past the ceiling.
    TooDeep,
    /// A concept file has no non-empty `type`.
    TypeMissing,
    /// `generated` is not a mapping whose `at` is an RFC 3339 timestamp with an offset.
    BadTimestamp {
        /// The offending key (`generated` or `generated.at`).
        key: String,
    },
    /// The file may not carry this key (or any frontmatter).
    NotAllowed {
        /// The first key of the block.
        key: String,
    },
}

impl FrontmatterReason {
    /// The Fluent key of the reason (`knowledge-frontmatter-*`).
    #[must_use]
    pub fn key(&self) -> &'static str {
        match self {
            FrontmatterReason::Missing => "knowledge-frontmatter-missing",
            FrontmatterReason::Unterminated => "knowledge-frontmatter-unterminated",
            FrontmatterReason::TooLarge => "knowledge-frontmatter-too-large",
            FrontmatterReason::NotYaml => "knowledge-frontmatter-not-yaml",
            FrontmatterReason::NotMapping => "knowledge-frontmatter-not-mapping",
            FrontmatterReason::DuplicateKey { .. } => "knowledge-frontmatter-duplicate-key",
            FrontmatterReason::Alias => "knowledge-frontmatter-alias",
            FrontmatterReason::Tag => "knowledge-frontmatter-tag",
            FrontmatterReason::TooDeep => "knowledge-frontmatter-too-deep",
            FrontmatterReason::TypeMissing => "knowledge-frontmatter-type-missing",
            FrontmatterReason::BadTimestamp { .. } => "knowledge-frontmatter-bad-timestamp",
            FrontmatterReason::NotAllowed { .. } => "knowledge-frontmatter-not-allowed",
        }
    }

    /// The key the reason names, when it carries one (the `$key` of its message).
    #[must_use]
    pub fn subject(&self) -> Option<&str> {
        match self {
            FrontmatterReason::DuplicateKey { key }
            | FrontmatterReason::BadTimestamp { key }
            | FrontmatterReason::NotAllowed { key } => Some(key),
            _ => None,
        }
    }

    /// The reason said in the locale of `l`, with its `$key` argument when it has one.
    #[must_use]
    pub fn message(&self, l: &oku_slides_i18n::Localizer) -> String {
        match self.subject() {
            Some(key) => l.tr_with(self.key(), &[("key", key.into())]),
            None => l.tr(self.key()),
        }
    }
}

impl std::fmt::Display for FrontmatterReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrontmatterReason::Missing => f.write_str("the file must start with a --- frontmatter block"),
            FrontmatterReason::Unterminated => f.write_str("the frontmatter block has no closing ---"),
            FrontmatterReason::TooLarge => f.write_str("the frontmatter block is too large"),
            FrontmatterReason::NotYaml => f.write_str("the frontmatter is not valid YAML"),
            FrontmatterReason::NotMapping => f.write_str("the frontmatter must be a mapping with text keys"),
            FrontmatterReason::DuplicateKey { key } => write!(f, "the frontmatter key \"{key}\" appears twice"),
            FrontmatterReason::Alias => f.write_str("the frontmatter must not use YAML anchors or aliases"),
            FrontmatterReason::Tag => f.write_str("the frontmatter must not use YAML tags"),
            FrontmatterReason::TooDeep => f.write_str("the frontmatter is nested too deeply"),
            FrontmatterReason::TypeMissing => f.write_str("the frontmatter needs a non-empty type"),
            FrontmatterReason::BadTimestamp { key } => {
                write!(f, "the frontmatter value \"{key}\" must be an RFC 3339 date-time with an offset")
            }
            FrontmatterReason::NotAllowed { key } => {
                write!(f, "the frontmatter key \"{key}\" is not allowed in this file")
            }
        }
    }
}

/// A knowledge ceiling a bundle may pass on read: said by its [`KnowledgeReason`] key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeLimit {
    /// More files than allowed.
    TooManyFiles,
    /// A file over the byte ceiling.
    FileTooLarge,
    /// The bundle over the byte ceiling.
    TotalTooLarge,
}

impl KnowledgeLimit {
    /// The Fluent key of the ceiling: that of the matching [`KnowledgeReason`].
    #[must_use]
    pub fn key(self) -> &'static str {
        KnowledgeReason::from(self).key()
    }
}

impl From<KnowledgeLimit> for KnowledgeReason {
    fn from(limit: KnowledgeLimit) -> Self {
        match limit {
            KnowledgeLimit::TooManyFiles => KnowledgeReason::TooManyFiles,
            KnowledgeLimit::FileTooLarge => KnowledgeReason::FileTooLarge,
            KnowledgeLimit::TotalTooLarge => KnowledgeReason::TotalTooLarge,
        }
    }
}

/// Why a knowledge op is refused: said by `knowledge-reason-*` (Fluent), in English by `Display`.
///
/// The `Path` and `Frontmatter` reasons are said by their own keys.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KnowledgeReason {
    /// The path is refused.
    Path(PathReason),
    /// The text's frontmatter is refused.
    Frontmatter(FrontmatterReason),
    /// No file at this path.
    NotFound,
    /// A file already sits at this path.
    Exists,
    /// A new file needs an id.
    IdMissing,
    /// The id is not 32 lowercase hex digits.
    IdInvalid,
    /// Another file already has this id.
    IdTaken,
    /// The file at this path has another id.
    IdMismatch,
    /// An edit needs a non-empty text to replace.
    EditEmptyOld,
    /// The text to replace is not in the file.
    EditNoMatch,
    /// The text to replace is in the file more than once.
    EditAmbiguous,
    /// The bundle already has the most files allowed.
    TooManyFiles,
    /// The file would be over the byte ceiling.
    FileTooLarge,
    /// The bundle would be over the byte ceiling.
    TotalTooLarge,
}

impl KnowledgeReason {
    /// The Fluent key of the reason: `knowledge-reason-*`, or the inner reason's own key.
    #[must_use]
    pub fn key(&self) -> &'static str {
        match self {
            KnowledgeReason::Path(reason) => reason.key(),
            KnowledgeReason::Frontmatter(reason) => reason.key(),
            KnowledgeReason::NotFound => "knowledge-reason-not-found",
            KnowledgeReason::Exists => "knowledge-reason-exists",
            KnowledgeReason::IdMissing => "knowledge-reason-id-missing",
            KnowledgeReason::IdInvalid => "knowledge-reason-id-invalid",
            KnowledgeReason::IdTaken => "knowledge-reason-id-taken",
            KnowledgeReason::IdMismatch => "knowledge-reason-id-mismatch",
            KnowledgeReason::EditEmptyOld => "knowledge-reason-edit-empty-old",
            KnowledgeReason::EditNoMatch => "knowledge-reason-edit-no-match",
            KnowledgeReason::EditAmbiguous => "knowledge-reason-edit-ambiguous",
            KnowledgeReason::TooManyFiles => "knowledge-reason-too-many-files",
            KnowledgeReason::FileTooLarge => "knowledge-reason-file-too-large",
            KnowledgeReason::TotalTooLarge => "knowledge-reason-total-too-large",
        }
    }

    /// The reason said in the locale of `l`.
    #[must_use]
    pub fn message(&self, l: &oku_slides_i18n::Localizer) -> String {
        match self {
            KnowledgeReason::Frontmatter(reason) => reason.message(l),
            _ => l.tr(self.key()),
        }
    }
}

impl std::fmt::Display for KnowledgeReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnowledgeReason::Path(reason) => reason.fmt(f),
            KnowledgeReason::Frontmatter(reason) => reason.fmt(f),
            KnowledgeReason::NotFound => f.write_str("no file at this path"),
            KnowledgeReason::Exists => f.write_str("a file already exists at this path"),
            KnowledgeReason::IdMissing => f.write_str("a new file needs an id"),
            KnowledgeReason::IdInvalid => f.write_str("a file id is 32 lowercase hexadecimal digits"),
            KnowledgeReason::IdTaken => f.write_str("another file already has this id"),
            KnowledgeReason::IdMismatch => f.write_str("the file at this path has another id"),
            KnowledgeReason::EditEmptyOld => f.write_str("the text to replace must not be empty"),
            KnowledgeReason::EditNoMatch => f.write_str("the text to replace is not in the file"),
            KnowledgeReason::EditAmbiguous => f.write_str("the text to replace is in the file more than once"),
            KnowledgeReason::TooManyFiles => f.write_str("the bundle has too many files"),
            KnowledgeReason::FileTooLarge => f.write_str("the file is too large"),
            KnowledgeReason::TotalTooLarge => f.write_str("the bundle is too large"),
        }
    }
}

#[cfg(test)]
#[path = "reason_tests.rs"]
mod reason_tests;
