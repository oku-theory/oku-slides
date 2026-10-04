# Fil de l'agent — suites de revue (t-454 à t-458)

Spec : `docs/superpowers/specs/2026-10-03-agent-a2ui-design.md` (autorité). Base : d61b1214 (t-448 à t-452).
Chaque étape est un ticket existant ; le texte de chaque tâche reprend le ticket.

## Global Constraints

- Rust : guidelines du dépôt (Snafu, pas de `unwrap`/`expect` hors tests, tests côte à côte `_tests.rs`, speculoos, `insta`, nommage `<unit>_<scenario>_<expected>`). Fichiers < 500 lignes visés, 1000 max.
- TS : tests Bun côte à côte (`*.test.ts(x)`, `*.render.test.tsx`), pas de `export *`.
- TDD : test rouge d'abord. Chaque correction de bug a un test de régression.
- i18n : tout texte visible passe par Fluent, 20 langues (`crates/oku-slides-i18n/src/locales/*/`).
- Commentaires et docs en français, comme le code environnant.
- Commits : Conventional Commits en français, avec le ticket `(t-N)` ; jamais de ligne « Co-Authored-By ». `git add` dans un appel séparé de `git commit`.
- Toute exécution de l'app, du serveur ou de `just e2e` : `OKU_SLIDES_PATHS__DATA` et `OKU_SLIDES_PATHS__DECKS` pointent vers un dossier temporaire.
- Vérification avant commit : `cargo nextest run` sur les crates touchées, `cd ui && bun run test` si l'UI change, `cargo clippy` sans avertissement sur les crates touchées.
- Pas de shim ni de compatibilité ascendante.

## Task 1: Deux questions parallèles : la dictée et les touches 1-4 peuvent viser des questions différentes (t-454)

`crates/oku-slides-mcp/src/hub.rs` (≈250-261) enregistre les questions en attente et les surfaces visibles sous deux verrous distincts ; avec deux `ask_choice` parallèles, `answer_pending` (réponse dictée) et `quickTarget` côté UI (`ui/src/agent/useQuickKeys.ts` ou voisin : touches 1-4) peuvent désigner des questions différentes. Le scénario e2e 7 a exactement cette forme. Attendu : une seule source d'ordre (même verrou, ou l'une dérivée de l'autre) pour que la dictée et les touches visent toujours la même question ; test de régression qui échoue avant (Rust ; et UI si la règle de ciblage côté UI change).

## Task 2: Fil de l'agent : petites finitions de code après t-449–t-452 (t-457)

Chaque point avec un test quand il change un comportement :
1. `slide_ref` dans un second `impl Session` (oku-slides-session) : fusionner dans le premier bloc pertinent.
2. `SlideRef` déclaré deux fois côté TS (interface + schéma zod) : dériver l'un de l'autre (`z.infer`).
3. Chiffres 1-4 affichés alors que `useQuickKeys` est désactivé (édition, pointage, widget actif) : ne plus les afficher dans ce cas.
4. Le chemin Entrée « option par défaut » prendrait l'Entrée de l'app si une question du dock avait un défaut : Entrée ne va à la question que si le focus est dans le dock (ou supprimer le chemin s'il n'a pas d'usage) — trancher d'après le code et le documenter.
5. Commentaire z-index de `.agent-thread` (`ui/src/styles.css` ≈556) devenu faux : le corriger.
6. Bloc log/notify dupliqué dans `crates/oku-slides-agent/src/bin/fake_agent.rs` : factoriser.
7. `observe(el.parentElement)` dépend de la structure DOM : passer par une ref explicite.
8. `aria-relevant="additions"` sur toute la liste peut réannoncer des re-rendus A2UI : limiter la région live aux ajouts de tours (clé stable), test.
9. ARCHITECTURE de oku-slides-session : décrire le mode historique et les états transitoires des miniatures de reçu.
10. Échap n'annule pas une question du dock : Échap avec le focus dans le dock annule la question ciblée (comme sur une feuille), sans voler l'Échap de l'app ailleurs ; test.

## Task 3: Fil de l'agent : tests restants (t-458)

1. `rx.recv()` non borné dans `crates/oku-slides-mcp/tests/edit.rs` (≈17, 78) : attente bornée (timeout).
2. `report_surface` accepte un fait blanc mêlé à des faits non blancs : écarter les faits blancs (ou refuser) — trancher, test d'abord.
3. Le test de légende d'état compare la clé Fluent, pas le texte traduit : tester le texte rendu (charger les traductions dans ce test ou tester côté Rust).
4. Trois tests du dock ne vérifient que le texte de la feuille de style : les remplacer par des assertions de mise en page/attributs observables (ou e2e) ; ne pas garder de test sur le texte CSS.
5. Une slide dont la page échoue au rendu donne un reçu sans référence (avant : erreur) : test de ce comportement.
6. Test de session qui atteint la carte du reçu par indices codés en dur : chercher la carte par id/type.
7. Scénario e2e sans `report` ni `tool_calls` (captures bruitées) : ajouter `report` et `tool_calls` au script du faux agent (`fixtures/agent/e2e.json`) pour les scénarios concernés, et des assertions sur le reçu et le compteur d'outils. Lancer `just e2e` (données isolées).

## Task 4: Traductions — t-455 et t-456

Deux commits, un par ticket.

t-455 : en ru/uk, `agent-instructions-widgets` dit « колода » alors qu'`agent-instructions` dit « презентация » (aligner sur презентация / презентація) ; en ja/zh-Hant, `cli.ftl` garde une espace entre nombre et compteur (`{ $pages } ページ` / `頁`) : la retirer, et vérifier zh-Hans et ko dans cli.ftl ; th : compteur d'outils sans classificateur → `เครื่องมือ { $tools } รายการ` ; étendre le test d'espacement CJK (`crates/oku-slides-i18n/src/lib_tests.rs`) à cli.ftl et à tout blanc Unicode (`char::is_whitespace`).

t-456 (décision du 2026-10-04) : vi « tác tử » → « tác nhân AI » dans toute la locale (ui, agent, session…, ≈24 occurrences ; adapter la grammaire : « tác nhân AI » en tête de phrase, majuscule, etc.) ; zh-Hant 智慧代理 → « AI 代理 » dans toute la locale. Ensuite grep : plus aucune occurrence de l'ancien terme.
