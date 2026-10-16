# crates/oku-slides-i18n — agent rules

## Invariants

- Translate every new key in each locale; never copy the English text. (plan:2026-10-09-agent-detachable, plan:2026-10-05-themes-packs-editeur)
- Write no space between a number and its CJK counter in ja, ko and zh messages. (plan:2026-10-10-import-pptx-lot2, plan:2026-10-04-agent-thread-followups-2)
- Keep ui.ftl as the last file included by the locale macro in lib.rs. (plan:2026-09-26-agent-vocal)
- Derive LOCALES and ENGLISH_NAMES from the single locales macro list; never keep two lists. (t-782)
- Keep code tokens and Fluent variables unchanged when reviewing or editing translations. (plan:2026-10-04-suivis-listes, t-535)
- Add every new Fluent key to all 20 locales, in the same key order everywhere. (spec:2026-10-08-widget-tableau-design)

## Pitfalls

- Never hard-code 1 in a plural branch; show the count variable, since one also covers 21 in some languages. (plan:2026-10-03-agent-a2ui)
- Put symbols such as the check mark inside the Fluent string, since their position follows text direction. (plan:2026-10-03-agent-a2ui)
- Avoid literal braces in Fluent text such as Typst samples; Fluent parses them as placeables. (plan:2026-10-02-creation-widgets)
- Leave JSON field names such as align and dataset untranslated in Fluent messages. (plan:2026-10-08-widget-tableau)
- In th, never reuse a noun as its own classifier; write the noun, the number, then a classifier. (t-466)

## Settled decisions

- Treat en as the reference locale and write each new Fluent message in English first. (spec:2026-09-27-sources-loro-design, t-9)
- Put import report keys in document.ftl with prefix import- and CLI keys in cli.ftl with prefix cli-. (plan:2026-10-10-import-pptx-lot2)
- Use a single term for agent within each locale, as decided in t-456 for vi and zh-Hant. (plan:2026-10-04-agent-thread-followups-2)
- Write pt as Brazilian Portuguese only: aba, arquivo, salvar, em um, tela, “quotes”. (t-758)
- Name the dashed empty-field outlines guides in every locale (fr repères), never ghosts. (t-363)
