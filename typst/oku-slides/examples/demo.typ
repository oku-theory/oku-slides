#import "../lib.typ": *

#show: deck.with(
  theme: papier,
  title: "Feuille de route {{annee}}",
  subtitle: "Trois chantiers pour {{client}}",
  author: "Matthieu",
  date: datetime(year: 2026, month: 10, day: 1),
  organization: "Darticorp",
  vars: (client: "Acme", annee: 2027),
  // Surcharge ponctuelle d'un jeton :
  tokens: (color: (accent: "#3f6b5e")),
)

#title-slide()
#toc()

#chapter("Contexte", subtitle: "Où en est {{client}} aujourd'hui")

#slide(title: "Ce qui a changé en {{annee}}", kicker: "Constat")[
  - Le marché a doublé en deux ans.
  - Les clients attendent des réponses en temps réel.
  - Notre socle technique date de 2019.
]

#slide(title: "Trois priorités", layout: "columns")[
  *Fiabilité* \ Diviser par deux les incidents.
][
  *Vitesse* \ Livrer chaque semaine.
][
  *Clients* \ Un interlocuteur unique pour #var("client").
]

#chapter("Plan d'action")

#slide(title: "Une idée par slide", layout: "quote")[
  « Si une slide a besoin d'une légende, elle a deux idées. »
]

#slide(title: "Avant et après", layout: "split", ratio: (2fr, 1fr))[
  Le texte principal occupe les deux tiers de la largeur.
][
  #rect(width: 100%, height: 300pt, fill: luma(235))
]

#end-slide[Merci]
