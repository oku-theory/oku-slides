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

#[cfg(test)]
#[path = "reason_tests.rs"]
mod reason_tests;
