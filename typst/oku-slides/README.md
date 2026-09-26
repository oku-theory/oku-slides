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

## Compiler les exemples

```sh
cd examples
typst compile --root .. demo.typ
typst compile --root .. from-json.typ
```

Les polices du thème Papier (Newsreader, Instrument Sans) doivent être
installées ou passées avec `--font-path` ; sinon Typst se replie sur
Libertinus Serif.
