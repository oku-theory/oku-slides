# fixtures — agent rules

## Pitfalls

- Never let fixtures/*.json gain CRLF line endings; tests compare them byte for byte. (t-218)

## Testing

- Never hand-edit fixtures/markdown-corpus.json; fix inline.rs or markdown.ts with a targeted test instead. (plan:2026-09-27-markdown-lezer)
- Keep fixtures/widgets/base-registry.json equal to the live core registry plus the base pack; a render test enforces it. (plan:2026-10-18-packs-lot2-base)
- Write import fixtures as hand-written unzipped OOXML that tests zip on the fly, and snapshot SourceDeck and ImportPlan. (spec:2026-10-09-import-pptx-design)
- Keep the list test widget's Typst source only in fixtures/widgets/frise-test.typ and reuse the shared helpers. (plan:2026-10-04-suivis-listes, t-532)
