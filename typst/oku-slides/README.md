# oku-slides — Typst library

16:9 slides (1920 × 1080 pt) with a fixed structure: title slide, table of
contents, chapters, content slides with header and footer, end slide. All
styling goes through **tokens**; the content can be written in Typst or supplied
as **JSON** (the data model of the Oku Slides app).

## Getting started

```typ
#import "lib.typ": *

#show: deck.with(
  theme: papier,                       // jeu de jetons (optionnel)
  tokens: (color: (accent: "#8a5a1c")), // surcharges ponctuelles
  title: "Feuille de route {{annee}}",
  author: "Matthieu",
  date: datetime.today(),
  organization: "Darticorp",
  vars: (client: "Acme", annee: 2027),
)

#title-slide()
#toc()
#chapter("Contexte")
#slide(title: "Trois priorités", layout: "columns")[Fiabilité][Vitesse][Clients #var("client")]
#end-slide[Merci]
```

From a JSON: `#render-deck(json("deck.json"), theme: papier)`
(see `examples/deck.json` and [Deck JSON](#deck-json-v4)).

## Slide types

| Function | Role | Chrome |
| --- | --- | --- |
| `title-slide(title, subtitle, author, date, kicker, body)` | Cover; defaults = deck parameters | no |
| `toc(title, pages)` | Table of contents of the chapters, clickable links | yes |
| `chapter(title, subtitle)` | Opens a chapter (level-1 heading): feeds the table of contents and the header | no |
| `slide(title, kicker, layout, ratio, ..corps)` | Content; title = level-2 heading | yes |
| `end-slide(body)` | End slide | no |
| `blank-slide(body, chrome)` | Free page | optional |
| `card(title, kicker, highlight, body)` | Card with an accent rule; `highlight` puts it on a dark background | — |

`slide` layouts: `default` (stacked), `columns` (one body per column),
`split` (two bodies, `ratio: (2fr, 1fr)`), `center`, `quote`.

## Deck parameters

`title`, `subtitle`, `author`, `date` (string or `datetime`, formatted in
French), `organization`, `logo`, `lang`, `numbering` (`"{n} / {total}"`,
`"{n}"` or `none`), `chapter-numbering` (`"1"`, `"01"`, `"I"`… or `none`),
`toc-title`, `header` and `footer` (`auto`, `none`, or a function
`ctx => content`).

`ctx` contains: `title`, `chapter`, `chapter-number`, `page`, `total`,
`params`, `vars`, `tokens`.

## Variables

`vars: (client: "Acme")` at the deck level, then `#var("client")` in the
content, or `{{client}}` in titles, parameters and JSON texts.

## Tokens

JSON primitives only, deep-merged:
`default-tokens` ← `theme` ← `tokens`. The defaults are in
`src/default-tokens.json` (`null` = undefined token, derived from the background
and the ink for `app`).

| Family | Example | Values |
| --- | --- | --- |
| `color` | `accent: "#2f5fb3"` | hex, or the name of another color |
| `font` | `display: ("Newsreader",)` | fallback families |
| `size`, `space`, `radius`, `stroke` | `body: 32` | pt, or the name of another token of the family |
| `weight`, `leading` | `strong: 600`, `normal: 0.6` | weight; em |
| `script-style` | `display: "serif"` | style (`"sans"`, `"serif"`) of the non-Latin fallback fonts (CJK…), per role |
| `page` | `margin-x: 120` | page geometry and header/footer positions |
| `chart` | `palette: ("accent", "#c4622d")` | series colors, in order |
| `app`, `app-dark`, `app-light` | `desk: "#ece8e0"` | colors, radii (px) and elevations of the app interface, not of the slides |
| `component.*` | `chapter: (bg: "accent")` | style of each component, by references |

Components: `header`, `footer`, `kicker`, `title`, `title-slide`, `chapter`,
`toc`, `card`, `end-slide`, `list`, `quote`, `columns`, `table`.

Every text component accepts `size`, `color`, `weight`, `font`,
`tracking` (pt) and `case` (`"upper"`, `"lower"`). In the app's theme editor,
a component's `font` only offers the roles (`"display"`, `"body"`,
`"mono"`) and `weight` only the numeric weights (100 to 900) or the names of
the `weight` tokens (`"strong"`…): Typst's named weights (`"bold"`) are not
offered, even though the library accepts them.

- **Header and footer**: `left` and `right` are templates, for example
  `"{n} · {title}"` or `"{confidential}"`. Fields: `title`, `subtitle`,
  `author`, `date`, `organization`, `chapter`, `chapter-number`,
  `chapter-label`, `n`, `total`, `page-label`, plus any deck variable. A
  segment (separated by ` · `) with an empty field disappears.
- **Slide title**: `rule: (width, height, color)` draws a rule between the
  kicker and the title.
- **Cover**: `layout: "plain"` or `"band"` (light banner at the top with
  `top-left`, `top-right`, `badge`, then a dark band `band-bg` that takes up
  `band-ratio` of the height).
- **Card**: `height: "auto"` or `"fill"`.
- **List**: `marker` (the bullet, one character) in `marker-color`; `null`
  draws it in the text color (`list-marker`).
- **Table**: `body-bg` paints the body rows outside stripes (`stripe-bg`);
  `null` (the default) leaves them without a background.

## Bundled themes

Each theme is a file `themes/<name>.json`:
`{ id, name, description, parent, created, tokens }`. `lib.typ` exports its
`tokens` under the theme's name: `theme: papier` and `theme: pro` are written
as before, and any token dictionary works too.

- `papier`: the Papier design system (Newsreader, Instrument Sans, sage green).
- `pro`: the Template Pro design system (Inter, navy and cyan, banded cover,
  uppercase kickers, footer “n · title” and “CONFIDENTIAL”).
  See `examples/pro.typ`.

In the app, these two files are the built-in themes (fixed identifiers);
the user creates versioned copies of them (the `themes/` folder of the app
data) in the “Theme” tab of templates mode. A deck embeds the version of its
theme.

### Theme notes

JSON has no comments: what used to explain the values is here.

- **Pro** translates the Template Pro design system: its px values are
  pt on the 1920 × 1080 grid. Mapping: `color.bg` = surface,
  `surface` and `rule` = border (background of the placeholder frames), `ink` =
  body-text, `ink-muted` = muted-text, `accent` = accent (strokes, borders,
  badge), `on-accent` = on-accent-text; `size.display` = cover-title,
  `title` = slide-title, `subtitle` = cover-subline, `lead` = card-title,
  `kicker` = eyebrow. The other colors (`brand-ink`, `surface-dark`,
  `accent-text`, `accent-on-dark`, `on-dark-body`, `on-dark-muted`, `white`)
  are specific to the system. Charts: the system's blues (accent text,
  cyan, navy), then their light tones. Interface: light desk (“border”
  tint), navy ink, cyan accent; in dark, the navy desk of the
  chapters, cyan accent.
- **Papier** takes over the tokens of the Papier design system (day). Serif
  titles (Newsreader): `script-style.display` is `"serif"`, to keep a serif
  in Japanese, Chinese and Korean. Charts: the accent first, then muted warm
  paper tones. Dark interface: the warm Papier de nuit, with the accent
  lightened for text.
- **Defaults** (`src/default-tokens.json`): non-Latin scripts go
  through `script-fallback`, added by `deck`; `script-style` is `"sans"` for
  each role. `component.card.height`: `"auto"` (content height) or
  `"fill"` (all the available height). `app`: `null` derives the color from the
  slides' background and ink.

## Deck JSON (v4)

Each slide and each element names its widget (`kind`) and carries its
parameters (`params`), described by its schema (see [Widgets](#widgets)).
A slot (`blocks`, `elements`, `children`) is an array of elements; an
element of a free slot has a `frame` `{ x, y, w, h }` (pt, relative to the
body area) or a `cell` of the slide's grid.

```json
{
  "version": 5,
  "params": { "title": "Feuille de route", "subtitle": "Pour {{client}}", "author": "Matthieu" },
  "vars": { "client": "Acme" },
  "slides": [
    { "id": "s1", "kind": "title" },
    { "id": "s2", "kind": "chapter", "params": { "title": "Contexte" } },
    { "id": "s3", "kind": "content", "params": { "title": "Trois priorités", "layout": "columns",
      "blocks": [
        { "id": "c1", "kind": "card", "params": { "title": "Fiabilité",
          "body": [ { "runs": [ { "t": "Diviser par deux les incidents" } ] } ] } }
      ] } },
    { "id": "s4", "kind": "blank", "params": { "elements": [
      { "id": "e1", "kind": "text", "frame": { "x": 40, "y": 40, "w": 600, "h": 200 },
        "params": { "body": [ { "runs": [ { "t": "Posé à 40 pt du coin" } ] } ] } }
    ] } },
    { "id": "s5", "kind": "end", "params": { "text": "Merci {{client}}" } }
  ]
}
```

An element also carries an optional `style` (size, color, weight,
font, `tracking` (letter spacing, in pt), alignment, background, stroke,
`radius`…). The `text` widget additionally reads `valign` (`"top"`, `"middle"`,
`"bottom"`: anchoring in the frame, framed elements only), `inset`
(four margins in pt: top, right, bottom, left) and `spacing` (space between
two paragraphs, list items included, in pt: from the last baseline
of one to the top of the first line of the next; without the key, the theme's)
and `indent` (two numbers in pt: the left margin of the paragraphs, then the
offset of their first line; negative, a hanging indent where the bullet or
number of an item fits, its text starting at the margin).
The `shape` widget draws a
path with `shape: "path"` and `path: { "d": "M 0 0 L 1 0 L 0.5 1 Z" }`:
commands `M`, `L`, `C`, `Q`, `Z`, each letter and number separated by
whitespace, coordinates from 0 to 1 stretched to the frame, non-zero fill.

`tokens` (optional) overrides the theme's tokens for this deck. `theme`
(optional) is the `{ id, version }` reference of an app theme: the host
resolves it and passes its tokens to `render-deck(…, theme:)`; the library
does not read it.

Bound parameters (`bind`, like the title and subtitle of the `title`
slide) are read from the deck's `params`, not from the slide's.

`render-deck(data, theme: none, image-loader: none, widgets: none)` :

- `image-loader`: function `(element, asset: none, fit: "cover", chart: false)
  => content`, supplied by the host for images (`asset`, `fit`) and for the
  charts it draws (`chart: true`); without it, a gray frame.
- `widgets`: name → widget dictionary, typically
  `by-name(base + pack)`; `none` renders with the base widgets. A `kind`
  with no widget of the right category gives an “unknown element” box
  (element) or an “unknown slide” page (slide).

## Introspection

Each JSON element with an `id` is tagged `<e-ID>`:
`query(<e-c1>).first().location().position()` gives its actual position, for
the app's selection layer. Its box is emitted under `<oku-slides-box>`.

Editable texts are emitted under `<oku-slides-field>` by `field()`
(see below), with `owner` (id of the element or slide) and `field`
(parameter name). Slides too, one field per displayed `plain` parameter,
empty or not: `kicker` (`plain` layout only), `title` and
`subtitle` for the title slide, `title` and `subtitle` for chapters,
`kicker` and `title` for content slides, `title` for the table of contents,
`text` for the end slide. The slide functions
(`slide`, `title-slide`, `chapter`, `toc`, `end-slide`) take a
`wrap: (name, content, style) => content` for this, like `card`. A pack widget
that passes `wrap` to `slide()` thus receives the `kicker` and `title` fields; the
base `_toc` filters out `kicker`, which is not one of its parameters.

## Widgets

A **widget** is a component declared with the schema of its parameters; the
base registry (14 widgets : `text`, `card`, `image`, `shape`, `chart`,
`table`, `stack`, `canvas`, `title`, `toc`, `chapter`, `content`, `end`, `blank`) is
read by the host (`<oku-widgets>` probe) and is used to render the JSON deck.

A **template pack** can add its own widgets, or override those of the
base: **one file per widget**, `widgets/<name>.typ`, whose `widget` binding
is the result of `widget("<name>", …)` (the declared name is the file's,
`[a-z][a-z0-9-]*`). The optional file `widgets/_commun.typ` holds the shared
code: a widget imports it with `#import "_commun.typ": …`; it is not a
widget. The app generates the pack's `registry` (an assembly module that
imports each widget); a widget that does not compile is dropped from it with its
diagnostics, the others remain.

Example: an indicator, `widgets/kpi.typ` and `widgets/_commun.typ`.

```typ
// Code partagé des widgets du pack : `#import "_commun.typ": carte`.
#import "/oku-slides/lib.typ": color-of

/// Un cadre aux couleurs du thème.
#let carte(t, body) = block(width: 100%, inset: 24pt, radius: 8pt, fill: color-of(t, "surface"), body)
```

```typ
// Un indicateur : une valeur en grand, sa légende dessous.
#import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of
#import "_commun.typ": carte

#let widget = widget("kpi", label: "KPI", description: "Une valeur en grand, sa légende dessous",
  params: (value: p.plain(default: "42 %"), label: p.plain(), trend: p.choice("up", "down", "flat")),
  (args, ctx) => carte(ctx.tokens, fields-stack(spacing: 8pt,
    field(ctx, "value", { set text(size: size-of(ctx.tokens, "display"), fill: color-of(ctx.tokens, "accent")); args.value }),
    field(ctx, "label", { set text(size: size-of(ctx.tokens, "body"), fill: color-of(ctx.tokens, "ink-muted")); args.label }),
  )))
```

For its colors and sizes, a widget reads the theme tokens
(`color-of(ctx.tokens, "accent")`, `size-of(ctx.tokens, "body")`) rather than
hard-coded values.

`lib.typ` exports everything a pack needs: `widget`, `field`,
`by-name`, `list-item`, the `params` module (to import `as p`), `free-area`,
`zone-marker`, `grid-frame`, `default-grid`, `_paragraphs`, `_text-args`,
`_text-over`.

### Parameter descriptors (`p.*`)

Each widget parameter is declared with a `p.*` descriptor, imported
from `params.typ` (`#import "/oku-slides/lib.typ": params as p`):

| Descriptor | Role |
| --- | --- |
| `p.plain(label:, default:, bind:)` | Single-line text; `bind` ties the parameter to a `deck.params` key |
| `p.rich(label:)` | Paragraphs made of runs |
| `p.flag(label:, default:)` | Boolean |
| `p.number(label:, default:, min:, max:)` | Bounded number |
| `p.choice(..values, label:, default:)` | One value among `values`; the first by default |
| `p.asset(label:)` | Asset identifier `sha256:…` |
| `p.slot(label:, mode:, accepts:)` | Child elements; `mode`: `"flow"` or `"free"`; `accepts`: accepted widget names, or `none` |
| `p.grid(label:, default:)` | Grid `(columns:, rows:, gap:)` |
| `p.list(label:, item-label:, item:, min:, max:, default:)` | List of items with typed columns (`plain`, `rich`, `number`, `flag`, `choice`, `asset`, or a `p.list` nested one level only), entered as a table; absent list = empty, `default` only seeds a newly created object |
| `p.host(name, label:)` | Type supplied and validated by the host (`"chart"`, `"table"`, `"path"`) |

### `widget(name, label:, description:, category:, params:, render)`

`render` is a function `(args, ctx) => content`:

- `args`: the parameter values, ready to render. An entry of a `flow` slot
  carries `(kind, id, body)`; an entry of a `free` slot carries
  `(kind, id, frame, cell, show)`, where `show(frame)` renders the child.
- `ctx`: `id` (the element or slide), `tokens`, `vars`, `style` (the
  element's effective typography), `framed` (the element has a frame),
  `load` (host loader, or `none`): `(ctx.load)(asset: …, fit: …)`
  for an image (a slide widget too: its decoration), `(ctx.load)(chart: true)`
  for a chart (element only).

`category` is `"element"` (slide body widget) or `"slide"`.
`label` is the name shown in the app; `description`, what the widget shows in
one sentence: the app's add carousel filters on the name, the label and the
description. Both are shown as written, in every interface language, and must be
text (`str`): any other value sets the widget file aside with an error naming
the argument. A blank label or description counts as none.

### `field(ctx, name, body, style:)`

Emits the editable box of `body` for the `name` parameter of the object
`ctx.id`, tagged `<oku-slides-field>`: this is what makes a widget text
editable in place in the app. `ctx.id` is `none` for an object
(element or slide) without an `id`; `field` then renders `body` as is. The schema
probe never calls `render`: `ctx.id` has nothing to do with it.

### `list-item(ctx, name, it, body)`

Renders an item of a `p.list` list: `it` is the item (a dictionary of its
columns, plus `id`), `body` a function `c => content`. Inside, fields
are emitted with `field(c, "column", …)` (not `ctx`): `c` carries the
item's identity, which makes each column editable in place and the
item movable in the app. The names `id`, `kind` and `/` are reserved
for columns. An empty column stays editable with
`field(c, name, box(width: 3em, hide[0]), empty: true)`.

```typ
#let widget = widget("frise", params: (etapes: p.list(
  label: "Étapes", item-label: "étape", max: 8,
  item: (date: p.plain(label: "Date"), texte: p.plain(label: "Étape")),
  default: ((date: "2019", texte: "Création"),))),
  (args, ctx) => grid(columns: (1fr,) * args.etapes.len(),
    ..args.etapes.map(e => list-item(ctx, "etapes", e,
      c => stack(field(c, "date", e.date), field(c, "texte", e.texte))))))
```

A complete example, including an empty frise and empty fields, is in the
Oku Slides repository (`fixtures/widgets/frise-test.typ`).
An absent list is the empty list; `default` only seeds a newly created
object.

#### Nested list

A column can itself be a `p.list`: each item then carries its own
list (steps with subtasks). Nesting stops at one level:
a nested list that still declares a `p.list` column is rejected.
Its items are rendered with `list-item` called on the `c` of the parent item;
their fields are addressed `etapes/<step>/taches/<task>/<column>`, and the
nested list of a newly added item receives the `default` of its
column (its rows do not need an `id`).

```typ
#let widget = widget("plan", params: (etapes: p.list(item-label: "étape",
  item: (titre: p.plain(), taches: p.list(item-label: "tâche", max: 8,
    item: (texte: p.plain(), fait: p.flag()))))),
  (args, ctx) => grid(columns: (1fr,) * args.etapes.len(),
    ..args.etapes.map(e => list-item(ctx, "etapes", e, c => stack(
      field(c, "titre", e.titre),
      ..e.taches.map(x => list-item(c, "taches", x, cc => field(cc, "texte", x.texte))))))))
```

The complete example is `fixtures/widgets/plan-test.typ` in the Oku Slides repository.

### Empty fields

`field()` of an absent value (`none` or `""`) renders nothing: it renders a
marker that takes no space, so the field stays reachable in the app without
changing the layout. To stack fields, use
`fields-stack(spacing: …, ..items)` instead of `stack`: it places the empty
field's box at the top of the next item (or at the bottom of the last). With `stack`, the
marker receives the spacing and the empty field has no anchor.

### Overriding a base widget

A pack widget can replace a base widget if it declares the same
`name`, keeps its `category`, and each of its parameters remains, with a
compatible type and `bind`: any value valid for the base parameter
must remain valid for the pack's.

- `p.number()`: the pack's range must contain the base's (its
  `min` is `none`, or less than or equal to the base's if the base has
  one; without a `min` in the base, the pack cannot add one — symmetric
  for `max`).
- `p.slot()`: same `mode`, and the pack's `accepts` is `none` (anything accepted) or
  a superset of the base's; without `accepts` in the base (anything accepted),
  the pack cannot restrict it.
- `bind`: identical to the base parameter's (`none` included).
- `p.choice()`: the pack's `values` must contain the base's
  (superset).
- `p.list()`: a list must remain identical (same columns, same
  types, same `item-label`, same bounds), nested list included.
- The other types (`plain`, `rich`, `flag`, `asset`, `grid`, `host`)
  must be identical.

A different `category` makes the registry load fail with a
`Schema` error (`WidgetErrorKind::IncompatibleCategory`). A missing or
incompatible base parameter makes the registry load fail with
a `Schema` error (`WidgetErrorKind::IncompatibleOverride`).

## Compiling the examples

```sh
cd examples
typst compile --root .. demo.typ
typst compile --root .. from-json.typ
```

The Papier theme fonts (Newsreader, Instrument Sans) must be
installed or passed with `--font-path`; otherwise Typst falls back to
Libertinus Serif.
