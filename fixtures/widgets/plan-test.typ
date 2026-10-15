// Widget de pack des tests (Rust et e2e) : un plan d'action, étapes à sous-tâches (liste
// imbriquée, t-495). Une colonne par étape : son titre, puis ses tâches, cochées ou non.
#import "/oku-slides/lib.typ": widget, field, list-item, params as p, color-of, size-of
#let widget = widget("plan", label: "Plan d'action",
  params: (etapes: p.list(label: "Étapes", item-label: "étape", max: 6,
    item: (
      titre: p.plain(label: "Étape", default: "Nouvelle étape"),
      taches: p.list(label: "Tâches", item-label: "tâche", max: 8,
        item: (texte: p.plain(label: "Tâche", default: "Nouvelle tâche"), fait: p.flag(label: "Fait")),
        default: ((texte: "Première tâche"),)),
    ),
    default: ((titre: "Cadrer", taches: ((texte: "Objectifs"), (texte: "Équipe", fait: true))),
      (titre: "Livrer", taches: ((texte: "Prototype"),))))),
  (args, ctx) => {
    let t = ctx.tokens
    let accent = color-of(t, "accent")
    set text(size: size-of(t, "body"))
    // Un champ vide (item tout juste ajouté) garde une boîte d'une ligne, éditable.
    let cell(c, name, v) = if v == none or v == "" {
      field(c, name, box(width: 3em, hide[0]), empty: true)
    } else { field(c, name, v) }
    // Une tâche : sa case (pleine si faite), puis son texte.
    let task(c, x) = grid(columns: (0.8em, 1fr), column-gutter: 0.4em, align: horizon,
      box(width: 0.7em, height: 0.7em, stroke: 1pt + accent, fill: if x.fait == true { accent }),
      cell(c, "texte", x.texte))
    // Une étape : son titre en gras, puis ses tâches, chacune item de la liste imbriquée.
    let step(c, e) = stack(spacing: 0.6em,
      text(weight: "bold", fill: accent, cell(c, "titre", e.titre)),
      ..e.taches.map(x => list-item(c, "taches", x, cc => task(cc, x))))
    if args.etapes.len() == 0 {
      // Sans étape, le plan garde la hauteur d'une ligne : l'élément reste sélectionnable.
      hide(text(weight: "bold", "0"))
    } else {
      grid(columns: (1fr,) * args.etapes.len(), column-gutter: 1em,
        ..args.etapes.map(e => list-item(ctx, "etapes", e, c => step(c, e))))
    }
  })
