// Descriptors of a widget's parameters, imported as a module:
// `#import "params.typ" as p`, then `p.plain(label: "Title", help: "Above the body")`.
// `label` and `help` (one line under the field) are a text or a dictionary of texts by language.
// Each descriptor is a serializable dictionary, read by the host; it
// carries only the keys of its type.

// `help` is written only when given, so a descriptor without one keeps its serialized form.
#let _base(type, label, help, default) = (type: type, label: label, default: default) + if help == none { (:) } else { (help: help) }

/// Texte sur une ligne. bind : clé de `deck.params` où lire et écrire la valeur.
#let plain(label: none, help: none, default: none, bind: none) = _base("plain", label, help, default) + (bind: bind)
/// Paragraphes à runs.
#let rich(label: none, help: none) = _base("rich", label, help, none)
#let flag(label: none, help: none, default: false) = _base("flag", label, help, default)
#let number(label: none, help: none, default: none, min: none, max: none) = _base("number", label, help, default) + (min: min, max: max)
/// Une valeur parmi `values` ; par défaut la première.
#let choice(..values, label: none, help: none, default: auto) = {
  let vs = values.pos()
  _base("choice", label, help, if default == auto { vs.at(0, default: none) } else { default }) + (values: vs)
}
/// Identifiant d'asset `sha256:…`.
#let asset(label: none, help: none) = _base("asset", label, help, none)
/// Éléments enfants. mode : "flow" (agencés par le widget) ou "free" (sur
/// leur cadre ou leur cellule). accepts : noms de widgets admis, ou none.
#let slot(label: none, help: none, mode: "flow", accepts: none) = _base("slot", label, help, none) + (mode: mode, accepts: accepts)
/// Liste d'items aux colonnes typées (`item` : dictionnaire de descripteurs),
/// saisie en tableau dans le dock. item-label : nom d'un item (« étape »).
#let list(label: none, help: none, item-label: none, item: (:), min: 0, max: none, default: none) = (
  _base("list", label, help, default)
    + (item-label: item-label, item: item.pairs().map(((k, d)) => (name: k) + d), min: min, max: max)
)
/// Grille { columns, rows, gap }.
#let grid(label: none, help: none, default: none) = _base("grid", label, help, default)
/// Type fourni et validé par l'hôte (`"chart"`, `"table"` ou `"path"`).
#let host(name, label: none, help: none) = _base("host", label, help, none) + (host: name)
