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
  weight: (display: 500, title: 500, label: 500, strong: 600),
  component: (
    chapter: (bg: "surface", ink: "ink", number-color: "accent", number-opacity: 1),
    title-slide: (bg: "bg", bar-color: "accent"),
    end-slide: (bg: "surface"),
  ),
)
