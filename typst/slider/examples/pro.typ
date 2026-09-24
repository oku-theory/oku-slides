// Même structure que demo.typ, rendue avec le thème Template Pro.
#import "../lib.typ": *

#show: deck.with(
  theme: pro,
  title: "Feuille de route {{annee}}",
  subtitle: "Trois chantiers pour {{client}}",
  author: "Matthieu",
  date: datetime(year: 2026, month: 10, day: 1),
  organization: "Darticorp",
  vars: (client: "Acme", annee: 2027, confidential: "CONFIDENTIAL"),
)

#title-slide()
#toc()

#chapter("Contexte", subtitle: "Où en est {{client}} aujourd'hui")

#slide(title: "Ce qui a changé en {{annee}}", kicker: "Constat")[
  - Le marché a doublé en deux ans.
  - Les clients attendent des réponses en temps réel.
  - Notre socle technique date de 2019.
]

#slide(title: "Trois priorités", kicker: "Organisation", layout: "columns")[
  #card(kicker: "Pilier 1", title: "Fiabilité")[
    - Diviser par deux les incidents
    - Astreinte partagée
  ]
][
  #card(kicker: "Pilier 2", title: "Vitesse")[
    - Livrer chaque semaine
    - Déploiement continu
  ]
][
  #card(kicker: "Pilier 3", title: "Clients", highlight: true)[
    - Un interlocuteur unique pour #var("client")
    - Revue trimestrielle
  ]
]

#chapter("Plan d'action")

#slide(title: "Une idée par slide", kicker: "Principe", layout: "quote")[
  « Si une slide a besoin d'une légende, elle a deux idées. »
]

#slide(title: "Avant et après", kicker: "Comparaison", layout: "split", ratio: (2fr, 1fr))[
  Le texte principal occupe les deux tiers de la largeur.
][
  #rect(width: 100%, height: 300pt, fill: luma(235))
]

#end-slide[Merci]
