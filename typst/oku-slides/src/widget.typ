// Widgets : composants déclarés avec le schéma de leurs paramètres.
//
//   #import "params.typ" as p
//   #let card = widget("card", params: (title: p.plain(), body: p.rich()),
//     (args, ctx) => field(ctx, "title", args.title) + args.body)
//
// args : valeurs des paramètres, prêtes à rendre. Slot flow : entrées
// (kind, id, body) ; slot free : entrées (kind, id, frame, cell, show),
// `show(frame)` rendant l'enfant. Liste : items (id, colonnes prêtes à
// rendre), chacun rendu dans `list-item` pour que ses champs s'adressent à lui.
// ctx : id (élément ou slide), tokens, vars, style (style de l'élément),
// framed (l'élément a un cadre), load (chargeur d'image ou de graphique de
// l'hôte, ou none) : `(ctx.load)(asset: …, fit: …)` pour une image (élément
// ou slide), `(ctx.load)(chart: true)` pour un graphique dessiné par l'hôte.

#import "core.typ": _measured, text-probe

#let _types = ("plain", "rich", "flag", "number", "choice", "asset", "slot", "grid", "host", "list")

/// Déclare un widget. category : "element" ou "slide" ; description : ce
/// qu'il montre, en une phrase (le carrousel d'ajout la filtre).
#let widget(name, label: none, description: none, category: "element", params: (:), render) = {
  assert(type(name) == str and name != "", message: "oku-slides : un widget a besoin d'un nom")
  assert(category in ("slide", "element"),
    message: "oku-slides : widget « " + name + " » : catégorie inconnue « " + repr(category) + " »")
  // Un label ou une description d'un autre type casserait la lecture de tout le registre :
  // seul ce fichier est écarté, avec un message qui nomme l'argument.
  assert(label == none or type(label) == str,
    message: "oku-slides : widget « " + name + " » : le label doit être un texte")
  assert(description == none or type(description) == str,
    message: "oku-slides : widget « " + name + " » : la description doit être un texte")
  // Vide ou d'espaces : rien à montrer, comme absent.
  let label = if label == none or label.trim() == "" { none } else { label }
  let description = if description == none or description.trim() == "" { none } else { description }
  let list = ()
  for (key, d) in params {
    assert(type(d) == dictionary and d.at("type", default: none) in _types,
      message: "oku-slides : widget « " + name + " » : le paramètre « " + key + " » n'est pas un descripteur p.*")
    list.push((name: key) + d)
  }
  (name: name, schema: (name: name, label: label, description: description, category: category, params: list), render: render)
}

/// Widgets d'un registre, par nom ; un nom répété remplace le précédent.
#let by-name(registry) = {
  let d = (:)
  for w in registry { d.insert(w.name, w) }
  d
}

/// Vide : `none`, chaîne vide ou contenu vide.
#let _is-empty(body) = body == none or body == "" or body == []

/// Marqueur d'un champ vide sans place : un `metadata` sans étiquette, inerte
/// hors de `fields-stack`.
#let _is-empty-marker(it) = (type(it) == content and it.func() == metadata
  and type(it.value) == dictionary and it.value.at("oku-empty-field", default: false))

/// Champ éditable : émet la boîte de `body` pour le paramètre `name` de
/// l'objet `ctx.id` (<oku-slides-field>), avec sa typographie.
/// `body` absent (`none`, `""`) : un marqueur, que `fields-stack` pose sans
/// lui donner de place. `empty` : le contenu est rendu mais vide (corps
/// riche sans paragraphe) ; `auto` le déduit de `body`.
#let field(ctx, name, body, style: (:), empty: auto) = if ctx.id == none { body } else {
  let meta = (owner: str(ctx.id), field: ctx.at("field-prefix", default: "") + name)
  if body == none or body == "" {
    metadata(meta + (oku-empty-field: true, style: style))
  } else {
    let empty = if empty == auto { _is-empty(body) } else { empty }
    _measured(meta + (empty: empty), "oku-slides-field", body, style)
  }
}

/// Item `it` de la liste `name` : boîte <oku-slides-item> (poignées de la
/// slide) ; `body(c)` reçoit un ctx dont les champs s'adressent à l'item
/// (`<liste>/<id>/<colonne>`). Appelé avec le ctx d'un item, il rend un item
/// de sa liste imbriquée : liste `<liste>/<id>/<colonne>`, champs préfixés de même.
#let list-item(ctx, name, it, body) = if ctx.id == none { body(ctx) } else {
  let id = str(it.id)
  let list = ctx.at("field-prefix", default: "") + name
  let c = ctx + (field-prefix: list + "/" + id + "/")
  _measured((owner: str(ctx.id), list: list, item: id), "oku-slides-item", body(c), (:))
}

// Boîte d'un champ vide, posée hors flux à la position verticale courante
// (`place` sans alignement vertical) : largeur de la zone disponible.
// `place(left, …)` impose `left` au contenu posé : l'alignement du champ est
// lu avant, dans le flux (sinon un deck RTL aurait un champ vide à gauche).
#let _empty-box(v, side) = context {
  let style = (align: align.alignment) + v.style
  place(left, layout(sz => context {
    let p = here().position()
    [#metadata((owner: v.owner, field: v.field, page: p.page, x: p.x / 1pt, y: p.y / 1pt,
      w: 0, h: 0, ax: p.x / 1pt, aw: sz.width / 1pt, style: text-probe(style),
      empty: true, side: side))<oku-slides-field>]
  }))
}

/// `stack` qui accepte des champs vides (marqueurs de `field`) : ils ne
/// prennent ni place ni espacement ; leur boîte est posée au haut de l'item
/// suivant (`before`), ou au bas du dernier (`after`) ; sans item, à la place
/// de la pile (un bloc de hauteur nulle, que l'alignement ancre). Un widget qui empile
/// des champs l'utilise à la place de `stack`.
// Chaque marqueur est collé à un item voisin : un enfant de `stack` à lui
// seul recevrait l'espacement, et envelopper le `stack` (`layout`) déplace
// le rendu d'une fraction de pixel.
#let fields-stack(spacing: 0pt, ..items) = {
  let out = ()
  let pending = ()
  for it in items.pos().filter(x => x != none) {
    if _is-empty-marker(it) { pending.push(it.value) } else {
      out.push(pending.map(v => _empty-box(v, "before")).join() + it)
      pending = ()
    }
  }
  // Que des marqueurs : un bloc de hauteur nulle, que `align(bottom)` sait ancrer
  // (des `place` seuls, sans hauteur, restent au haut de la région).
  if out.len() == 0 {
    return if pending.len() > 0 { block(height: 0pt, pending.map(v => _empty-box(v, "before")).join()) }
  }
  if pending.len() > 0 {
    out.last() = out.last() + pending.map(v => _empty-box(v, "after")).join()
  }
  stack(spacing: spacing, ..out)
}
