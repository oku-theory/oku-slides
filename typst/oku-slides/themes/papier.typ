// Thème « Papier » (jour) : jetons du design system Papier appliqués aux slides.
#let papier = (
  color: (
    bg: "#fffdf9",
    surface: "#f7f5f0",
    ink: "#2e2b26",
    ink-muted: "#686258",
    accent: "#3f6b5e",
    on-accent: "#fffdf9",
    accent-soft: "#dde7e1",
    rule: "#ddd7cc",
  ),
  font: (
    display: ("Newsreader", "Newsreader 16pt", "Libertinus Serif"),
    body: ("Instrument Sans", "Libertinus Serif"),
  ),
  // Titres serif (Newsreader) : un serif aussi en japonais, chinois, coréen.
  script-style: (display: "serif"),
  weight: (display: 500, title: 500, label: 500, strong: 600),
  // Graphiques : l'accent d'abord, puis des tons chauds et sourds du papier.
  chart: (palette: ("accent", "#c4622d", "#2f5d8a", "#c9a227", "#8a4f7d", "ink-muted")),
  // Interface : les jetons du design system Papier (jour).
  app: (
    scheme: "light",
    desk: "#ece8e0", sheet: "#f7f5f0", piece: "#fffdf9",
    ink: "ink", ink-muted: "ink-muted", rule: "rule", edge: "#8a8378",
    accent: "accent", accent-text: "accent", on-accent: "on-accent", accent-soft: "accent-soft",
    danger: "#a3452e", shadow: "#2e2b26",
  ),
  // Interface en schéma sombre : le Papier de nuit, chaud, l'accent éclairci pour le texte.
  app-dark: (
    desk: "#1f1d1a", sheet: "#2a2824", piece: "#34312c",
    ink: "#f1ece3", ink-muted: "#aaa398", rule: "#45413a", edge: "#6f685d",
    accent: "accent", accent-text: "#8fbfae", on-accent: "on-accent", accent-soft: "#2c3b35",
    danger: "#e07a5f", shadow: "#000000",
  ),
  component: (
    chapter: (bg: "surface", ink: "ink", number-color: "accent", number-opacity: 1),
    title-slide: (bg: "bg", bar-color: "accent"),
    end-slide: (bg: "surface"),
  ),
)
