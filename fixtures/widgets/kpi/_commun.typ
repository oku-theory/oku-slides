// Code partagé des widgets du pack : `#import "_commun.typ": carte`.
#import "/oku-slides/lib.typ": color-of

/// Un cadre aux couleurs du thème.
#let carte(t, body) = block(width: 100%, inset: 24pt, radius: 8pt, fill: color-of(t, "surface"), body)
