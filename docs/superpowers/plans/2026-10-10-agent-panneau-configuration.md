# Panneau de configuration des agents — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** choisir l'agent ACP actif dans une liste définie par la configuration, depuis une fiche de la zone de l'agent, avec bascule immédiate.

**Architecture:** `[agent]` devient `selected` + `[agent.presets.<id>]` (`oku-slides-platform`). `Link` gagne une bascule synchrone (`switch`, époque + jeton `stop` par connexion). `Stack` construit une surface A2UI `agent-picker` (module pur `picker`), l'émet à l'onglet qui la demande (`AgentEvent::Picker`) et bascule sur `AgentCommand::SelectAgent`. L'UI l'affiche dans `AgentPicker.tsx`, ouvert depuis l'en-tête d'`AgentThread`.

**Tech Stack:** Rust (snafu, config, tokio, agent-client-protocol 2.2, speculoos, insta), React/TypeScript (Bun test, @a2ui/web_core), Fluent (20 langues), Playwright (e2e).

**Spec:** `docs/superpowers/specs/2026-10-10-agent-panneau-configuration-design.md` (t-775).

## Global Constraints

- Tâche : **t-775** ; chaque commit se termine par `(t-775)` ; messages en français, Conventional Commits ; **jamais** de ligne `Co-Authored-By`.
- Worktree : `.worktrees/775-panneau-agents`, branche `775-feat/panneau-agents`. Commits avec `git commit -F <fichier du scratchpad>` (jamais de heredoc) ; exemption de revue SDD : `git config --worktree oku.review deferred` (posée au démarrage, retirée à la fin).
- Cargo : `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target` (dossier partagé, décision de l'utilisateur).
- App, `serve`, e2e : **toujours** `OKU_SLIDES_PATHS__DATA` et `OKU_SLIDES_PATHS__DECKS` vers un dossier temporaire neuf ; e2e sur un port libre (`just e2e <port>`, jamais 4872 par défaut si une autre session tourne).
- Presets livrés, versions exactes : `@agentclientprotocol/claude-agent-acp@0.81.2`, `@agentclientprotocol/codex-acp@2.2.2`, `@google/gemini-cli@0.63.0` + `--acp`.
- Pas de shim (ADR-015) : `agent.command`, `agent.args`, `OKU_SLIDES_AGENT__COMMAND`/`ARGS` disparaissent, tous les consommateurs suivent.
- Pas de `.unwrap()`/`.expect()` en production ; erreurs snafu avec `backtrace` ; `std::fs` interdit hors tests (écriture de `config.toml` par `write_value` existant).
- Tout texte montré à l'utilisateur passe par Fluent dans les **20** langues (`crates/oku-slides-i18n/src/locales/*/`) ; `lib_tests.rs` impose exactement les mêmes clés partout, et `ui.ftl` finit chaque fichier par les messages UI.
- Fichiers : 500 lignes visées, 1000 max ; tests côte à côte (`foo_tests.rs`, `Foo.render.test.tsx`).
- Nommage des tests : `<unité>_<scénario>_<résultat>`.

---

## Carte des fichiers

| Fichier | Rôle | Tâche |
|---|---|---|
| `crates/oku-slides-platform/src/config.rs` | `AgentSettings`, `AgentPreset`, validation, `locked` | 1 |
| `crates/oku-slides-platform/src/config.default.toml` | `[agent] selected` + 3 presets | 1 |
| `crates/oku-slides-platform/src/config_tests.rs` | tests des presets | 1 |
| `crates/oku-slides-agent/src/stack.rs` | `Stack::start` lit `active()` ; puis commandes `Picker`/`SelectAgent` | 1, 5 |
| `crates/oku-slides-agent/tests/common/mod.rs` | piles de test sur presets ; second agent | 1, 5 |
| `crates/oku-slides-agent/examples/smoke.rs` | (lit déjà `Platform` : vérifier) | 1 |
| `justfile`, `ui/e2e/agent.cjs` | faux agent par `config.toml` isolé | 1 |
| `README.md` | tableau des réglages | 1 |
| `crates/oku-slides-i18n/src/locales/*/agent.ftl` | `agent-not-found` ; clés `agent-picker-*` | 1, 4, 5 |
| `crates/oku-slides-agent/src/link.rs` | `switch`, époque, `stop` | 2 |
| `crates/oku-slides-agent/src/tab_agent.rs:261` | `fallback_cwd()` rend un `PathBuf` | 2 |
| `crates/oku-slides-agent/tests/launch.rs` | tests de `switch` | 2 |
| `crates/oku-slides-agent/src/settings_port.rs` (+ `_tests`) | refus de `agent.*` | 3 |
| `crates/oku-slides-agent/src/picker.rs` (+ `picker_tests.rs`, snapshots) | surface `agent-picker`, pure | 4 |
| `crates/oku-slides-agent/src/events.rs` | `AgentEvent::Picker`, sans `Eq` | 4 |
| `crates/oku-slides-agent/src/turns.rs` | ignore `Picker` comme `Draft` | 4 |
| `crates/oku-slides-agent/src/command_error.rs` (+ `_tests`) | `UnknownAgent`, `AgentLocked`, `AgentNotSaved` | 5 |
| `crates/oku-slides-agent/tests/stack.rs` | bascule de bout en bout | 5 |
| `crates/oku-slides-agent/ARCHITECTURE.md`, `src/lib.rs` | docs | 5 |
| `ui/src/api.ts` | types `picker`, `agentApi.picker/selectAgent` | 7 |
| `ui/src/icons.tsx` | icône `settings` | 7 |
| `ui/src/agent/AgentPicker.tsx` (+ render test) | la fiche | 7 |
| `ui/src/agent/AgentThread.tsx`, `AgentPanel.tsx` (+ tests) | bouton, état | 7 |
| `crates/oku-slides-i18n/src/locales/*/ui.ftl` | `ui-agent-picker` | 7 |
| `fixtures/agent/e2e-bis.json`, `ui/e2e/agent-picker.cjs`, `ui/package.json`, `justfile` | e2e | 8 |

---

### Task 0: Mise en place du worktree

**Files:** aucun fichier de code.

- [ ] **Step 1: Exemption de revue SDD, portée worktree**

```bash
git -C /Users/matthieu/Projects/slider/.worktrees/775-panneau-agents config --worktree oku.review deferred
git -C /Users/matthieu/Projects/slider/.worktrees/775-panneau-agents config --show-scope --get-all oku.review
```
Expected: `worktree	deferred`.

- [ ] **Step 2: Committer la spec et ce plan**

Message (fichier `msg.txt` du scratchpad) : `docs(plan) : panneau de configuration des agents — spec et plan en 8 tâches (t-775)`

```bash
git add docs/superpowers/specs/2026-10-10-agent-panneau-configuration-design.md docs/superpowers/plans/2026-10-10-agent-panneau-configuration.md
git commit -F <scratchpad>/msg.txt
```

- [ ] **Step 3: Sous-tâches Oku** — une par tâche 1 à 8 de ce plan, titre identique :

```bash
oku cli task create --project slider --parent t-775 --name "<titre exact de la tâche N>"
```
Puis mettre à jour le corps de t-775 avec le chemin du plan et la liste des titres (`oku cli task update t-775 --body …`). Chaque sous-tâche passe `oku cli task done <id>` à la fin de sa tâche.

---

### Task 1: Liste d'agents dans la configuration

**Files:**
- Modify: `crates/oku-slides-platform/src/config.rs:240-275` (types, `load`, `environment`, `from_sources`)
- Modify: `crates/oku-slides-platform/src/config.default.toml:122-127`
- Test: `crates/oku-slides-platform/src/config_tests.rs:121-140`
- Modify: `crates/oku-slides-agent/src/stack.rs:143-150`
- Modify: `crates/oku-slides-agent/tests/common/mod.rs:418-424`
- Modify: `justfile:61-65`, `ui/e2e/agent.cjs:3`, `README.md:185-186`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (ligne `agent-not-found`, 20 fichiers)

**Interfaces:**
- Produces (dans `oku_slides_platform::config`, réexporté comme `Settings` l'est déjà par `oku_slides_session`) :
  - `pub struct AgentSettings { pub selected: String, pub presets: BTreeMap<String, AgentPreset>, #[serde(skip)] pub locked: bool }`
  - `pub struct AgentPreset { pub name: String, pub command: String, pub args: Vec<String> }`, `#[non_exhaustive]`, `AgentPreset::new(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self`
  - `impl AgentSettings { pub fn active(&self) -> &AgentPreset }` — sûr après chargement (validé) ; panique (invariant) si appelé sur des réglages non validés construits à la main.

- [ ] **Step 1: Tests rouges** — remplacer les deux tests `defaults_agent_runs_claude_agent_acp_through_npx` et `env_agent_args_split_on_spaces` par :

```rust
#[test]
fn defaults_agent_presets_are_claude_codex_gemini_with_claude_selected() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.agent.presets.keys().cloned().collect::<Vec<_>>())
        .is_equal_to(vec!["claude".to_owned(), "codex".to_owned(), "gemini".to_owned()]);
    assert_that!(s.agent.selected.as_str()).is_equal_to("claude");
    let claude = s.agent.active();
    assert_that!(claude.command.as_str()).is_equal_to("npx");
    assert_that!(claude.args.clone())
        .is_equal_to(vec!["-y".to_owned(), "@agentclientprotocol/claude-agent-acp@0.81.2".to_owned()]);
    assert_that!(s.agent.presets["codex"].args.clone())
        .is_equal_to(vec!["-y".to_owned(), "@agentclientprotocol/codex-acp@2.2.2".to_owned()]);
    assert_that!(s.agent.presets["gemini"].args.clone())
        .is_equal_to(vec!["-y".to_owned(), "@google/gemini-cli@0.63.0".to_owned(), "--acp".to_owned()]);
    assert_that!(s.agent.locked).is_false();
}

#[test]
fn config_file_preset_is_added_without_dropping_the_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("config.toml");
    std::fs::write(&file, "[agent]\nselected = \"faux\"\n[agent.presets.faux]\nname = \"Faux\"\ncommand = \"/bin/faux\"\nargs = [\"--x\"]\n").unwrap();
    let s = Settings::from_sources(Some(&file), Settings::environment().source(Some(HashMap::new()))).unwrap();
    assert_that!(s.agent.presets.len()).is_equal_to(4);
    assert_that!(s.agent.active().command.as_str()).is_equal_to("/bin/faux");
}

#[test]
fn unknown_selected_agent_fails_to_load_and_names_the_known_ones() {
    let env = Settings::environment()
        .source(Some(HashMap::from([("OKU_SLIDES_AGENT__SELECTED".to_owned(), "mistral".to_owned())])));
    let e = Settings::from_sources(None, env).unwrap_err();
    let chain = snafu::Report::from_error(&e).to_string();
    assert_that!(chain.contains("mistral") && chain.contains("claude, codex, gemini")).is_true();
}

#[test]
fn env_selected_agent_wins_over_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("config.toml");
    std::fs::write(&file, "[agent]\nselected = \"codex\"\n").unwrap();
    let env = Settings::environment()
        .source(Some(HashMap::from([("OKU_SLIDES_AGENT__SELECTED".to_owned(), "gemini".to_owned())])));
    let s = Settings::from_sources(Some(&file), env).unwrap();
    assert_that!(s.agent.selected.as_str()).is_equal_to("gemini");
}
```

(Ajouter `use std::collections::HashMap;` en tête du fichier de tests s'il manque. `tempfile` est déjà une dev-dependency de la plateforme ; sinon l'ajouter `tempfile.workspace = true`.)

- [ ] **Step 2: Les voir échouer**

Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-platform config_tests`
Expected: erreurs de compilation (`presets`, `selected`, `active` inconnus).

- [ ] **Step 3: Implémenter**

`config.default.toml`, remplacer le bloc `[agent]` (lignes 122-127) par :

```toml
[agent]
# Agent ACP lancé par ⌘⇧D : une clé de [agent.presets], choisie dans le
# panneau de l'agent (écrite dans config.toml). Il ne reçoit que les outils
# d'Oku Slides (serveur MCP local), ni fichiers ni terminal.
# OKU_SLIDES_AGENT__SELECTED impose le choix (le panneau est alors verrouillé).
# Un config.toml peut ajouter un agent : [agent.presets.<id>] name, command, args.
selected = "claude"

[agent.presets.claude]
name = "Claude"
command = "npx"
args = ["-y", "@agentclientprotocol/claude-agent-acp@0.81.2"]

[agent.presets.codex]
name = "Codex"
command = "npx"
args = ["-y", "@agentclientprotocol/codex-acp@2.2.2"]

[agent.presets.gemini]
name = "Gemini"
command = "npx"
args = ["-y", "@google/gemini-cli@0.63.0", "--acp"]
```

`config.rs` : remplacer `AgentSettings` par :

```rust
/// Agents ACP proposés et celui qui est lancé par ⌘⇧D.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct AgentSettings {
    /// L'agent actif : une clé de `presets` (vérifiée au chargement).
    pub selected: String,
    /// Les agents proposés, par identifiant (ordre d'affichage : celui des clés).
    pub presets: BTreeMap<String, AgentPreset>,
    /// `OKU_SLIDES_AGENT__SELECTED` impose le choix : le panneau ne le change pas.
    #[serde(skip)]
    pub locked: bool,
}

/// Un agent proposé : son nom affiché et comment le lancer.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct AgentPreset {
    /// Nom propre affiché tel quel (pas traduit).
    pub name: String,
    /// Programme lancé (cherché dans le `PATH`).
    pub command: String,
    pub args: Vec<String>,
}

impl AgentPreset {
    /// Un agent proposé (tests, outils).
    pub fn new(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self { name: name.into(), command: command.into(), args }
    }
}

impl AgentSettings {
    /// L'agent actif. `selected` est vérifié au chargement : l'absence est un invariant violé.
    pub fn active(&self) -> &AgentPreset {
        self.presets.get(&self.selected).unwrap_or_else(|| panic!("agent.selected = {:?} non vérifié", self.selected))
    }

    /// `selected` désigne-t-il un agent de la liste ? Sinon, le message de l'erreur.
    fn check(&self) -> Result<(), String> {
        if self.presets.contains_key(&self.selected) {
            return Ok(());
        }
        let known: Vec<&str> = self.presets.keys().map(String::as_str).collect();
        Err(format!("agent.selected = {:?} names no [agent.presets] entry (known: {})", self.selected, known.join(", ")))
    }
}
```

Note : `active()` panique sur un invariant violé (réglages validés par `from_sources`) ; c'est la règle « panic pour une erreur de programmation ». Les tests qui modifient `settings.agent` à la main doivent garder `selected` dans `presets`.

`environment()` : retirer `.list_separator(" ")` et `.with_list_parse_key("agent.args")`, et mettre à jour son commentaire (« Variables `OKU_SLIDES_<SECTION>__<CLÉ>` »).

`from_sources` : après `try_deserialize`, valider :

```rust
        let settings: Self =
            builder.add_source(env).build().and_then(config::Config::try_deserialize).context(ConfigSnafu)?;
        settings.agent.check().map_err(config::ConfigError::Message).context(ConfigSnafu)?;
        Ok(settings)
```

`load()` : poser le verrou après `from_sources` :

```rust
        let mut settings = Self::from_sources(Some(&data.join("config.toml")), env)?;
        settings.agent.locked = std::env::var_os("OKU_SLIDES_AGENT__SELECTED").is_some();
        Ok(settings)
```

Ajouter `use std::collections::BTreeMap;`.

`stack.rs` (`Stack::start`) :

```rust
        let a = platform.settings.agent.active();
        let link = Link::new(Launch {
            command: a.command.clone(),
            args: a.args.clone(),
            fallback_cwd: platform.settings.paths.data(),
            ready_within: crate::READY_WITHIN,
        });
```

`tests/common/mod.rs` (`build_stack`) : remplacer les deux affectations `settings.agent.command/args` par :

```rust
        settings.agent.presets.insert("faux".into(), AgentPreset::new("Faux", command, args));
        settings.agent.selected = "faux".into();
```
(import `use oku_slides_platform::config::AgentPreset;` — ou le chemin réexporté par `oku_slides_session` si la plateforme n'est pas une dev-dependency ; vérifier avec `grep -n platform crates/oku-slides-agent/Cargo.toml`.)

`justfile` (recette `e2e`) : remplacer les lignes `OKU_SLIDES_AGENT__COMMAND=… \` et `OKU_SLIDES_AGENT__ARGS=… \` par l'écriture du `config.toml` isolé avant le lancement :

```bash
    mkdir -p "$tmp/data"
    printf '[agent]\nselected = "e2e"\n\n[agent.presets.e2e]\nname = "Faux"\ncommand = "%s"\nargs = ["--script", "%s"]\n' \
      "$bin/oku-slides-fake-agent" "$PWD/fixtures/agent/e2e.json" > "$tmp/data/config.toml"
    "$bin/oku-slides" --fonts {{fonts}} serve "{{deck}}" --ui ui/dist --port "$port" &
```

`ui/e2e/agent.cjs:3` : « lancé avec le faux agent comme agent actif (`config.toml` du dossier de données isolé ; just e2e s'en charge) ».

`README.md:185-186` : remplacer les deux lignes par
`| [agent] selected / OKU_SLIDES_AGENT__SELECTED | claude |` et
`| [agent.presets.<id>] name, command, args | Claude, Codex, Gemini (npx, versions épinglées) |`.

`agent-not-found` dans les 20 `agent.ftl` : remplacer la fin « Réglage [agent] command. » par le renvoi à `[agent.presets]`. FR : `agent-not-found = Agent introuvable : { $command }. Réglage [agent.presets].` EN : `agent-not-found = Agent not found: { $command }. Setting [agent.presets].` Les 18 autres : même changement, seule la référence `[agent] command` devient `[agent.presets]` (le reste de la phrase ne bouge pas).

- [ ] **Step 4: Vert, et le reste du workspace compile**

Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-platform -p oku-slides-agent -p oku-slides-i18n`
Expected: PASS. Puis `cargo check --workspace --all-targets` : aucun autre consommateur de `agent.command` (`grep -rn "agent\.command\|agent\.args\|AGENT__COMMAND\|AGENT__ARGS" --include='*.rs' --include='*.cjs' --include='justfile' --include='*.md' crates src-tauri ui justfile README.md` → seules les docs historiques `docs/superpowers/` restent).

- [ ] **Step 5: Commit** — `feat(agent)! : liste d'agents dans la configuration ([agent.presets], selected) (t-775)`

---

### Task 2: Bascule d'agent dans `Link`

**Files:**
- Modify: `crates/oku-slides-agent/src/link.rs` (`Live`, `Inner`, `Link::new/agent_id/fallback_cwd/connect/launch`, `Serving`)
- Modify: `crates/oku-slides-agent/src/tab_agent.rs:261`
- Test: `crates/oku-slides-agent/tests/launch.rs`

**Interfaces:**
- Consumes: `Launch { command, args, fallback_cwd, ready_within }` (inchangé).
- Produces: `impl Link { pub fn switch(&self, launch: Launch) }` (synchrone) ; `pub(crate) fn fallback_cwd(&self) -> PathBuf` (était `&Path`).

- [ ] **Step 1: Tests rouges** (dans `tests/launch.rs`) :

```rust
/// Un second faux agent, à côté du premier : son scénario et son journal.
fn second_launch(h: &Harness, script: serde_json::Value) -> (oku_slides_agent::Launch, std::path::PathBuf) {
    let (path, log) = (h.log.with_file_name("script-bis.json"), h.log.with_file_name("log-bis.jsonl"));
    std::fs::write(&path, script.to_string()).unwrap();
    let launch = oku_slides_agent::Launch {
        command: env!("CARGO_BIN_EXE_oku-slides-fake-agent").to_owned(),
        args: vec!["--script".into(), path.to_string_lossy().into_owned(), "--log".into(), log.to_string_lossy().into_owned()],
        fallback_cwd: h.log.parent().unwrap().to_owned(),
        ready_within: oku_slides_agent::READY_WITHIN,
    };
    (launch, log)
}

#[tokio::test(flavor = "multi_thread")]
async fn switch_between_turns_sends_the_next_prompt_to_the_new_agent() {
    let h = start(json!({ "turns": [{ "chunks": ["un"] }, { "chunks": ["deux"] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("premier".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    let before = h.link.agent_id();
    let (launch, log_bis) = second_launch(&h, json!({ "turns": [{ "chunks": ["bis"] }] }));
    h.link.switch(launch);
    assert_that!(h.link.agent_id()).is_not_equal_to(before);
    t.agent.prompt("second".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Chunk { text, .. } if text == "bis")).await;
    assert_that!(logged(&log_bis, "session/prompt").len()).is_equal_to(1);
    assert_that!(logged(&h.log, "session/prompt").len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn switch_during_a_turn_stops_it_and_the_next_prompt_reaches_the_new_agent() {
    // Le premier agent ne finit jamais son tour (question sans réponse).
    let h = start(json!({ "turns": [{ "mcp": [{ "tool": "ask_choice", "args": { "question": "?",
        "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("premier".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Status { state: AgentState::Working, .. })).await;
    let (launch, log_bis) = second_launch(&h, json!({ "turns": [{ "chunks": ["bis"] }] }));
    h.link.switch(launch);
    until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await;
    t.agent.prompt("second".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Chunk { text, .. } if text == "bis")).await;
    assert_that!(logged(&log_bis, "session/prompt").len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_after_switch_is_still_final() {
    let h = start(json!({ "turns": [] })).await;
    let (launch, _) = second_launch(&h, json!({ "turns": [] }));
    h.link.switch(launch);
    h.link_shutdown().await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("après".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await;
}
```

`Harness::link_shutdown` : si `Link::shutdown` est `pub(crate)` et qu'aucun test ne l'appelle encore, utiliser à la place la voie existante des tests (`grep -n "shutdown" crates/oku-slides-agent/tests/*.rs`) ; à défaut, rendre `Link::shutdown` `pub` (documenté « fin de l'app ») et appeler `h.link.shutdown(Duration::from_secs(2)).await`. Le message d'erreur attendu est `agent-stopped` dans la langue du test (comparer au test existant `agent_that_never_gets_ready_is_stopped_and_the_next_start_works`).

Le test « bascule pendant un lancement » : ajouter

```rust
#[tokio::test(flavor = "multi_thread")]
async fn switch_while_launching_relaunches_with_the_new_agent() {
    // Le premier agent met 1 s à répondre à `initialize` (`"init_delay_ms"` du faux agent,
    // à ajouter dans bin/fake_agent s'il n'existe pas : attente avant la réponse).
    let h = start(json!({ "init_delay_ms": 1000, "turns": [{ "chunks": ["un"] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("premier".into(), Default::default());
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let (launch, log_bis) = second_launch(&h, json!({ "turns": [{ "chunks": ["bis"] }] }));
    h.link.switch(launch);
    until(&mut rx, |e| matches!(e, AgentEvent::Chunk { text, .. } if text == "bis")).await;
    assert_that!(logged(&h.log, "session/prompt")).is_empty();
    assert_that!(logged(&log_bis, "session/prompt").len()).is_equal_to(1);
}
```

Avant d'écrire ce test, lire `crates/oku-slides-agent/src/bin/fake_agent*` : si un délai d'`initialize` existe déjà (champ du scénario), l'utiliser ; sinon l'ajouter (`init_delay_ms: Option<u64>`, `tokio::time::sleep` avant la réponse à `initialize`) dans le même commit.

- [ ] **Step 2: Rouge**

Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-agent --test launch switch`
Expected: compilation échoue (`switch` inconnu).

- [ ] **Step 3: Implémenter** dans `link.rs` :

1. `Live` gagne `pub epoch: u64` et `pub stop: CancellationToken`.
2. `Inner` : remplacer `launch: Launch` par

```rust
    /// Le lancement courant et son époque ; `switch` change les deux ensemble.
    launch: StdMutex<(Launch, u64)>,
    /// Le jeton `stop` de la connexion vivante : `switch` l'annule.
    current_stop: StdMutex<Option<CancellationToken>>,
```

3. `Link::new` : `launch: StdMutex::new((launch, 0))`, `current_stop: StdMutex::default()`.
4. `agent_id()` lit `self.launch()` (helper privé `fn launch_now(&self) -> (Launch, u64)` : clone sous le verrou, `unwrap_or_else(PoisonError::into_inner)`).
5. `fallback_cwd(&self) -> PathBuf` : `self.launch_now().0.fallback_cwd`. Dans `tab_agent.rs:261` : `dir.unwrap_or_else(|| self.link.fallback_cwd())`.
6. Nouvelle méthode :

```rust
    /// Change d'agent : le prochain [`Link::connect`] lance `launch`.
    ///
    /// La connexion vivante est arrêtée tout de suite (ses tours échouent comme pour tout arrêt) ;
    /// une connexion en cours de lancement l'est dès qu'elle est prête. Sans effet après `shutdown`.
    pub fn switch(&self, launch: Launch) {
        {
            let mut current = self.inner.launch.lock().unwrap_or_else(PoisonError::into_inner);
            current.0 = launch;
            current.1 += 1;
        }
        if let Some(stop) = self.inner.current_stop.lock().unwrap_or_else(PoisonError::into_inner).take() {
            stop.cancel();
        }
        tracing::info!("agent changé");
    }
```

7. `connect` : boucle jusqu'à une connexion de l'époque courante :

```rust
    pub(crate) async fn connect(&self) -> Result<Live, AgentError> {
        let mut state = self.inner.state.lock().await;
        loop {
            if self.inner.shutdown.is_cancelled() {
                return Err(StoppedSnafu { message: "shut down".to_owned() }.build().into());
            }
            let epoch = self.launch_now().1;
            if let Some(live) = &state.live {
                if !live.closed.is_cancelled() && live.epoch == epoch {
                    return Ok(live.clone());
                }
                // D'une autre époque : l'agent a été changé, celui-ci s'arrête.
                live.stop.cancel();
            }
            state.live = None;
            state.generation += 1;
            let live = self.launch(state.generation).await?;
            *self.inner.current_stop.lock().unwrap_or_else(PoisonError::into_inner) = Some(live.stop.clone());
            state.live = Some(live);
            // Changé pendant la poignée de main : on repasse par le haut de la boucle.
        }
    }
```

8. `launch(generation)` : lire `let (Launch { command, args, ready_within, .. }, epoch) = self.launch_now();` au début ; créer `let stop = self.inner.shutdown.child_token();` ; le passer à `Serving { …, stop: stop.clone(), epoch }` ; `ready_within` remplace `self.inner.launch.ready_within`.
9. `Serving` : champs `stop: CancellationToken` et `epoch: u64` ; dans la poignée de main et la boucle de service, remplacer `shutdown.cancelled()` par `stop.cancelled()` (le jeton enfant est annulé aussi par `shutdown`), garder le commentaire en l'adaptant (« fin de l'app ou agent changé ») ; `Live { …, epoch, stop: stop.clone() }`. Le champ `shutdown` de `Serving` ne sert plus : le retirer.

Mettre à jour la doc de module de `link.rs` (« relancé après un arrêt ou un changement d'agent (`switch`) »).

- [ ] **Step 4: Vert** — `cargo nextest run -p oku-slides-agent` (toute la crate : `launch`, `tabs`, `stack`, `scopes`, `turns`). Expected: PASS.

- [ ] **Step 5: Commit** — `feat(agent) : bascule d'agent dans Link (époque, arrêt de la connexion vivante) (t-775)`

---

### Task 3: `set_setting` refuse les réglages de l'agent

**Files:**
- Modify: `crates/oku-slides-agent/src/settings_port.rs:71-82`
- Test: `crates/oku-slides-agent/src/settings_port_tests.rs`

- [ ] **Step 1: Test rouge**

```rust
#[rstest::rstest]
#[case("agent.selected")]
#[case("agent.presets.x.command")]
#[case("agent")]
fn write_agent_key_is_refused_and_config_untouched(#[case] key: &str) {
    let (p, _dir) = platform();
    let port = AppSettings::new(&p, None);
    assert_that!(port.write(key, &json!("x"))).is_equal_to(Err("agent settings are reserved to the user".to_owned()));
    assert_that!(p.settings.config_file().exists()).is_false();
}
```
(Si `rstest` n'est pas une dev-dependency de la crate, écrire trois `#[test]` ; ne pas ajouter de dépendance pour ça.)

- [ ] **Step 2: Rouge** — `cargo nextest run -p oku-slides-agent settings_port`. Expected: FAIL (écrit dans `config.toml`).

- [ ] **Step 3: Implémenter** — en tête de `write` :

```rust
        // L'agent ne choisit ni ne lance un autre agent : réservé au panneau de l'utilisateur.
        if key == "agent" || key.starts_with("agent.") {
            return Err("agent settings are reserved to the user".to_owned());
        }
```
`read` n'expose déjà aucune clé `agent.*` : le vérifier, ne rien ajouter. Mettre à jour la doc de module (« … sauf les réglages de l'agent, réservés à l'utilisateur »).

- [ ] **Step 4: Vert** — même commande. Expected: PASS.

- [ ] **Step 5: Commit** — `feat(agent) : set_setting refuse les réglages de l'agent (t-775)`

---

### Task 4: Surface `agent-picker` et évènement `Picker`

**Files:**
- Create: `crates/oku-slides-agent/src/picker.rs`, `crates/oku-slides-agent/src/picker_tests.rs`, snapshots `crates/oku-slides-agent/src/snapshots/*picker*`
- Modify: `crates/oku-slides-agent/src/events.rs` (variante, derive), `crates/oku-slides-agent/src/turns.rs:159,263`, `crates/oku-slides-agent/src/lib.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (3 clés, 20 fichiers)

**Interfaces:**
- Consumes: `AgentPreset` (tâche 1) ; `oku_slides_session::agent::{Surface, Place}` ; `oku_slides_a2ui::{Component, ComponentKind, ChoiceOption, Dynamic, Action, TextVariant, ServerMessage}`.
- Produces:
  - `pub(crate) const PICKER_ID: &str = "agent-picker";`
  - `pub(crate) fn picker_surface(presets: &BTreeMap<String, AgentPreset>, active: &str, busy: bool, locked: bool, l: &Localizer) -> Surface`
  - `AgentEvent::Picker { surface: Surface }` (JSON `{"kind":"picker","surface":{…}}`) ; `AgentEvent` dérive `Debug, Clone, PartialEq, Serialize` (plus `Eq`).
  - Action de la surface : évènement `select`, contexte `{ "id": <path /choice> }`.

- [ ] **Step 1: Tests rouges** (`picker_tests.rs`) :

```rust
use std::collections::BTreeMap;

use insta::assert_json_snapshot;
use oku_slides_i18n::Localizer;
use oku_slides_platform::config::AgentPreset;
use speculoos::prelude::*;

use super::{PICKER_ID, picker_surface};

fn presets() -> BTreeMap<String, AgentPreset> {
    BTreeMap::from([
        ("claude".to_owned(), AgentPreset::new("Claude", "npx", vec!["-y".into(), "claude-agent-acp".into()])),
        ("gemini".to_owned(), AgentPreset::new("Gemini", "npx", vec!["-y".into(), "gemini".into(), "--acp".into()])),
    ])
}

#[test]
fn picker_surface_idle_offers_every_preset_with_the_active_one_chosen() {
    let s = picker_surface(&presets(), "gemini", false, false, &Localizer::new(Some("fr")));
    assert_that!(s.id.as_str()).is_equal_to(PICKER_ID);
    assert_json_snapshot!(s);
}

#[test]
fn picker_surface_busy_warns_that_the_turn_will_stop() {
    assert_json_snapshot!(picker_surface(&presets(), "claude", true, false, &Localizer::new(Some("fr"))));
}

#[test]
fn picker_surface_locked_has_no_action() {
    let s = picker_surface(&presets(), "claude", false, true, &Localizer::new(Some("fr")));
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json.contains("\"select\"")).is_false();
    assert_json_snapshot!(s);
}
```

Et dans `events_tests.rs` :

```rust
#[test]
fn picker_event_serializes_with_its_surface() {
    let s = crate::picker::picker_surface(&BTreeMap::new(), "x", false, false, &Localizer::new(Some("fr")));
    let v = serde_json::to_value(AgentEvent::Picker { surface: s }).unwrap();
    assert_that!(v["kind"].clone()).is_equal_to(json!("picker"));
    assert_that!(v["surface"]["id"].clone()).is_equal_to(json!("agent-picker"));
}
```
(Adapter les imports aux conventions du fichier ; `oku-slides-platform` doit être accessible depuis la crate, sinon passer par le réexport de `oku_slides_session`.)

- [ ] **Step 2: Rouge** — `cargo nextest run -p oku-slides-agent picker events`. Expected: compilation échoue.

- [ ] **Step 3: Implémenter** `picker.rs` :

```rust
//! Le panneau de l'agent : choisir l'agent ACP de l'app parmi ceux de la configuration.
//!
//! Surface A2UI pure, construite par la pile à la demande d'une vue (`AgentCommand::Picker`).

use std::collections::BTreeMap;

use oku_slides_a2ui::{Action, ChoiceOption, Component, ComponentKind, Dynamic, ServerMessage, TextVariant, Tone};
use oku_slides_i18n::Localizer;
use oku_slides_platform::config::AgentPreset;
use oku_slides_session::agent::{Place, Surface};
use serde_json::json;

/// Identifiant de la surface du panneau.
pub(crate) const PICKER_ID: &str = "agent-picker";

/// Le panneau : une pastille par agent, l'actif choisi.
///
/// `busy` : un tour est ouvert dans un onglet (il sera interrompu) ; `locked` : le choix
/// est imposé par l'environnement, les pastilles n'envoient rien.
pub(crate) fn picker_surface(
    presets: &BTreeMap<String, AgentPreset>,
    active: &str,
    busy: bool,
    locked: bool,
    l: &Localizer,
) -> Surface {
    let options = presets
        .iter()
        .map(|(id, p)| ChoiceOption {
            id: id.clone(),
            label: p.name.clone(),
            hint: Some(std::iter::once(p.command.as_str()).chain(p.args.iter().map(String::as_str)).collect::<Vec<_>>().join(" ")),
        })
        .collect();
    let action = (!locked).then(|| Action::event("select", [("id", Dynamic::path("/choice"))]));
    let mut children = vec!["title", "agents"];
    let mut components = vec![
        Component::text("title", l.tr("agent-picker-title"), Some(TextVariant::H4)),
        Component::new("agents", ComponentKind::ChoiceChips { options, value: Dynamic::path("/choice"), action }),
    ];
    if busy {
        children.push("busy");
        components.push(Component::text("busy", l.tr("agent-picker-busy"), None));
    }
    if locked {
        children.push("locked");
        components.push(Component::text("locked", l.tr("agent-picker-locked"), None));
    }
    components.insert(0, Component::column("root", &children));
    Surface {
        id: PICKER_ID.to_owned(),
        place: Place::Sheet,
        anchor: None,
        turn: None,
        live: true,
        question: false,
        messages: vec![
            ServerMessage::create_surface(PICKER_ID),
            ServerMessage::update_components(PICKER_ID, components),
            ServerMessage::set_data_model(PICKER_ID, json!({ "choice": active })),
        ],
    }
}

#[cfg(test)]
#[path = "picker_tests.rs"]
mod picker_tests;
```
(Vérifier les signatures réelles de `Component::text`/`column`/`Action::event` dans `crates/oku-slides-a2ui/src/message.rs:492-550` et retirer `Tone` s'il ne sert pas ; vérifier que la racine attendue par l'UI s'appelle bien `root` en comparant avec `forms::choice`, qui utilise `Component::card("root", "body")` — garder `root` comme racine.)

`events.rs` : `#[derive(Debug, Clone, PartialEq, Serialize)]` sur `AgentEvent` (retirer `Eq` ; si un autre type qui contient `AgentEvent` dérive `Eq`, retirer aussi le sien — le compilateur le dira) ; ajouter

```rust
    /// Le panneau de l'agent, pour les vues de l'onglet qui l'ont demandé.
    Picker {
        surface: Surface,
    },
```
avec `use oku_slides_session::agent::Surface;`.

`turns.rs:159` et `:263` : ajouter `| AgentEvent::Picker { .. }` aux bras qui ignorent `Draft`.

`lib.rs` : `mod picker;` (privé).

Clés Fluent, dans les 20 `agent.ftl` (après le bloc des erreurs de l'agent ; FR et EN ci-dessous, les 18 autres traduites fidèlement, même ordre de clés partout) :

```ftl
# fr
agent-picker-title = Agent
agent-picker-busy = Changer d'agent interrompra le tour en cours.
agent-picker-locked = Choix imposé par l'environnement (OKU_SLIDES_AGENT__SELECTED).
```
```ftl
# en
agent-picker-title = Agent
agent-picker-busy = Switching agents will stop the current turn.
agent-picker-locked = Set by the environment (OKU_SLIDES_AGENT__SELECTED).
```

- [ ] **Step 4: Vert, snapshots relus** — `cargo nextest run -p oku-slides-agent -p oku-slides-i18n` puis `cargo insta review` (accepter après lecture : pastilles, `hint`, `choice`, absence de `select` si verrouillé). Expected: PASS.

- [ ] **Step 5: Commit** — `feat(agent) : surface du panneau des agents et évènement Picker (t-775)`

---

### Task 5: Choisir l'agent depuis la pile

**Files:**
- Modify: `crates/oku-slides-agent/src/stack.rs` (`AgentCommand`, `Stack` : champ `active`, `handle`, `select_agent`, `emit_picker`)
- Modify: `crates/oku-slides-agent/src/command_error.rs` (+ `_tests.rs`)
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (3 clés, 20 fichiers)
- Modify: `crates/oku-slides-agent/tests/common/mod.rs` (`build_stack` : second agent ; `TestStack::data`)
- Test: `crates/oku-slides-agent/tests/stack.rs`
- Modify: `crates/oku-slides-agent/ARCHITECTURE.md`, `crates/oku-slides-agent/src/lib.rs` (doc)

**Interfaces:**
- Consumes: `Link::switch` (tâche 2), `picker_surface`, `PICKER_ID`, `AgentEvent::Picker` (tâche 4), `AgentSettings { selected, presets, locked }` (tâche 1), `write_value(&Storage, &Path, &str, &Value)` (`oku_slides_platform::config_write`).
- Produces:
  - `AgentCommand::Picker { tab: String }` (JSON `{"cmd":"picker","tab":…}`)
  - `AgentCommand::SelectAgent { tab: String, id: String }` (JSON `{"cmd":"select_agent","tab":…,"id":…}`)
  - `CommandInner::{UnknownAgent { id }, AgentLocked, AgentNotSaved { source: oku_slides_platform::config_write::<Erreur> }}` localisées `agent-error-unknown-agent`, `agent-error-agent-locked`, `agent-error-agent-not-saved`.

- [ ] **Step 1: Outillage de test** — dans `tests/common/mod.rs`, ajouter à `build_stack` un paramètre `bis: Option<Value>` (les appelants existants passent `None`) : s'il est donné, écrire `script-bis.json`, journal `log-bis.jsonl` dans le même dossier, et

```rust
        if let Some(args_bis) = args_bis {
            settings.agent.presets.insert("bis".into(), AgentPreset::new("Bis", command, args_bis));
        }
```
Exposer :

```rust
/// Une pile à deux faux agents, `faux` (actif) et `bis` ; rend aussi le dossier de données.
pub async fn stack_with_two_agents(script: Value, bis: Value, tabs: &[&str]) -> (TestStack, Relayed, PathBuf, PathBuf) {
    let (stack, seen, log, _) = build_stack_with(script, Some(bis), tabs, First::Fixture, false).await;
    let data = log.parent().unwrap_or_else(|| panic!("dossier")).to_owned();
    (stack, seen, log, data)
}
```
et une variante verrouillée (`locked: bool` passé jusqu'à `settings.agent.locked`) :

```rust
/// Comme [`stack_with_two_agents`] sur `t-1`, choix imposé par l'environnement (`locked`).
pub async fn stack_locked(script: Value, bis: Value) -> (TestStack, Relayed) {
    let (stack, seen, _, _) = build_stack_with(script, Some(bis), &["t-1"], First::Fixture, true).await;
    (stack, seen)
}
``` Garder `build_stack` comme fine enveloppe de `build_stack_with(…, None, …, false)` pour ne pas toucher aux autres appelants.

- [ ] **Step 2: Tests rouges** (`tests/stack.rs`) :

```rust
fn picker_of<'a>(relayed: &'a [(&'static str, serde_json::Value)], tab: &str) -> Option<&'a serde_json::Value> {
    relayed.iter().rev().find(|(ch, v)| *ch == "agent" && v["tab"] == tab && v["event"]["kind"] == "picker").map(|(_, v)| v)
}

#[tokio::test(flavor = "multi_thread")]
async fn picker_lists_the_agents_with_the_active_one_chosen() {
    let (stack, rx, _, _) = stack_with_two_agents(json!({ "turns": [] }), json!({ "turns": [] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "picker", "tab": "t-1" }))).unwrap();
    let ev = wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "picker").await;
    let text = ev.1["event"]["surface"].to_string();
    assert_that!(text.contains("\"Faux\"") && text.contains("\"Bis\"") && text.contains("\"choice\":\"faux\"")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_saves_the_choice_and_the_next_prompt_reaches_it() {
    let (stack, rx, log, data) =
        stack_with_two_agents(json!({ "turns": [{ "chunks": ["un"] }] }), json!({ "turns": [{ "chunks": ["bis"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "select_agent", "tab": "t-1", "id": "bis" }))).unwrap();
    let toml = std::fs::read_to_string(data.join("config.toml")).unwrap();
    assert_that!(toml.contains("selected = \"bis\"")).is_true();
    let ev = wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "picker").await;
    assert_that!(ev.1["event"]["surface"].to_string().contains("\"choice\":\"bis\"")).is_true();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(sessions_in(&data.join("log-bis.jsonl"), "session/prompt").len()).is_equal_to(1);
    assert_that!(sessions_in(&log, "session/prompt")).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_during_a_turn_stops_it_with_agent_stopped() {
    let ask = json!({ "turns": [{ "mcp": [{ "tool": "ask_choice", "args": { "question": "?",
        "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }] }] });
    let (stack, rx, _, _) = stack_with_two_agents(ask, json!({ "turns": [] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "?" }))).unwrap();
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    stack.handle(cmd(json!({ "cmd": "picker", "tab": "t-1" }))).unwrap();
    let busy = wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "picker").await;
    assert_that!(busy.1["event"]["surface"].to_string().contains("interrompra")).is_true();
    stack.handle(cmd(json!({ "cmd": "select_agent", "tab": "t-1", "id": "bis" }))).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"].to_string().contains("L'agent s'est arrêté")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_unknown_or_locked_or_unsaved_is_refused_and_nothing_switches() {
    let (stack, _, _, data) = stack_with_two_agents(json!({ "turns": [] }), json!({ "turns": [] }), &["t-1"]).await;
    let unknown = stack.handle(cmd(json!({ "cmd": "select_agent", "tab": "t-1", "id": "mistral" })));
    assert_that!(unknown).is_err();
    // `config.toml` impossible à écrire : un dossier à sa place.
    std::fs::create_dir_all(data.join("config.toml")).unwrap();
    let unsaved = stack.handle(cmd(json!({ "cmd": "select_agent", "tab": "t-1", "id": "bis" })));
    assert_that!(unsaved).is_err();
    let (locked, _) = stack_locked(json!({ "turns": [] }), json!({ "turns": [] })).await;
    assert_that!(locked.handle(cmd(json!({ "cmd": "select_agent", "tab": "t-1", "id": "bis" })))).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_does_not_resume_the_other_agent_session() {
    let (stack, rx, _, data) =
        stack_with_two_agents(json!({ "turns": [{ "chunks": ["un"] }] }), json!({ "turns": [{ "chunks": ["bis"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    stack.handle(cmd(json!({ "cmd": "select_agent", "tab": "t-1", "id": "bis" }))).unwrap();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "deux" }))).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    let bis = data.join("log-bis.jsonl");
    assert_that!(sessions_in(&bis, "session/load")).is_empty();
    assert_that!(sessions_in(&bis, "session/resume")).is_empty();
    let relayed = rx.lock().unwrap().clone();
    assert_that!(relayed.iter().any(|(_, v)| v.to_string().contains("memory-lost"))).is_false();
}
```

Vérifier avant d'écrire : le texte exact du fil pour `agent-stopped` (le chercher dans `turns_tests.rs` ou dans un test existant ; adapter le prédicat du 3e test au canal et à la forme réels) et l'identifiant de la carte « mémoire perdue » (`grep -rn "memory\|mémoire" crates/oku-slides-agent/src/turns.rs crates/oku-slides-session/src/agent`). Ajouter `stack_with_two_agents`, `stack_locked` aux imports de `common`.

Test d'erreur localisée (`command_error_tests.rs`) :

```rust
#[test]
fn unknown_agent_is_said_in_the_user_language() {
    let e = CommandError::from(UnknownAgentSnafu { id: "x" }.build());
    assert_that!(e.localize(&Localizer::new(Some("fr")))).is_equal_to("Cet agent n'est plus dans la configuration.".to_owned());
}
```

- [ ] **Step 3: Rouge** — `cargo nextest run -p oku-slides-agent --test stack picker select_agent` et `command_error`. Expected: échec (commandes inconnues).

- [ ] **Step 4: Implémenter**

`command_error.rs` : trois variantes (le message localisé ne cite aucune donnée de l'utilisateur, comme les autres) :

```rust
    #[snafu(display("agent {id:?} is not in the configuration"))]
    UnknownAgent { id: String, backtrace: Backtrace },
    #[snafu(display("agent choice is set by OKU_SLIDES_AGENT__SELECTED"))]
    AgentLocked { backtrace: Backtrace },
    #[snafu(display("agent choice could not be saved"))]
    AgentNotSaved { source: oku_slides_platform::config_write::WriteError, backtrace: Backtrace },
```
(Le type d'erreur réel de `write_value` est à lire dans `crates/oku-slides-platform/src/config_write.rs` ; utiliser son nom exact.) `localize` : `agent-error-unknown-agent`, `agent-error-agent-locked`, `agent-error-agent-not-saved`.

Clés Fluent (20 `agent.ftl`, à côté des autres `agent-error-*`) :

```ftl
# fr
agent-error-unknown-agent = Cet agent n'est plus dans la configuration.
agent-error-agent-locked = Le choix de l'agent est imposé par l'environnement.
agent-error-agent-not-saved = Le choix de l'agent n'a pas pu être enregistré.
```
```ftl
# en
agent-error-unknown-agent = This agent is no longer in the configuration.
agent-error-agent-locked = The agent choice is set by the environment.
agent-error-agent-not-saved = The agent choice could not be saved.
```

`stack.rs` :
- `AgentCommand` : variantes `Picker { tab: String }` et `SelectAgent { tab: String, id: String }` (docs : « L'interface ouvre le panneau des agents : la surface est émise à cet onglet. » / « Choisit l'agent de l'app (bascule immédiate), puis réémet le panneau à cet onglet. ») ; `tab()` les couvre.
- `Stack` : champ `active: Mutex<String>` initialisé à `platform.settings.agent.selected.clone()`.
- `handle` :

```rust
            AgentCommand::Picker { tab } => self.emit_picker(&tab),
            AgentCommand::SelectAgent { tab, id } => {
                if let Err(e) = self.select_agent(&id) {
                    tracing::info!(tab, error = %snafu::Report::from_error(&e), "choix d'agent refusé");
                    return Err(e.localize(&self.platform.localizer));
                }
                self.emit_picker(&tab);
            }
```

- méthodes :

```rust
    /// Choisit l'agent `id` pour toute l'app : enregistré dans `config.toml`, puis bascule.
    ///
    /// Rien ne change si le choix est refusé (inconnu, imposé, non enregistré).
    fn select_agent(&self, id: &str) -> Result<(), CommandError> {
        let agent = &self.platform.settings.agent;
        ensure!(!agent.locked, AgentLockedSnafu);
        let preset = agent.presets.get(id).context(UnknownAgentSnafu { id })?;
        let mut active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
        if *active == id {
            return Ok(());
        }
        write_value(&self.platform.storage, &self.platform.settings.config_file(), "agent.selected", &json!(id))
            .context(AgentNotSavedSnafu)?;
        self.link.switch(Launch {
            command: preset.command.clone(),
            args: preset.args.clone(),
            fallback_cwd: self.platform.settings.paths.data(),
            ready_within: crate::READY_WITHIN,
        });
        id.clone_into(&mut active);
        Ok(())
    }

    /// Émet le panneau des agents aux vues de `tab` ; un tour ouvert dans un onglet l'annonce.
    fn emit_picker(&self, tab: &str) {
        let busy = self.table().tabs.values().any(|e| e.link.hub.turn().is_some());
        let active = self.active.lock().unwrap_or_else(PoisonError::into_inner).clone();
        let agent = &self.platform.settings.agent;
        let surface = picker_surface(&agent.presets, &active, busy, agent.locked, &self.platform.localizer);
        emit(&self.out_tx, "agent", tab, &AgentEvent::Picker { surface });
    }
```
(Les sélecteurs snafu de `CommandInner` sont `pub(crate)` ; `CommandError::from(inner)` comme pour les brouillons — adapter `?` en `.map_err(CommandError::from)` si `From` ne s'applique pas tout seul. Vérifier que `TabLink` expose `hub` (`stack.rs:180` utilise déjà `t.hub`).)

Docs : `ARCHITECTURE.md` — dans « Modules », ajouter `picker` (« surface du panneau des agents, pure ») ; `config` mentionne que `Launch` vient de `AgentSettings::active()` ; « Cycle de vie » : « `select_agent` (panneau) écrit `agent.selected` puis `Link::switch` : la connexion vivante s'arrête (tours → `agent-stopped`), le prompt suivant lance le nouvel agent ; pas de reprise d'une session de l'autre agent (empreinte différente). » ; « Sécurité » : « `set_setting` refuse `agent.*`. » `lib.rs` : compléter la ligne `Stack` (« … et le panneau des agents (`picker`, `select_agent`) »).

- [ ] **Step 5: Vert** — `cargo nextest run -p oku-slides-agent -p oku-slides-i18n`. Expected: PASS.

- [ ] **Step 6: Commit** — `feat(agent) : choisir l'agent de l'app depuis la pile (Picker, SelectAgent) (t-775)`

---

### Task 6: Vérifier les trois agents livrés

**Files:**
- Create (scratchpad seulement, jamais committé) : `<scratchpad>/acp_probe.mjs`
- Modify (seulement si un agent échoue) : `crates/oku-slides-platform/src/config.default.toml`, `config_tests.rs`, spec

- [ ] **Step 1: Sonde** — `acp_probe.mjs` lance une commande, envoie `initialize` (JSON-RPC sur stdio, une ligne JSON) et affiche `agentCapabilities.mcpCapabilities.http` :

```js
// node acp_probe.mjs npx -y @agentclientprotocol/codex-acp@2.2.2
import { spawn } from 'node:child_process';
const [cmd, ...args] = process.argv.slice(2);
const child = spawn(cmd, args, { stdio: ['pipe', 'pipe', 'inherit'] });
const timer = setTimeout(() => { console.log('TIMEOUT'); child.kill(); process.exit(2); }, 180000);
let buf = '';
child.stdout.on('data', (d) => {
  buf += d;
  for (const line of buf.split('\n').slice(0, -1)) {
    try {
      const m = JSON.parse(line);
      if (m.id === 1) {
        clearTimeout(timer);
        console.log(JSON.stringify({ ok: !m.error, http: m.result?.agentCapabilities?.mcpCapabilities?.http, error: m.error }));
        child.kill(); process.exit(0);
      }
    } catch {}
  }
  buf = buf.slice(buf.lastIndexOf('\n') + 1);
});
child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'initialize',
  params: { protocolVersion: 1, clientCapabilities: { fs: { readTextFile: false, writeTextFile: false }, terminal: false } } }) + '\n');
```

- [ ] **Step 2: Lancer pour chaque preset** (depuis un dossier vide du scratchpad, pour que `npx` n'exécute rien du dépôt) :

```bash
node <scratchpad>/acp_probe.mjs npx -y @agentclientprotocol/claude-agent-acp@0.81.2
node <scratchpad>/acp_probe.mjs npx -y @agentclientprotocol/codex-acp@2.2.2
node <scratchpad>/acp_probe.mjs npx -y @google/gemini-cli@0.63.0 --acp
```
Expected: `{"ok":true,"http":true}` pour chacun.

- [ ] **Step 3: Décider** — tous `http: true` : rien à changer, noter le résultat dans un commentaire de t-775 (`oku cli task comment t-775 …`). Un agent qui échoue (erreur, `http` faux, délai) : le retirer de `config.default.toml` et du test `defaults_agent_presets_are_claude_codex_gemini_with_claude_selected`, mettre à jour la décision 5 de la spec, **et le signaler au contrôleur** (G9) avant de continuer.

- [ ] **Step 4: Commit** (seulement si un fichier a changé) — `fix(agent) : retirer <agent> des agents livrés (pas de MCP HTTP) (t-775)`

---

### Task 7: Fiche du panneau dans la zone de l'agent

**Files:**
- Modify: `ui/src/api.ts:588-590` (`AgentEvent`), `:635-666` (`AgentCommand`, `agentApi`)
- Modify: `ui/src/icons.tsx` (icône `settings`)
- Create: `ui/src/agent/AgentPicker.tsx`, `ui/src/agent/AgentPicker.render.test.tsx`
- Modify: `ui/src/agent/AgentThread.tsx:30-110`, `ui/src/agent/AgentThread.render.test.tsx`
- Modify: `ui/src/agent/AgentPanel.tsx:128-293`, `ui/src/agent/AgentPanel.render.test.tsx`
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (1 clé, 20 fichiers), styles de la fiche dans la feuille où vit `.agent-thread-head` (`grep -rn "agent-thread-head" ui/src/*.css`)

**Interfaces:**
- Consumes: `{"kind":"picker","surface":Surface}` sur le canal `agent` ; commandes `{"cmd":"picker","tab"}` et `{"cmd":"select_agent","tab","id"}` (tâche 5).
- Produces:
  - `AgentEvent` TS : `| { kind: "picker"; surface: Surface }`
  - `agentApi(tab).picker(): Promise<void>` et `agentApi(tab).selectAgent(id: string): Promise<void>`
  - `AgentPicker` props : `{ surface: Surface | null; onSelect: (id: string) => void; onClose: () => void; onError: (m: string) => void }`
  - `AgentThread` prop optionnelle : `onPicker?: () => void` et `pickerOpen?: boolean` ; `picker?: ReactNode` (la fiche rendue sous l'en-tête).

- [ ] **Step 1: Tests rouges**

`AgentPicker.render.test.tsx` (suivre la mise en place des autres `*.render.test.tsx` du dossier : happy-dom, `render` de `@testing-library/react` ou l'outil déjà utilisé — le lire dans `AgentSurfaces.render.test.tsx`) :
  - `AgentPicker with a surface shows every agent` : surface construite comme le snapshot de la tâche 4 (copier le JSON du snapshot `picker_surface_idle…` dans une constante de test) → les libellés « Claude » et « Gemini » sont visibles.
  - `AgentPicker choosing an agent calls onSelect with its id` : clic sur la pastille « Claude » → `onSelect("claude")`.
  - `AgentPicker escape calls onClose` : `keydown` Échap sur la fiche → `onClose()`.
  - `AgentPicker without surface renders nothing` : `surface={null}` → conteneur vide.

`AgentThread.render.test.tsx` :
  - `AgentThread with onPicker shows the agents button even when empty` : fil vide, `onPicker` donné → bouton de libellé `ui-agent-picker` présent, et la section n'a pas la classe `vacant`.
  - `AgentThread agents button calls onPicker`.

`AgentPanel.render.test.tsx` (le faux `subscribe` du fichier permet d'injecter des évènements `onAgent`) :
  - `AgentPanel agents button asks the host for the picker` : clic → `agent.picker` appelé une fois.
  - `AgentPanel picker event shows the sheet and a choice sends select_agent then closes` : clic, puis évènement `{kind:"picker", surface}` → fiche visible ; clic « Claude » → `agent.selectAgent("claude")` ; promesse résolue → fiche fermée.
  - `AgentPanel refused choice keeps the sheet and says the error` : `selectAgent` rejette `"refus"` → `say("refus", true)`, fiche toujours ouverte.

- [ ] **Step 2: Rouge** — `cd ui && bun test src/agent/AgentPicker src/agent/AgentThread src/agent/AgentPanel`. Expected: FAIL.

- [ ] **Step 3: Implémenter**

`api.ts` :

```ts
export type AgentEvent =
  | { kind: "status"; state: AgentState; scope?: AgentScope }
  | { kind: "draft"; text: string | null; origin: string | null }
  | { kind: "picker"; surface: Surface };
```
`AgentCommand` : `| { cmd: "picker"; tab: string } | { cmd: "select_agent"; tab: string; id: string }` ; dans `agentApi` :

```ts
    /** Demande le panneau des agents (évènement `picker` pour cet onglet). */
    picker: () => agentCall({ cmd: "picker", tab }),
    /** Choisit l'agent de l'app ; l'hôte réémet le panneau. */
    selectAgent: (id: string) => agentCall({ cmd: "select_agent", tab, id }),
```
Vérifier que le réducteur d'état (`agentStatusReducer`, `useAgentScope.ts`) ignore les `kind` qu'il ne traite pas ; sinon y ajouter `picker` aux cas ignorés.

`icons.tsx` : `settings: stroke(<><circle cx="12" cy="12" r="3" /><path d="M12 3v2.5M12 18.5V21M3 12h2.5M18.5 12H21M5.6 5.6l1.8 1.8M16.6 16.6l1.8 1.8M5.6 18.4l1.8-1.8M16.6 7.4l1.8-1.8" /></>),` (à côté de `newChat`).

`AgentPicker.tsx` :

```tsx
// Panneau des agents : choisir l'agent ACP de l'app (surface `agent-picker` de l'hôte).

import { lazy, Suspense, useEffect, useRef } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { t } from "../i18n";
import type { Surface } from "../api";
import { fromA2ui } from "./quick";

// Même morceau que le fil de l'agent : chargé une fois.
const SurfaceView = lazy(() => import("../a2ui/SurfaceView").then((m) => ({ default: m.SurfaceView })));

interface Props {
  surface: Surface | null;
  onSelect: (id: string) => void;
  onClose: () => void;
  onError: (message: string) => void;
}

/** La fiche, sous l'en-tête du fil ; Échap la ferme. */
export function AgentPicker({ surface, onSelect, onClose, onError }: Props) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => box.current?.focus(), [surface !== null]);
  if (!surface) return null;
  const onAction = (a: A2uiClientAction) => {
    const action = fromA2ui(a);
    const id = action.context.id;
    if (action.name === "select" && typeof id === "string") onSelect(id);
  };
  return (
    <div
      ref={box}
      className="agent-picker"
      role="dialog"
      aria-label={t("ui-agent-picker")}
      tabIndex={-1}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          onClose();
        }
      }}
    >
      <Suspense fallback={null}>
        <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={onAction} onError={onError} />
      </Suspense>
    </div>
  );
}
```
(Vérifier la forme de `fromA2ui` dans `ui/src/agent/quick.ts` — `name` et `context` — et ajuster.)

`AgentThread.tsx` : props `onPicker?: () => void; pickerOpen?: boolean; picker?: ReactNode` ; `const vacant = empty && !windowButtons && !onPicker;` ; dans `agent-thread-head`, avant `agent-detach` :

```tsx
          {onPicker && <button type="button" className={`token ghost agent-picker-toggle${pickerOpen ? " active" : ""}`} aria-label={t("ui-agent-picker")} aria-expanded={!!pickerOpen} title={t("ui-agent-picker")} onClick={onPicker}>{Icon.settings}</button>}
```
et `{picker}` juste après `</header>`.

`AgentPanel.tsx` : état `const [picker, setPicker] = useState<Surface | null>(null); const [pickerOpen, setPickerOpen] = useState(false);` ; dans `onAgent` : `if (e.kind === "picker") setPicker(e.surface);` (passer par `latest` si le rappel ne voit pas le setter — un setter de `useState` est stable, il peut être lu directement) ; 

```tsx
  const togglePicker = () => {
    if (pickerOpen) return setPickerOpen(false);
    setPickerOpen(true);
    void agent.picker().catch((err) => say(String(err), true));
  };
  const selectAgent = (id: string) =>
    void agent.selectAgent(id).then(() => setPickerOpen(false), (err) => say(String(err), true));
```
Passer à `AgentThread` : `onPicker={togglePicker} pickerOpen={pickerOpen} picker={pickerOpen ? <AgentPicker surface={picker} onSelect={selectAgent} onClose={() => setPickerOpen(false)} onError={onError} /> : null}`. Le type `AgentHandle`/`AgentApi` du fichier dérive de `agentApi` : les faux des tests doivent gagner `picker` et `selectAgent`.

`ui.ftl` (20 fichiers, à côté de `ui-agent-new-conversation`) : FR `ui-agent-picker = Choisir l'agent` ; EN `ui-agent-picker = Choose the agent` ; 18 autres traduites.

Style : `.agent-picker` sous l'en-tête, même carte que `.agent-sheet-box` (réutiliser ses variables ; pas de nouvelle couleur en dur) ; `.agent-picker-toggle.active` comme les autres boutons `token ghost` actifs s'il en existe, sinon `aria-expanded` suffit.

- [ ] **Step 4: Vert** — `cd ui && bun test && bun run typecheck`, puis `cargo nextest run -p oku-slides-i18n`. Expected: PASS.

- [ ] **Step 5: Vérification visuelle** (preview impossible depuis le worktree : `preview_start` sert `ui/dist` du checkout principal) — `cd ui && bun run build`, puis lancer `serve` du worktree sur un port libre avec données isolées :

```bash
tmp=$(mktemp -d /private/tmp/claude-501/-Users-matthieu-Projects-slider/e6cf6677-bc34-43db-b46a-8a1e48f9f213/scratchpad/okuXXXX)
OKU_SLIDES_PATHS__DATA=$tmp/data OKU_SLIDES_PATHS__DECKS=$tmp/decks CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo run -q -p oku-slides-cli --bin oku-slides -- serve fixtures/roadmap.json --ui ui/dist --port 4893
```
Ouvrir `http://127.0.0.1:4893/` dans le navigateur intégré, ouvrir la fiche, faire une capture (clair et sombre) ; aucune erreur console.

- [ ] **Step 6: Commit** — `feat(ui) : fiche du panneau des agents dans la zone de l'agent (t-775)`

---

### Task 8: Parcours e2e du panneau

**Files:**
- Create: `fixtures/agent/e2e-bis.json`, `ui/e2e/agent-picker.cjs`
- Modify: `justfile` (recette `e2e` : second preset), `ui/package.json:11` (script `e2e`)

- [ ] **Step 1: Scénario du second faux agent** — `fixtures/agent/e2e-bis.json` :

```json
{
  "http": true,
  "turns": [{ "chunks": ["Réponse de l'agent bis."] }]
}
```

- [ ] **Step 2: Second preset dans la recette** — étendre le `printf` de la tâche 1 :

```bash
    printf '[agent]\nselected = "e2e"\n\n[agent.presets.e2e]\nname = "Faux"\ncommand = "%s"\nargs = ["--script", "%s"]\n\n[agent.presets.e2e-bis]\nname = "Faux bis"\ncommand = "%s"\nargs = ["--script", "%s"]\n' \
      "$bin/oku-slides-fake-agent" "$PWD/fixtures/agent/e2e.json" "$bin/oku-slides-fake-agent" "$PWD/fixtures/agent/e2e-bis.json" > "$tmp/data/config.toml"
```

- [ ] **Step 3: Spec e2e (rouge tant que l'UI manque, verte après la tâche 7)** — `ui/e2e/agent-picker.cjs`, sur le modèle d'`agent.cjs` (mêmes helpers `post`, `until`, `URL`) :
  1. ouvrir la page, cliquer `button.agent-picker-toggle` ;
  2. attendre `.agent-picker` et ses deux pastilles « Faux » et « Faux bis » ; « Faux » est choisie ;
  3. cliquer « Faux bis » ; la fiche se ferme ;
  4. `prompt(p, 'bonjour')` puis attendre dans `.agent-turn` le texte « Réponse de l'agent bis. » ;
  5. rouvrir la fiche : « Faux bis » est choisie ; revenir à « Faux » (les specs suivantes éventuelles retrouvent l'agent par défaut) ;
  6. `assert` qu'aucune erreur console n'a été vue.

Ajouter `&& bun e2e/agent-picker.cjs` **à la fin** du script `e2e` de `ui/package.json` (il change l'agent de tout le serveur : dernier de la liste).

- [ ] **Step 4: Lancer** — sur un port libre (vérifier qu'il l'est) :

```bash
CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target just e2e 4891
```
Expected: toutes les specs passent, `agent-picker.cjs` compris.

- [ ] **Step 5: Commit** — `test(e2e) : choisir un autre agent dans le panneau (t-775)`

---

### Fin du plan

- [ ] `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target just check` vert dans le worktree.
- [ ] Revue finale de la branche (SDD), puis `git config --worktree --unset oku.review`.
- [ ] Toutes les sous-tâches et t-775 passent `done` après la fusion sur `main` (squash, voir les mémoires du projet sur l'index de `main`).
