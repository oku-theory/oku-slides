# oku-slides — Typst library

16:9 slides (1920 × 1080 pt) with a fixed structure: title slide, table of
contents, chapters, content slides with header and footer, end slide. All
styling goes through **tokens**; the content can be written in Typst or supplied
as **JSON** (the data model of the Oku Slides app).

## Getting started

```typ
#import "lib.typ": *
#import "packs/essentiel/layouts/_commun.typ": title-slide, toc, chapter, slide, end-slide

// Les fonctions de slide lisent les jetons de composant de leur pack dans `ctx.component`.
#let theme = merge(pack-tokens("essentiel", json("packs/essentiel/layouts.json")), papier)
#let ess = (component: theme.component.essentiel)

#show: deck.with(
  theme: theme,                        // jeu de jetons (optionnel)
  tokens: (color: (accent: "#8a5a1c")), // surcharges ponctuelles
  title: "Feuille de route {{annee}}",
  author: "Matthieu",
  date: datetime.today(),
  organization: "Darticorp",
  vars: (client: "Acme", annee: 2027),
)

#title-slide(ess)
#toc(ess)
#chapter(ess, "Contexte")
#slide(ess, title: "Trois priorités", layout: "columns")[Fiabilité][Vitesse][Clients #var("client")]
#end-slide(ess)[Merci]
```

The pack files import `/oku-slides/lib.typ`: compile with `--root` at `typst/`.

From a JSON: `#render-deck(json("deck.json"), theme: merge(packs, papier), widgets: …)`,
where `packs` merges the `pack-tokens(…)` of each pack
(see `examples/deck.json` and [Deck JSON](#deck-json-v9)); slide kinds
need the `essentiel` pack's layouts and element kinds the `base` pack's widgets
(see `examples/from-json.typ`).

## Slide types

The slide functions live in the `essentiel` pack (`packs/essentiel/layouts/_commun.typ`)
and `card` in the `base` pack (`packs/base/widgets/card.typ`); `lib.typ` exports only
`deck`, `blank-slide` and `kicker`. Each takes the widget `ctx` first: it reads its
pack's component tokens in `ctx.component`.

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
`default-tokens` ← packs' component defaults ← theme chain ← deck `tokens`. The
defaults are in `src/default-tokens.json` (`null` = undefined token, derived from
the background and the ink for `app`). A pack declares its component tokens in
its manifest (`tokens` entries `{path, default, control, …}`, `path` relative to
`component.<namespace>`); `pack-tokens(namespace, manifest)` gives
`(component: ((namespace): defaults))`. The app sends the packs' defaults then
the linearized theme chain as one layer (`render-deck(…, theme:)`).

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
| `header`, `footer` | `left: "{n} · {title}"` | page chrome: templates and text style |
| `list` | `marker: "–"` | bullet, its color, indent and spacing |
| `kicker` | `case: "upper"` | the kicker text style, shared by slides and cards |
| `heading` | `h2: (size: "subtitle", …)` | style of body headings h1 to h6, by token references |
| `component.<namespace>.*` | `essentiel: (chapter: (bg: "accent"))` | style of each pack component, by references |

Pack components: `base` has `card` and `table`; `essentiel` has `title`,
`title-slide`, `chapter`, `toc`, `end-slide`, `quote` and `columns`.

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
`{ id, name, namespace, extends, description, parent, created, tokens }`
(`extends`: ordered parent namespaces, omitted when empty; the app applies each
parent first, later ones winning). `lib.typ` exports its `tokens` under the
theme's name: `theme: papier`, `theme: pro` and `theme: neutre`; merge them over
`pack-tokens(…)` for the pack components (see `examples/from-json.typ`), and any
token dictionary works too.

- `papier`: the Papier design system (Newsreader, Instrument Sans, sage green).
- `pro`: the Template Pro design system (Inter, navy and cyan, banded cover,
  uppercase kickers, footer “n · title” and “CONFIDENTIAL”).
  See `examples/pro.typ`.
- `neutre`: no token of its own: the defaults and the packs' component
  defaults. The PowerPoint import pins it and writes the deck's tokens.

In the app, these three files are the built-in themes (fixed identifiers);
the user edits them in the “Theme” tab of pack mode: saving an edited built-in
creates a variant that extends it (a new theme in the `themes/` folder of the
app data), not a versioned copy. A deck embeds the versions of its theme and of
its `extends` closure.

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
  each role. `component.base.card.height`: `"auto"` (content height) or
  `"fill"` (all the available height). `app`: `null` derives the color from the
  slides' background and ink.

## Deck JSON (v9)

Each slide and each element names its widget (`kind`) and carries its
parameters (`params`), described by its schema (see [Widgets](#widgets)).
A slot (`blocks`, `elements`, `children`) is an array of elements; an
element of a free slot has a `frame` `{ x, y, w, h }` (pt, relative to the
body area) or a `cell` of the slide's grid.

```json
{
  "version": 9,
  "params": { "title": "Feuille de route", "subtitle": "Pour {{client}}", "author": "Matthieu" },
  "vars": { "client": "Acme" },
  "slides": [
    { "id": "s1", "kind": "essentiel/title" },
    { "id": "s2", "kind": "essentiel/chapter", "params": { "title": "Contexte" } },
    { "id": "s3", "kind": "essentiel/content", "params": { "title": "Trois priorités", "layout": "columns",
      "blocks": [
        { "id": "c1", "kind": "base/card", "params": { "title": "Fiabilité",
          "body": [ { "runs": [ { "t": "Diviser par deux les incidents" } ] } ] } }
      ] } },
    { "id": "s4", "kind": "essentiel/blank", "params": { "elements": [
      { "id": "e1", "kind": "base/text", "frame": { "x": 40, "y": 40, "w": 600, "h": 200 },
        "params": { "body": [ { "runs": [ { "t": "Posé à 40 pt du coin" } ] } ] } }
    ] } },
    { "id": "s5", "kind": "essentiel/end", "params": { "text": "Merci {{client}}" } }
  ]
}
```

An element also carries an optional `style` (size, color, weight,
font, `tracking` (letter spacing, in pt), alignment, background, stroke,
`radius`…). The `base/text` widget additionally reads `valign` (`"top"`, `"middle"`,
`"bottom"`: anchoring in the frame, framed elements only), `inset`
(four margins in pt: top, right, bottom, left) and `spacing` (space between
two paragraphs, list items included, in pt: from the last baseline
of one to the top of the first line of the next; without the key, the theme's)
and `indent` (two numbers in pt: the left margin of the paragraphs, then the
offset of their first line; negative, a hanging indent where the bullet or
number of an item fits, its text starting at the margin).
The `base/shape` widget draws a
path with `shape: "path"` and `path: { "d": "M 0 0 L 1 0 L 0.5 1 Z" }`:
commands `M`, `L`, `C`, `Q`, `Z`, each letter and number separated by
whitespace, coordinates from 0 to 1 stretched to the frame, non-zero fill.

`tokens` (optional) overrides the theme's tokens for this deck. `theme`
(optional) is the namespace of the deck's app theme (`null`: the default
theme) and `themes` maps each namespace of its `extends` closure to its pin
`{ id, version, extends }`: the host resolves them and passes the merged tokens
to `render-deck(…, theme:)`; the library does not read them.

Bound parameters (`bind`, like the title and subtitle of the `essentiel/title`
slide) are read from the deck's `params`, not from the slide's.

`render-deck(data, theme: none, image-loader: none, widgets: none)` :

- `image-loader`: function `(element, asset: none, fit: "cover", chart: false,
  flow: false) => content`, supplied by the host for images (`asset`, `fit`;
  `flow: true` sizes a flow image at its proportions, within the available
  height) and for the charts it draws (`chart: true`); without it, a gray frame.
- `widgets` (required): name → widget dictionary, typically
  `by-name(essentiel + base + pack)`; the core declares no widget, and `none`
  fails with “render-deck: widgets is required”. A `kind`
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
(`slide`, `title-slide`, `chapter`, `toc`, `end-slide`) take the widget `ctx` and a
`wrap: (name, content, style) => content` for this, like `card`. A pack widget
that passes `wrap` to `slide()` thus receives the `kicker` and `title` fields; the
`essentiel/toc` layout filters out `kicker`, which is not one of its parameters.

## Widgets

A **widget** is a component declared with the schema of its parameters. The
core registers none: every widget comes from a pack, whose registry the host
reads (`<oku-widgets>` probe) and uses to render the JSON deck.

A pack has a kind. A **widgets** pack (manifest `widgets.json`, files
`widgets/*.typ`) declares element widgets (`category: "element"`); a
**layouts** pack (manifest `layouts.json`, files `layouts/*.typ`) declares
slide layouts (`category: "slide"`). A file whose category contradicts its
pack's kind is dropped with a diagnostic.

Two packs are built in, and the app embeds and renders them like any pack:

- **`base`** (`packs/base`, widgets) holds `base/text`, `base/card`,
  `base/image`, `base/shape`, `base/chart`, `base/table` and `base/stack`;
- **`essentiel`** (`packs/essentiel`, layouts) holds the six slide layouts
  `essentiel/title`, `essentiel/toc`, `essentiel/chapter`, `essentiel/content`,
  `essentiel/end` and `essentiel/blank`. Its manifest also gives each layout's
  starting values (`examples`) and the import presets (`profiles`).

A manifest may also carry a `styles` key: a map from a widget file name to the
style the app gives that widget when the add grid inserts it (for example
`"styles": {"shape": {"fill": "surface", "radius": "md"}}` in `base`). An
entry with a malformed style or an invalid widget name is ignored.

In `examples`, a text of the form `ftl:session-example-<key>` is a localization
marker. Only the built-in packs (`base`, `essentiel`) use it: the app replaces it
with the text of that key in the user's language. In any other pack the text
stays as written, marker included, and a copy of a built-in pack has its
markers resolved to plain text.

Their files import `/oku-slides/lib.typ`, so they compile only under the
app's virtual root: the folder that contains `oku-slides/` (`--root`
pointing at `typst/`).

A **widget pack** adds its own widgets, which never replace those of the
base; the deck pins each pack under a namespace, and its widgets are named
`namespace/name`: **one file per widget**, `widgets/<name>.typ`, whose `widget` binding
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
hard-coded values. An unknown color name fails the render, so a typo in a
pack or theme shows at once. Deck data alone (element styles, text runs) is
resolved with `color-of(t, v, lenient: true)`: a name the theme lacks (a deck
made under another theme) is drawn in `ink-muted`, and the app lists it among
the deck warnings.

`lib.typ` exports everything a pack needs: `widget`, `field`,
`by-name`, `list-item`, the `params` module (to import `as p`), `free-area`,
`zone-marker`, `grid-frame`, `default-grid`, `_paragraphs`, `_text-args`,
`_text-over`, the token readers (`color-of`, `size-of`, `space-of`,
`radius-of`, `stroke-of`, `weight-of`, `font-of`, `leading-of`,
`list-marker`) and `format-number`, `deck-lang`.

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

### `widget(name, label:, description:, category:, grow:, typography:, params:, render)`

`render` is a function `(args, ctx) => content`:

- `args`: the parameter values, ready to render. An entry of a `flow` slot
  carries `(kind, id, grow, body)` (`grow`: that of the child's widget); an
  entry of a `free` slot carries `(kind, id, frame, cell, show)`, where
  `show(frame)` renders the child.
- `ctx`: `id` (the element or slide), `tokens`, `component` (the tokens
  `component.<namespace>` of the widget's pack, empty when it declares none),
  `vars`, `style` (the
  element's effective typography), `framed` (the element has a frame),
  `load` (host loader, or `none`): `(ctx.load)(asset: …, fit: …, flow: …)`
  for an image (a slide widget too: its decoration; `flow: not ctx.framed`
  sizes a flow image at its proportions), `(ctx.load)(chart: true)` for a
  chart (element only).

`grow` (element only) says how a flow block of this widget takes the height of
a content slide: `false` (the default) its own height; `true` a share of the
height left by the others; `"fit"` its own height if it fits a share, else a
share. A widget that grows is measured in that height for its box.
`typography: true` (element only) makes the element's box report the
typography of its `style`, for the app's in-place editor. A slide widget
declares neither: the probe refuses it.

`category` is `"element"` (slide body widget) or `"slide"`.
`label` is the name shown in the app; `description`, what the widget shows in
one sentence: the app's add grid filters on the name, the label and the
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

### Qualified pack widgets

A deck pins several widget packs, each under its own namespace. A pack
widget's kind is `namespace/name`, qualified by the namespace of the pack that
declares it; slides too (`essentiel/content`). A pack cannot override a widget
of another pack: a widget file named like a `base` widget is simply a different
widget, `namespace/card`.
A pack may require other packs (`requires`); the host adds them to the deck
with it.

## Compiling the examples

```sh
cd examples
typst compile --root ../.. demo.typ
typst compile --root ../.. from-json.typ
```

The examples import the `base` and `essentiel` packs, whose files import
`/oku-slides/lib.typ`: their root is `typst/`.

The Papier theme fonts (Newsreader, Instrument Sans) must be
installed or passed with `--font-path`; otherwise Typst falls back to
Libertinus Serif.
