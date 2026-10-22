// Code partagé des layouts du pack `essentiel`.

#import "/oku-slides/lib.typ": field

// Champs d'une slide : `wrap` des fonctions de slide, posé par le layout.
#let fields(ctx) = (name, body, style) => field(ctx, name, body, style: style)
