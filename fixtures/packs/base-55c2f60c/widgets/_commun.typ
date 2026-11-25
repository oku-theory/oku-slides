// Code partagé des widgets du pack `base`.

#import "/oku-slides/lib.typ": color-of, radius-of, size-of

// Cadre de substitution : un fond discret et un libellé (image absente,
// graphique non dessiné, tableau sans données).
#let placeholder(t, label, height: 100%) = rect(
  width: 100%, height: height, fill: color-of(t, "surface"), radius: radius-of(t, "sm"),
  align(center + horizon, text(size: size-of(t, "caption"), fill: color-of(t, "ink-muted"), label)),
)
