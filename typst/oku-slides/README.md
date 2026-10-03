# oku-slides — bibliothèque Typst

Slides 16:9 (1920 × 1080 pt) à structure imposée : slide de titre, sommaire,
chapitres, slides de contenu avec en-tête et pied, slide de fin. Tout le style
passe par des **jetons** ; le contenu peut être écrit en Typst ou fourni en
**JSON** (le modèle de données de l'app Oku Slides).

## Démarrage

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

Depuis un JSON : `#render-deck(json("deck.json"), theme: papier)`
(voir `examples/deck.json` et [Deck JSON](#deck-json-v3)).

## Types de slides

| Fonction | Rôle | Chrome |
| --- | --- | --- |
| `title-slide(title, subtitle, author, date, kicker, body)` | Couverture ; valeurs par défaut = paramètres du deck | non |
| `toc(title, pages)` | Sommaire des chapitres, liens cliquables | oui |
| `chapter(title, subtitle)` | Ouvre un chapitre (heading niveau 1) : alimente sommaire et en-tête | non |
| `slide(title, kicker, layout, ratio, ..corps)` | Contenu ; titre = heading niveau 2 | oui |
| `end-slide(body)` | Slide de fin | non |
| `blank-slide(body, chrome)` | Page libre | au choix |
| `card(title, kicker, highlight, body)` | Carte avec filet d'accent ; `highlight` la passe sur fond sombre | — |

Layouts de `slide` : `default` (empilé), `columns` (un corps par colonne),
`split` (deux corps, `ratio: (2fr, 1fr)`), `center`, `quote`.

## Paramètres du deck

`title`, `subtitle`, `author`, `date` (chaîne ou `datetime`, formatée en
français), `organization`, `logo`, `lang`, `numbering` (`"{n} / {total}"`,
`"{n}"` ou `none`), `chapter-numbering` (`"1"`, `"01"`, `"I"`… ou `none`),
`toc-title`, `header` et `footer` (`auto`, `none`, ou une fonction
`ctx => contenu`).

`ctx` contient : `title`, `chapter`, `chapter-number`, `page`, `total`,
`params`, `vars`, `tokens`.

## Variables

`vars: (client: "Acme")` au niveau du deck, puis `#var("client")` dans le
contenu, ou `{{client}}` dans les titres, les paramètres et les textes JSON.

## Jetons

Primitives JSON uniquement, fusionnées en profondeur :
`default-tokens` ← `theme` ← `tokens`.

| Famille | Exemple | Valeurs |
| --- | --- | --- |
| `color` | `accent: "#2f5fb3"` | hex, ou nom d'une autre couleur |
| `font` | `display: ("Newsreader",)` | familles de repli |
| `size`, `space`, `radius`, `stroke` | `body: 32` | pt, ou nom d'un autre jeton de la famille |
| `weight`, `leading` | `strong: 600`, `normal: 0.6` | graisse ; em |
| `page` | `margin-x: 120` | géométrie de la page et positions en-tête/pied |
| `component.*` | `chapter: (bg: "accent")` | style de chaque composant, par références |

Composants : `header`, `footer`, `kicker`, `title`, `title-slide`, `chapter`,
`toc`, `card`, `end-slide`, `list`, `quote`, `columns`.

Tout composant de texte accepte `size`, `color`, `weight`, `font`,
`tracking` (pt) et `case` (`"upper"`, `"lower"`).

- **En-tête et pied** : `left` et `right` sont des gabarits, par exemple
  `"{n} · {title}"` ou `"{confidential}"`. Champs : `title`, `subtitle`,
  `author`, `date`, `organization`, `chapter`, `chapter-number`,
  `chapter-label`, `n`, `total`, `page-label`, plus toute variable du deck. Un
  segment (séparé par ` · `) dont un champ est vide disparaît.
- **Titre de slide** : `rule: (width, height, color)` trace un filet entre le
  surtitre et le titre.
- **Couverture** : `layout: "plain"` ou `"band"` (bandeau clair en haut avec
  `top-left`, `top-right`, `badge`, puis une bande sombre `band-bg` qui occupe
  `band-ratio` de la hauteur).
- **Carte** : `height: "auto"` ou `"fill"`.

## Thèmes fournis

- `papier` : le design system Papier (Newsreader, Instrument Sans, vert sauge).
- `pro` : le design system Template Pro (Inter, navy et cyan, couverture en
  bande, surtitres en capitales, pied « n · titre » et « CONFIDENTIAL »).
  Voir `examples/pro.typ`.

## Deck JSON (v3)

Chaque slide et chaque élément nomment leur widget (`kind`) et portent ses
paramètres (`params`), décrits par son schéma (voir [Widgets](#widgets)).
Un slot (`blocks`, `elements`, `children`) est un tableau d'éléments ; un
élément d'un slot libre a un `frame` `{ x, y, w, h }` (pt, relatif à la
zone de corps) ou une `cell` de la grille de la slide.

```json
{
  "version": 3,
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

Les paramètres liés (`bind`, comme le titre et le sous-titre de la slide
`title`) se lisent dans `params` du deck, pas dans ceux de la slide.

`render-deck(data, theme: none, image-loader: none, widgets: none)` :

- `image-loader` : fonction `(élément, asset: none, fit: "cover", chart: false)
  => contenu`, fournie par l'hôte pour les images (`asset`, `fit`) et les
  graphiques qu'il dessine (`chart: true`) ; sans elle, un cadre gris.
- `widgets` : dictionnaire nom → widget, typiquement
  `by-name(base + pack)` ; `none` rend avec les widgets de base. Un `kind`
  sans widget de la bonne catégorie donne un encart « élément inconnu »
  (élément) ou une page « slide inconnue » (slide).

## Introspection

Chaque élément JSON avec un `id` est étiqueté `<e-ID>` :
`query(<e-c1>).first().location().position()` donne sa position réelle, pour
la couche de sélection de l'app. Sa boîte est émise sous `<oku-slides-box>`.

Les textes éditables sont émis sous `<oku-slides-field>` par `field()`
(voir plus bas), avec `owner` (id de l'élément ou de la slide) et `field`
(nom du paramètre). Les slides aussi, un champ par paramètre `plain`
affiché, vide ou non : `kicker` (mise en page `plain` seulement), `title` et
`subtitle` pour la slide de titre, `title` et `subtitle` pour les chapitres,
`kicker` et `title` pour les slides de contenu, `title` pour le sommaire,
`text` pour la slide de fin. Les fonctions de slide
(`slide`, `title-slide`, `chapter`, `toc`, `end-slide`) prennent pour cela un
`wrap: (nom, contenu, style) => contenu`, comme `card`. Un widget de pack
qui passe `wrap` à `slide()` reçoit ainsi les champs `kicker` et `title` ; le
`_toc` de base filtre `kicker`, qui n'est pas un de ses paramètres.

## Widgets

Un **widget** est un composant déclaré avec le schéma de ses paramètres ; le
registre de base (13 widgets : `text`, `card`, `image`, `shape`, `chart`,
`stack`, `canvas`, `title`, `toc`, `chapter`, `content`, `end`, `blank`) est
lu par l'hôte (sonde `<oku-widgets>`) et sert au rendu du deck JSON.

Un **pack de modèles** peut ajouter ses propres widgets, ou surcharger ceux
de la base : **un fichier par widget**, `widgets/<nom>.typ`, dont la liaison
`widget` est le résultat de `widget("<nom>", …)` (le nom déclaré est celui du
fichier, `[a-z][a-z0-9-]*`). Le fichier facultatif `widgets/_commun.typ` porte
le code partagé : un widget l'importe par `#import "_commun.typ": …` ; ce n'est
pas un widget. L'app génère le `registry` du pack (un module d'assemblage qui
importe chaque widget) ; un widget qui ne compile pas en est écarté avec ses
diagnostics, les autres restent.

Exemple : un indicateur, `widgets/kpi.typ` et `widgets/_commun.typ`.

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

#let widget = widget("kpi", label: "KPI",
  params: (value: p.plain(default: "42 %"), label: p.plain(), trend: p.choice("up", "down", "flat")),
  (args, ctx) => carte(ctx.tokens, fields-stack(spacing: 8pt,
    field(ctx, "value", { set text(size: size-of(ctx.tokens, "display"), fill: color-of(ctx.tokens, "accent")); args.value }),
    field(ctx, "label", { set text(size: size-of(ctx.tokens, "body"), fill: color-of(ctx.tokens, "ink-muted")); args.label }),
  )))
```

Pour ses couleurs et ses tailles, un widget lit les jetons du thème
(`color-of(ctx.tokens, "accent")`, `size-of(ctx.tokens, "body")`) plutôt que
des valeurs en dur.

`lib.typ` exporte tout ce dont un pack a besoin : `widget`, `field`,
`by-name`, le module `params` (à importer `as p`), `free-area`,
`zone-marker`, `grid-frame`, `default-grid`, `_paragraphs`, `_text-args`,
`_text-over`.

### Descripteurs de paramètres (`p.*`)

Chaque paramètre d'un widget se déclare avec un descripteur `p.*`, importé
depuis `params.typ` (`#import "/oku-slides/lib.typ": params as p`) :

| Descripteur | Rôle |
| --- | --- |
| `p.plain(label:, default:, bind:)` | Texte sur une ligne ; `bind` lie le paramètre à une clé de `deck.params` |
| `p.rich(label:)` | Paragraphes à runs |
| `p.flag(label:, default:)` | Booléen |
| `p.number(label:, default:, min:, max:)` | Nombre borné |
| `p.choice(..values, label:, default:)` | Une valeur parmi `values` ; par défaut la première |
| `p.asset(label:)` | Identifiant d'asset `sha256:…` |
| `p.slot(label:, mode:, accepts:)` | Éléments enfants ; `mode` : `"flow"` ou `"free"` ; `accepts` : noms de widgets admis, ou `none` |
| `p.grid(label:, default:)` | Grille `(columns:, rows:, gap:)` |
| `p.host(name, label:)` | Type fourni et validé par l'hôte (`"chart"`) |

### `widget(name, label:, category:, params:, render)`

`render` est une fonction `(args, ctx) => contenu` :

- `args` : les valeurs des paramètres, prêtes à rendre. Une entrée de slot
  `flow` porte `(kind, id, body)` ; une entrée de slot `free` porte
  `(kind, id, frame, cell, show)`, où `show(frame)` rend l'enfant.
- `ctx` : `id` (l'élément ou la slide), `tokens`, `vars`, `style` (la
  typographie effective de l'élément), `framed` (l'élément a un cadre),
  `load` (chargeur de l'hôte, ou `none`) : `(ctx.load)(asset: …, fit: …)`
  pour une image, `(ctx.load)(chart: true)` pour un graphique.

`category` vaut `"element"` (widget de corps de slide) ou `"slide"`.

### `field(ctx, name, body, style:)`

Émet la boîte éditable de `body` pour le paramètre `name` de l'objet
`ctx.id`, étiquetée `<oku-slides-field>` : c'est ce qui rend un texte de
widget éditable en place dans l'app. `ctx.id` vaut `none` pour un objet
(élément ou slide) sans `id` ; `field` rend alors `body` tel quel. La sonde
de schéma n'appelle jamais `render` : `ctx.id` n'a rien à voir avec elle.

### Champs vides

`field()` d'une valeur absente (`none` ou `""`) ne rend rien : il rend un
marqueur sans place, pour que le champ reste atteignable dans l'app sans
changer la mise en page. Pour empiler des champs, utiliser
`fields-stack(spacing: …, ..items)` au lieu de `stack` : il pose la boîte du
champ vide au haut de l'item suivant (ou au bas du dernier). Avec `stack`, le
marqueur reçoit l'espacement et le champ vide n'a pas d'ancre.

### Surcharger un widget de base

Un widget d'un pack peut remplacer un widget de base s'il déclare le même
`name`, garde sa `category`, et chacun de ses paramètres reste, avec un
type et un `bind` compatibles : toute valeur valide pour le paramètre de
base doit rester valide pour celui du pack.

- `p.number()` : l'intervalle du pack doit contenir celui de la base (son
  `min` est `none`, ou inférieur ou égal à celui de la base si la base en a
  un ; sans `min` à la base, le pack ne peut pas en ajouter — symétrique
  pour `max`).
- `p.slot()` : même `mode`, et `accepts` du pack est `none` (tout admis) ou
  sur-ensemble de celui de la base ; sans `accepts` à la base (tout admis),
  le pack ne peut pas le restreindre.
- `bind` : identique à celui du paramètre de base (`none` inclus).
- `p.choice()` : `values` du pack doit contenir celui de la base
  (sur-ensemble).
- Les autres types (`plain`, `rich`, `flag`, `asset`, `grid`, `host`)
  doivent être identiques.

Une `category` différente fait échouer le chargement du registre avec une
erreur `Schema` (`WidgetErrorKind::IncompatibleCategory`). Un paramètre de
base absent, ou incompatible, fait échouer le chargement du registre avec
une erreur `Schema` (`WidgetErrorKind::IncompatibleOverride`).

## Compiler les exemples

```sh
cd examples
typst compile --root .. demo.typ
typst compile --root .. from-json.typ
```

Les polices du thème Papier (Newsreader, Instrument Sans) doivent être
installées ou passées avec `--font-path` ; sinon Typst se replie sur
Libertinus Serif.
