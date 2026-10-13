// Descripteurs des paramètres d'un widget, à importer comme module :
// `#import "params.typ" as p`, puis `p.plain(label: "Titre")`.
// Chaque descripteur est un dictionnaire sérialisable, lu par l'hôte ; il
// ne porte que les clés de son type.

#let _base(type, label, default) = (type: type, label: label, default: default)

/// Texte sur une ligne. bind : clé de `deck.params` où lire et écrire la valeur.
#let plain(label: none, default: none, bind: none) = _base("plain", label, default) + (bind: bind)
/// Paragraphes à runs.
#let rich(label: none) = _base("rich", label, none)
#let flag(label: none, default: false) = _base("flag", label, default)
#let number(label: none, default: none, min: none, max: none) = _base("number", label, default) + (min: min, max: max)
/// Une valeur parmi `values` ; par défaut la première.
#let choice(..values, label: none, default: auto) = {
  let vs = values.pos()
  _base("choice", label, if default == auto { vs.at(0, default: none) } else { default }) + (values: vs)
}
/// Identifiant d'asset `sha256:…`.
#let asset(label: none) = _base("asset", label, none)
/// Éléments enfants. mode : "flow" (agencés par le widget) ou "free" (sur
/// leur cadre ou leur cellule). accepts : noms de widgets admis, ou none.
#let slot(label: none, mode: "flow", accepts: none) = _base("slot", label, none) + (mode: mode, accepts: accepts)
/// Liste d'items aux colonnes typées (`item` : dictionnaire de descripteurs),
/// saisie en tableau dans le dock. item-label : nom d'un item (« étape »).
#let list(label: none, item-label: none, item: (:), min: 0, max: none, default: none) = (
  _base("list", label, default)
    + (item-label: item-label, item: item.pairs().map(((k, d)) => (name: k) + d), min: min, max: max)
)
/// Grille { columns, rows, gap }.
#let grid(label: none, default: none) = _base("grid", label, default)
/// Type fourni et validé par l'hôte (`"chart"`, `"table"` ou `"path"`).
#let host(name, label: none) = _base("host", label, none) + (host: name)
