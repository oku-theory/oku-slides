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
(voir `examples/deck.json`).

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

## Introspection

Chaque élément JSON avec un `id` est étiqueté `<e-ID>` :
`query(<e-c1>).first().location().position()` donne sa position réelle, pour
la couche de sélection de l'app.

## Widgets

Un **widget** est un composant déclaré avec le schéma de ses paramètres ; le
registre de base (13 widgets : `text`, `card`, `image`, `shape`, `chart`,
`stack`, `canvas`, `title`, `toc`, `chapter`, `content`, `end`, `blank`) est
lu par l'hôte (sonde `<oku-widgets>`) et sert au rendu du deck JSON.

Un **pack de modèles** peut ajouter ses propres widgets, ou surcharger ceux
de la base, avec un fichier `widgets.typ` qui expose un tableau `registry` :

```typ
#import "/oku-slides/lib.typ": widget, field, params as p

#let quote = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))

#let registry = (quote,)
```

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
  `load` (chargeur d'image ou de graphique fourni par l'hôte, fonction sans
  argument, ou `none`).

`category` vaut `"element"` (widget de corps de slide) ou `"slide"`.

### `field(ctx, name, body, style:)`

Émet la boîte éditable de `body` pour le paramètre `name` de l'objet
`ctx.id`, étiquetée `<oku-slides-field>` : c'est ce qui rend un texte de
widget éditable en place dans l'app. `ctx.id` vaut `none` pour un objet
(élément ou slide) sans `id` ; `field` rend alors `body` tel quel. La sonde
de schéma n'appelle jamais `render` : `ctx.id` n'a rien à voir avec elle.

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
