// Widgets de slide du noyau (le lot 3 les déplace) ; les widgets d'élément
// sont dans le pack `base` (`packs/base`). `registry` est lu par l'hôte
// (sonde <oku-widgets>) et par le rendu du deck.

#import "params.typ" as p
#import "widget.typ": widget, field, _is-empty
#import "slides.typ": slide, title-slide, chapter, toc, end-slide, blank-slide
#import "zone.typ": free-area, zone-marker, default-grid

// Champs d'une slide : `wrap` des fonctions de slide, posé par le widget.
#let _fields(ctx) = (name, body, style) => field(ctx, name, body, style: style)

#let _title = widget("title", category: "slide",
  params: (
    title: p.plain(bind: "title"),
    subtitle: p.plain(bind: "subtitle"),
    author: p.plain(bind: "author"),
    date: p.plain(bind: "date"),
    kicker: p.plain(bind: "organization"),
  ),
  (args, ctx) => title-slide(title: args.title, subtitle: args.subtitle, author: args.author,
    date: args.date, kicker: args.kicker, id: ctx.id, wrap: _fields(ctx)))

#let _toc = widget("toc", category: "slide", params: (title: p.plain()),
  // Seul `title` est un paramètre : pas de champ pour le surtitre de `slide`.
  (args, ctx) => toc(title: if args.title == none { auto } else { args.title }, id: ctx.id,
    wrap: (name, body, style) => if name == "title" { field(ctx, name, body, style: style) } else { body }))

#let _chapter = widget("chapter", category: "slide", params: (title: p.plain(), subtitle: p.plain()),
  (args, ctx) => chapter(if args.title == none { "" } else { args.title }, subtitle: args.subtitle, id: ctx.id,
    // Un titre vide rend un en-tête vide (le sommaire en dépend) : champ mesuré, vide.
    wrap: (name, body, style) => field(ctx, name, body, style: style,
      empty: if name == "title" { _is-empty(args.title) } else { auto })))

#let _content = widget("content", category: "slide",
  params: (
    kicker: p.plain(),
    title: p.plain(),
    layout: p.choice("default", "columns", "split", "center", "quote"),
    grid: p.grid(default: default-grid),
    blocks: p.slot(mode: "flow"),
    elements: p.slot(mode: "free"),
  ),
  (args, ctx) => {
    let t = ctx.tokens
    // Sans blocs mis en page, la zone de corps est libre, sur sa grille.
    let free = args.blocks.len() == 0
    slide(
      // Slide à blocs : la grille existe aussi (l'app y accroche un bloc
      // dès qu'on le déplace, et la slide passe alors sur la grille).
      zone: if free { none } else { zone-marker(t, args.grid) },
      title: args.title, kicker: args.kicker, layout: args.layout, id: ctx.id, wrap: _fields(ctx),
      // Un bloc prend la hauteur selon le `grow` de son widget (voir `_layout`).
      grow: if free { () } else { args.blocks.map(b => b.grow) },
      ..if free { (free-area(t, args.elements, grid: args.grid),) } else { args.blocks.map(b => b.body) },
    )
  })

#let _end = widget("end", category: "slide", params: (text: p.plain()),
  (args, ctx) => end-slide(if args.text == none { "" } else { args.text }, id: ctx.id,
    wrap: (name, body, style) => field(ctx, name, body, style: style, empty: _is-empty(args.text))))

#let _blank = widget("blank", category: "slide",
  params: (grid: p.grid(default: default-grid), elements: p.slot(mode: "free")),
  (args, ctx) => blank-slide(free-area(ctx.tokens, args.elements, grid: args.grid), id: ctx.id))

#let registry = (_title, _toc, _chapter, _content, _end, _blank)
