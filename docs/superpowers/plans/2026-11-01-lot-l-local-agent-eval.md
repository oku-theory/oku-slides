# Local agent evaluation (Task 14, t-1461)

Plan: `2026-11-01-lot-l-local-agent.md`, Task 14. Set: `crates/oku-slides-local-agent/eval/requests.json`
(30 requests at first, 34 since t-1738; French and English, across edit, present, files, configure,
troubleshoot and packs, each with the families that serve it and the first tools a good turn calls). Run:

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

## 2026-11-25 — `knowledge` family (t-1738), Apple Foundation Models

The set grows to 34 requests: 4 for the new `knowledge` family (2 English, 2 French: pricing search, market
note read, listing the notes, searching for competitors). Same engine and command, fresh data dirs.
Bar: the 30 earlier requests keep route and first tool at 28/30 or more, the 4 new ones 3/4 or more.

| Run | Old 30 route | Old 30 first tool | New 4 route | New 4 first tool | `knowledge` summary |
|---|---|---|---|---|---|
| 1 | 24/30 | 22/30 | 4/4 | 4/4 | read or search the deck's knowledge base: notes, sources, references |
| 2 | 24/30 | 22/30 | 4/4 | 4/4 | read or search the notes of the deck's knowledge base, only when asked for the knowledge base |
| 3 | 25/30 | 23/30 | 4/4 | 4/4 | the knowledge base: read or search its notes |

Totals over 34: run 1 28 and 26, run 2 28 and 26, run 3 29 and 27. Run 3 is kept. The new requests all
route to `knowledge` and call a knowledge tool, but the old set stays below the bar of 28/30: **not met
after the 3 tuning runs** (follow-up t-1742: retune the other families on the old 30). The three runs differ
by one request, within model noise, so single runs cannot tell the summaries apart. Seen: "Use Claude as the
app agent" and the token-cost question still drift towards `knowledge` or `files`; "Why doesn't my agent
start?" is the only old miss that is run-3-only (it may be variance); a few first steps vary from run to run
("Delete the second slide" calls `slide`, "Enregistre une copie du deck" calls `deck_outline`). Narrowing the
`knowledge` wording alone did not recover the old requests; the baseline below shows why.

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

## 2026-11-25 — `knowledge` family (t-1738), Apple Foundation Models

The set grows to 34 requests: 4 for the new `knowledge` family (2 English, 2 French: pricing search, market
note read, listing the notes, searching for competitors). Same engine and command, fresh data dirs.
Bar: the 30 earlier requests keep route and first tool at 28/30 or more, the 4 new ones 3/4 or more.

| Run | Old 30 route | Old 30 first tool | New 4 route | New 4 first tool | `knowledge` summary |
|---|---|---|---|---|---|
| 1 | 24/30 | 22/30 | 4/4 | 4/4 | read or search the deck's knowledge base: notes, sources, references |
| 2 | 24/30 | 22/30 | 4/4 | 4/4 | read or search the notes of the deck's knowledge base, only when asked for the knowledge base |
| 3 | 25/30 | 23/30 | 4/4 | 4/4 | the knowledge base: read or search its notes |

Totals over 34: run 1 28 and 26, run 2 28 and 26, run 3 29 and 27. Run 3 is kept. The new requests all
route to `knowledge` and call a knowledge tool, but the old set stays below the bar of 28/30: **not met
after the 3 tuning runs.** Seen: a seventh family dilutes the routing of old requests (the dictation model
download, "Why doesn't my agent start?" and the PowerPoint import failure leave their families), "Use Claude
as the app agent" and the token-cost question still drift towards `knowledge` or `files`, and a few first
steps vary from run to run ("Delete the second slide" calls `slide`, "Enregistre une copie du deck" calls
`deck_outline`). Narrowing the `knowledge` wording alone did not recover them; a follow-up should look at
the other families' summaries.

Final run (3), Apple engine, about 5.5 s per request (route + first Act):

    ok  route edit,files               ok  tool deck_outline               7.2s | en Rename the deck to Roadmap 2027
    ok  route edit,files               ok  tool deck_outline               6.0s | fr Renomme le deck en Feuille de route 2027
    ok  route edit,present             ok  tool deck_outline               5.4s | en Add a slide titled Next steps after the last one
    ok  route edit,present             ok  tool deck_outline               5.6s | fr Ajoute une slide « Conclusion » à la fin
    ok  route edit                     ok  tool slide                      5.2s | en Make the title of slide 2 shorter
    ok  route edit                     ok  tool slide                      5.2s | fr Mets le titre de la slide 3 en gras
    ok  route edit                     BAD tool slide                      5.4s | en Delete the second slide
    ok  route edit,packs               ok  tool show_themes                6.5s | fr Change le thème du deck
    ok  route edit,present             ok  tool chart_kind                 5.5s | en Turn the table on slide 4 into a bar chart
    ok  route present,edit             ok  tool focus                      5.1s | fr Va à la slide 5
    ok  route present                  ok  tool present                    4.4s | en Start the presentation
    ok  route present                  ok  tool present                    4.3s | fr Lance la présentation en plein écran
    ok  route files                    ok  tool save                       4.3s | en Save the deck
    ok  route files                    BAD tool deck_outline               4.5s | fr Enregistre une copie du deck
    ok  route files,present            ok  tool new                        4.4s | en Create a new deck
    ok  route files                    ok  tool history                    4.7s | fr Montre-moi l'historique des versions
    ok  route files                    ok  tool ask_choice                 6.7s | en Merge another deck into this one
    ok  route configure                ok  tool settings                   5.3s | en Switch the interface to dark mode
    ok  route configure,edit           ok  tool set_interface              6.8s | fr Passe l'interface en anglais
    ok  route configure,edit           ok  tool settings                   6.5s | en Change the dictation language to German
    BAD route files                    BAD tool ask_choice                10.2s | fr Télécharge le modèle de dictée
    BAD route edit,knowledge           BAD tool deck_outline               7.5s | en Use Claude as the app agent
    BAD route knowledge,files          BAD tool deck_outline               4.6s | fr Combien m'a coûté ce deck en tokens ?
    ok  route configure                ok  tool settings                   5.0s | en Show the app settings
    BAD route configure,present        BAD tool settings                   5.4s | en Why doesn't my agent start?
    ok  route troubleshoot,present     ok  tool diagnose                   4.6s | fr Pourquoi l'agent Claude ne marche plus ?
    ok  route configure,troubleshoot   ok  tool diagnose                   5.4s | en The dictation doesn't hear me, what's wrong?
    ok  route troubleshoot,present     ok  tool diagnose                   4.6s | fr La slide 2 ne s'affiche pas, il y a une erreur de rendu
    BAD route files                    BAD tool deck_outline               4.5s | fr L'import du PowerPoint a raté, pourquoi ?
    ok  route edit,packs               ok  tool packs                      6.9s | en Edit the widgets of the deck's own pack
    ok  route knowledge                ok  tool knowledge_search           4.3s | en What does the knowledge base say about our pricing?
    ok  route knowledge,present        ok  tool knowledge_list             4.3s | fr Lis la fiche sur le marché dans la base de connaissance
    ok  route knowledge                ok  tool knowledge_list             4.2s | en List the notes in the deck's knowledge base
    ok  route knowledge                ok  tool knowledge_search           4.2s | fr Cherche dans la base de connaissance ce qu'on sait des concurrents

Baseline, same day, same engine (scratch run, `Family::Knowledge` removed from `ROUTABLE`, nothing kept): the
old 30 score **24/30 route and 22/30 first tool**, so the shortfall is not caused by the knowledge family
(run 3 with it: 25/30 and 23/30). Baseline misses, route: dictation model download, "Use Claude as the app
agent", the token-cost question, the PowerPoint import failure, "Change le thème du deck" and "Pourquoi l'agent
Claude ne marche plus ?" (the model refused the request: "Response may contain sensitive or unsafe content").
First tool, the same six plus "Delete the second slide" and "Enregistre une copie du deck". Shared with run 3:
the dictation model download, "Use Claude as the app agent", the token-cost question and the import failure
(route and first tool), plus "Delete the second slide" and "Enregistre une copie du deck" (first tool). Only in
run 3: "Why doesn't my agent start?"; only in the baseline: the theme change and the refused request. The
engine has drifted since 2026-11-01 (28/30 then); the new family is not the cause. The 4 new requests miss in
the baseline, as expected (no family to route to).
