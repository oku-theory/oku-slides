// Oku Slides — bibliothèque Typst de slides structurées et pilotées par jetons.

// Ce qu'un widget de pack peut importer ; le pack `base` s'en sert.
#import "src/tokens.typ": default-tokens, default-params, merge, color-of, size-of, space-of, font-of, weight-of
#import "src/tokens.typ": radius-of, stroke-of, leading-of, list-marker
#import "src/numbers.typ": format-number, deck-lang
#import "src/core.typ": var, interp, config, current-chapter, page-context, format-date, fill-template, template-fields
#import "src/slides.typ": deck, slide, title-slide, chapter, toc, end-slide, blank-slide, kicker, card
#import "src/data.typ": render-deck, render-slide, body-size
#let papier = json("themes/papier.json").tokens
#let pro = json("themes/pro.json").tokens

// Widgets : pour déclarer ceux d'un pack (`#import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p`).
#import "src/params.typ" as params
#import "src/widget.typ": widget, field, fields-stack, list-item, by-name
#import "src/zone.typ": free-area, zone-marker, grid-frame, default-grid
#import "src/content.typ": _paragraphs, _text-args, _text-over
