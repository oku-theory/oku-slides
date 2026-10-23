// Oku Slides — bibliothèque Typst de slides structurées et pilotées par jetons.

// Ce qu'un widget de pack peut importer ; les packs `base` et `essentiel` s'en servent.
#import "src/tokens.typ": default-tokens, default-params, merge, pack-tokens, color-of, size-of, space-of, font-of
#import "src/tokens.typ": weight-of, radius-of, stroke-of, leading-of, list-marker, styled, text-style, case-of
#import "src/numbers.typ": format-number, deck-lang
#import "src/core.typ": var, interp, config, current-chapter, page-context, format-date, fill-template, template-fields
#import "src/core.typ": chapter-number, chapter-counter
#import "src/slides.typ": deck, blank-slide, kicker, _heading, _mark
#import "src/data.typ": render-deck, render-slide, body-size
#let papier = json("themes/papier.json").tokens
#let pro = json("themes/pro.json").tokens
#let neutre = json("themes/neutre.json").tokens

// Widgets : pour déclarer ceux d'un pack (`#import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p`).
#import "src/params.typ" as params
#import "src/widget.typ": widget, field, fields-stack, list-item, by-name, _is-empty
#import "src/zone.typ": free-area, zone-marker, grid-frame, default-grid
#import "src/content.typ": _paragraphs, _text-args, _text-over
