// Thème « Template Pro » : traduction du design system Template Pro
// (navy / cyan, Inter, institutionnel) en jetons Slider.
// Les valeurs en px du design system valent des pt sur la grille 1920 × 1080.
#let pro = (
  color: (
    bg: "#ffffff",            // surface
    surface: "#e1e5ea",       // border : fond des cadres de remplacement
    ink: "#3d4756",           // body-text
    ink-muted: "#5b6675",     // muted-text
    accent: "#00bef4",        // accent : traits, bordures, badge
    on-accent: "#00304f",     // on-accent-text
    rule: "#e1e5ea",          // border
    // Couleurs propres au système.
    brand-ink: "#00154e",
    surface-dark: "#00154e",
    accent-text: "#0072b0",
    accent-on-dark: "#00bef4",
    on-dark-body: "#69bcd3",
    on-dark-muted: "#7c93b8",
    white: "#ffffff",
  ),
  font: (
    display: ("Inter",),
    body: ("Inter",),
  ),
  size: (
    display: 104,   // cover-title
    title: 56,      // slide-title
    subtitle: 32,   // cover-subline
    lead: 26,       // card-title
    body: 21,
    small: 19,
    caption: 19,
    kicker: 19,     // eyebrow
  ),
  weight: (display: 800, title: 800, body: 400, strong: 700, label: 700),
  leading: (tight: 0.12, normal: 0.62),
  space: (xs: 8, sm: 16, md: 24, lg: 32, xl: 44, xxl: 64),
  page: (margin-x: 128, margin-top: 112, margin-bottom: 160, header-y: 56, footer-y: 64),
  radius: (sm: 4, md: 8, lg: 8),
  stroke: (hairline: 1, rule: 2, accent: 6),
  component: (
    header: (left: none, right: none),
    footer: (
      left: "{n} · {title}", right: "{confidential}",
      size: 20, color: "ink-muted", tracking: 1,
    ),
    kicker: (size: "kicker", color: "accent-text", weight: 700, tracking: 1, case: "upper"),
    title: (
      size: "title", color: "brand-ink", weight: 800, tracking: -1, gap: 44,
      rule: (width: 64, height: 5, color: "accent"),
    ),
    title-slide: (
      layout: "band",
      bg: "bg", band-bg: "surface-dark", band-ratio: 0.62,
      title-size: "display", title-tracking: -2, band-title-color: "white",
      subtitle-size: "subtitle", band-subtitle-color: "on-dark-body",
      meta-size: "small", band-meta-color: "on-dark-muted",
      bar: true, bar-color: "accent", bar-width: 88, bar-height: 6,
      top-left: "{organization}", top-right: none, top-rule-color: "accent", top-rule-width: 6,
      badge: "Confidential", badge-bg: "accent", badge-color: "on-accent",
    ),
    chapter: (
      bg: "surface-dark", ink: "white", number-color: "accent", number-opacity: 1,
      title-tracking: -2, number-size: 180,
    ),
    toc: (number-color: "accent-text", size: 32),
    card: (
      bg: "bg", border-color: "rule", border-width: "hairline", radius: "md",
      top-rule-color: "accent", top-rule-width: 4, padding-x: 32, padding-y: 36,
      title-size: "lead", title-weight: 800, title-color: "brand-ink", gap: "sm",
      highlight-bg: "surface-dark", highlight-ink: "on-dark-body",
      highlight-title: "white", highlight-kicker: "accent-on-dark",
    ),
    list: (marker: "•", marker-color: "accent-text"),
    quote: (size: 40, color: "brand-ink", bar-color: "accent"),
    columns: (gap: 24),
    end-slide: (bg: "surface-dark", color: "white"),
  ),
)
