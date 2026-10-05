// Widget de pack des tests (Rust et e2e) : une frise chronologique à liste.
// Une colonne par étape (date, point, texte), une ligne d'accent passant par les points.
#import "/oku-slides/lib.typ": widget, field, list-item, params as p, color-of, size-of
#let widget = widget("frise", label: "Frise chronologique",
  params: (etapes: p.list(label: "Étapes", item-label: "étape", min: 0, max: 8,
    item: (date: p.plain(label: "Date"), texte: p.plain(label: "Étape", default: "Nouvelle étape")),
    default: ((date: "2019", texte: "Création"), (date: "2021", texte: "Premier client"),
      (date: "2023", texte: "International"), (date: "2025", texte: "Série B")))),
  (args, ctx) => {
    let t = ctx.tokens
    let accent = color-of(t, "accent")
    set text(size: size-of(t, "body"))
    let (date-h, gap, r) = (2.4em, 0.5em, 0.45em)
    // Un champ vide (étape tout juste ajoutée) garde une boîte d'une ligne,
    // mesurée et marquée vide : l'app y pose son placeholder et l'édite.
    let cell(c, name, v) = if v == none or v == "" {
      field(c, name, box(width: 3em, hide[0]), empty: true)
    } else { field(c, name, v) }
    // Une étape : date en gras au-dessus, point, texte ; centrée dans sa colonne.
    let step(date, texte) = block(width: 100%, align(center, stack(spacing: gap,
      box(height: date-h, align(bottom, text(weight: "bold", date))),
      circle(radius: r, fill: accent),
      texte)))
    block(width: 100%, {
      // La ligne passe par le centre des points : sous la date et l'espacement.
      place(top + left, dy: date-h + gap + r, line(length: 100%, stroke: 1.5pt + accent))
      if args.etapes.len() == 0 {
        // Sans étape, la frise garde la hauteur d'une rangée : l'élément reste
        // sélectionnable et la ligne passe là où seraient les points.
        hide(step("0", "0"))
      } else {
        // Chaque item occupe toute sa colonne : sa boîte couvre l'étape centrée.
        grid(columns: (1fr,) * args.etapes.len(), column-gutter: 0.5em,
          ..args.etapes.map(e => list-item(ctx, "etapes", e,
            c => step(cell(c, "date", e.at("date", default: none)), cell(c, "texte", e.at("texte", default: none))))))
      }
    })
  })
