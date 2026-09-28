// Widgets : composants déclarés avec le schéma de leurs paramètres.
//
//   #import "params.typ" as p
//   #let card = widget("card", params: (title: p.plain(), body: p.rich()),
//     (args, ctx) => field(ctx, "title", args.title) + args.body)
//
// args : valeurs des paramètres, prêtes à rendre. Slot flow : entrées
// (kind, id, body) ; slot free : entrées (kind, id, frame, cell, show),
// `show(frame)` rendant l'enfant.
// ctx : id (élément ou slide), tokens, vars, style (style de l'élément),
// framed (l'élément a un cadre), load (chargeur d'image ou de graphique de
// l'hôte, fonction sans argument, ou none).

#import "core.typ": _measured

#let _types = ("plain", "rich", "flag", "number", "choice", "asset", "slot", "grid", "host")

/// Déclare un widget. category : "element" ou "slide".
#let widget(name, label: none, category: "element", params: (:), render) = {
  assert(type(name) == str and name != "", message: "oku-slides : un widget a besoin d'un nom")
  assert(category in ("slide", "element"),
    message: "oku-slides : widget « " + name + " » : catégorie inconnue « " + repr(category) + " »")
  let list = ()
  for (key, d) in params {
    assert(type(d) == dictionary and d.at("type", default: none) in _types,
      message: "oku-slides : widget « " + name + " » : le paramètre « " + key + " » n'est pas un descripteur p.*")
    list.push((name: key) + d)
  }
  (name: name, schema: (name: name, label: label, category: category, params: list), render: render)
}

/// Widgets d'un registre, par nom ; un nom répété remplace le précédent.
#let by-name(registry) = {
  let d = (:)
  for w in registry { d.insert(w.name, w) }
  d
}

/// Champ éditable : émet la boîte de `body` pour le paramètre `name` de
/// l'objet `ctx.id` (<oku-slides-field>), avec sa typographie.
#let field(ctx, name, body, style: (:)) = if ctx.id == none { body } else {
  _measured((owner: str(ctx.id), field: name), "oku-slides-field", body, style)
}
