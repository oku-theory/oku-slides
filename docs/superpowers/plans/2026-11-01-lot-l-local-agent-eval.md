# Local agent evaluation (Task 14, t-1461)

Plan: `2026-11-01-lot-l-local-agent.md`, Task 14. Set: `crates/oku-slides-local-agent/eval/requests.json`
(30 requests, French and English, across edit, present, files, configure, troubleshoot and packs, each with
the families that serve it and the first tools a good turn calls). Run:

    OKU_SLIDES_PATHS__DATA=<fresh dir> OKU_SLIDES_PATHS__DECKS=<fresh dir> \
      cargo nextest run -p oku-slides-local-agent --run-ignored only -E 'test(eval_requests)' --no-capture

It runs Route, then the first Act step, on the engine `local_agent.engine` picks (`auto`), with the tools
of a deck tab on `fixtures/roadmap.json`. A route counts when it names one expected family; a first step
when it calls one of the expected tools. Reading first (`deck_outline`, `slide`, `settings`) counts where
the instructions ask for it.

## 2026-11-01 — Apple Foundation Models (macOS 27.0.1), Gguf model not downloaded

Context fix first: the Apple model's context is 4,096 tokens. One Act reply with every routed tool's
schema measured 13,914 tokens for `edit` (21 tools; overflow, `EngineError`) and 3,544 for `configure`
(18 tools). One tool alone is not enough either: `apply` is 4,797 tokens, `show_variants` 5,124 (their
`Op` union, 27 branches). **Decision:** the Apple Act is 2-3 guided replies (`apple_steps`), the Gguf path
unchanged:

| Step | Schema | Measured (`SystemLanguageModel.tokenCount`) |
|---|---|---|
| pick: a tool name among the routed ones, descriptions in the instructions | `{tool: enum}` | edit 858 instructions + 144 schema + 56 prompt; configure 754 + 149 + 56 |
| kinds, only when the call schema passes `FILL_BYTES` (6,000 bytes): 1-3 tags of its biggest tagged union | `{kinds: [enum]}` | 214 |
| call: that tool's schema alone, the union cut to the picked tags, unreached `$defs` pruned | `{tool, arguments}` | `apply` with its 3 biggest ops 1,841; `show_variants` 2,014; largest unnarrowed `show_params` 467 |

Every step leaves at least ~1,800 tokens for the history. A turn that still overflows ends with its
failure report, as before. The call step offers no `{text}` branch: with it, the model wrote text instead of
the call it had just picked.

Tuning, on the results of each run (table tests in `families_tests.rs` follow each change):

| Run | Route | First tool | Change after it |
|---|---|---|---|
| 1 | 26/30 | 21/30 | `focus` only in `edit` (the pick chose it over `present`); `edit` names the deck's theme; `configure` names the agent, model downloads, usage and cost; `packs` says widget packs and theme designs; `merge` may read the outline first, dark mode may read `settings` first |
| 2 | 28/30 | 28/30 | `files` says deck files |
| 3 | 28/30 | 28/30 | kept (a 4th wording of `configure` fell to 26/30 and was reverted) |

Final run (3), Apple engine, about 5.5 s per request (route + first Act):

    ok  route edit,present             ok  tool deck_outline               7.5s | en Rename the deck to Roadmap 2027
    ok  route edit                     ok  tool deck_outline               5.5s | fr Renomme le deck en Feuille de route 2027
    ok  route edit,present             ok  tool deck_outline               5.5s | en Add a slide titled Next steps after the last one
    ok  route edit                     ok  tool deck_outline               5.9s | fr Ajoute une slide « Conclusion » à la fin
    ok  route edit                     ok  tool slide                      5.1s | en Make the title of slide 2 shorter
    ok  route edit                     ok  tool slide                      5.1s | fr Mets le titre de la slide 3 en gras
    ok  route edit,present             ok  tool deck_outline               6.0s | en Delete the second slide
    ok  route configure,edit           ok  tool show_themes                7.5s | fr Change le thème du deck
    ok  route edit,present             ok  tool chart_kind                 5.8s | en Turn the table on slide 4 into a bar chart
    ok  route present                  ok  tool deck_outline               4.7s | fr Va à la slide 5
    ok  route present                  ok  tool present                    4.7s | en Start the presentation
    ok  route present                  ok  tool present                    4.4s | fr Lance la présentation en plein écran
    ok  route files                    ok  tool save                       4.4s | en Save the deck
    ok  route files                    ok  tool save_copy                  5.0s | fr Enregistre une copie du deck
    ok  route present,files            ok  tool new                        4.8s | en Create a new deck
    ok  route files                    ok  tool history                    4.7s | fr Montre-moi l'historique des versions
    ok  route files                    ok  tool deck_outline               4.5s | en Merge another deck into this one
    ok  route configure                ok  tool set_interface              5.0s | en Switch the interface to dark mode
    ok  route configure                ok  tool set_interface              6.2s | fr Passe l'interface en anglais
    ok  route configure                ok  tool settings                   5.6s | en Change the dictation language to German
    BAD route files                    BAD tool ask_choice                 6.8s | fr Télécharge le modèle de dictée
    BAD route edit,present             BAD tool slide                      5.4s | en Use Claude as the app agent
    ok  route configure,files          ok  tool usage                      6.1s | fr Combien m'a coûté ce deck en tokens ?
    ok  route configure                ok  tool settings                   4.7s | en Show the app settings
    ok  route configure,troubleshoot   ok  tool diagnose                   5.0s | en Why doesn't my agent start?
    ok  route troubleshoot             ok  tool diagnose                   4.3s | fr Pourquoi l'agent Claude ne marche plus ?
    ok  route configure,troubleshoot   ok  tool diagnose                   5.5s | en The dictation doesn't hear me, what's wrong?
    ok  route troubleshoot,present     ok  tool diagnose                   5.4s | fr La slide 2 ne s'affiche pas, il y a une erreur de rendu
    ok  route files,troubleshoot       ok  tool diagnose                   5.3s | fr L'import du PowerPoint a raté, pourquoi ?
    ok  route packs                    ok  tool packs                      6.5s | en Edit the widgets of the deck's own pack

Left misrouted: a dictation model download goes to `files` and "Use Claude as the app agent" to `edit`.
Both fail safely (a question, or a read), and `configure` is always offered, so a second request names it.
Also seen: tools without arguments get invented ones (`deck_outline {name: …}`), which the server ignores.
