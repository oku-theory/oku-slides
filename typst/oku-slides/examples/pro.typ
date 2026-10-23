// Même structure que demo.typ, rendue avec le thème Template Pro.
#import "../lib.typ": *
#import "../packs/essentiel/layouts/_commun.typ": title-slide, toc, chapter, slide, end-slide
#import "../packs/base/widgets/card.typ": card

// Les fonctions de slide (et la carte) viennent des packs : elles lisent leurs
// jetons de composant, défauts du pack puis thème, dans `ctx.component`.
#let theme = merge(merge(pack-tokens("essentiel", json("../packs/essentiel/layouts.json")),
  pack-tokens("base", json("../packs/base/widgets.json"))), pro)
#let ess = (component: theme.component.essentiel)
#let base = (component: theme.component.base)

#show: deck.with(
  theme: theme,
  title: "Feuille de route {{annee}}",
  subtitle: "Trois chantiers pour {{client}}",
  author: "Matthieu",
  date: datetime(year: 2026, month: 10, day: 1),
  organization: "Darticorp",
  vars: (client: "Acme", annee: 2027, confidential: "CONFIDENTIAL"),
)

#title-slide(ess)
#toc(ess)

#chapter(ess, "Contexte", subtitle: "Où en est {{client}} aujourd'hui")

#slide(ess, title: "Ce qui a changé en {{annee}}", kicker: "Constat")[
  - Le marché a doublé en deux ans.
  - Les clients attendent des réponses en temps réel.
  - Notre socle technique date de 2019.
]

#slide(ess, title: "Trois priorités", kicker: "Organisation", layout: "columns")[
  #card(base, kicker: "Pilier 1", title: "Fiabilité")[
    - Diviser par deux les incidents
    - Astreinte partagée
  ]
][
  #card(base, kicker: "Pilier 2", title: "Vitesse")[
    - Livrer chaque semaine
    - Déploiement continu
  ]
][
  #card(base, kicker: "Pilier 3", title: "Clients", highlight: true)[
    - Un interlocuteur unique pour #var("client")
    - Revue trimestrielle
  ]
]

#chapter(ess, "Plan d'action")

#slide(ess, title: "Une idée par slide", kicker: "Principe", layout: "quote")[
  « Si une slide a besoin d'une légende, elle a deux idées. »
]

#slide(ess, title: "Avant et après", kicker: "Comparaison", layout: "split", ratio: (2fr, 1fr))[
  Le texte principal occupe les deux tiers de la largeur.
][
  #rect(width: 100%, height: 300pt, fill: luma(235))
]

#end-slide(ess)[Merci]
