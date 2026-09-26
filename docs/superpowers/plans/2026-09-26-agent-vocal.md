# Agent vocal : ACP, MCP et surfaces A2UI — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** ⌘⇧D dicte une consigne envoyée par ACP à un agent qui modifie Oku Slides uniquement par des outils MCP servis par l'app, et qui répond par des surfaces A2UI (choix, confirmation, reformulation, paramètres, variantes).

**Architecture:** deux nouveaux crates. `oku-slides-mcp` sert les outils en HTTP streamable sur `127.0.0.1` (jeton par lancement) et tient le `SurfaceHub` (surfaces ouvertes, réponses attendues). `oku-slides-agent` lance l'agent en sous-processus (ACP sur stdio), gère sessions et tours, et assemble le tout dans une `Stack` que l'app Tauri et `oku-slides serve` démarrent à l'identique. La session reste synchrone ; elle gagne les groupes d'annulation, la composition des surfaces, le rendu des variantes et le bloc de contexte.

**Tech Stack:** Rust 2024 (tokio, snafu, speculoos, insta), `agent-client-protocol` =2.2.0, `rust-mcp-sdk` =1.1.0 + `rust-mcp-axum` =1.0.2 (axum 0.8), `typst-render` 0.15.1, `similar`, `toml_edit` ; React 19 + `@a2ui/react` 0.11, Bun, Playwright.

**Spec:** `docs/superpowers/specs/2026-09-26-agent-vocal-design.md`

**Tâche Oku :** t-89 (projet `slider`), sous-tâches t-99 à t-119 (une par tâche, voir « Sous-tâches »). Chaque commit cite **sa sous-tâche** ; pas de ligne `Co-Authored-By`.

## Prérequis

- Le plan `docs/superpowers/plans/2026-09-26-resume-state.md` (t-81) est exécuté : `History` et sa `Transaction` dérivent `Serialize`/`Deserialize`, `Session` a `ui: UiState` (`slide`, `grid`) et `Request::Ui`. La Task 1 ci-dessous s'appuie sur ces dérivations.

## Écarts à la spec (validés au moment du plan)

1. **Outil `chart_palette` retiré** : la requête `chart_palette` n'existe plus depuis t-80 (la palette vient du thème). L'agent colore une série par `apply` + `set_chart_option`.
2. **Agent par défaut** : `claude-code-acp` est déprécié. Défaut : `command = "npx"`, `args = ["-y", "@agentclientprotocol/claude-agent-acp@0.81.2"]` (vérifié : annonce `mcpCapabilities.http: true`).
3. **`PermissionGate`** : pas de trait fourni par l'hôte. La confirmation des outils non annulables est une méthode du `SurfaceHub` (`confirm`), qui ouvre la forme confirmation ; l'hôte n'a rien à implémenter.
4. **Clés de `set_setting`** : `dictation.model`, `dictation.language`, `dictation.auto_stop`, `dictation.silence` (appliquées tout de suite par la dictée), `i18n.locale`, `history.limit`, `stt.vad.<clé>` (écrites dans `config.toml`, effet au prochain lancement). Le préfixe `dictation.` évite de confondre les réglages de la dictée (`<models>/config.json`) avec la section `[stt]` de `config.toml`.
5. **Bout à bout** : la consigne est envoyée par `POST /api/agent` depuis le test, pas par un faux micro (le test n'aurait pas de modèle Whisper). Les deux modes du micro sont couverts par les tests de l'interface (Task 19).
6. **`rust-mcp-sdk` 1.1.0, pas 2.0** : la 2.0 ne parle que MCP 2026-07-28 (sans état) ; le client MCP du SDK Claude (`@anthropic-ai/claude-agent-sdk` 0.3.280) parle au plus 2025-11-25. Vérifié : 1.1.0 négocie 2025-11-25, 2025-06-18, 2025-03-26, 2024-11-05 avec `@modelcontextprotocol/sdk` 1.30.1. Limite de 1.x : le schéma d'entrée d'un outil ne garde que `properties` et `required` à la racine ; les `$defs` d'un schéma sont donc déplacés sous la propriété (Task 8, `nest_schema`).
7. **`History::rollback`** (ajout) : `Session::apply` défaisait un rendu raté par `undo`, ce qui laissait un « rétablir » fantôme et, dans un tour, aurait défait tout le tour. `rollback` ne défait que le dernier commit.

## Global Constraints

- Pas de `.unwrap()` / `.expect()` en production ; erreurs Snafu avec `backtrace: Backtrace` dans chaque variante ; erreurs publiques opaques (`pub struct XError(Inner)`).
- Pas de `std::fs` en production : `Storage` (OpenDAL). `std::fs` permis dans les tests.
- Async : tokio ; futures `Send` ; jamais de verrou `std::sync::Mutex` tenu à travers un `.await` ; travail synchrone de la session dans `tokio::task::spawn_blocking`.
- Tests : unitaires side-by-side `x_tests.rs` inclus par `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;` ; intégration sous `tests/` ; noms `<unité>_<cas>_<résultat>` ; `speculoos` ; `insta` pour les instantanés.
- Texte utilisateur via Fluent ; **toute clé ajoutée l'est dans les 20 locales** (`crates/oku-slides-i18n/src/locales/<l>/*.ftl`) : le plan donne `en` et `fr`, les 18 autres sont traduites à l'identique de sens (vérifié par `every_locale_has_exactly_the_reference_keys`, et côté UI par `ui/src/i18n-keys.test.ts`).
- Messages d'erreur d'outil MCP (lus par le modèle) en anglais, stables ; jamais traduits.
- Le jeton MCP n'apparaît ni dans une URL ni dans un journal.
- Journaux : `info` lancement/arrêt de l'agent, `debug` nom + durée de chaque outil (sans arguments), `trace` le flux ACP.
- Commentaires et docs en français, comme le code existant. Fichiers < 1000 lignes (cible 500) ; `crates/oku-slides-session/src/lib.rs` (≈720 lignes) ne grossit presque pas : le code de l'agent va dans `src/agent/`.
- Dépendances : versions dans `[workspace.dependencies]`, crates en `.workspace = true`.
- `rust-version` du workspace passe à `"1.92"` (exigé par typst 0.15.1 et `typst-render`).
- Commandes : `cargo nextest run -p <crate>`, `just test`, `just check`, `bun test` et `bunx tsc -b` dans `ui/`, `just e2e`.

## Sous-tâches

Créées sous t-89 (projet `slider`), une par tâche, titre identique. Chaque commit de la Task N cite sa sous-tâche ; à la fin de la tâche : `oku cli task done --project slider --id <sous-tâche>`.

| Task | Sous-tâche | Titre |
| --- | --- | --- |
| 1 | t-99 | Historique : groupes d'annulation et rollback |
| 2 | t-100 | Réglages : section [agent] et écriture de config.toml |
| 3 | t-101 | A2UI : catalogue v2 |
| 4 | t-102 | Rendu : slide en PNG |
| 5 | t-103 | Session : formes de surface |
| 6 | t-104 | Session : variantes et gabarits de paramètres |
| 7 | t-105 | Session : contexte de l'agent et tours |
| 8 | t-106 | MCP : serveur, jeton et outils de lecture |
| 9 | t-107 | MCP : outils de modification et évènements |
| 10 | t-108 | MCP : SurfaceHub et outils de surface |
| 11 | t-109 | MCP : outils non annulables et réglages |
| 12 | t-110 | Agent : lancement ACP et faux agent |
| 13 | t-111 | Agent : sessions, tours et annulation |
| 14 | t-112 | Agent : permissions, instructions et Stack |
| 15 | t-113 | Hôte Tauri : agent et évènements |
| 16 | t-114 | Hôte serve : agent et évènements SSE |
| 17 | t-115 | UI : api de l'agent, fil et surfaces |
| 18 | t-116 | UI : catalogue v2 |
| 19 | t-117 | UI : micro en deux modes et fil de l'agent |
| 20 | t-118 | UI : fiche, tiroir, post-it et fenêtre |
| 21 | t-119 | Bout à bout, documentation et vérification |

## Carte des fichiers

| Fichier | Rôle |
| --- | --- |
| `crates/oku-slides-core/src/history.rs` | Groupes d'annulation, `rollback` |
| `crates/oku-slides-platform/src/config.rs`, `config.default.toml` | `[agent]` |
| `crates/oku-slides-platform/src/config_write.rs` | Écriture d'une clé dans `config.toml` (toml_edit) |
| `crates/oku-slides-a2ui/src/message.rs` | Catalogue v2 : `Row`, `Button`, `ChoiceChips`, `Slider`, `ColorField`, `Select`, `SlideThumb`, `DiffText` |
| `crates/oku-slides-render/src/lib.rs`, `error.rs` | `Compiled::png` |
| `crates/oku-slides-session/src/agent/mod.rs` | `impl Session` : tours, PNG, formes qui lisent le deck |
| `crates/oku-slides-session/src/agent/forms.rs` | Arguments des outils de surface, `Surface`, composition A2UI |
| `crates/oku-slides-session/src/agent/diff.rs` | Segments mot à mot (`similar`) |
| `crates/oku-slides-session/src/agent/variants.rs` | Rendu des variantes sur un clone, gabarits `$value` |
| `crates/oku-slides-session/src/agent/context.rs` | Bloc `<oku-slides-context>`, notes `since_last_turn` |
| `crates/oku-slides-mcp/` | Serveur MCP : `server.rs`, `handler.rs`, `tools.rs`, `read_tools.rs`, `edit_tools.rs`, `surface_tools.rs`, `guarded_tools.rs`, `hub.rs`, `paths.rs`, `settings.rs`, `error.rs` |
| `crates/oku-slides-agent/` | Client ACP : `config.rs`, `events.rs`, `actor.rs`, `connection.rs`, `stack.rs`, `settings_port.rs`, `error.rs`, `src/bin/fake_agent.rs`, `examples/smoke.rs` |
| `src-tauri/src/main.rs` | `Stack`, commandes `agent_*` / `surface_action`, `emit` |
| `crates/oku-slides-cli/src/serve.rs` | `Stack`, `POST /api/agent`, `GET /api/events` (SSE) |
| `ui/src/api.ts`, `ui/src/agent/*` | Commandes, évènements, fil, surfaces |
| `ui/src/a2ui/catalog.tsx` | Catalogue v2 |
| `ui/src/Dictation.tsx`, `ui/src/App.tsx` | Deux modes, placement des surfaces |
| `ui/e2e/agent.cjs`, `fixtures/agent/e2e.json` | Bout à bout avec le faux agent |

---
### Task 1 : Historique : groupes d'annulation et rollback

**Files:**
- Modify: `crates/oku-slides-core/src/history.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (`Session::apply`)
- Modify: `Cargo.toml` (workspace : `rust-version = "1.92"`)
- Test: `crates/oku-slides-core/tests/ops.rs` (à côté des tests `history_*`)

**Interfaces:**
- Consumes: `History: Serialize + Deserialize` (plan de reprise, Task 1).
- Produces: `History::begin_group(&mut self, label: impl Into<String>)`, `History::end_group(&mut self)`, `History::in_group(&self) -> bool`, `History::rollback(&mut self, deck: &mut Deck) -> Result<(), TransactionError>`. `undo`/`redo` ferment le groupe ouvert.

- [ ] **Step 1 : tests qui échouent** — ajouter dans `crates/oku-slides-core/tests/ops.rs` :

```rust
fn theme(t: &str) -> Vec<Op> {
    vec![Op::SetTheme { theme: Some(t.into()) }]
}

#[test]
fn history_group_merges_commits_into_one_undo() {
    let mut deck = fixture();
    let before = deck.clone();
    let mut h = History::new(500);
    h.begin_group("Agent : titre plus percutant");
    h.commit(&mut deck, "ignoré", sample_ops()).unwrap();
    h.commit(&mut deck, "ignoré aussi", theme("pro")).unwrap();
    h.end_group();
    assert_that!(h.undo_label()).is_equal_to(Some("Agent : titre plus percutant"));
    assert_that!(h.undo(&mut deck).unwrap()).is_equal_to(Some("Agent : titre plus percutant"));
    assert_that!(deck).is_equal_to(&before);
    assert_that!(h.can_undo()).is_false();
}

#[test]
fn history_group_redo_replays_every_commit() {
    let mut deck = fixture();
    let mut h = History::new(500);
    h.begin_group("Tour");
    h.commit(&mut deck, "a", sample_ops()).unwrap();
    h.commit(&mut deck, "b", theme("pro")).unwrap();
    h.end_group();
    let after = deck.clone();
    h.undo(&mut deck).unwrap();
    assert_that!(h.redo(&mut deck).unwrap()).is_equal_to(Some("Tour"));
    assert_that!(deck).is_equal_to(&after);
}

#[test]
fn history_empty_group_creates_no_entry() {
    let mut deck = fixture();
    let mut h = History::new(500);
    h.commit(&mut deck, "Avant", theme("pro")).unwrap();
    h.begin_group("Tour vide");
    h.end_group();
    assert_that!(h.undo_label()).is_equal_to(Some("Avant"));
}

#[test]
fn history_undo_inside_group_closes_it() {
    let mut deck = fixture();
    let mut h = History::new(500);
    h.begin_group("Tour");
    h.commit(&mut deck, "a", theme("pro")).unwrap();
    h.undo(&mut deck).unwrap();
    assert_that!(h.in_group()).is_false();
    h.commit(&mut deck, "Seul", theme("papier")).unwrap();
    assert_that!(h.undo_label()).is_equal_to(Some("Seul"));
}

#[test]
fn history_rollback_in_group_undoes_only_last_commit() {
    let mut deck = fixture();
    let mut h = History::new(500);
    h.begin_group("Tour");
    h.commit(&mut deck, "a", sample_ops()).unwrap();
    let after_first = deck.clone();
    h.commit(&mut deck, "b", theme("pro")).unwrap();
    h.rollback(&mut deck).unwrap();
    assert_that!(deck).is_equal_to(&after_first);
    assert_that!(h.undo_label()).is_equal_to(Some("Tour"));
    assert_that!(h.can_redo()).is_false();
}

#[test]
fn history_rollback_outside_group_removes_transaction_without_redo() {
    let mut deck = fixture();
    let before = deck.clone();
    let mut h = History::new(500);
    h.commit(&mut deck, "Seul", theme("pro")).unwrap();
    h.rollback(&mut deck).unwrap();
    assert_that!(deck).is_equal_to(&before);
    assert_that!(h.can_undo()).is_false();
    assert_that!(h.can_redo()).is_false();
}

#[test]
fn history_group_serde_round_trip_keeps_open_group() {
    let mut deck = fixture();
    let mut h = History::new(500);
    h.begin_group("Tour");
    h.commit(&mut deck, "a", theme("pro")).unwrap();
    let mut back: History = serde_json::from_str(&serde_json::to_string(&h).unwrap()).unwrap();
    back.commit(&mut deck, "b", theme("papier")).unwrap();
    back.end_group();
    assert_that!(back.undo_label()).is_equal_to(Some("Tour"));
    back.undo(&mut deck).unwrap();
    assert_that!(back.can_undo()).is_false();
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-core history_` → échec de compilation (`begin_group`, `rollback` absents).

- [ ] **Step 3 : implémentation** dans `crates/oku-slides-core/src/history.rs`. Compléter le commentaire de module par : « Un groupe (un tour de l'agent) réunit plusieurs commits en une seule transaction. » Puis :

```rust
/// Groupe ouvert : ses commits forment une seule transaction.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Group {
    label: String,
    /// La transaction du groupe est au sommet de la pile d'annulation.
    started: bool,
}

/// Sérialisable : la reprise de l'app le garde d'un lancement à l'autre.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct History {
    undo: VecDeque<Transaction>,
    redo: VecDeque<Transaction>,
    limit: usize,
    /// Groupe ouvert (un tour de l'agent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    group: Option<Group>,
    /// Inverses ajoutés par le dernier commit, en tête de la dernière transaction.
    #[serde(skip)]
    last: Option<usize>,
}
```

Dans `impl History` (`new` initialise `group: None, last: None`) :

```rust
    /// Applique `ops` en une transaction. Atomique : en cas d'erreur, rien n'a changé.
    /// Dans un groupe, la transaction du groupe s'agrandit (son libellé reste).
    pub fn commit(&mut self, deck: &mut Deck, label: impl Into<String>, ops: Vec<Op>) -> Result<(), TransactionError> {
        let inverse = apply_all(deck, ops)?;
        self.last = Some(inverse.len());
        self.redo.clear();
        let label = match &mut self.group {
            Some(g) if g.started => {
                if let Some(t) = self.undo.back_mut() {
                    // Défaire le groupe : les inverses du dernier commit d'abord.
                    let older = std::mem::replace(&mut t.ops, inverse);
                    t.ops.extend(older);
                    return Ok(());
                }
                g.label.clone()
            }
            Some(g) => {
                g.started = true;
                g.label.clone()
            }
            None => label.into(),
        };
        self.undo.push_back(Transaction { label, ops: inverse });
        if self.undo.len() > self.limit {
            self.undo.pop_front();
        }
        Ok(())
    }

    /// Ouvre un groupe : les commits suivants forment une seule transaction
    /// nommée `label`, jusqu'à [`History::end_group`]. Un groupe vide ne laisse rien.
    pub fn begin_group(&mut self, label: impl Into<String>) {
        self.group = Some(Group { label: label.into(), started: false });
    }

    /// Ferme le groupe ouvert, s'il y en a un.
    pub fn end_group(&mut self) {
        self.group = None;
    }

    pub fn in_group(&self) -> bool {
        self.group.is_some()
    }

    /// Défait le seul dernier commit (un rendu raté juste après) ; rien ne
    /// passe dans la pile de rétablissement.
    pub fn rollback(&mut self, deck: &mut Deck) -> Result<(), TransactionError> {
        let Some(n) = self.last.take() else { return Ok(()) };
        let Some(t) = self.undo.back_mut() else { return Ok(()) };
        let n = n.min(t.ops.len());
        apply_all(deck, t.ops[..n].to_vec())?;
        t.ops.drain(..n);
        if t.ops.is_empty() {
            self.undo.pop_back();
            if let Some(g) = &mut self.group {
                g.started = false;
            }
        }
        Ok(())
    }

    /// Défait la dernière transaction ; renvoie son libellé. Ferme le groupe ouvert.
    pub fn undo(&mut self, deck: &mut Deck) -> Result<Option<&str>, TransactionError> {
        self.group = None;
        self.last = None;
        step(&mut self.undo, &mut self.redo, deck)
    }

    /// Rejoue la dernière transaction défaite ; renvoie son libellé. Ferme le groupe ouvert.
    pub fn redo(&mut self, deck: &mut Deck) -> Result<Option<&str>, TransactionError> {
        self.group = None;
        self.last = None;
        step(&mut self.redo, &mut self.undo, deck)
    }
```

Dans `crates/oku-slides-session/src/lib.rs`, `Session::apply` : remplacer `doc.history.undo(&mut doc.sources.deck).ok();` par `doc.history.rollback(&mut doc.sources.deck).ok();`. (Pas de test de session : un rendu raté ne se provoque pas depuis un deck ; `rollback` est couvert ci-dessus.)

Dans `Cargo.toml` du workspace : `rust-version = "1.92"`.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-core -p oku-slides-session` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add Cargo.toml crates/oku-slides-core/src/history.rs crates/oku-slides-core/tests/ops.rs crates/oku-slides-session/src/lib.rs
git commit -m "feat(core) : groupes d'annulation et rollback du dernier commit (t-99)"
```

---

### Task 2 : Réglages : section [agent] et écriture de config.toml

**Files:**
- Modify: `crates/oku-slides-platform/src/config.rs`, `crates/oku-slides-platform/src/config.default.toml`, `crates/oku-slides-platform/src/lib.rs`, `crates/oku-slides-platform/Cargo.toml`
- Create: `crates/oku-slides-platform/src/config_write.rs`, `crates/oku-slides-platform/src/config_write_tests.rs`
- Modify: `Cargo.toml` (workspace : `toml_edit = "0.25"`)
- Test: `crates/oku-slides-platform/src/config_tests.rs`

**Interfaces:**
- Produces: `Settings.agent: AgentSettings { pub command: String, pub args: Vec<String> }` ; `oku_slides_platform::config_write::{write_value, ConfigWriteError}` avec `pub fn write_value(storage: &Storage, path: &Path, key: &str, value: &serde_json::Value) -> Result<(), ConfigWriteError>` ; `Settings::config_file(&self) -> PathBuf` (`<data>/config.toml`).

- [ ] **Step 1 : tests qui échouent** — dans `config_tests.rs` :

```rust
#[test]
fn defaults_agent_runs_claude_agent_acp_through_npx() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.agent.command.as_str()).is_equal_to("npx");
    assert_that!(s.agent.args.clone())
        .is_equal_to(vec!["-y".to_owned(), "@agentclientprotocol/claude-agent-acp@0.81.2".to_owned()]);
}

#[test]
fn env_agent_args_split_on_spaces() {
    let env = Settings::environment().source(Some(std::collections::HashMap::from([
            ("OKU_SLIDES_AGENT__COMMAND".to_owned(), "/bin/faux".to_owned()),
            ("OKU_SLIDES_AGENT__ARGS".to_owned(), "--scenario e2e.json".to_owned()),
        ])));
    let s = Settings::from_sources(None, env).unwrap();
    assert_that!(s.agent.command.as_str()).is_equal_to("/bin/faux");
    assert_that!(s.agent.args.clone()).is_equal_to(vec!["--scenario".to_owned(), "e2e.json".to_owned()]);
}
```

Et `config_write_tests.rs` :

```rust
use serde_json::json;
use speculoos::prelude::*;

use crate::config_write::write_value;
use crate::storage::Storage;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("oku-slides-config-write-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("config.toml")
}

#[test]
fn write_value_missing_file_creates_nested_table() {
    let path = temp("absent");
    std::fs::remove_file(&path).ok();
    write_value(&Storage::local().unwrap(), &path, "stt.vad.pause", &json!(0.9)).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_that!(text.contains("[stt.vad]")).is_true();
    assert_that!(text.contains("pause = 0.9")).is_true();
}

#[test]
fn write_value_keeps_comments_and_other_keys() {
    let path = temp("commentaires");
    std::fs::write(&path, "# à moi\n[history]\n# garder\nlimit = 10\n\n[i18n]\nlocale = \"en\"\n").unwrap();
    write_value(&Storage::local().unwrap(), &path, "i18n.locale", &json!("fr")).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_that!(text.as_str()).is_equal_to("# à moi\n[history]\n# garder\nlimit = 10\n\n[i18n]\nlocale = \"fr\"\n");
}

#[test]
fn write_value_null_removes_key() {
    let path = temp("retrait");
    std::fs::write(&path, "[i18n]\nlocale = \"en\"\n").unwrap();
    write_value(&Storage::local().unwrap(), &path, "i18n.locale", &json!(null)).unwrap();
    assert_that!(std::fs::read_to_string(&path).unwrap().contains("locale")).is_false();
}

#[test]
fn write_value_invalid_toml_returns_error_and_keeps_file() {
    let path = temp("invalide");
    std::fs::write(&path, "[[[").unwrap();
    assert_that!(write_value(&Storage::local().unwrap(), &path, "history.limit", &json!(5))).is_err();
    assert_that!(std::fs::read_to_string(&path).unwrap().as_str()).is_equal_to("[[[");
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-platform` → échec de compilation (`agent`, `config_write` absents).

- [ ] **Step 3 : implémentation.**

`config.default.toml`, à la fin :

```toml
[agent]
# Agent ACP lancé par ⌘⇧D : commande et arguments. Il ne reçoit que les
# outils d'Oku Slides (serveur MCP local), ni fichiers ni terminal.
# OKU_SLIDES_AGENT__ARGS se découpe sur les espaces.
command = "npx"
args = ["-y", "@agentclientprotocol/claude-agent-acp@0.81.2"]
```

`config.rs` : champ `pub agent: AgentSettings` dans `Settings` ;

```rust
/// Agent ACP de la dictée vers l'agent (⌘⇧D).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct AgentSettings {
    /// Programme lancé (cherché dans le `PATH`).
    pub command: String,
    pub args: Vec<String>,
}
```

L'environnement est construit à un seul endroit ; `Settings::load` appelle `Self::environment()` au lieu de le construire :

```rust
    /// Variables `OKU_SLIDES_<SECTION>__<CLÉ>` ; `agent.args` se découpe sur les espaces.
    pub(crate) fn environment() -> config::Environment {
        config::Environment::with_prefix("OKU_SLIDES")
            .prefix_separator("_")
            .separator("__")
            .try_parsing(true)
            .list_separator(" ")
            .with_list_parse_key("agent.args")
    }
```

Et dans `impl Settings` :

```rust
    /// `config.toml` du dossier de données (facultatif, écrit par `set_setting`).
    pub fn config_file(&self) -> PathBuf {
        self.paths.data().join("config.toml")
    }
```

`config_write.rs` :

```rust
//! Écriture d'une clé dans `config.toml` : commentaires, ordre et autres
//! clés restent tels quels (toml_edit). L'effet vient au prochain lancement.

use std::path::{Path, PathBuf};

use serde_json::Value;
use snafu::{Backtrace, OptionExt, ResultExt, Snafu};
use toml_edit::{DocumentMut, Item, Table, value};

use crate::storage::{Storage, StorageError, StorageErrorKind};

/// `config.toml` n'a pas pu être modifié ; le fichier est intact.
#[derive(Debug, Snafu)]
pub struct ConfigWriteError(ConfigWriteInner);

#[derive(Debug, Snafu)]
enum ConfigWriteInner {
    #[snafu(display("cannot read {}", path.display()))]
    Read { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("{} is not valid TOML", path.display()))]
    Parse { path: PathBuf, source: toml_edit::TomlError, backtrace: Backtrace },
    #[snafu(display("{key} is not a settings key"))]
    Key { key: String, backtrace: Backtrace },
    #[snafu(display("cannot write {}", path.display()))]
    Write { path: PathBuf, source: StorageError, backtrace: Backtrace },
}

/// Écrit `json` à la clé pointée `key` (`history.limit`, `stt.vad.pause`) ;
/// `null` retire la clé.
pub fn write_value(storage: &Storage, path: &Path, key: &str, json: &Value) -> Result<(), ConfigWriteError> {
    let text = match storage.read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(e) if e.kind() == StorageErrorKind::NotFound => String::new(),
        Err(source) => return Err(ReadSnafu { path }.into_error(source).into()),
    };
    let mut doc: DocumentMut = text.parse().context(ParseSnafu { path })?;
    let mut parts: Vec<&str> = key.split('.').collect();
    let leaf = parts.pop().filter(|l| !l.is_empty()).context(KeySnafu { key })?;
    let mut table: &mut Table = doc.as_table_mut();
    for part in parts {
        let item = table.entry(part).or_insert_with(|| Item::Table(Table::new()));
        table = item.as_table_mut().context(KeySnafu { key })?;
    }
    match toml_value(json) {
        Some(v) => {
            table.insert(leaf, value(v));
        }
        None => {
            table.remove(leaf);
        }
    }
    storage.write_atomic(path, doc.to_string()).context(WriteSnafu { path })?;
    Ok(())
}

/// Une valeur JSON simple en TOML ; `null` (et ce qui n'a pas d'équivalent) : rien.
fn toml_value(json: &Value) -> Option<toml_edit::Value> {
    match json {
        Value::Bool(b) => Some((*b).into()),
        Value::Number(n) => n.as_i64().map(Into::into).or_else(|| n.as_f64().map(Into::into)),
        Value::String(s) => Some(s.as_str().into()),
        Value::Array(items) => Some(toml_edit::Value::Array(items.iter().filter_map(toml_value).collect())),
        Value::Null | Value::Object(_) => None,
    }
}

#[cfg(test)]
#[path = "config_write_tests.rs"]
mod config_write_tests;
```

`lib.rs` : `pub mod config_write;` et, dans la doc du module, la ligne « - [`config_write`] : écriture d'une clé dans `config.toml`. » ; réexporter `AgentSettings` là où les autres réglages le sont (`pub use config::{…, AgentSettings}` s'il y a une liste). `Cargo.toml` de la plateforme : `toml_edit.workspace = true`, `serde_json.workspace = true` (s'il manque). Workspace : `toml_edit = "0.25"`.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-platform` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add Cargo.toml Cargo.lock crates/oku-slides-platform
git commit -m "feat(config) : section [agent] et écriture d'une clé de config.toml (t-100)"
```

---
### Task 3 : A2UI : catalogue v2

**Files:**
- Modify: `crates/oku-slides-a2ui/src/message.rs`, `crates/oku-slides-a2ui/src/lib.rs`, `crates/oku-slides-a2ui/Cargo.toml`
- Modify: `crates/oku-slides-session/src/panel.rs` (constructeurs partagés)
- Modify: `ui/src/a2ui/catalog.tsx`, `ui/src/a2ui/catalog.test.tsx` (identifiant du catalogue)
- Test: `crates/oku-slides-a2ui/src/message_tests.rs`

**Interfaces:**
- Produces: `CATALOG_ID = "urn:oku-slides:a2ui:catalog:v2"` ; nouvelles variantes de `ComponentKind` : `Row { children: Vec<String> }`, `Button { label, primary: Option<bool>, action: Action }`, `ChoiceChips { options: Vec<ChoiceOption>, value: Dynamic<String>, action: Option<Action> }`, `Slider { label, value: Dynamic<f64>, min: f64, max: f64, step: Option<f64>, action: Option<Action> }`, `ColorField { label, value: Dynamic<String>, palette: Vec<String>, action: Option<Action> }`, `Select { label, value: Dynamic<String>, options: Vec<ChoiceOption>, action: Option<Action> }`, `SlideThumb { svg: String, label: Dynamic<String>, action: Option<Action> }`, `DiffText { segments: Vec<DiffSegment> }` ; types `ChoiceOption { pub id: String, pub label: String, pub hint: Option<String> }`, `DiffSegment { pub kind: DiffKind, pub text: String }`, `DiffKind { Keep, Del, Ins }` (JSON `keep|del|ins`). Tous `Debug, Clone, PartialEq, Serialize` ; `ChoiceOption` aussi `Deserialize + JsonSchema`. Constructeurs partagés : `Component::card(id, child: &str)`, `Component::column(id, children: &[&str])`, `Component::row(id, children: &[&str])`, `Component::text(id, text: impl Into<String>, variant: Option<TextVariant>)`, `Component::button(id, label: impl Into<String>, primary: bool, action: Action)` ; `Action::named(name)` (évènement sans contexte). `panel.rs` de la session les utilise (ses helpers `column`/`title` disparaissent).

- [ ] **Step 1 : tests qui échouent** — dans `message_tests.rs`, remplacer `catalog_id_names_oku_slides` et ajouter :

```rust
#[test]
fn catalog_id_is_v2() {
    assert_that!(CATALOG_ID).is_equal_to("urn:oku-slides:a2ui:catalog:v2");
}

fn component_json(kind: ComponentKind) -> Option<Value> {
    serde_json::to_value(Component::new("c", kind)).ok()
}

#[test]
fn button_serializes_label_primary_and_action() {
    let action = Action::event("confirm", [("confirmed", Dynamic::Literal(json!(true)))]);
    assert_that!(component_json(ComponentKind::Button {
        label: Dynamic::Literal("Autoriser".into()),
        primary: Some(true),
        action,
    }))
    .is_some()
    .is_equal_to(json!({
        "id": "c", "component": "Button", "label": "Autoriser", "primary": true,
        "action": { "event": { "name": "confirm", "context": { "confirmed": true } } }
    }));
}

#[test]
fn choice_chips_serializes_options_with_optional_hint() {
    let options = vec![
        ChoiceOption { id: "a".into(), label: "Deux colonnes".into(), hint: None },
        ChoiceOption { id: "b".into(), label: "Citation".into(), hint: Some("sobre".into()) },
    ];
    assert_that!(component_json(ComponentKind::ChoiceChips {
        options,
        value: Dynamic::path("/choice"),
        action: None,
    }))
    .is_some()
    .is_equal_to(json!({
        "id": "c", "component": "ChoiceChips",
        "options": [{ "id": "a", "label": "Deux colonnes" }, { "id": "b", "label": "Citation", "hint": "sobre" }],
        "value": { "path": "/choice" }
    }));
}

#[test]
fn diff_text_serializes_segment_kinds_in_lowercase() {
    let segments = vec![
        DiffSegment { kind: DiffKind::Del, text: "Nos".into() },
        DiffSegment { kind: DiffKind::Ins, text: "Trois".into() },
        DiffSegment { kind: DiffKind::Keep, text: " objectifs".into() },
    ];
    assert_that!(component_json(ComponentKind::DiffText { segments })).is_some().is_equal_to(json!({
        "id": "c", "component": "DiffText",
        "segments": [
            { "kind": "del", "text": "Nos" }, { "kind": "ins", "text": "Trois" }, { "kind": "keep", "text": " objectifs" }
        ]
    }));
}

#[test]
fn slider_color_select_thumb_and_row_serialize_their_fields() {
    let slider = component_json(ComponentKind::Slider {
        label: Dynamic::Literal("Espacement".into()),
        value: Dynamic::path("/f0"),
        min: 0.0,
        max: 64.0,
        step: Some(4.0),
        action: None,
    });
    assert_that!(slider).is_some().is_equal_to(json!({
        "id": "c", "component": "Slider", "label": "Espacement", "value": { "path": "/f0" },
        "min": 0.0, "max": 64.0, "step": 4.0
    }));
    let color = component_json(ComponentKind::ColorField {
        label: Dynamic::Literal("Accent".into()),
        value: Dynamic::path("/f1"),
        palette: vec!["#c2573a".into()],
        action: None,
    });
    assert_that!(color.and_then(|v| v.get("palette").cloned())).is_some().is_equal_to(json!(["#c2573a"]));
    let select = component_json(ComponentKind::Select {
        label: Dynamic::Literal("Mise en page".into()),
        value: Dynamic::path("/f2"),
        options: vec![ChoiceOption { id: "columns".into(), label: "Colonnes".into(), hint: None }],
        action: None,
    });
    assert_that!(select.and_then(|v| v.get("component").cloned())).is_some().is_equal_to(json!("Select"));
    let thumb = component_json(ComponentKind::SlideThumb {
        svg: "<svg/>".into(),
        label: Dynamic::Literal("1 · deux colonnes".into()),
        action: Some(Action::event("variant", [("index", Dynamic::Literal(json!(0)))])),
    });
    assert_that!(thumb.and_then(|v| v.get("svg").cloned())).is_some().is_equal_to(json!("<svg/>"));
    let row = component_json(ComponentKind::Row { children: vec!["a".into(), "b".into()] });
    assert_that!(row).is_some().is_equal_to(json!({ "id": "c", "component": "Row", "children": ["a", "b"] }));
}

#[test]
fn shared_constructors_build_the_same_components() {
    assert_that!(Component::column("b", &["x", "y"]))
        .is_equal_to(Component::new("b", ComponentKind::Column { children: vec!["x".into(), "y".into()] }));
    assert_that!(Component::card("r", "b")).is_equal_to(Component::new("r", ComponentKind::Card { child: "b".into() }));
    assert_that!(Component::text("t", "Grille", Some(TextVariant::H4))).is_equal_to(Component::new(
        "t",
        ComponentKind::Text { text: Dynamic::Literal("Grille".into()), variant: Some(TextVariant::H4) },
    ));
    assert_that!(serde_json::to_value(Action::named("dismiss")).ok())
        .is_some()
        .is_equal_to(json!({ "event": { "name": "dismiss" } }));
}
```

Ajouter `ChoiceOption, DiffKind, DiffSegment` à l'import du haut du fichier de tests.

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-a2ui` → échec de compilation.

- [ ] **Step 3 : implémentation** dans `message.rs` : `pub const CATALOG_ID: &str = "urn:oku-slides:a2ui:catalog:v2";` (doc : « Catalogue des surfaces d'Oku Slides (panneau et agent), implémenté par l'interface (`ui/src/a2ui`). »). Ajouter à `ComponentKind` (doc de l'enum : « `Stepper`, `ChoiceChips`, `ColorField`, `SlideThumb`, `DiffText` sont propres à Oku Slides. ») :

```rust
    /// Enfants côte à côte.
    Row { children: Vec<String> },
    /// Un bouton ; `primary` : l'action principale de la surface.
    Button {
        label: Dynamic<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        primary: Option<bool>,
        action: Action,
    },
    /// Des pastilles, une par option ; choisir écrit l'`id` dans `value` puis envoie `action`.
    ChoiceChips {
        options: Vec<ChoiceOption>,
        value: Dynamic<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Un curseur ; `action` part quand on lâche.
    Slider {
        label: Dynamic<String>,
        value: Dynamic<f64>,
        min: f64,
        max: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Une couleur (hex), choisie dans `palette` ou libre.
    ColorField {
        label: Dynamic<String>,
        value: Dynamic<String>,
        palette: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Une liste déroulante.
    Select {
        label: Dynamic<String>,
        value: Dynamic<String>,
        options: Vec<ChoiceOption>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Miniature d'une slide rendue par la session (SVG), cliquable.
    SlideThumb {
        svg: String,
        label: Dynamic<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Un texte et ses changements, mot à mot.
    DiffText { segments: Vec<DiffSegment> },
```

Et, après `TextVariant` :

```rust
/// Une option d'un choix ou d'une liste. Aussi argument des outils de
/// l'agent (`ask_choice`, champs `select`) : désérialisable, avec schéma.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Un morceau de texte gardé, retiré ou ajouté.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiffSegment {
    pub kind: DiffKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Keep,
    Del,
    Ins,
}
```

Ajouter dans `impl Component` :

```rust
    /// Un post-it autour de `child`.
    pub fn card(id: impl Into<String>, child: &str) -> Self {
        Self::new(id, ComponentKind::Card { child: child.to_owned() })
    }

    pub fn column(id: impl Into<String>, children: &[&str]) -> Self {
        Self::new(id, ComponentKind::Column { children: children.iter().map(|c| (*c).to_owned()).collect() })
    }

    pub fn row(id: impl Into<String>, children: &[&str]) -> Self {
        Self::new(id, ComponentKind::Row { children: children.iter().map(|c| (*c).to_owned()).collect() })
    }

    pub fn text(id: impl Into<String>, text: impl Into<String>, variant: Option<TextVariant>) -> Self {
        Self::new(id, ComponentKind::Text { text: Dynamic::Literal(text.into()), variant })
    }

    pub fn button(id: impl Into<String>, label: impl Into<String>, primary: bool, action: Action) -> Self {
        Self::new(id, ComponentKind::Button { label: Dynamic::Literal(label.into()), primary: primary.then_some(true), action })
    }
```

Dans `impl Action` :

```rust
    /// L'évènement `name`, sans contexte.
    pub fn named(name: impl Into<String>) -> Self {
        Self { event: Event { name: name.into(), context: BTreeMap::new() } }
    }
```

Dans `crates/oku-slides-session/src/panel.rs`, supprimer les fonctions `column` et `title` et écrire `Component::column("template-body", &[…])`, `Component::text("template-title", l.tr("session-panel-template"), Some(TextVariant::H4))`, `Component::card("template", "template-body")` (idem pour la grille) ; `root` devient `Component::new("root", ComponentKind::Column { children: cards })` (inchangé). Les tests `panel_*` de `tests/session.rs` garantissent le même JSON.

Ajouter dans `impl ServerMessage` :

```rust
    /// Écrit `value` au chemin `path` du modèle de données.
    pub fn update_data_model(surface_id: impl Into<String>, path: impl Into<String>, value: Value) -> Self {
        Self::v0_9(ServerBody::UpdateDataModel(UpdateDataModel {
            surface_id: surface_id.into(),
            path: Some(path.into()),
            value: Some(value),
        }))
    }
```

`crates/oku-slides-a2ui/Cargo.toml` : `schemars.workspace = true`.

`lib.rs` : doc du module (catalogue : Column, Row, Card, Text, TextField, Stepper, Button, ChoiceChips, Slider, ColorField, Select, SlideThumb, DiffText) et réexporter `ChoiceOption, DiffKind, DiffSegment`. Le test existant `create_surface_names_the_oku_slides_catalog` passe tel quel (il lit `CATALOG_ID`).

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-a2ui -p oku-slides-session` → tout passe (la session ne dépend que de `CATALOG_ID`).
- [ ] **Step 5 :** `ui/src/a2ui/catalog.tsx` et `catalog.test.tsx` écrivent `urn:oku-slides:a2ui:catalog:panel:v1` : remplacer par `urn:oku-slides:a2ui:catalog:v2` dans les deux (les composants TS viennent en Task 18) ; `cd ui && bun test src/a2ui` → passe.
- [ ] **Step 6 : commit**

```bash
git add crates/oku-slides-a2ui crates/oku-slides-session/src/panel.rs ui/src/a2ui/catalog.tsx ui/src/a2ui/catalog.test.tsx
git commit -m "feat(a2ui) : catalogue v2, composants des surfaces de l'agent (t-101)"
```

---

### Task 4 : Rendu : slide en PNG

**Files:**
- Modify: `crates/oku-slides-render/src/lib.rs`, `crates/oku-slides-render/src/error.rs`, `crates/oku-slides-render/Cargo.toml`
- Modify: `Cargo.toml` (workspace : `typst-render = "0.15.1"`)
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl` (clé `render-image`, 20 locales ; `render-*` y sont déjà — sinon dans le fichier où se trouve `render-compile`)
- Create: `crates/oku-slides-session/src/agent/mod.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (`mod agent;`)
- Test: `crates/oku-slides-render/tests/png.rs`, `crates/oku-slides-session/tests/agent.rs`

**Interfaces:**
- Produces: `Compiled::png(&self, index: usize, pixel_per_pt: f32) -> Result<Vec<u8>, RenderError>` ; `RenderErrorKind::Image` ; `Session::png(&self, id: &str) -> Result<Vec<u8>>` (0,5 px/pt : 960 × 540 pour une slide de 1920 × 1080 pt).

- [ ] **Step 1 : tests qui échouent.**

`crates/oku-slides-render/tests/png.rs` (s'inspirer de l'en-tête de `tests/render.rs` pour charger le deck d'exemple ; si `tests/render.rs` a une fonction qui construit un `Renderer` et un deck, la copier ici à l'identique) :

```rust
use oku_slides_core::Deck;
use oku_slides_render::{Fonts, Renderer};
use speculoos::prelude::*;

fn roadmap() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("{path} : {e}"))
}

#[test]
fn png_first_page_half_scale_is_960_by_540_png() {
    let mut renderer = Renderer::new(Fonts::default()).unwrap();
    let compiled = renderer.render(&roadmap(), &Default::default()).unwrap();
    let png = compiled.png(0, 0.5).unwrap();
    assert_that!(png[..8].to_vec()).is_equal_to(b"\x89PNG\r\n\x1a\n".to_vec());
    // IHDR : largeur puis hauteur, big-endian, octets 16..24.
    let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    assert_that!((w, h)).is_equal_to((960, 540));
}

#[test]
fn png_missing_page_returns_no_page_error() {
    let mut renderer = Renderer::new(Fonts::default()).unwrap();
    let compiled = renderer.render(&roadmap(), &Default::default()).unwrap();
    let err = compiled.png(999, 0.5).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_render::RenderErrorKind::NoPage);
}
```

`crates/oku-slides-session/tests/agent.rs` (nouveau fichier ; les tâches 5 à 7 y ajoutent leurs tests et leurs imports) :

```rust
use oku_slides_session::{Fonts, Platform, Session, SessionErrorKind, Settings};
use serde_json::json;
use speculoos::prelude::*;

/// Une session isolée, en français (comme `tests/session.rs`).
fn session() -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    let data = std::env::temp_dir().join(format!("oku-slides-agent-session-{}-{}", std::process::id(), unique()));
    settings.paths.data = Some(data);
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session impossible : {}", e.one_line()))
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

#[test]
fn png_of_slide_returns_png_bytes() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let png = s.png("s-libre").unwrap();
    assert_that!(png[..4].to_vec()).is_equal_to(b"\x89PNG".to_vec());
}

#[test]
fn png_of_unknown_slide_returns_no_slide() {
    let s = session();
    assert_that!(s.png("inconnue").unwrap_err().kind()).is_equal_to(SessionErrorKind::NoSlide);
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-render --test png` et `-p oku-slides-session --test agent` → échec de compilation.

- [ ] **Step 3 : implémentation.**

Workspace `Cargo.toml` : `typst-render = "0.15.1"` ; `crates/oku-slides-render/Cargo.toml` : `typst-render.workspace = true`.

`error.rs` : variante interne et genre :

```rust
    #[snafu(display("PNG export failed: {message}"))]
    Image { message: String, backtrace: Backtrace },
```

`RenderErrorKind::Image` (doc : « L'export d'une image a échoué. ») ; `kind()` : `RenderInner::Image { .. } => RenderErrorKind::Image` ; `localize` : `RenderErrorKind::Image => "render-image"`.

`lib.rs`, dans `impl Compiled`, après `svg` :

```rust
    /// PNG de la page `index` (0 = première), à `pixel_per_pt` pixels par point.
    pub fn png(&self, index: usize, pixel_per_pt: f32) -> Result<Vec<u8>, RenderError> {
        let page = self.doc.pages().get(index).context(NoPageSnafu { index })?;
        let options = typst_render::RenderOptions { pixel_per_pt: f64::from(pixel_per_pt).into(), ..Default::default() };
        typst_render::render(page, &options)
            .encode_png()
            .map_err(|e| ImageSnafu { message: e.to_string() }.build().into())
    }
```

(`NoPageSnafu`, `ImageSnafu` : sélecteurs du module `error`, importés comme `NoPageSnafu` l'est déjà pour `svg`.)

Fluent `render-image` : `en` « The image of the slide could not be produced. » ; `fr` « L'image de la slide n'a pas pu être produite. » ; 18 autres locales traduites.

`crates/oku-slides-session/src/agent/mod.rs` :

```rust
//! Ce que la session offre à l'agent : images des slides, tours d'annulation,
//! surfaces (formes, variantes) et contexte. Les outils MCP (`oku-slides-mcp`)
//! et le client ACP (`oku-slides-agent`) passent par ici.

use snafu::{OptionExt, ResultExt};

use crate::error::{NoSlideSnafu, RenderSnafu};
use crate::{Result, Session};

/// Pixels par point des images pour l'agent : 960 × 540 pour une slide.
const AGENT_PIXEL_PER_PT: f32 = 0.5;

impl Session {
    /// Image PNG de la slide `id`, pour que l'agent voie le rendu.
    pub fn png(&self, id: &str) -> Result<Vec<u8>> {
        let compiled = self.compiled.as_ref().context(NoSlideSnafu { id })?;
        let page = compiled.layout().page_of(id).context(NoSlideSnafu { id })?;
        Ok(compiled.png(page - 1, AGENT_PIXEL_PER_PT).context(RenderSnafu)?)
    }
}
```

`lib.rs` de la session : `mod agent;` avec les autres modules.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-render -p oku-slides-session -p oku-slides-i18n` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add Cargo.toml Cargo.lock crates/oku-slides-render crates/oku-slides-session crates/oku-slides-i18n
git commit -m "feat(render) : slide en PNG pour l'agent (t-102)"
```

---
### Task 5 : Session : formes de surface

**Files:**
- Create: `crates/oku-slides-session/src/agent/forms.rs`, `crates/oku-slides-session/src/agent/forms_tests.rs`
- Create: `crates/oku-slides-session/src/agent/diff.rs`, `crates/oku-slides-session/src/agent/diff_tests.rs`
- Modify: `crates/oku-slides-session/src/agent/mod.rs`, `crates/oku-slides-session/src/text.rs`, `crates/oku-slides-session/src/error.rs`, `crates/oku-slides-session/src/lib.rs`, `crates/oku-slides-session/Cargo.toml`
- Modify: `Cargo.toml` (workspace : `similar = "3"`, `insta = { version = "1", features = ["json"] }`)
- Create: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (20 locales, nouveau domaine « agent »)
- Modify: `crates/oku-slides-i18n/src/lib.rs` (macro `locale!` : inclure `agent.ftl` avant `ui.ftl`)
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (clé `session-agent-form`, 20 locales)
- Test: `crates/oku-slides-session/tests/agent.rs`

**Interfaces:**
- Consumes: `Component::{card, column, row, text, button}`, `Action::named`, `ChoiceOption`, `DiffSegment` (Task 3).
- Produces (réexportés par `oku_slides_session::agent`, module public) :
  - `Place { Sheet, Thread, Drawer, Postit, Modal }` ; `Surface { pub id: String, pub place: Place, pub anchor: Option<Anchor>, pub messages: Vec<ServerMessage> }` (Serialize, Clone) ;
  - arguments (Deserialize + JsonSchema) : `Anchor { slide, element }`, `AskChoice { question, options: Vec<ChoiceOption>, anchor: Option<Anchor> }`, `AskConfirm { message, danger: bool }`, `AskRewrite { slide, element, proposals: Vec<String> }`, `FieldKind`, `ParamField`, `ShowParams`, `Variant`, `ShowVariants` (les trois derniers servent en Task 6) ;
  - composition : `forms::choice(id, &AskChoice, &Localizer) -> Surface`, `forms::confirm(id, &AskConfirm, &Localizer) -> Surface`, `forms::permission(id, title: &str, options: &[String]) -> Surface` ;
  - `diff::diff_words(old: &str, new: &str) -> Vec<DiffSegment>` ;
  - bornes `forms::{CHOICES, PROPOSALS, VARIANTS, FIELDS}: (usize, usize)` et `agent::check_len(what: &str, len: usize, bounds: (usize, usize)) -> Result<()>` (publics : le serveur MCP les réutilise) ;
  - `Session::rewrite_surface(&self, id: &str, spec: &AskRewrite) -> Result<Surface>` ; `Session::rewrite_ops(&self, spec: &AskRewrite, text: &str) -> Result<Vec<Op>>` ;
  - `text::text_to_paragraphs(text: &str) -> Vec<Paragraph>` (réexporté comme `paragraphs_to_text`) ;
  - `SessionErrorKind::AgentForm` (arguments de surface invalides ; le message anglais va à l'agent).
- Actions de surface (noms et contexte, renvoyés par l'interface) : `choose {choice}`, `confirm {confirmed}`, `permission {index}`, `rewrite {kept, text}`, `param {field, value}`, `variant {index}`, `dismiss {}`, `answer {text}`.

- [ ] **Step 1 : tests qui échouent.**

`diff_tests.rs` :

```rust
use oku_slides_a2ui::{DiffKind, DiffSegment};
use speculoos::prelude::*;

use super::diff_words;

fn seg(kind: DiffKind, text: &str) -> DiffSegment {
    DiffSegment { kind, text: text.into() }
}

#[test]
fn diff_words_replaced_word_gives_del_then_ins() {
    assert_that!(diff_words("Nos objectifs", "Nos paris")).is_equal_to(vec![
        seg(DiffKind::Keep, "Nos "),
        seg(DiffKind::Del, "objectifs"),
        seg(DiffKind::Ins, "paris"),
    ]);
}

#[test]
fn diff_words_identical_text_is_one_keep() {
    assert_that!(diff_words("Même texte", "Même texte")).is_equal_to(vec![seg(DiffKind::Keep, "Même texte")]);
}

#[test]
fn diff_words_merges_neighbouring_segments_of_same_kind() {
    let segments = diff_words("a b", "x y");
    assert_that!(segments.iter().filter(|s| s.kind == DiffKind::Del).count()).is_equal_to(1);
    assert_that!(segments.iter().filter(|s| s.kind == DiffKind::Ins).count()).is_equal_to(1);
}
```

`forms_tests.rs` (instantanés JSON ; `cargo insta accept` au premier passage après relecture du JSON produit) :

```rust
use oku_slides_a2ui::ChoiceOption;
use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::{Anchor, AskChoice, AskConfirm, Place, choice, confirm, permission};

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn options() -> Vec<ChoiceOption> {
    vec![
        ChoiceOption { id: "a".into(), label: "Deux colonnes".into(), hint: None },
        ChoiceOption { id: "b".into(), label: "Citation".into(), hint: Some("sobre".into()) },
    ]
}

#[test]
fn choice_with_anchor_goes_to_sheet() {
    let spec = AskChoice {
        question: "Quelle mise en page ?".into(),
        options: options(),
        anchor: Some(Anchor { slide: "s-4".into(), element: "e-2".into() }),
    };
    let s = choice("ask-1", &spec, &fr());
    assert_that!(s.place).is_equal_to(Place::Sheet);
    insta::assert_json_snapshot!(s);
}

#[test]
fn choice_without_anchor_goes_to_thread() {
    let spec = AskChoice { question: "Laquelle ?".into(), options: options(), anchor: None };
    assert_that!(choice("ask-2", &spec, &fr()).place).is_equal_to(Place::Thread);
}

#[test]
fn confirm_is_a_modal_with_two_buttons() {
    let s = confirm("ask-3", &AskConfirm { message: "Remplacer le fichier ?".into(), danger: true }, &fr());
    assert_that!(s.place).is_equal_to(Place::Modal);
    insta::assert_json_snapshot!(s);
}

#[test]
fn permission_has_one_button_per_option() {
    let s = permission("perm-1", "Écrire le fichier ?", &["Autoriser une fois".into(), "Refuser".into()]);
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json.matches("\"permission\"").count()).is_equal_to(2);
}
```

Dans `crates/oku-slides-session/tests/agent.rs` :

```rust
use oku_slides_session::agent::{AskRewrite, Place};

#[test]
fn rewrite_surface_of_text_element_diffs_each_proposal() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = AskRewrite {
        slide: "s-libre".into(),
        element: "e-texte".into(),
        proposals: vec!["Un bloc posé tout près du coin".into(), "Un bloc libre".into()],
    };
    let surface = s.rewrite_surface("ask-9", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Sheet);
    let json = serde_json::to_string(&surface).unwrap();
    assert_that!(json.matches("\"DiffText\"").count()).is_equal_to(2);
    assert_that!(json.contains("\"draft\":\"Un bloc posé tout près du coin\"")).is_true();
}

#[test]
fn rewrite_surface_of_title_field_reads_slide_title() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let title = s.deck().slide("s-constat").and_then(|sl| sl.title.clone()).unwrap();
    let spec = AskRewrite { slide: "s-constat".into(), element: "@title".into(), proposals: vec!["A".into(), "B".into()] };
    let json = serde_json::to_string(&s.rewrite_surface("ask-10", &spec).unwrap()).unwrap();
    assert_that!(json.contains(&title)).is_true();
}

#[test]
fn rewrite_with_one_proposal_returns_agent_form_error() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = AskRewrite { slide: "s-libre".into(), element: "e-texte".into(), proposals: vec!["Seule".into()] };
    assert_that!(s.rewrite_surface("ask-11", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn rewrite_ops_then_apply_changes_element_text() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = AskRewrite { slide: "s-libre".into(), element: "e-texte".into(), proposals: vec!["a".into(), "b".into()] };
    let ops = s.rewrite_ops(&spec, "Nouveau\n- une puce").unwrap();
    s.apply("Reformulation", ops).unwrap();
    let view = s.slide("s-libre").unwrap();
    let text = view.targets.iter().find(|t| t.id == "e-texte").and_then(|t| t.text.clone()).unwrap();
    assert_that!(text.as_ref()).is_equal_to("Nouveau\n- une puce");
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-session` → échec de compilation.

- [ ] **Step 3 : implémentation.**

Workspace : `similar = "3"`, `insta = { version = "1", features = ["json"] }`. Session `Cargo.toml` : `similar.workspace = true`, `schemars.workspace = true` ; `[dev-dependencies]` `insta.workspace = true`.

`text.rs`, après `paragraphs_to_text` :

```rust
/// L'inverse de [`paragraphs_to_text`] : une ligne par paragraphe, `- ` puce, `1. ` liste numérotée.
pub fn text_to_paragraphs(text: &str) -> Vec<Paragraph> {
    text.lines()
        .map(|line| {
            let numbered = line.split_once(". ").filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
            match (line.strip_prefix("- "), numbered) {
                (Some(rest), _) => Paragraph { list: Some(ListKind::Bullet), ..Paragraph::plain(rest) },
                (None, Some((_, rest))) => Paragraph { list: Some(ListKind::Number), ..Paragraph::plain(rest) },
                (None, None) => Paragraph::plain(line),
            }
        })
        .collect()
}
```

`lib.rs` : `pub use text::{interp, paragraphs_to_text, text_to_paragraphs};` et `mod agent;` devient `pub mod agent;`.

`error.rs` : variante `#[snafu(display("invalid surface: {reason}"))] AgentForm { reason: String, backtrace: Backtrace }`, genre `SessionErrorKind::AgentForm` (doc : « Arguments d'une surface de l'agent invalides. »), `localize` : `AgentForm { reason, .. } => l.tr_with("session-agent-form", &[("reason", reason.as_str().into())])`.

Fluent. Dans `crates/oku-slides-i18n/src/lib.rs`, la macro `locale!` gagne `include_str!(concat!("locales/", $lang, "/agent.ftl")),` juste avant la ligne de `ui.ftl` (qui reste la dernière). Créer `agent.ftl` dans les 20 dossiers de `locales/`, en-tête `# Agent : surfaces, confirmations, fil.` ; `session-agent-form` va dans `session.ftl`. Clés (20 locales) :

| Clé | Fichier | en | fr |
| --- | --- | --- | --- |
| `session-agent-form` | session.ftl | `The agent's request is invalid: { $reason }` | `La demande de l'agent est invalide : { $reason }` |
| `agent-surface-cancel` | agent.ftl | `Cancel` | `Annuler` |
| `agent-surface-confirm` | agent.ftl | `Confirm` | `Confirmer` |
| `agent-surface-confirm-danger` | agent.ftl | `Yes, do it` | `Oui, le faire` |
| `agent-surface-rewrite` | agent.ftl | `Rewording` | `Reformulation` |
| `agent-surface-pick` | agent.ftl | `Choose { $n }` | `Choisir { $n }` |
| `agent-surface-keep` | agent.ftl | `Keep the original` | `Garder l'original` |
| `agent-surface-edit` | agent.ftl | `Edit before applying` | `Retoucher avant d'appliquer` |
| `agent-surface-close` | agent.ftl | `Close` | `Fermer` |
| `agent-surface-variants` | agent.ftl | `Variants` | `Variantes` |

`diff.rs` :

```rust
//! Différences mot à mot d'une reformulation, pour la surface `ask_rewrite`.

use oku_slides_a2ui::{DiffKind, DiffSegment};
use similar::{ChangeTag, TextDiff};

/// Segments gardés, retirés, ajoutés ; deux voisins de même nature sont fusionnés.
pub fn diff_words(old: &str, new: &str) -> Vec<DiffSegment> {
    let mut out: Vec<DiffSegment> = Vec::new();
    for change in TextDiff::from_words(old, new).iter_all_changes() {
        let kind = match change.tag() {
            ChangeTag::Equal => DiffKind::Keep,
            ChangeTag::Delete => DiffKind::Del,
            ChangeTag::Insert => DiffKind::Ins,
        };
        match out.last_mut() {
            Some(last) if last.kind == kind => last.text.push_str(change.value()),
            _ => out.push(DiffSegment { kind, text: change.value().to_owned() }),
        }
    }
    out
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod diff_tests;
```

Si l'ordre `Del`/`Ins` ou la place d'une espace diffère de `diff_words_replaced_word_gives_del_then_ins` (tokenisation de `similar`), corriger l'attendu du test d'après la sortie réelle, en gardant : un `Keep` initial, puis un `Del` et un `Ins`.

`forms.rs` :

```rust
//! Formes de surface de l'agent : arguments des outils `ask_*` / `show_*`
//! et leur composition en messages A2UI (catalogue v2).
//!
//! Actions renvoyées par l'interface : `choose {choice}`, `confirm
//! {confirmed}`, `permission {index}`, `rewrite {kept, text}`, `param
//! {field, value}`, `variant {index}`, `dismiss`, `answer {text}` (réponse
//! dictée à une question en attente).

use oku_slides_a2ui::{Action, ChoiceOption, Component, ComponentKind, Dynamic, ServerMessage, TextVariant};
use oku_slides_core::Op;
use oku_slides_i18n::Localizer;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Où l'interface place une surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// Fiche ancrée sous un élément de la slide.
    Sheet,
    /// Dans le fil de l'agent (panneau de droite).
    Thread,
    /// Tiroir sous la scène (variantes).
    Drawer,
    /// Post-it du panneau de droite (paramètres).
    Postit,
    /// Fenêtre modale (confirmation, permission).
    Modal,
}

/// Une surface prête à afficher : ses messages A2UI et sa place.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Surface {
    pub id: String,
    pub place: Place,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    pub messages: Vec<ServerMessage>,
}

/// L'élément sous lequel afficher une fiche.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Anchor {
    pub slide: String,
    /// Identifiant d'élément, ou champ de la slide : `@title`, `@subtitle`, `@kicker`, `@text`.
    pub element: String,
}

/// `ask_choice` : une question et ses options ; l'outil renvoie l'option choisie.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct AskChoice {
    /// La question, courte, dans la langue de l'utilisateur.
    pub question: String,
    /// 2 à 6 options.
    pub options: Vec<ChoiceOption>,
    /// Élément concerné : la question s'affiche dessous ; sinon dans le fil.
    #[serde(default)]
    pub anchor: Option<Anchor>,
}

/// `ask_confirm` : oui ou non.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct AskConfirm {
    pub message: String,
    /// Action destructive : le bouton le dit.
    #[serde(default)]
    pub danger: bool,
}

/// `ask_rewrite` : 2 ou 3 reformulations d'un texte ; celle retenue est appliquée.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct AskRewrite {
    pub slide: String,
    /// Élément texte (ou carte), ou `@title`, `@subtitle`, `@kicker`, `@text`.
    pub element: String,
    /// 2 ou 3 propositions ; une ligne par paragraphe, `- ` pour une puce.
    pub proposals: Vec<String>,
}

/// Nature d'un champ de `show_params`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Stepper,
    Slider,
    Color,
    Select,
    Text,
}

/// Un champ de `show_params` et l'op qu'il applique.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct ParamField {
    pub label: String,
    pub kind: FieldKind,
    /// Valeur actuelle.
    pub value: Value,
    /// Une op du deck où la chaîne `"$value"` sera remplacée par la valeur choisie.
    pub op_template: Value,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    /// Options d'un champ `select`.
    #[serde(default)]
    pub options: Vec<ChoiceOption>,
}

/// `show_params` : un post-it de réglages qui s'appliquent tout de suite.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct ShowParams {
    pub title: String,
    /// Slide réglée : le post-it disparaît si elle est supprimée.
    pub slide: String,
    /// 1 à 8 champs.
    pub fields: Vec<ParamField>,
}

/// Une variante : un libellé et les ops qui la produisent.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct Variant {
    pub label: String,
    pub ops: Vec<Op>,
}

/// `show_variants` : 2 à 4 variantes d'une slide, rendues ; la choisie est appliquée.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct ShowVariants {
    pub slide: String,
    pub variants: Vec<Variant>,
}

/// Bornes du nombre d'options, de propositions, de variantes et de champs.
pub const CHOICES: (usize, usize) = (2, 6);
pub const PROPOSALS: (usize, usize) = (2, 3);
pub const VARIANTS: (usize, usize) = (2, 4);
pub const FIELDS: (usize, usize) = (1, 8);

/// Surface : création, composants, modèle de données.
pub(crate) fn surface(id: &str, place: Place, anchor: Option<Anchor>, components: Vec<Component>, data: Value) -> Surface {
    Surface {
        id: id.to_owned(),
        place,
        anchor,
        messages: vec![
            ServerMessage::create_surface(id),
            ServerMessage::update_components(id, components),
            ServerMessage::set_data_model(id, data),
        ],
    }
}

pub(crate) fn lit(v: Value) -> Dynamic<Value> {
    Dynamic::Literal(v)
}

/// `ask_choice` : pastilles ; fiche si ancrée, sinon dans le fil.
pub fn choice(id: &str, spec: &AskChoice, l: &Localizer) -> Surface {
    let components = vec![
        Component::card("root", "body"),
        Component::column("body", &["question", "options", "cancel"]),
        Component::text("question", spec.question.clone(), Some(TextVariant::H4)),
        Component::new(
            "options",
            ComponentKind::ChoiceChips {
                options: spec.options.clone(),
                value: Dynamic::path("/choice"),
                action: Some(Action::event("choose", [("choice", Dynamic::path("/choice"))])),
            },
        ),
        Component::button("cancel", l.tr("agent-surface-cancel"), false, Action::named("dismiss")),
    ];
    let place = if spec.anchor.is_some() { Place::Sheet } else { Place::Thread };
    surface(id, place, spec.anchor.clone(), components, json!({ "choice": "" }))
}

/// `ask_confirm` : fenêtre modale, confirmer ou annuler.
pub fn confirm(id: &str, spec: &AskConfirm, l: &Localizer) -> Surface {
    let yes = if spec.danger { "agent-surface-confirm-danger" } else { "agent-surface-confirm" };
    let answer = |v: bool| Action::event("confirm", [("confirmed", lit(json!(v)))]);
    let components = vec![
        Component::card("root", "body"),
        Component::column("body", &["message", "buttons"]),
        Component::text("message", spec.message.clone(), Some(TextVariant::Body)),
        Component::row("buttons", &["yes", "no"]),
        Component::button("yes", l.tr(yes), true, answer(true)),
        Component::button("no", l.tr("agent-surface-cancel"), false, answer(false)),
    ];
    surface(id, Place::Modal, None, components, json!({}))
}

/// Permission demandée par l'agent (ACP) : un bouton par option (Échap : `dismiss`).
pub fn permission(id: &str, title: &str, options: &[String]) -> Surface {
    let ids: Vec<String> = (0..options.len()).map(|i| format!("option-{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut components = vec![
        Component::card("root", "body"),
        Component::column("body", &["title", "buttons"]),
        Component::text("title", title, Some(TextVariant::H4)),
        Component::row("buttons", &refs),
    ];
    for (i, (bid, name)) in ids.iter().zip(options).enumerate() {
        components.push(Component::button(bid, name.clone(), i == 0, Action::event("permission", [("index", lit(json!(i)))])));
    }
    surface(id, Place::Modal, None, components, json!({}))
}

/// `ask_rewrite` : l'original et les propositions, différences surlignées.
pub(crate) fn rewrite(id: &str, spec: &AskRewrite, original: &str, l: &Localizer) -> Surface {
    let mut body: Vec<String> = vec!["title".into()];
    let mut components = vec![Component::text("title", l.tr("agent-surface-rewrite"), Some(TextVariant::H4))];
    for (i, proposal) in spec.proposals.iter().enumerate() {
        let (diff, pick) = (format!("diff-{i}"), format!("pick-{i}"));
        components.push(Component::new(&diff, ComponentKind::DiffText { segments: super::diff::diff_words(original, proposal) }));
        let label = l.tr_with("agent-surface-pick", &[("n", (i + 1).into())]);
        let action = Action::event("rewrite", [("kept", lit(json!(i))), ("text", lit(json!(proposal)))]);
        components.push(Component::button(&pick, label, i == 0, action));
        body.extend([diff, pick]);
    }
    let keep = Action::event("rewrite", [("kept", lit(json!("original"))), ("text", lit(json!(original)))]);
    components.push(Component::button("keep", l.tr("agent-surface-keep"), false, keep));
    let edited = Action::event("rewrite", [("kept", lit(json!("edited"))), ("text", Dynamic::path("/draft"))]);
    components.push(Component::new(
        "edit",
        ComponentKind::TextField {
            label: Dynamic::Literal(l.tr("agent-surface-edit")),
            value: Dynamic::path("/draft"),
            action: Some(edited),
        },
    ));
    body.extend(["keep".into(), "edit".into()]);
    let refs: Vec<&str> = body.iter().map(String::as_str).collect();
    components.insert(0, Component::column("body", &refs));
    components.insert(0, Component::card("root", "body"));
    let anchor = Anchor { slide: spec.slide.clone(), element: spec.element.clone() };
    let draft = spec.proposals.first().cloned().unwrap_or_default();
    surface(id, Place::Sheet, Some(anchor), components, json!({ "draft": draft }))
}

#[cfg(test)]
#[path = "forms_tests.rs"]
mod forms_tests;
```

`agent/mod.rs` : déclarer `pub mod forms; pub mod diff;` et réexporter `pub use forms::{Anchor, AskChoice, AskConfirm, AskRewrite, FieldKind, ParamField, Place, ShowParams, ShowVariants, Surface, Variant};`. Ajouter :

```rust
use oku_slides_core::{Op, SlideField};
use serde_json::Value;

use crate::error::AgentFormSnafu;
use crate::text::{paragraphs_to_text, text_to_paragraphs};

/// Refuse un nombre d'entrées hors de `min..=max` (`AgentForm`).
pub fn check_len(what: &str, len: usize, (min, max): (usize, usize)) -> Result<()> {
    snafu::ensure!(
        (min..=max).contains(&len),
        AgentFormSnafu { reason: format!("{what}: expected {min} to {max} entries, got {len}") }
    );
    Ok(())
}

/// Champ de slide visé par `@title`, `@subtitle`, `@kicker`, `@text`.
fn slide_field(element: &str) -> Option<SlideField> {
    match element {
        "@title" => Some(SlideField::Title),
        "@subtitle" => Some(SlideField::Subtitle),
        "@kicker" => Some(SlideField::Kicker),
        "@text" => Some(SlideField::Text),
        _ => None,
    }
}

impl Session {
    /// Texte actuel d'un élément ou d'un champ, pour une reformulation.
    fn text_of(&self, slide: &str, element: &str) -> Result<String> {
        let sl = self.doc.sources.deck.slide(slide).context(NoSlideSnafu { id: slide })?;
        let text = match slide_field(element) {
            Some(SlideField::Title) => sl.title.clone(),
            Some(SlideField::Subtitle) => sl.subtitle.clone(),
            Some(SlideField::Kicker) => sl.kicker.clone(),
            Some(_) => sl.text.clone(),
            None => sl.locate(element).and_then(|l| l.element.body.text().map(paragraphs_to_text)),
        };
        text.context(AgentFormSnafu { reason: format!("{slide}/{element}: no text to rewrite") })
    }

    /// Surface `ask_rewrite` : vérifie les propositions et lit le texte actuel.
    pub fn rewrite_surface(&self, id: &str, spec: &AskRewrite) -> Result<Surface> {
        check_len("proposals", spec.proposals.len(), forms::PROPOSALS)?;
        let original = self.text_of(&spec.slide, &spec.element)?;
        Ok(forms::rewrite(id, spec, &original, &self.localizer))
    }

    /// Ops qui remplacent le texte visé par `text`.
    pub fn rewrite_ops(&self, spec: &AskRewrite, text: &str) -> Result<Vec<Op>> {
        self.text_of(&spec.slide, &spec.element)?;
        Ok(vec![match slide_field(&spec.element) {
            Some(field) => Op::SetSlideField { slide: spec.slide.clone(), field, value: Value::String(text.to_owned()) },
            None => Op::SetText { slide: spec.slide.clone(), element: spec.element.clone(), text: text_to_paragraphs(text) },
        }])
    }
}
```

(Import de `NoSlideSnafu` et `OptionExt` déjà présents depuis la Task 4.)

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session -p oku-slides-i18n` ; au premier passage, `cargo insta review` : vérifier que le JSON de `choice_with_anchor_goes_to_sheet` a `place: "sheet"`, `anchor`, trois messages (`createSurface` avec `urn:oku-slides:a2ui:catalog:v2`, `updateComponents`, `updateDataModel {choice: ""}`), puis accepter. Tout passe.
- [ ] **Step 5 : commit**

```bash
git add Cargo.toml Cargo.lock crates/oku-slides-session crates/oku-slides-i18n
git commit -m "feat(session) : formes de surface de l'agent et reformulations (t-103)"
```

---

### Task 6 : Session : variantes et gabarits de paramètres

**Files:**
- Create: `crates/oku-slides-session/src/agent/variants.rs`, `crates/oku-slides-session/src/agent/variants_tests.rs`
- Modify: `crates/oku-slides-session/src/agent/mod.rs`, `crates/oku-slides-session/src/agent/forms.rs`, `crates/oku-slides-session/src/lib.rs` (champ `variant_renderer`)
- Test: `crates/oku-slides-session/tests/agent.rs`

**Interfaces:**
- Consumes: `ShowParams`, `ShowVariants`, `ParamField`, `FieldKind`, `check_len`, `forms::surface` (Task 5).
- Produces: `variants::fill_template(template: &Value, value: &Value) -> Value` ; `Session::param_op(&self, field: &ParamField, value: &Value) -> Result<Op>` ; `Session::params_surface(&self, id: &str, spec: &ShowParams) -> Result<Surface>` ; `Session::variants_surface(&mut self, id: &str, spec: &ShowVariants) -> Result<Surface>`.

- [ ] **Step 1 : tests qui échouent.**

`variants_tests.rs` :

```rust
use serde_json::json;
use speculoos::prelude::*;

use super::fill_template;

#[test]
fn fill_template_replaces_every_value_marker() {
    let t = json!({ "op": "set_grid", "slide": "s-1", "grid": { "columns": "$value", "rows": 6 }, "tags": ["$value"] });
    assert_that!(fill_template(&t, &json!(8)))
        .is_equal_to(json!({ "op": "set_grid", "slide": "s-1", "grid": { "columns": 8, "rows": 6 }, "tags": [8] }));
}

#[test]
fn fill_template_leaves_other_strings() {
    let t = json!({ "op": "set_theme", "theme": "$value-x" });
    assert_that!(fill_template(&t, &json!("pro"))).is_equal_to(t);
}
```

Dans `tests/agent.rs` :

```rust
use oku_slides_session::agent::{FieldKind, ParamField, ShowParams, ShowVariants, Variant};
use oku_slides_core::{Op, Paragraph};

fn theme_field(template: serde_json::Value) -> ParamField {
    ParamField {
        label: "Thème".into(),
        kind: FieldKind::Select,
        value: json!("papier"),
        op_template: template,
        min: None,
        max: None,
        step: None,
        options: vec![],
    }
}

#[test]
fn params_surface_valid_template_goes_to_postit() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams {
        title: "Apparence".into(),
        slide: "s-libre".into(),
        fields: vec![theme_field(json!({ "op": "set_theme", "theme": "$value" }))],
    };
    let surface = s.params_surface("show-1", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Postit);
    assert_that!(serde_json::to_string(&surface).unwrap().contains("\"f0\":\"papier\"")).is_true();
}

#[test]
fn params_surface_template_that_is_not_an_op_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams {
        title: "x".into(),
        slide: "s-libre".into(),
        fields: vec![theme_field(json!({ "op": "no_such_op", "v": "$value" }))],
    };
    let err = s.params_surface("show-2", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("fields[0]")).is_true();
}

#[test]
fn params_surface_template_on_missing_element_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let t = json!({ "op": "set_text", "slide": "s-libre", "element": "absent", "text": [{ "runs": [{ "t": "$value" }] }] });
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![theme_field(t)] };
    assert_that!(s.params_surface("show-3", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn param_op_fills_template_with_chosen_value() {
    let s = session();
    let field = theme_field(json!({ "op": "set_theme", "theme": "$value" }));
    assert_that!(s.param_op(&field, &json!("pro")).unwrap()).is_equal_to(Op::SetTheme { theme: Some("pro".into()) });
}

fn retitle(title: &str) -> Vec<Op> {
    vec![Op::SetText { slide: "s-libre".into(), element: "e-texte".into(), text: vec![Paragraph::plain(title)] }]
}

#[test]
fn variants_surface_renders_each_variant_without_touching_deck() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let before = s.deck().clone();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![Variant { label: "Un".into(), ops: retitle("Un") }, Variant { label: "Deux".into(), ops: retitle("Deux") }],
    };
    let surface = s.variants_surface("show-4", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Drawer);
    let json = serde_json::to_string(&surface).unwrap();
    assert_that!(json.matches("\"SlideThumb\"").count()).is_equal_to(2);
    assert_that!(json.contains("<svg")).is_true();
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn variants_surface_with_invalid_ops_fails_whole_call() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let bad = vec![Op::SetText { slide: "s-libre".into(), element: "absent".into(), text: vec![] }];
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![Variant { label: "Bonne".into(), ops: retitle("Ok") }, Variant { label: "Mauvaise".into(), ops: bad }],
    };
    let err = s.variants_surface("show-5", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("variants[1]")).is_true();
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-session` → échec de compilation.

- [ ] **Step 3 : implémentation.**

`lib.rs` : champ de `Session` `variant_renderer: Option<Renderer>` (doc : « Rendu des variantes proposées par l'agent, séparé du deck. »), initialisé à `None` dans `new` ; remis à `None` là où `template_renderer` l'est (changement de polices dans `load`).

`forms.rs`, ajouter :

```rust
/// `show_params` : post-it ; chaque champ `f<i>` envoie `param {field, value}`.
pub(crate) fn params(id: &str, spec: &ShowParams, palette: &[String], l: &Localizer) -> Surface {
    let mut body: Vec<String> = vec!["title".into()];
    let mut components = vec![Component::text("title", spec.title.clone(), Some(TextVariant::H4))];
    let mut data = serde_json::Map::new();
    for (i, f) in spec.fields.iter().enumerate() {
        let key = format!("f{i}");
        let path = format!("/{key}");
        let action = Some(Action::event("param", [("field", lit(json!(i))), ("value", Dynamic::path(&path))]));
        let label = Dynamic::Literal(f.label.clone());
        let kind = match f.kind {
            FieldKind::Stepper => ComponentKind::Stepper {
                label,
                value: Dynamic::path(&path),
                min: f.min.map(|v| v.round() as i64),
                max: f.max.map(|v| v.round() as i64),
                action,
            },
            FieldKind::Slider => ComponentKind::Slider {
                label,
                value: Dynamic::path(&path),
                min: f.min.unwrap_or(0.0),
                max: f.max.unwrap_or(100.0),
                step: f.step,
                action,
            },
            FieldKind::Color => ComponentKind::ColorField { label, value: Dynamic::path(&path), palette: palette.to_vec(), action },
            FieldKind::Select => ComponentKind::Select { label, value: Dynamic::path(&path), options: f.options.clone(), action },
            FieldKind::Text => ComponentKind::TextField { label, value: Dynamic::path(&path), action },
        };
        components.push(Component::new(&key, kind));
        data.insert(key.clone(), f.value.clone());
        body.push(key);
    }
    components.push(Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss")));
    body.push("close".into());
    let refs: Vec<&str> = body.iter().map(String::as_str).collect();
    components.insert(0, Component::column("body", &refs));
    components.insert(0, Component::card("root", "body"));
    surface(id, Place::Postit, None, components, Value::Object(data))
}

/// `show_variants` : tiroir de miniatures ; `variant {index}` au clic.
pub(crate) fn variants(id: &str, spec: &ShowVariants, svgs: &[std::sync::Arc<str>], l: &Localizer) -> Surface {
    let ids: Vec<String> = (0..svgs.len()).map(|i| format!("v{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut components = vec![
        Component::column("root", &["title", "row", "close"]),
        Component::text("title", l.tr("agent-surface-variants"), Some(TextVariant::H4)),
        Component::row("row", &refs),
        Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss")),
    ];
    for (i, (vid, (svg, v))) in ids.iter().zip(svgs.iter().zip(&spec.variants)).enumerate() {
        components.push(Component::new(
            vid,
            ComponentKind::SlideThumb {
                svg: svg.to_string(),
                label: Dynamic::Literal(format!("{} · {}", i + 1, v.label)),
                action: Some(Action::event("variant", [("index", lit(json!(i)))])),
            },
        ));
    }
    surface(id, Place::Drawer, None, components, json!({}))
}
```

`variants.rs` :

```rust
//! Variantes et paramètres proposés par l'agent : les variantes sont
//! rendues sur un clone du deck ; les gabarits `"$value"` des paramètres
//! sont vérifiés à l'appel, jamais quand l'utilisateur tourne le bouton.

use std::sync::Arc;

use oku_slides_core::{Op, apply_all};
use serde_json::Value;
use snafu::{OptionExt, ResultExt};

use super::check_len;
use super::forms::{self, ParamField, ShowParams, ShowVariants, Surface};
use crate::error::{AgentFormSnafu, NoSlideSnafu, RenderSnafu};
use crate::{Result, Session, new_renderer};

/// Remplace chaque chaîne exactement égale à `"$value"` par `value`.
pub fn fill_template(template: &Value, value: &Value) -> Value {
    match template {
        Value::String(s) if s == "$value" => value.clone(),
        Value::Array(items) => Value::Array(items.iter().map(|t| fill_template(t, value)).collect()),
        Value::Object(map) => Value::Object(map.iter().map(|(k, t)| (k.clone(), fill_template(t, value))).collect()),
        other => other.clone(),
    }
}

impl Session {
    /// L'op d'un champ pour `value`.
    pub fn param_op(&self, field: &ParamField, value: &Value) -> Result<Op> {
        serde_json::from_value(fill_template(&field.op_template, value))
            .map_err(|e| AgentFormSnafu { reason: format!("op_template: {e}") }.build().into())
    }

    /// Surface `show_params` ; chaque gabarit est rempli avec sa valeur
    /// actuelle et essayé sur un clone du deck.
    pub fn params_surface(&self, id: &str, spec: &ShowParams) -> Result<Surface> {
        check_len("fields", spec.fields.len(), forms::FIELDS)?;
        self.doc.sources.deck.slide(&spec.slide).context(NoSlideSnafu { id: &spec.slide })?;
        for (i, field) in spec.fields.iter().enumerate() {
            let op = self.param_op(field, &field.value).map_err(|e| {
                AgentFormSnafu { reason: format!("fields[{i}]: {}", e.one_line()) }.build()
            })?;
            let mut deck = self.doc.sources.deck.clone();
            apply_all(&mut deck, vec![op])
                .map_err(|e| AgentFormSnafu { reason: format!("fields[{i}]: {e}") }.build())?;
        }
        let palette: Vec<String> = self.appearance.slide.values().cloned().collect();
        Ok(forms::params(id, spec, &palette, &self.localizer))
    }

    /// Surface `show_variants` : chaque variante appliquée à un clone, rendue,
    /// puis jetée. Une variante invalide fait échouer tout l'appel.
    pub fn variants_surface(&mut self, id: &str, spec: &ShowVariants) -> Result<Surface> {
        check_len("variants", spec.variants.len(), forms::VARIANTS)?;
        self.doc.sources.deck.slide(&spec.slide).context(NoSlideSnafu { id: &spec.slide })?;
        let renderer = match &mut self.variant_renderer {
            Some(r) => r,
            None => self.variant_renderer.insert(new_renderer(self.render_fonts.clone(), &self.localizer)?),
        };
        let mut svgs: Vec<Arc<str>> = Vec::with_capacity(spec.variants.len());
        for (i, variant) in spec.variants.iter().enumerate() {
            let mut deck = self.doc.sources.deck.clone();
            apply_all(&mut deck, variant.ops.clone())
                .map_err(|e| AgentFormSnafu { reason: format!("variants[{i}]: {e}") }.build())?;
            let compiled = renderer.render(&deck, &self.doc.sources.assets).context(RenderSnafu)?;
            let page = compiled
                .layout()
                .page_of(&spec.slide)
                .context(AgentFormSnafu { reason: format!("variants[{i}]: slide {} removed", spec.slide) })?;
            svgs.push(compiled.svg(page - 1).context(RenderSnafu)?.into());
        }
        Ok(forms::variants(id, spec, &svgs, &self.localizer))
    }
}

#[cfg(test)]
#[path = "variants_tests.rs"]
mod variants_tests;
```

`new_renderer` est une fonction privée de `lib.rs` : la passer en `pub(crate)`. `oku_slides_core::apply_all` est public (réexporté ; sinon `oku_slides_core::ops::apply_all`). `e.one_line()` sur `SessionError` existe. `agent/mod.rs` : `pub mod variants;`.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session
git commit -m "feat(session) : variantes rendues sur un clone et gabarits de paramètres (t-104)"
```

---

### Task 7 : Session : contexte de l'agent et tours

**Files:**
- Create: `crates/oku-slides-session/src/agent/context.rs`, `crates/oku-slides-session/src/agent/context_tests.rs`
- Modify: `crates/oku-slides-session/src/agent/mod.rs`, `crates/oku-slides-session/src/lib.rs` (champ `agent_notes`)
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (clé `agent-undo-turn`, 20 locales)
- Test: `crates/oku-slides-session/tests/agent.rs`

**Interfaces:**
- Produces: `AgentUi { pub slide: Option<String>, pub element: Option<String>, pub field: Option<String>, pub editing: bool }` (Serialize, Deserialize, Default) ; `Session::begin_turn(&mut self, prompt: &str)` (libellé « Agent : <40 premiers caractères> ») ; `Session::end_turn(&mut self)` ; `Session::note(&mut self, text: impl Into<String>)` ; `Session::agent_context(&mut self, ui: &AgentUi) -> String` (vide les notes) ; `Session::document_dir(&self) -> Option<PathBuf>`.

- [ ] **Step 1 : tests qui échouent** — `context_tests.rs` :

```rust
use speculoos::prelude::*;

use super::turn_label;

#[test]
fn turn_label_keeps_short_prompt() {
    assert_that!(turn_label("titre plus court")).is_equal_to("titre plus court".to_owned());
}

#[test]
fn turn_label_cuts_long_prompt_at_forty_chars_with_ellipsis() {
    let long = "rends ce titre beaucoup plus percutant et essaie trois mises en page";
    let label = turn_label(long);
    assert_that!(label.chars().count()).is_equal_to(41);
    assert_that!(label.ends_with('…')).is_true();
}
```

Dans `tests/agent.rs` :

```rust
use oku_slides_session::agent::AgentUi;

#[test]
fn agent_context_describes_mode_slide_selection_and_drains_notes() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.note("variant picked: \"image à gauche\" (s-libre)");
    let ui = AgentUi { slide: Some("s-libre".into()), element: Some("e-texte".into()), field: None, editing: false };
    let ctx = s.agent_context(&ui);
    insta::assert_snapshot!(ctx);
    assert_that!(s.agent_context(&ui).contains("since_last_turn")).is_false();
}

#[test]
fn turn_groups_every_apply_into_one_undo() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.begin_turn("titre plus percutant");
    s.apply("a", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    s.apply("b", vec![Op::SetTheme { theme: Some("papier".into()) }]).unwrap();
    s.end_turn();
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Agent : titre plus percutant"));
    s.undo().unwrap();
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn end_turn_after_edit_templates_closes_stashed_group_too() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.begin_turn("modèles");
    s.apply("a", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    s.edit_templates("essentiel").unwrap();
    s.end_turn();
    s.close_templates().unwrap();
    s.apply("Seul", vec![Op::SetTheme { theme: Some("papier".into()) }]).unwrap();
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Seul"));
}

#[test]
fn document_dir_of_unsaved_deck_is_none() {
    assert_that!(session().document_dir()).is_none();
}
```

(L'identifiant de l'ensemble intégré par défaut est `DEFAULT_SET` : utiliser `oku_slides_session::DEFAULT_SET` au lieu de `"essentiel"` si la valeur diffère.)

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-session` → échec de compilation.

- [ ] **Step 3 : implémentation.**

`lib.rs` : champ `agent_notes: Vec<String>` (doc : « Ce que l'utilisateur a fait sans l'agent depuis son dernier tour. »), `Vec::new()` dans `new`.

Fluent `agent-undo-turn` (agent.ftl, 20 locales) : en `Agent: { $text }` ; fr `Agent : { $text }`.

`context.rs` :

```rust
//! Ce que l'agent sait à chaque tour : le bloc `<oku-slides-context>`
//! (mode, langue, thème, slide, sélection, actions faites sans lui), et
//! les tours d'annulation (un tour = une entrée ⌘Z).

use std::path::PathBuf;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::{Mode, Session};

/// Longueur du début de consigne dans le libellé d'annulation.
const LABEL_CHARS: usize = 40;

/// Ce que l'interface montre au moment de la consigne.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentUi {
    #[serde(default)]
    pub slide: Option<String>,
    #[serde(default)]
    pub element: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub editing: bool,
}

/// Début de la consigne, coupé à 40 caractères.
pub(crate) fn turn_label(prompt: &str) -> String {
    let prompt = prompt.trim();
    if prompt.chars().count() <= LABEL_CHARS {
        return prompt.to_owned();
    }
    let mut cut: String = prompt.chars().take(LABEL_CHARS).collect();
    cut.push('…');
    cut
}

impl Session {
    /// Ouvre le groupe d'annulation d'un tour de l'agent.
    pub fn begin_turn(&mut self, prompt: &str) {
        let label = self.localizer.tr_with("agent-undo-turn", &[("text", turn_label(prompt).into())]);
        self.doc.history.begin_group(label);
    }

    /// Ferme le groupe du tour, y compris sur le deck mis de côté par les modèles.
    pub fn end_turn(&mut self) {
        self.doc.history.end_group();
        if let Mode::Templates { stash, .. } = &mut self.mode {
            stash.history.end_group();
        }
    }

    /// Une action faite sans l'agent, qu'il apprendra au prochain tour.
    pub fn note(&mut self, text: impl Into<String>) {
        self.agent_notes.push(text.into());
    }

    /// Dossier du document (le `cwd` de la session ACP) ; `None` s'il n'est pas enregistré.
    pub fn document_dir(&self) -> Option<PathBuf> {
        self.doc.path.as_ref().and_then(|p| p.parent()).map(ToOwned::to_owned)
    }

    /// Bloc de contexte joint à la consigne ; vide les notes.
    pub fn agent_context(&mut self, ui: &AgentUi) -> String {
        let deck = &self.doc.sources.deck;
        let mut out = String::from("<oku-slides-context>\n");
        let mode = match &self.mode {
            Mode::Deck => "deck".to_owned(),
            Mode::Templates { edit, .. } => format!("templates(set={})", edit.set),
        };
        let _ = writeln!(out, "mode: {mode}");
        let _ = writeln!(
            out,
            "locale: {} · theme: {} · template_set: {}",
            self.localizer.locale(),
            deck.theme.as_deref().unwrap_or("none"),
            self.deck_set()
        );
        if let Some((i, slide)) = ui.slide.as_deref().and_then(|id| deck.slides.iter().enumerate().find(|(_, s)| s.id == id)) {
            let kind = serde_json::to_value(slide.kind).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
            let _ = writeln!(
                out,
                "slide: {} ({}/{}) kind={kind} title={:?}",
                slide.id,
                i + 1,
                deck.slides.len(),
                slide.title.as_deref().unwrap_or("")
            );
        }
        if ui.element.is_some() || ui.field.is_some() {
            let _ = writeln!(
                out,
                "selection: element={} field={} editing={}",
                ui.element.as_deref().unwrap_or("-"),
                ui.field.as_deref().unwrap_or("-"),
                ui.editing
            );
        }
        if !self.agent_notes.is_empty() {
            out.push_str("since_last_turn:\n");
            for note in self.agent_notes.drain(..) {
                let _ = writeln!(out, "  - {note}");
            }
        }
        out.push_str("</oku-slides-context>");
        out
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
```

`Mode` et `deck_set` sont privés à `lib.rs` : les rendre `pub(crate)` (enum `Mode` et méthode `deck_set`). `agent/mod.rs` : `pub mod context;` et `pub use context::AgentUi;`.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session -p oku-slides-i18n` ; `cargo insta review` : l'instantané doit contenir `mode: deck`, `locale: fr`, `slide: s-libre (…/8) kind=…`, `selection: element=e-texte field=- editing=false`, `since_last_turn:` et la note ; accepter. Tout passe.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n
git commit -m "feat(session) : contexte de l'agent et tours d'annulation (t-105)"
```

---
### Task 8 : MCP : serveur, jeton et outils de lecture

**Files:**
- Create: `crates/oku-slides-mcp/Cargo.toml`, `crates/oku-slides-mcp/ARCHITECTURE.md`
- Create: `crates/oku-slides-mcp/src/lib.rs`, `src/error.rs`, `src/server.rs`, `src/handler.rs`, `src/tools.rs`, `src/tools_tests.rs`, `src/read_tools.rs`, `src/hub.rs`, `src/settings.rs`
- Create: `crates/oku-slides-mcp/tests/common/mod.rs`, `crates/oku-slides-mcp/tests/server.rs`
- Modify: `Cargo.toml` (workspace : membres déjà en `crates/*` ; dépendances ci-dessous)
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (clé `agent-tools-unavailable`, 20 locales)

**Interfaces:**
- Consumes: `Session::png`, `Session::deck`, `Session::snapshot`, `Session::handle` ; `oku_slides_session::agent::Surface`.
- Produces :
  - `pub type SharedSession = Arc<std::sync::Mutex<Session>>` ;
  - `SettingsPort` (trait, `Send + Sync + Debug`) : `fn read(&self) -> BTreeMap<String, Value>` ; `fn write(&self, key: &str, value: &Value) -> Result<Applied, String>` ; `Applied { Now, NextLaunch }` ;
  - `SurfaceHub` (première moitié) : `SurfaceHub::new(session: SharedSession, localizer: Arc<Localizer>) -> Arc<SurfaceHub>`, `subscribe(&self) -> broadcast::Receiver<McpEvent>`, `emit(&self, McpEvent)`, `next_id(&self, prefix: &str) -> String` ;
  - `McpEvent` (Serialize, `tag = "kind"`) : `Changed`, `Surface { surface: Surface }`, `CloseSurface { id: String }`, `Focus { slide: String, element: Option<String> }` ;
  - `Tools::new(session: SharedSession, hub: Arc<SurfaceHub>, settings: Arc<dyn SettingsPort>, platform: &Platform) -> Tools` ;
  - `ToolServer::start(tools: Tools, cancel: CancellationToken) -> Result<ToolServer, McpError>` (async) ; `ToolServer::url(&self) -> &str` ; `ToolServer::authorization(&self) -> String` (« Bearer <jeton> », à ne jamais journaliser) ;
  - `McpError` (opaque, `Localize` → `agent-tools-unavailable`).
- Outils de cette tâche : `deck_outline`, `slide {id}`, `render_slide {id}`, `templates`, `settings`.

- [ ] **Step 1 : le crate et ses tests qui échouent.**

Workspace `Cargo.toml` (`[workspace.dependencies]`) :

```toml
oku-slides-mcp = { path = "crates/oku-slides-mcp" }
oku-slides-agent = { path = "crates/oku-slides-agent" }
# Agent : MCP 1.x (négocie 2025-11-25 et avant, comme le SDK Claude) et ACP
rust-mcp-sdk = "=1.1.0"
rust-mcp-axum = "=1.0.2"
agent-client-protocol = "=2.2.0"
tokio-util = { version = "0.7", features = ["compat", "rt"] }
async-trait = "0.1"
base64 = "0.22"
getrandom = "0.3"
```

`crates/oku-slides-mcp/Cargo.toml` :

```toml
[package]
name = "oku-slides-mcp"
description = "Outils MCP d'Oku Slides pour l'agent : lecture, modifications, surfaces, réglages."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
publish.workspace = true

[dependencies]
oku-slides-core.workspace = true
oku-slides-session.workspace = true
oku-slides-i18n.workspace = true
oku-slides-platform.workspace = true
oku-slides-chart.workspace = true
rust-mcp-sdk.workspace = true
rust-mcp-axum.workspace = true
tokio = { workspace = true, features = ["net", "sync", "macros", "time", "rt"] }
tokio-util.workspace = true
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
schemars.workspace = true
snafu.workspace = true
tracing.workspace = true
base64.workspace = true
getrandom.workspace = true
hex.workspace = true

[dev-dependencies]
speculoos.workspace = true
insta.workspace = true

[lints]
workspace = true
```

`tests/common/mod.rs` (partagé par les tests d'intégration du crate) :

```rust
//! Serveur de test : une session isolée, des réglages en mémoire, un client MCP.
#![allow(dead_code, reason = "chaque fichier de test n'utilise qu'une partie de l'outillage")]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use oku_slides_mcp::{Applied, McpEvent, SettingsPort, SharedSession, SurfaceHub, ToolServer, Tools};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use rust_mcp_sdk::mcp_client::{ClientHandler, ClientRuntime, client_runtime};
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, Implementation, InitializeRequestParams, ProtocolVersion,
};
use rust_mcp_sdk::{McpClient, RequestOptions, StreamableTransportOptions};
use serde_json::{Map, Value};
use tokio_util::sync::CancellationToken;

pub fn platform() -> Platform {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut settings = Settings::defaults().unwrap();
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    settings.paths.data = Some(std::env::temp_dir().join(format!("oku-slides-mcp-{}-{n}", std::process::id())));
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap()
}

pub fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// Réglages en mémoire ; `write` note ce qu'on lui demande.
#[derive(Debug, Default)]
pub struct MemorySettings {
    pub values: Mutex<BTreeMap<String, Value>>,
}

impl SettingsPort for MemorySettings {
    fn read(&self) -> BTreeMap<String, Value> {
        self.values.lock().map(|v| v.clone()).unwrap_or_default()
    }
    fn write(&self, key: &str, value: &Value) -> Result<Applied, String> {
        self.values.lock().map_err(|e| e.to_string())?.insert(key.to_owned(), value.clone());
        Ok(if key.starts_with("dictation.") { Applied::Now } else { Applied::NextLaunch })
    }
}

pub struct Harness {
    pub server: ToolServer,
    pub hub: Arc<SurfaceHub>,
    pub session: SharedSession,
    pub settings: Arc<MemorySettings>,
    pub cancel: CancellationToken,
}

pub async fn start() -> Harness {
    let platform = platform();
    let mut session = Session::new(Fonts::default(), &platform).unwrap();
    session.open(&fixture_path()).unwrap();
    let session: SharedSession = Arc::new(Mutex::new(session));
    let hub = SurfaceHub::new(session.clone(), platform.localizer.clone());
    let settings = Arc::new(MemorySettings::default());
    let tools = Tools::new(session.clone(), hub.clone(), settings.clone(), &platform);
    let cancel = CancellationToken::new();
    let server = ToolServer::start(tools, cancel.clone()).await.unwrap();
    Harness { server, hub, session, settings, cancel }
}

struct Noop;
#[async_trait]
impl ClientHandler for Noop {}

pub async fn client(url: &str, authorization: Option<String>) -> Result<Arc<ClientRuntime>, String> {
    let mut headers = HashMap::new();
    if let Some(a) = authorization {
        headers.insert("Authorization".to_owned(), a);
    }
    let transport = StreamableTransportOptions {
        mcp_url: url.to_owned(),
        request_options: RequestOptions { custom_headers: Some(headers), ..RequestOptions::default() },
    };
    let details = InitializeRequestParams {
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test".into(),
            version: "0.1.0".into(),
            title: None,
            description: None,
            icons: vec![],
            website_url: None,
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        meta: None,
    };
    let client = client_runtime::with_transport_options(details, transport, Noop, None, None, None);
    client.clone().start().await.map_err(|e| e.to_string())?;
    Ok(client)
}

pub async fn call(client: &ClientRuntime, name: &str, args: Value) -> CallToolResult {
    let args: Map<String, Value> = args.as_object().cloned().unwrap_or_default();
    client.request_tool_call(CallToolRequestParams::new(name).with_arguments(args)).await.unwrap()
}

/// Premier bloc texte d'un résultat, en JSON si possible.
pub fn text_of(r: &CallToolResult) -> String {
    serde_json::to_value(&r.content)
        .ok()
        .and_then(|v| v.get(0).and_then(|b| b.get("text")).and_then(|t| t.as_str().map(str::to_owned)))
        .unwrap_or_default()
}

pub fn json_of(r: &CallToolResult) -> Value {
    serde_json::from_str(&text_of(r)).unwrap_or(Value::Null)
}

/// Prochaine surface poussée par le hub.
pub async fn next_surface(rx: &mut tokio::sync::broadcast::Receiver<McpEvent>) -> oku_slides_session::agent::Surface {
    loop {
        if let McpEvent::Surface { surface } = rx.recv().await.unwrap() {
            return surface;
        }
    }
}
```

`tests/server.rs` :

```rust
mod common;

use common::*;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn without_token_client_is_refused() {
    let h = start().await;
    assert_that!(client(h.server.url(), None).await).is_err();
    assert_that!(client(h.server.url(), Some("Bearer faux".into())).await).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_list_names_every_read_tool() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let names: Vec<String> = c.request_tool_list(None).await.unwrap().tools.into_iter().map(|t| t.name).collect();
    for n in ["deck_outline", "slide", "render_slide", "templates", "settings"] {
        assert_that!(names.contains(&n.to_owned())).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_lists_slides_with_titles() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", serde_json::json!({})).await);
    assert_that!(outline["slides"].as_array().map(Vec::len)).is_equal_to(Some(8));
    assert_that!(outline["mode"].as_str()).is_equal_to(Some("deck"));
}

#[tokio::test(flavor = "multi_thread")]
async fn render_slide_returns_png_image() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "render_slide", serde_json::json!({ "id": "s-libre" })).await;
    let block = serde_json::to_value(&r.content).unwrap();
    assert_that!(block[0]["type"].as_str()).is_equal_to(Some("image"));
    assert_that!(block[0]["mimeType"].as_str()).is_equal_to(Some("image/png"));
}

#[tokio::test(flavor = "multi_thread")]
async fn slide_unknown_id_returns_tool_error() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "slide", serde_json::json!({ "id": "inconnue" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).contains("slide not found")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_returns_port_values() {
    let h = start().await;
    h.settings.values.lock().unwrap().insert("dictation.model".into(), serde_json::json!("turbo"));
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let v = json_of(&call(&c, "settings", serde_json::json!({})).await);
    assert_that!(v["dictation.model"].as_str()).is_equal_to(Some("turbo"));
}
```

`src/tools_tests.rs` :

```rust
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{list, nest_schema};

/// Chaque `$ref` du schéma désigne un chemin qui existe.
fn refs_resolve(root: &Value, node: &Value) -> bool {
    match node {
        Value::Object(map) => map.iter().all(|(k, v)| {
            if k == "$ref" {
                v.as_str().and_then(|r| r.strip_prefix('#')).is_some_and(|p| root.pointer(p).is_some())
            } else {
                refs_resolve(root, v)
            }
        }),
        Value::Array(items) => items.iter().all(|v| refs_resolve(root, v)),
        _ => true,
    }
}

#[test]
fn nest_schema_moves_defs_under_first_property_and_rewrites_refs() {
    let schema = json!({
        "type": "object",
        "properties": { "ops": { "type": "array", "items": { "$ref": "#/$defs/Op" } } },
        "required": ["ops"],
        "$defs": { "Op": { "type": "object", "properties": { "next": { "$ref": "#/$defs/Op" } } } }
    });
    let nested = nest_schema(schema);
    assert_that!(nested.get("$defs")).is_none();
    assert_that!(nested.pointer("/properties/ops/$defs/Op")).is_some();
    assert_that!(refs_resolve(&nested, &nested)).is_true();
}

#[test]
fn every_tool_schema_keeps_only_properties_and_required_at_root_and_resolves() {
    for tool in list() {
        let schema = serde_json::to_value(&tool.input_schema).unwrap();
        assert_that!(refs_resolve(&schema, &schema)).named(&tool.name).is_true();
    }
}

#[test]
fn tool_names_are_unique() {
    let mut names: Vec<String> = list().into_iter().map(|t| t.name).collect();
    let n = names.len();
    names.sort();
    names.dedup();
    assert_that!(names.len()).is_equal_to(n);
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-mcp` → échec de compilation.

- [ ] **Step 3 : implémentation.**

`src/lib.rs` :

```rust
//! Outils MCP d'Oku Slides pour l'agent, servis en HTTP streamable sur
//! `127.0.0.1` avec un jeton par lancement.
//!
//! - [`ToolServer`] : démarre le serveur (port choisi par le système).
//! - [`Tools`] : les outils (lecture, `apply`, surfaces, confirmations).
//! - [`SurfaceHub`] : surfaces ouvertes, réponses attendues, évènements
//!   vers l'interface ([`McpEvent`]).
//! - [`SettingsPort`] : réglages lus et écrits, fournis par l'hôte.
//!
//! Dépend de `oku-slides-session` ; utilisé par `oku-slides-agent`.

mod error;
mod handler;
mod hub;
mod read_tools;
mod server;
mod settings;
mod tools;

use std::sync::{Arc, Mutex};

pub use error::McpError;
pub use hub::{McpEvent, SurfaceHub};
pub use server::ToolServer;
pub use settings::{Applied, SettingsPort};
pub use tools::Tools;

/// La session partagée entre l'interface, les outils et l'agent.
pub type SharedSession = Arc<Mutex<oku_slides_session::Session>>;
```

`src/error.rs` :

```rust
//! Erreurs du serveur d'outils : il n'a pas pu démarrer.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// Le serveur d'outils de l'agent n'a pas démarré.
#[derive(Debug, Snafu)]
pub struct McpError(pub(crate) McpInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum McpInner {
    #[snafu(display("cannot listen on 127.0.0.1"))]
    Bind { source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("no randomness for the token"))]
    Token { source: getrandom::Error, backtrace: Backtrace },
}

impl Localize for McpError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr("agent-tools-unavailable")
    }
}
```

Fluent `agent-tools-unavailable` : en « The agent's tools could not start. » ; fr « Les outils de l'agent n'ont pas pu démarrer. »

`src/settings.rs` :

```rust
//! Réglages que l'agent peut lire et, sur liste blanche, écrire.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

/// Quand un réglage écrit prend effet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Applied {
    Now,
    NextLaunch,
}

/// Implémenter ce trait pour donner à l'agent les réglages de l'app : la
/// dictée (effet immédiat) et `config.toml` (au prochain lancement).
pub trait SettingsPort: Send + Sync + std::fmt::Debug {
    /// Valeurs par clé pointée (`dictation.model`, `history.limit`…).
    fn read(&self) -> BTreeMap<String, Value>;
    /// Écrit une clé déjà autorisée ([`writable`]) ; l'erreur est lue par l'agent.
    fn write(&self, key: &str, value: &Value) -> Result<Applied, String>;
}

/// Clés que l'agent peut écrire, hors `stt.vad.*`.
const WRITABLE: &[&str] =
    &["dictation.model", "dictation.language", "dictation.auto_stop", "dictation.silence", "i18n.locale", "history.limit"];
/// Clés de `[stt.vad]`.
const VAD: &[&str] = &["threshold", "onset_frames", "pause", "min_phrase", "long_pause", "pre_roll", "post_roll", "max_phrase"];

/// La clé est sur la liste blanche de `set_setting`.
pub fn writable(key: &str) -> bool {
    WRITABLE.contains(&key) || key.strip_prefix("stt.vad.").is_some_and(|k| VAD.contains(&k))
}
```

`src/hub.rs` (première moitié ; la Task 10 ajoute les demandes et les actions) :

```rust
//! Surfaces ouvertes par l'agent et évènements vers l'interface.

use std::sync::{Arc, Mutex};

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::Surface;
use serde::Serialize;
use tokio::sync::broadcast;

use crate::SharedSession;

/// Évènements vers l'interface (Tauri `emit`, SSE).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpEvent {
    /// Le deck ou la session a changé : relire l'instantané.
    Changed,
    Surface { surface: Surface },
    CloseSurface { id: String },
    /// Montrer une slide (et un élément).
    Focus { slide: String, element: Option<String> },
}

#[derive(Debug, Default)]
struct HubState {
    next: u64,
}

/// Les surfaces de l'agent : ouvertes, attendues, fermées.
#[derive(Debug)]
pub struct SurfaceHub {
    pub(crate) session: SharedSession,
    pub(crate) localizer: Arc<Localizer>,
    events: broadcast::Sender<McpEvent>,
    state: Mutex<HubState>,
}

impl SurfaceHub {
    pub fn new(session: SharedSession, localizer: Arc<Localizer>) -> Arc<Self> {
        let (events, _) = broadcast::channel(64);
        Arc::new(Self { session, localizer, events, state: Mutex::new(HubState::default()) })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<McpEvent> {
        self.events.subscribe()
    }

    /// Envoie un évènement ; personne à l'écoute n'est pas une erreur.
    pub fn emit(&self, event: McpEvent) {
        let _ = self.events.send(event);
    }

    /// Identifiant de surface neuf : `ask-3`, `show-4`…
    pub fn next_id(&self, prefix: &str) -> String {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        state.next += 1;
        format!("{prefix}-{}", state.next)
    }
}
```

`src/tools.rs` :

```rust
//! Liste des outils, schémas d'entrée et résultats.
//!
//! MCP 1.x ne garde que `properties` et `required` à la racine d'un schéma :
//! [`nest_schema`] déplace les `$defs` sous la première propriété.

use std::sync::{Arc, OnceLock};

use base64::Engine as _;
use oku_slides_i18n::Localizer;
use oku_slides_platform::storage::Storage;
use oku_slides_session::Platform;
use rust_mcp_sdk::schema::{CallToolResult, ImageContent, TextContent, Tool, ToolInputSchema};
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::{SettingsPort, SharedSession, SurfaceHub};

/// Les outils de l'agent, servis par [`crate::ToolServer`].
#[derive(Debug, Clone)]
pub struct Tools {
    pub(crate) session: SharedSession,
    pub(crate) hub: Arc<SurfaceHub>,
    pub(crate) settings: Arc<dyn SettingsPort>,
    pub(crate) storage: Storage,
    pub(crate) localizer: Arc<Localizer>,
}

impl Tools {
    pub fn new(session: SharedSession, hub: Arc<SurfaceHub>, settings: Arc<dyn SettingsPort>, platform: &Platform) -> Self {
        Self { session, hub, settings, storage: platform.storage.clone(), localizer: platform.localizer.clone() }
    }

    /// Travail synchrone sur la session, hors du fil async ; le verrou n'est
    /// tenu que le temps de `f`.
    pub(crate) async fn with_session<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut oku_slides_session::Session) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        let session = self.session.clone();
        tokio::task::spawn_blocking(move || {
            let mut s = session.lock().map_err(|_| "session unavailable".to_owned())?;
            f(&mut s)
        })
        .await
        .map_err(|e| format!("session task failed: {e}"))?
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, JsonSchema)]
pub(crate) struct NoArgs {}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub(crate) struct SlideId {
    /// Identifiant de la slide (voir `deck_outline`).
    pub id: String,
}

/// Arguments d'un outil, ou le résultat d'erreur à renvoyer.
pub(crate) fn parse<T: DeserializeOwned>(args: Value) -> Result<T, CallToolResult> {
    serde_json::from_value(args).map_err(|e| fail(format!("invalid arguments: {e}")))
}

pub(crate) fn text(s: impl Into<String>) -> CallToolResult {
    CallToolResult::text_content(vec![TextContent::new(s.into(), None, None)])
}

pub(crate) fn json_result(v: &impl Serialize) -> CallToolResult {
    match serde_json::to_string(v) {
        Ok(s) => text(s),
        Err(e) => fail(format!("response not serializable: {e}")),
    }
}

pub(crate) fn fail(msg: impl Into<String>) -> CallToolResult {
    let mut r = text(msg);
    r.is_error = Some(true);
    r
}

pub(crate) fn png(bytes: &[u8]) -> CallToolResult {
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    CallToolResult::image_content(vec![ImageContent::new(b64, "image/png".into(), None, None)])
}

/// Déplace les `$defs` de la racine sous la première propriété et réécrit
/// les `$ref` (`#/$defs/X` → `#/properties/<p>/$defs/X`).
pub(crate) fn nest_schema(mut schema: Value) -> Value {
    let Some(defs) = schema.as_object_mut().and_then(|o| o.remove("$defs")) else { return schema };
    let Some(first) = schema.get("properties").and_then(Value::as_object).and_then(|p| p.keys().next().cloned()) else {
        return schema;
    };
    let prefix = format!("#/properties/{first}/$defs/");
    fn rewrite(v: &mut Value, prefix: &str) {
        match v {
            Value::Object(map) => {
                for (k, child) in map.iter_mut() {
                    if k == "$ref" {
                        if let Some(rest) = child.as_str().and_then(|r| r.strip_prefix("#/$defs/")) {
                            *child = Value::String(format!("{prefix}{rest}"));
                        }
                    } else {
                        rewrite(child, prefix);
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|c| rewrite(c, prefix)),
            _ => {}
        }
    }
    let mut defs = defs;
    rewrite(&mut defs, &prefix);
    rewrite(&mut schema, &prefix);
    if let Some(p) = schema.pointer_mut(&format!("/properties/{first}")).and_then(Value::as_object_mut) {
        p.insert("$defs".into(), defs);
    }
    schema
}

/// Schéma d'entrée MCP 1.x : `properties` et `required` seulement.
fn input_schema(schema: Value) -> ToolInputSchema {
    let schema = nest_schema(schema);
    let properties = schema.get("properties").and_then(Value::as_object).map(|p| {
        p.iter().map(|(k, v)| (k.clone(), v.as_object().cloned().unwrap_or_default())).collect()
    });
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    ToolInputSchema::new(required, properties, None)
}

fn tool<T: JsonSchema>(name: &str, description: &str) -> Tool {
    let schema = serde_json::to_value(schemars::schema_for!(T)).unwrap_or_else(|_| json!({}));
    Tool {
        name: name.into(),
        title: None,
        description: Some(description.into()),
        input_schema: input_schema(schema),
        output_schema: None,
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
    }
}

/// Tous les outils, dans l'ordre où l'agent les lit.
pub(crate) fn list() -> Vec<Tool> {
    static LIST: OnceLock<Vec<Tool>> = OnceLock::new();
    LIST.get_or_init(|| {
        vec![
            tool::<NoArgs>("deck_outline", "List the slides of the open deck: id, kind, title, element count; theme, template set, mode."),
            tool::<SlideId>("slide", "Full content of one slide: elements, styles, grid, notes, and their boxes on the page."),
            tool::<SlideId>("render_slide", "PNG image of one slide as rendered now. Look at it after changing the layout."),
            tool::<NoArgs>("templates", "Template sets (built-in and the user's) with their templates."),
            tool::<NoArgs>("settings", "App settings you can read (dictation, language, history)."),
        ]
    })
    .clone()
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;

```

`src/read_tools.rs` :

```rust
//! Outils de lecture : ils ne changent rien.

use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::{Value, json};

use crate::tools::{SlideId, Tools, fail, json_result, parse, png, text};

impl Tools {
    pub(crate) async fn deck_outline(&self) -> CallToolResult {
        let r = self
            .with_session(|s| {
                let snap = s.snapshot();
                let deck = s.deck();
                let slides: Vec<Value> = deck
                    .slides
                    .iter()
                    .map(|sl| {
                        json!({
                            "id": sl.id,
                            "kind": sl.kind,
                            "title": sl.title,
                            "elements": sl.element_ids().count(),
                        })
                    })
                    .collect();
                Ok(json!({
                    "mode": if snap.editing.is_some() { "templates" } else { "deck" },
                    "theme": snap.theme,
                    "themes": snap.themes,
                    "template_set": snap.templates,
                    "dirty": snap.dirty,
                    "slides": slides,
                }))
            })
            .await;
        match r {
            Ok(v) => json_result(&v),
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn slide(&self, args: Value) -> CallToolResult {
        let SlideId { id } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        match self.with_session(move |s| s.handle(Request::Slide { id }).map_err(|e| e.one_line())).await {
            Ok(json) => text(json),
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn render_slide(&self, args: Value) -> CallToolResult {
        let SlideId { id } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        match self.with_session(move |s| s.png(&id).map_err(|e| e.one_line())).await {
            Ok(bytes) => png(&bytes),
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn templates(&self) -> CallToolResult {
        match self.with_session(|s| s.handle(Request::TemplateSets).map_err(|e| e.one_line())).await {
            Ok(json) => text(json),
            Err(e) => fail(e),
        }
    }

    pub(crate) fn settings(&self) -> CallToolResult {
        json_result(&self.settings.read())
    }
}
```

`src/handler.rs` :

```rust
//! Le serveur MCP : liste des outils et appel, journalisé (nom et durée).

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, RpcError, schema_utils::CallToolError,
};
use serde_json::Value;

use crate::tools::{Tools, fail, list};

#[async_trait]
impl ServerHandler for Tools {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult { tools: list(), meta: None, next_cursor: None })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let started = Instant::now();
        let name = params.name.clone();
        let args = Value::Object(params.arguments.unwrap_or_default());
        let result = self.call(&name, args).await;
        tracing::debug!(tool = %name, ms = started.elapsed().as_millis(), error = result.is_error == Some(true), "outil de l'agent");
        Ok(result)
    }
}

impl Tools {
    /// Un appel d'outil ; une erreur revient à l'agent en `isError`.
    pub(crate) async fn call(&self, name: &str, args: Value) -> CallToolResult {
        match name {
            "deck_outline" => self.deck_outline().await,
            "slide" => self.slide(args).await,
            "render_slide" => self.render_slide(args).await,
            "templates" => self.templates().await,
            "settings" => self.settings(),
            _ => fail(format!("unknown tool: {name}")),
        }
    }
}
```

(`CallToolError` : chemin exact `rust_mcp_sdk::schema::schema_utils::CallToolError`, comme dans la sonde vérifiée ; si le compilateur le trouve ailleurs, suivre son message.)

`src/server.rs` :

```rust
//! Serveur HTTP streamable des outils : `127.0.0.1`, port libre, jeton exigé.

use std::sync::Arc;
use std::time::Duration;

use rust_mcp_axum::axum::extract::{Request, State};
use rust_mcp_axum::axum::http::StatusCode;
use rust_mcp_axum::axum::http::header::AUTHORIZATION;
use rust_mcp_axum::axum::middleware::{self, Next};
use rust_mcp_axum::axum::response::{IntoResponse, Response};
use rust_mcp_axum::{McpMountOptions, axum, mcp_routes};
use rust_mcp_sdk::id_generator::{FastIdGenerator, UuidGenerator};
use rust_mcp_sdk::mcp_http::{McpAppState, McpHttpHandler};
use rust_mcp_sdk::mcp_server::ToMcpServerHandler;
use rust_mcp_sdk::schema::{Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools};
use rust_mcp_sdk::session_store::InMemorySessionStore;
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;

use crate::Tools;
use crate::error::{BindSnafu, McpError, TokenSnafu};

/// Le serveur en marche : son adresse et son jeton.
pub struct ToolServer {
    url: String,
    token: String,
}

impl std::fmt::Debug for ToolServer {
    /// Le jeton n'apparaît jamais.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolServer").field("url", &self.url).finish_non_exhaustive()
    }
}

impl ToolServer {
    /// Démarre le serveur ; il s'arrête quand `cancel` est annulé.
    pub async fn start(tools: Tools, cancel: CancellationToken) -> Result<Self, McpError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).context(TokenSnafu)?;
        let token = hex::encode(bytes);
        let state = Arc::new(McpAppState {
            session_store: Arc::new(InMemorySessionStore::new()),
            id_generator: Arc::new(UuidGenerator {}),
            stream_id_gen: Arc::new(FastIdGenerator::new(Some("s_"))),
            server_details: Arc::new(details()),
            handler: tools.to_mcp_server_handler(),
            ping_interval: Duration::from_secs(12),
            transport_options: Default::default(),
            enable_json_response: false,
            event_store: None,
            task_store: None,
            client_task_store: None,
            message_observer: None,
        });
        let expected: Arc<str> = format!("Bearer {token}").into();
        let app = mcp_routes(state, &McpMountOptions::default(), McpHttpHandler::new(None, vec![], None))
            .layer(middleware::from_fn_with_state(expected, require_bearer));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.context(BindSnafu)?;
        let addr = listener.local_addr().context(BindSnafu)?;
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).with_graceful_shutdown(cancel.cancelled_owned()).await {
                tracing::warn!(error = %e, "serveur d'outils arrêté");
            }
        });
        tracing::info!(%addr, "outils de l'agent prêts");
        Ok(Self { url: format!("http://{addr}/mcp"), token })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Valeur de l'en-tête `Authorization` ; ne jamais la journaliser.
    pub fn authorization(&self) -> String {
        format!("Bearer {}", self.token)
    }
}

async fn require_bearer(State(expected): State<Arc<str>>, req: Request, next: Next) -> Response {
    let ok = req.headers().get(AUTHORIZATION).and_then(|v| v.to_str().ok()) == Some(&*expected);
    if !ok {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    next.run(req).await
}

fn details() -> InitializeResult {
    InitializeResult {
        server_info: Implementation {
            name: "oku-slides".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Oku Slides".into()),
            description: None,
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: None,
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    }
}
```

`ARCHITECTURE.md` (moins de 50 lignes) : rôle du crate, les modules ci-dessus, points d'extension (`SettingsPort`, ajout d'un outil = une ligne dans `tools::list` + un bras dans `Tools::call`), dépendances (`oku-slides-session`, `rust-mcp-sdk`/`rust-mcp-axum` 1.x et pourquoi pas 2.0), sécurité (127.0.0.1, jeton, jamais journalisé).

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-mcp -p oku-slides-i18n` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add Cargo.toml Cargo.lock crates/oku-slides-mcp crates/oku-slides-i18n
git commit -m "feat(mcp) : serveur d'outils sur 127.0.0.1 avec jeton, outils de lecture (t-106)"
```

---

### Task 9 : MCP : outils de modification et évènements

**Files:**
- Create: `crates/oku-slides-mcp/src/edit_tools.rs`
- Modify: `crates/oku-slides-mcp/src/tools.rs` (liste), `src/handler.rs` (bras), `src/lib.rs` (`mod edit_tools;`)
- Test: `crates/oku-slides-mcp/tests/edit.rs`

**Interfaces:**
- Consumes: `Session::handle(Request::{Apply, ChartKind, EditTemplates, CloseTemplates})`, `Session::begin_turn/end_turn` (Task 7), `McpEvent`.
- Produces: outils `apply {ops, label?}`, `chart_kind {slide, element, kind}`, `edit_templates {set}`, `close_templates`, `focus {slide, element?}`. Après tout changement : `McpEvent::Changed`. `focus` : `McpEvent::Focus`.

- [ ] **Step 1 : tests qui échouent** — `tests/edit.rs` :

```rust
mod common;

use common::*;
use oku_slides_mcp::McpEvent;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn apply_changes_deck_and_emits_changed() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": "pro" }] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(h.session.lock().unwrap().deck().theme.as_deref()).is_equal_to(Some("pro"));
    assert_that!(matches!(rx.recv().await.unwrap(), McpEvent::Changed)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_inside_turn_joins_the_turn_group() {
    let h = start().await;
    h.session.lock().unwrap().begin_turn("plus sobre");
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": "pro" }] })).await;
    call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": "papier" }] })).await;
    let mut s = h.session.lock().unwrap();
    s.end_turn();
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Agent : plus sobre"));
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_invalid_op_returns_error_with_path() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "apply", json!({ "ops": [{ "op": "set_text", "slide": "s-libre", "element": "absent", "text": [] }] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).contains("absent")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_schema_accepts_every_op_kind_in_tool_list() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let tools = c.request_tool_list(None).await.unwrap().tools;
    let apply = tools.into_iter().find(|t| t.name == "apply").unwrap();
    let schema = serde_json::to_string(&apply.input_schema).unwrap();
    for op in ["set_text", "set_chart_option", "add_slide", "set_theme"] {
        assert_that!(schema.contains(op)).named(op).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn focus_emits_focus_event_without_changing_deck() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "focus", json!({ "slide": "s-libre", "element": "e-texte" })).await;
    match rx.recv().await.unwrap() {
        McpEvent::Focus { slide, element } => {
            assert_that!(slide.as_str()).is_equal_to("s-libre");
            assert_that!(element.as_deref()).is_equal_to(Some("e-texte"));
        }
        other => panic!("évènement inattendu : {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_then_close_templates_round_trips_mode() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let set = oku_slides_session::DEFAULT_SET;
    call(&c, "edit_templates", json!({ "set": set })).await;
    assert_that!(h.session.lock().unwrap().snapshot().editing.is_some()).is_true();
    call(&c, "close_templates", json!({})).await;
    assert_that!(h.session.lock().unwrap().snapshot().editing.is_none()).is_true();
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-mcp --test edit` → échec (outil inconnu).

- [ ] **Step 3 : implémentation** — `src/edit_tools.rs` :

```rust
//! Outils qui modifient le deck ; tout s'annule (dans le groupe du tour s'il est ouvert).

use oku_slides_chart::SeriesType;
use oku_slides_core::Op;
use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::McpEvent;
use crate::tools::{Tools, fail, json_result, parse};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ApplyArgs {
    /// Ops du deck, appliquées ensemble (tout ou rien). Voir le schéma de chaque op.
    pub ops: Vec<Op>,
    /// Libellé d'annulation hors d'un tour (dans un tour, celui du tour).
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ChartKindArgs {
    pub slide: String,
    pub element: String,
    /// Nouveau type ; les données sont converties.
    pub kind: SeriesType,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetArgs {
    /// Identifiant d'un ensemble de modèles (voir `templates`).
    pub set: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FocusArgs {
    pub slide: String,
    #[serde(default)]
    pub element: Option<String>,
}

impl Tools {
    /// Une requête qui change la session ; `Changed` si elle aboutit.
    async fn change(&self, req: Request) -> CallToolResult {
        match self.with_session(move |s| s.handle(req).map(|_| ()).map_err(|e| e.one_line())).await {
            Ok(()) => {
                self.hub.emit(McpEvent::Changed);
                json_result(&json!({ "ok": true }))
            }
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn apply(&self, args: Value) -> CallToolResult {
        let ApplyArgs { ops, label } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = label.unwrap_or_else(|| self.localizer.tr("agent-undo-apply"));
        self.change(Request::Apply { label, ops }).await
    }

    pub(crate) async fn chart_kind(&self, args: Value) -> CallToolResult {
        let ChartKindArgs { slide, element, kind } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = self.localizer.tr("agent-undo-apply");
        self.change(Request::ChartKind { label, slide, element, kind }).await
    }

    pub(crate) async fn edit_templates(&self, args: Value) -> CallToolResult {
        match parse::<SetArgs>(args) {
            Ok(SetArgs { set }) => self.change(Request::EditTemplates { set }).await,
            Err(r) => r,
        }
    }

    pub(crate) async fn close_templates(&self) -> CallToolResult {
        self.change(Request::CloseTemplates).await
    }

    pub(crate) fn focus(&self, args: Value) -> CallToolResult {
        match parse::<FocusArgs>(args) {
            Ok(FocusArgs { slide, element }) => {
                self.hub.emit(McpEvent::Focus { slide, element });
                json_result(&json!({ "ok": true }))
            }
            Err(r) => r,
        }
    }
}
```

`SeriesType` doit dériver `JsonSchema` : c'est le cas si `Request::ChartKind` apparaît dans `ops_schema` ; sinon ajouter `schemars::JsonSchema` à ses dérivations dans `oku-slides-chart` (le crate a déjà `schemars` pour `schema.rs`).

`tools.rs::list()` : ajouter, après `settings` :

```rust
            tool::<crate::edit_tools::ApplyArgs>("apply", "Apply deck ops atomically (all or nothing). Every change the user can undo. Prefer one apply with several ops over many calls."),
            tool::<crate::edit_tools::ChartKindArgs>("chart_kind", "Change a chart's type; its data is converted."),
            tool::<crate::edit_tools::SetArgs>("edit_templates", "Switch to editing a template set (the deck is put aside, nothing is lost)."),
            tool::<NoArgs>("close_templates", "Leave template editing and get the deck back (unsaved template changes are dropped)."),
            tool::<crate::edit_tools::FocusArgs>("focus", "Show a slide (and an element) to the user."),
```

`handler.rs::call` : bras `"apply" => self.apply(args).await`, `"chart_kind" => self.chart_kind(args).await`, `"edit_templates" => self.edit_templates(args).await`, `"close_templates" => self.close_templates().await`, `"focus" => self.focus(args)`.

Fluent `agent-undo-apply` (agent.ftl) : en « Agent change » ; fr « Modification de l'agent ».

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-mcp -p oku-slides-i18n` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-mcp crates/oku-slides-chart crates/oku-slides-i18n
git commit -m "feat(mcp) : apply, graphiques, modèles et focus (t-107)"
```

---

### Task 10 : MCP : SurfaceHub et outils de surface

**Files:**
- Modify: `crates/oku-slides-mcp/src/hub.rs`
- Create: `crates/oku-slides-mcp/src/surface_tools.rs`
- Modify: `crates/oku-slides-mcp/src/tools.rs`, `src/handler.rs`, `src/lib.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (`agent-undo-variant`, `agent-undo-param`, `agent-undo-rewrite`)
- Test: `crates/oku-slides-mcp/tests/surfaces.rs`

**Interfaces:**
- Consumes: `forms::{choice, confirm, permission}`, `Session::{rewrite_surface, rewrite_ops, params_surface, param_op, variants_surface, note}`, `agent::check_len`, `forms::CHOICES`.
- Produces :
  - `SurfaceAction { pub surface: String, pub name: String, pub context: Map<String, Value> }` (Deserialize ; ce que l'hôte reçoit de l'interface) ;
  - `Answer { Action { name: String, context: Map<String, Value> }, Cancelled }` ;
  - `SurfaceHub::ask(&self, surface: Surface) -> Answer` (async) ; `show_params(&self, surface, spec: ShowParams)` ; `show_variants(&self, surface, spec: ShowVariants)` ; `act(&self, action: SurfaceAction) -> Result<(), String>` ; `cancel_asks(&self)` ; `has_pending_ask(&self) -> bool` ; `answer_pending(&self, text: &str) -> bool` ; `prune(&self)` ; `confirm(&self, message: String, danger: bool) -> bool` (async) ; `permission(&self, title: &str, options: &[String]) -> Option<usize>` (async).
  - Outils : `ask_choice`, `ask_confirm`, `ask_rewrite`, `show_params`, `show_variants`.
  - Résultats : `{"choice": id}` · `{"confirmed": bool}` · `{"kept": 0|1|2|"original"|"edited", "text": …}` · `{"answer_text": …}` (réponse dictée) · `{"cancelled": true}` · `{"shown": id}`.

- [ ] **Step 1 : tests qui échouent** — `tests/surfaces.rs` :

```rust
mod common;

use common::*;
use oku_slides_mcp::{McpEvent, SurfaceAction};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

fn action(surface: &str, name: &str, context: Value) -> SurfaceAction {
    SurfaceAction { surface: surface.into(), name: name.into(), context: context.as_object().cloned().unwrap_or_else(Map::new) }
}

fn choice_args() -> Value {
    json!({ "question": "Laquelle ?", "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] })
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_choice_blocks_until_user_chooses_while_session_stays_free() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    let surface = next_surface(&mut rx).await;
    // Pendant l'attente, l'interface peut toujours utiliser la session.
    assert_that!(h.session.try_lock().is_ok()).is_true();
    assert_that!(pending.is_finished()).is_false();
    h.hub.act(action(&surface.id, "choose", json!({ "choice": "b" }))).unwrap();
    let r = pending.await.unwrap();
    assert_that!(json_of(&r)).is_equal_to(json!({ "choice": "b" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_choice_dismissed_returns_cancelled_and_closes_surface() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    let surface = next_surface(&mut rx).await;
    h.hub.act(action(&surface.id, "dismiss", json!({}))).unwrap();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "cancelled": true }));
    let closed = loop {
        if let McpEvent::CloseSurface { id } = rx.recv().await.unwrap() {
            break id;
        }
    };
    assert_that!(closed).is_equal_to(surface.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn answer_pending_turns_dictation_into_answer_text() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    next_surface(&mut rx).await;
    assert_that!(h.hub.has_pending_ask()).is_true();
    assert_that!(h.hub.answer_pending("la deuxième")).is_true();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "answer_text": "la deuxième" }));
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_asks_releases_every_waiting_tool() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_confirm", json!({ "message": "Sûr ?" })).await }
    });
    next_surface(&mut rx).await;
    h.hub.cancel_asks();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "cancelled": true }));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_choice_with_one_option_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "ask_choice", json!({ "question": "?", "options": [{ "id": "a", "label": "A" }] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_applies_chosen_proposal() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "element": "e-texte", "proposals": ["Premier", "Second"] });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_rewrite", args).await }
    });
    let surface = next_surface(&mut rx).await;
    h.hub.act(action(&surface.id, "rewrite", json!({ "kept": 1, "text": "Second" }))).unwrap();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "kept": 1, "text": "Second" }));
    let s = h.session.lock().unwrap();
    let text = s.slide("s-libre").unwrap().targets.iter().find(|t| t.id == "e-texte").and_then(|t| t.text.clone());
    assert_that!(text.as_deref()).is_equal_to(Some("Second"));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_variants_returns_at_once_and_pick_applies_with_own_undo_and_note() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let variant = |t: &str| json!({ "label": t, "ops": [{ "op": "set_text", "slide": "s-libre", "element": "e-texte", "text": [{ "runs": [{ "t": t }] }] }] });
    let r = call(&c, "show_variants", json!({ "slide": "s-libre", "variants": [variant("Un"), variant("Deux")] })).await;
    let id = json_of(&r)["shown"].as_str().unwrap().to_owned();
    let surface = next_surface(&mut rx).await;
    assert_that!(surface.id.as_str()).is_equal_to(id.as_str());
    h.hub.act(action(&id, "variant", json!({ "index": 1 }))).unwrap();
    let mut s = h.session.lock().unwrap();
    assert_that!(s.snapshot().undo_label.map(str::to_owned)).is_equal_to(Some("Variante : Deux".to_owned()));
    let ctx = s.agent_context(&Default::default());
    assert_that!(ctx.contains("variant picked: \"Deux\" (s-libre)")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn show_params_change_applies_template_op() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "title": "Apparence", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "select", "value": "papier", "op_template": { "op": "set_theme", "theme": "$value" },
          "options": [{ "id": "papier", "label": "Papier" }, { "id": "pro", "label": "Pro" }] }
    ] });
    let id = json_of(&call(&c, "show_params", args).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    h.hub.act(action(&id, "param", json!({ "field": 0, "value": "pro" }))).unwrap();
    assert_that!(h.session.lock().unwrap().deck().theme.as_deref()).is_equal_to(Some("pro"));
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_closes_params_of_removed_slide() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "title": "x", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "text", "value": "papier", "op_template": { "op": "set_theme", "theme": "$value" } }
    ] });
    let id = json_of(&call(&c, "show_params", args).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    call(&c, "apply", json!({ "ops": [{ "op": "remove_slide", "slide": "s-libre" }] })).await;
    h.hub.prune();
    let closed = loop {
        if let McpEvent::CloseSurface { id } = rx.recv().await.unwrap() {
            break id;
        }
    };
    assert_that!(closed).is_equal_to(id);
    assert_that!(h.hub.act(action(&closed, "dismiss", json!({})))).is_err();
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-mcp --test surfaces` → échec de compilation.

- [ ] **Step 3 : implémentation.**

`hub.rs` : remplacer `HubState` et compléter :

```rust
use std::collections::HashMap;

use oku_slides_session::agent::{ShowParams, ShowVariants, forms};
use serde::Deserialize;
use serde_json::{Map, Value};
use tokio::sync::oneshot;

/// Réponse de l'utilisateur à une surface `ask_*`.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Action { name: String, context: Map<String, Value> },
    Cancelled,
}

/// Action d'une surface, telle que l'interface l'envoie à l'hôte.
#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceAction {
    pub surface: String,
    pub name: String,
    #[serde(default)]
    pub context: Map<String, Value>,
}

#[derive(Debug)]
struct PendingAsk {
    id: String,
    tx: oneshot::Sender<Answer>,
}

/// Surface persistante et ce qu'il faut pour appliquer ses actions.
#[derive(Debug)]
enum Shown {
    Params(ShowParams),
    Variants(ShowVariants),
}

#[derive(Debug, Default)]
struct HubState {
    next: u64,
    /// Dans l'ordre d'ouverture : une réponse dictée va à la dernière.
    asks: Vec<PendingAsk>,
    shows: HashMap<String, Shown>,
}
```

Méthodes ajoutées à `impl SurfaceHub` (le verrou `state` n'est jamais tenu pendant un `.await` ni en même temps que la session) :

```rust
    fn state(&self) -> std::sync::MutexGuard<'_, HubState> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Affiche `surface` et attend la réponse ; `dismiss` vaut `Cancelled`.
    pub async fn ask(&self, surface: Surface) -> Answer {
        let (tx, rx) = oneshot::channel();
        let id = surface.id.clone();
        self.state().asks.push(PendingAsk { id: id.clone(), tx });
        self.emit(McpEvent::Surface { surface });
        let answer = rx.await.unwrap_or(Answer::Cancelled);
        self.emit(McpEvent::CloseSurface { id });
        answer
    }

    /// Post-it de paramètres ; il reste jusqu'à sa fermeture.
    pub fn show_params(&self, surface: Surface, spec: ShowParams) {
        self.state().shows.insert(surface.id.clone(), Shown::Params(spec));
        self.emit(McpEvent::Surface { surface });
    }

    /// Tiroir de variantes ; remplace le précédent.
    pub fn show_variants(&self, surface: Surface, spec: ShowVariants) {
        let old: Vec<String> = {
            let mut state = self.state();
            let old = state.shows.iter().filter(|(_, s)| matches!(s, Shown::Variants(_))).map(|(id, _)| id.clone()).collect::<Vec<_>>();
            for id in &old {
                state.shows.remove(id);
            }
            state.shows.insert(surface.id.clone(), Shown::Variants(spec));
            old
        };
        for id in old {
            self.emit(McpEvent::CloseSurface { id });
        }
        self.emit(McpEvent::Surface { surface });
    }

    pub fn has_pending_ask(&self) -> bool {
        !self.state().asks.is_empty()
    }

    /// Une réponse dictée pour la dernière question en attente.
    pub fn answer_pending(&self, text: &str) -> bool {
        let Some(ask) = self.state().asks.pop() else { return false };
        let mut context = Map::new();
        context.insert("text".into(), Value::String(text.to_owned()));
        ask.tx.send(Answer::Action { name: "answer".into(), context }).is_ok()
    }

    /// Libère toutes les questions en attente (Échap, nouveau tour, nouvelle conversation).
    pub fn cancel_asks(&self) {
        for ask in std::mem::take(&mut self.state().asks) {
            let _ = ask.tx.send(Answer::Cancelled);
        }
    }

    /// Une action de l'interface : réponse à une question, ou changement
    /// d'une surface persistante (appliqué ici, noté pour l'agent).
    pub fn act(&self, action: SurfaceAction) -> Result<(), String> {
        let ask = {
            let mut state = self.state();
            state.asks.iter().position(|a| a.id == action.surface).map(|i| state.asks.remove(i))
        };
        if let Some(ask) = ask {
            let answer = if action.name == "dismiss" {
                Answer::Cancelled
            } else {
                Answer::Action { name: action.name, context: action.context }
            };
            let _ = ask.tx.send(answer);
            return Ok(());
        }
        let shown = self.state().shows.remove(&action.surface).ok_or_else(|| format!("unknown surface {}", action.surface))?;
        let keep = match (&shown, action.name.as_str()) {
            (_, "dismiss") => false,
            (Shown::Params(spec), "param") => {
                self.apply_param(spec, &action.context)?;
                true
            }
            (Shown::Variants(spec), "variant") => {
                self.apply_variant(spec, &action.context)?;
                false
            }
            (_, other) => {
                self.state().shows.insert(action.surface.clone(), shown);
                return Err(format!("unexpected action {other}"));
            }
        };
        if keep {
            self.state().shows.insert(action.surface, shown);
        } else {
            self.emit(McpEvent::CloseSurface { id: action.surface });
        }
        Ok(())
    }

    fn index(context: &Map<String, Value>, key: &str) -> Result<usize, String> {
        context.get(key).and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok()).ok_or_else(|| format!("missing {key}"))
    }

    fn apply_param(&self, spec: &ShowParams, context: &Map<String, Value>) -> Result<(), String> {
        let i = Self::index(context, "field")?;
        let field = spec.fields.get(i).ok_or_else(|| format!("no field {i}"))?;
        let value = context.get("value").cloned().unwrap_or(Value::Null);
        {
            let mut s = self.session.lock().map_err(|_| "session unavailable".to_owned())?;
            let op = s.param_op(field, &value).map_err(|e| e.one_line())?;
            let label = self.localizer.tr_with("agent-undo-param", &[("label", field.label.as_str().into())]);
            s.apply(&label, vec![op]).map_err(|e| e.one_line())?;
            s.note(format!("parameter: {:?} = {value} ({})", field.label, spec.slide));
        }
        self.emit(McpEvent::Changed);
        Ok(())
    }

    fn apply_variant(&self, spec: &ShowVariants, context: &Map<String, Value>) -> Result<(), String> {
        let i = Self::index(context, "index")?;
        let variant = spec.variants.get(i).ok_or_else(|| format!("no variant {i}"))?;
        {
            let mut s = self.session.lock().map_err(|_| "session unavailable".to_owned())?;
            let label = self.localizer.tr_with("agent-undo-variant", &[("label", variant.label.as_str().into())]);
            s.apply(&label, variant.ops.clone()).map_err(|e| e.one_line())?;
            s.note(format!("variant picked: {:?} ({})", variant.label, spec.slide));
        }
        self.emit(McpEvent::Changed);
        Ok(())
    }

    /// Ferme les post-its dont la slide n'existe plus.
    pub fn prune(&self) {
        let gone: Vec<String> = {
            let Ok(s) = self.session.lock() else { return };
            let deck = s.deck();
            let mut state = self.state();
            let gone: Vec<String> = state
                .shows
                .iter()
                .filter(|(_, shown)| {
                    let slide = match shown {
                        Shown::Params(p) => &p.slide,
                        Shown::Variants(v) => &v.slide,
                    };
                    deck.slide(slide).is_none()
                })
                .map(|(id, _)| id.clone())
                .collect();
            for id in &gone {
                state.shows.remove(id);
            }
            gone
        };
        for id in gone {
            self.emit(McpEvent::CloseSurface { id });
        }
    }

    /// Confirmation d'un outil non annulable ; `true` seulement sur « confirmer ».
    pub async fn confirm(&self, message: String, danger: bool) -> bool {
        let id = self.next_id("confirm");
        let surface = forms::confirm(&id, &oku_slides_session::agent::AskConfirm { message, danger }, &self.localizer);
        matches!(
            self.ask(surface).await,
            Answer::Action { name, context } if name == "confirm" && context.get("confirmed") == Some(&Value::Bool(true))
        )
    }

    /// Permission demandée par l'agent (ACP) : l'index de l'option choisie.
    pub async fn permission(&self, title: &str, options: &[String]) -> Option<usize> {
        let id = self.next_id("permission");
        match self.ask(forms::permission(&id, title, options)).await {
            Answer::Action { name, context } if name == "permission" => Self::index(&context, "index").ok(),
            _ => None,
        }
    }
```

Note d'ordre des verrous : `prune` prend la session puis `state` ; aucune autre méthode ne prend `state` puis la session (`act` relâche `state` avant `apply_*`). Garder cet ordre.

`lib.rs` : `pub use hub::{Answer, McpEvent, SurfaceAction, SurfaceHub};` et `mod surface_tools;`.

`src/surface_tools.rs` :

```rust
//! Outils de surface : `ask_*` attend l'utilisateur, `show_*` rend la main tout de suite.

use oku_slides_session::agent::{AskChoice, AskConfirm, AskRewrite, ShowParams, ShowVariants, check_len, forms};
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::{Value, json};

use crate::hub::Answer;
use crate::tools::{Tools, fail, json_result, parse};
use crate::McpEvent;

impl Tools {
    /// Résultat commun : réponse dictée, abandon, ou `f` pour l'action attendue.
    fn answered(answer: Answer, f: impl FnOnce(&str, &serde_json::Map<String, Value>) -> CallToolResult) -> CallToolResult {
        match answer {
            Answer::Cancelled => json_result(&json!({ "cancelled": true })),
            Answer::Action { name, context } if name == "answer" => {
                json_result(&json!({ "answer_text": context.get("text").cloned().unwrap_or(Value::Null) }))
            }
            Answer::Action { name, context } => f(&name, &context),
        }
    }

    pub(crate) async fn ask_choice(&self, args: Value) -> CallToolResult {
        let spec: AskChoice = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Err(e) = check_len("options", spec.options.len(), forms::CHOICES) {
            return fail(e.one_line());
        }
        let surface = forms::choice(&self.hub.next_id("ask"), &spec, &self.localizer);
        Self::answered(self.hub.ask(surface).await, |_, ctx| json_result(&json!({ "choice": ctx.get("choice") })))
    }

    pub(crate) async fn ask_confirm(&self, args: Value) -> CallToolResult {
        let spec: AskConfirm = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let surface = forms::confirm(&self.hub.next_id("ask"), &spec, &self.localizer);
        Self::answered(self.hub.ask(surface).await, |_, ctx| {
            json_result(&json!({ "confirmed": ctx.get("confirmed") == Some(&Value::Bool(true)) }))
        })
    }

    pub(crate) async fn ask_rewrite(&self, args: Value) -> CallToolResult {
        let spec: AskRewrite = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("ask");
        let built = {
            let spec = spec.clone();
            self.with_session(move |s| s.rewrite_surface(&id, &spec).map_err(|e| e.one_line())).await
        };
        let surface = match built {
            Ok(s) => s,
            Err(e) => return fail(e),
        };
        let (kept, text) = match self.hub.ask(surface).await {
            Answer::Action { name, context } if name == "rewrite" => (
                context.get("kept").cloned().unwrap_or(Value::Null),
                context.get("text").and_then(Value::as_str).unwrap_or_default().to_owned(),
            ),
            other => return Self::answered(other, |n, _| fail(format!("unexpected action {n}"))),
        };
        if kept != json!("original") {
            let (spec, t) = (spec.clone(), text.clone());
            let label = self.localizer.tr("agent-undo-rewrite");
            let done = self
                .with_session(move |s| {
                    let ops = s.rewrite_ops(&spec, &t).map_err(|e| e.one_line())?;
                    s.apply(&label, ops).map_err(|e| e.one_line())
                })
                .await;
            if let Err(e) = done {
                return fail(e);
            }
            self.hub.emit(McpEvent::Changed);
        }
        json_result(&json!({ "kept": kept, "text": text }))
    }

    pub(crate) async fn show_params(&self, args: Value) -> CallToolResult {
        let spec: ShowParams = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("show");
        let built = {
            let (spec, id) = (spec.clone(), id.clone());
            self.with_session(move |s| s.params_surface(&id, &spec).map_err(|e| e.one_line())).await
        };
        match built {
            Ok(surface) => {
                self.hub.show_params(surface, spec);
                json_result(&json!({ "shown": id }))
            }
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn show_variants(&self, args: Value) -> CallToolResult {
        let spec: ShowVariants = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("show");
        let built = {
            let (spec, id) = (spec.clone(), id.clone());
            self.with_session(move |s| s.variants_surface(&id, &spec).map_err(|e| e.one_line())).await
        };
        match built {
            Ok(surface) => {
                self.hub.show_variants(surface, spec);
                json_result(&json!({ "shown": id }))
            }
            Err(e) => fail(e),
        }
    }
}
```

`tools.rs::list()` : ajouter

```rust
            tool::<oku_slides_session::agent::AskChoice>("ask_choice", "Ask the user to choose (2-6 options). Blocks until they answer; returns {choice} or {answer_text} if they answered by voice, or {cancelled}. Use when the decision is theirs."),
            tool::<oku_slides_session::agent::AskConfirm>("ask_confirm", "Ask yes/no. Blocks; returns {confirmed} / {answer_text} / {cancelled}."),
            tool::<oku_slides_session::agent::AskRewrite>("ask_rewrite", "Propose 2-3 rewordings of a text element or slide field (@title, @subtitle, @kicker, @text). Blocks; the chosen text is applied by Oku Slides; returns {kept, text}."),
            tool::<oku_slides_session::agent::ShowParams>("show_params", "Show a panel of live settings; each field applies op_template with \"$value\" replaced. Returns at once; later changes are reported in the next context."),
            tool::<oku_slides_session::agent::ShowVariants>("show_variants", "Show 2-4 rendered variants of a slide; the user's pick is applied. Returns at once; the pick is reported in the next context."),
```

`handler.rs::call` : bras `"ask_choice"`, `"ask_confirm"`, `"ask_rewrite"`, `"show_params"`, `"show_variants"` vers les méthodes ci-dessus.

Fluent (agent.ftl, 20 locales) : `agent-undo-variant` en « Variant: { $label } », fr « Variante : { $label } » ; `agent-undo-param` en « Setting: { $label } », fr « Réglage : { $label } » ; `agent-undo-rewrite` en « Rewording », fr « Reformulation ».

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-mcp -p oku-slides-i18n` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-mcp crates/oku-slides-i18n
git commit -m "feat(mcp) : SurfaceHub, questions bloquantes et surfaces persistantes (t-108)"
```

---

### Task 11 : MCP : outils non annulables et réglages

**Files:**
- Create: `crates/oku-slides-mcp/src/guarded_tools.rs`, `crates/oku-slides-mcp/src/paths.rs`, `crates/oku-slides-mcp/src/paths_tests.rs`
- Modify: `crates/oku-slides-mcp/src/tools.rs`, `src/handler.rs`, `src/lib.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl`
- Test: `crates/oku-slides-mcp/tests/guarded.rs`

**Interfaces:**
- Consumes: `SurfaceHub::confirm`, `SettingsPort`, `settings::writable`, `Session::handle(Request::{Save, Open, New, SaveTemplates, DeleteTemplateSet})`.
- Produces: outils `save {path?}`, `open {path}`, `new`, `save_templates {as_new}`, `delete_template_set {set}`, `set_setting {key, value}` ; refus : `isError` « denied by user » ; `paths::checked(path: &str) -> Result<PathBuf, String>`.

- [ ] **Step 1 : tests qui échouent.**

`paths_tests.rs` :

```rust
use speculoos::prelude::*;

use super::checked;

#[test]
fn checked_relative_path_is_refused() {
    assert_that!(checked("deck.oku-slides.pdf")).is_err();
}

#[test]
fn checked_parent_component_is_refused() {
    assert_that!(checked("/Users/x/../y/deck.pdf")).is_err();
}

#[test]
fn checked_absolute_path_drops_current_dir_components() {
    assert_that!(checked("/Users/x/./deck.pdf")).is_ok().is_equal_to(std::path::PathBuf::from("/Users/x/deck.pdf"));
}
```

`tests/guarded.rs` :

```rust
mod common;

use common::*;
use oku_slides_mcp::SurfaceAction;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

fn answer(h: &Harness, surface: &str, confirmed: bool) {
    let mut context = Map::new();
    context.insert("confirmed".into(), Value::Bool(confirmed));
    h.hub.act(SurfaceAction { surface: surface.into(), name: "confirm".into(), context }).unwrap();
}

async fn guarded(h: &Harness, tool: &'static str, args: Value, confirmed: bool) -> rust_mcp_sdk::schema::CallToolResult {
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn(async move { call(&c, tool, args).await });
    let surface = next_surface(&mut rx).await;
    assert_that!(serde_json::to_value(&surface).unwrap()["place"].as_str()).is_equal_to(Some("modal"));
    answer(h, &surface.id, confirmed);
    pending.await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn save_denied_writes_nothing() {
    let h = start().await;
    let path = std::env::temp_dir().join(format!("oku-slides-guarded-{}.oku-slides.pdf", std::process::id()));
    std::fs::remove_file(&path).ok();
    let r = guarded(&h, "save", json!({ "path": path.to_string_lossy() }), false).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).as_str()).is_equal_to("denied by user");
    assert_that!(path.exists()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn save_confirmed_writes_file() {
    let h = start().await;
    let path = std::env::temp_dir().join(format!("oku-slides-guarded-ok-{}.oku-slides.pdf", std::process::id()));
    let r = guarded(&h, "save", json!({ "path": path.to_string_lossy() }), true).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(path.exists()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_with_parent_component_is_refused_without_asking() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "open", json!({ "path": "/tmp/../etc/deck.pdf" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_setting_outside_whitelist_is_refused_without_asking() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "set_setting", json!({ "key": "paths.data", "value": "/tmp" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(h.settings.read().contains_key("paths.data")).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_setting_confirmed_writes_through_port() {
    let h = start().await;
    let r = guarded(&h, "set_setting", json!({ "key": "dictation.language", "value": "fr" }), true).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "applied": "now" }));
    assert_that!(h.settings.read().get("dictation.language").cloned()).is_equal_to(Some(json!("fr")));
}
```

(`h.settings.read()` : importer `oku_slides_mcp::SettingsPort` dans le test pour appeler la méthode du trait.)

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-mcp` → échec.

- [ ] **Step 3 : implémentation.**

`paths.rs` :

```rust
//! Chemins donnés par l'agent : absolus, sans `..`, normalisés.

use std::path::{Component, Path, PathBuf};

/// Le chemin, s'il est absolu et ne remonte pas ; les `.` sont retirés.
pub(crate) fn checked(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if !p.is_absolute() {
        return Err(format!("path must be absolute: {path}"));
    }
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => return Err(format!("path must not contain '..': {path}")),
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod paths_tests;
```

`guarded_tools.rs` :

```rust
//! Outils qui ne s'annulent pas : l'utilisateur confirme d'abord.

use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::McpEvent;
use crate::paths::checked;
use crate::settings::writable;
use crate::tools::{Tools, fail, json_result, parse};

const DENIED: &str = "denied by user";

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SaveArgs {
    /// Chemin absolu du `.oku-slides.pdf` ; absent : le fichier actuel.
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct OpenArgs {
    /// Chemin absolu d'un `.oku-slides.pdf`.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SaveTemplatesArgs {
    /// Enregistrer comme nouvel ensemble.
    #[serde(default)]
    pub as_new: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetSettingArgs {
    /// `dictation.model` (turbo|large|base), `dictation.language` (code ou null), `dictation.auto_stop`, `dictation.silence`, `i18n.locale`, `history.limit`, `stt.vad.<clé>`.
    pub key: String,
    pub value: Value,
}

impl Tools {
    /// Demande à l'utilisateur, puis exécute `req` ; `Changed` si elle aboutit.
    async fn guarded(&self, message: String, danger: bool, req: Request) -> CallToolResult {
        if !self.hub.confirm(message, danger).await {
            return fail(DENIED);
        }
        match self.with_session(move |s| s.handle(req).map(|_| ()).map_err(|e| e.one_line())).await {
            Ok(()) => {
                self.hub.emit(McpEvent::Changed);
                json_result(&json!({ "ok": true }))
            }
            Err(e) => fail(e),
        }
    }

    fn dirty(&self) -> bool {
        self.session.lock().map(|s| s.snapshot().dirty).unwrap_or(false)
    }

    pub(crate) async fn save(&self, args: Value) -> CallToolResult {
        let SaveArgs { path } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let path = match path.as_deref().map(checked).transpose() {
            Ok(p) => p,
            Err(e) => return fail(e),
        };
        let shown = match &path {
            Some(p) => p.display().to_string(),
            None => match self.session.lock().ok().and_then(|s| s.snapshot().path.map(|p| p.display().to_string())) {
                Some(p) => p,
                None => return fail("no file yet: give an absolute path"),
            },
        };
        let exists = path.as_deref().is_some_and(|p| self.storage.exists(p).unwrap_or(false));
        let key = if exists { "agent-confirm-save-over" } else { "agent-confirm-save" };
        let message = self.localizer.tr_with(key, &[("path", shown.into())]);
        self.guarded(message, exists, Request::Save { path }).await
    }

    pub(crate) async fn open(&self, args: Value) -> CallToolResult {
        let OpenArgs { path } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let path = match checked(&path) {
            Ok(p) => p,
            Err(e) => return fail(e),
        };
        let dirty = self.dirty();
        let key = if dirty { "agent-confirm-open-dirty" } else { "agent-confirm-open" };
        let message = self.localizer.tr_with(key, &[("path", path.display().to_string().into())]);
        self.guarded(message, dirty, Request::Open { path }).await
    }

    pub(crate) async fn new_deck(&self) -> CallToolResult {
        let dirty = self.dirty();
        let message = self.localizer.tr(if dirty { "agent-confirm-new-dirty" } else { "agent-confirm-new" });
        self.guarded(message, dirty, Request::New).await
    }

    pub(crate) async fn save_templates(&self, args: Value) -> CallToolResult {
        let SaveTemplatesArgs { as_new } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let name = self.session.lock().ok().and_then(|s| s.snapshot().editing.map(|e| e.name.clone())).unwrap_or_default();
        let message = self.localizer.tr_with("agent-confirm-save-templates", &[("name", name.into())]);
        self.guarded(message, false, Request::SaveTemplates { as_new }).await
    }

    pub(crate) async fn delete_template_set(&self, args: Value) -> CallToolResult {
        let crate::edit_tools::SetArgs { set } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let message = self.localizer.tr_with("agent-confirm-delete-set", &[("name", set.as_str().into())]);
        self.guarded(message, true, Request::DeleteTemplateSet { set }).await
    }

    pub(crate) async fn set_setting(&self, args: Value) -> CallToolResult {
        let SetSettingArgs { key, value } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if !writable(&key) {
            return fail(format!("{key} cannot be changed by the agent"));
        }
        let old = self.settings.read().get(&key).cloned().unwrap_or(Value::Null);
        let message = self.localizer.tr_with(
            "agent-confirm-setting",
            &[("key", key.as_str().into()), ("old", old.to_string().into()), ("new", value.to_string().into())],
        );
        if !self.hub.confirm(message, false).await {
            return fail(DENIED);
        }
        match self.settings.write(&key, &value) {
            Ok(applied) => json_result(&json!({ "applied": applied })),
            Err(e) => fail(e),
        }
    }
}
```

`snapshot().editing` : `Option<&TemplateMode>` dont `name` est public (vérifier ; sinon utiliser le champ exposé par `views::TemplateMode`). `Snapshot.path` est `Option<&Path>`.

`tools.rs::list()` : ajouter

```rust
            tool::<crate::guarded_tools::SaveArgs>("save", "Save the deck as .oku-slides.pdf. The user confirms first."),
            tool::<crate::guarded_tools::OpenArgs>("open", "Open another .oku-slides.pdf (absolute path). The user confirms first."),
            tool::<NoArgs>("new", "Start a new blank deck. The user confirms first."),
            tool::<crate::guarded_tools::SaveTemplatesArgs>("save_templates", "Save the template set being edited. The user confirms first."),
            tool::<crate::edit_tools::SetArgs>("delete_template_set", "Delete one of the user's template sets. The user confirms first."),
            tool::<crate::guarded_tools::SetSettingArgs>("set_setting", "Change an app setting (whitelisted keys only). The user confirms first; returns {applied: now|next_launch}."),
```

`handler.rs::call` : `"save"`, `"open"`, `"new" => self.new_deck().await`, `"save_templates"`, `"delete_template_set"`, `"set_setting"`. `lib.rs` : `mod guarded_tools; mod paths;` ; `pub use settings::writable;` n'est pas nécessaire (usage interne).

Fluent (agent.ftl, 20 locales) :

| Clé | en | fr |
| --- | --- | --- |
| `agent-confirm-save` | `Save the presentation as { $path }?` | `Enregistrer la présentation sous { $path } ?` |
| `agent-confirm-save-over` | `Replace { $path }? The existing file will be overwritten.` | `Remplacer { $path } ? Le fichier existant sera écrasé.` |
| `agent-confirm-open` | `Open { $path }?` | `Ouvrir { $path } ?` |
| `agent-confirm-open-dirty` | `Open { $path }? Unsaved changes will be lost.` | `Ouvrir { $path } ? Les modifications non enregistrées seront perdues.` |
| `agent-confirm-new` | `Start a new presentation?` | `Commencer une nouvelle présentation ?` |
| `agent-confirm-new-dirty` | `Start a new presentation? Unsaved changes will be lost.` | `Commencer une nouvelle présentation ? Les modifications non enregistrées seront perdues.` |
| `agent-confirm-save-templates` | `Save the template set “{ $name }”?` | `Enregistrer l'ensemble de modèles « { $name } » ?` |
| `agent-confirm-delete-set` | `Delete the template set “{ $name }”? This cannot be undone.` | `Supprimer l'ensemble de modèles « { $name } » ? C'est définitif.` |
| `agent-confirm-setting` | `Change { $key }: { $old } → { $new }?` | `Changer { $key } : { $old } → { $new } ?` |

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-mcp -p oku-slides-i18n` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-mcp crates/oku-slides-i18n
git commit -m "feat(mcp) : outils non annulables confirmés par l'utilisateur, réglages sur liste blanche (t-109)"
```

---
### Task 12 : Agent : lancement ACP et faux agent

**Files:**
- Create: `crates/oku-slides-agent/Cargo.toml`, `crates/oku-slides-agent/ARCHITECTURE.md`
- Create: `crates/oku-slides-agent/src/lib.rs`, `src/config.rs`, `src/error.rs`, `src/events.rs`, `src/events_tests.rs`, `src/actor.rs`, `src/connection.rs`
- Create: `crates/oku-slides-agent/src/bin/fake_agent.rs`
- Create: `crates/oku-slides-agent/tests/common/mod.rs`, `crates/oku-slides-agent/tests/launch.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (`agent-not-found`, `agent-no-http`, `agent-stopped`)

**Interfaces:**
- Consumes: `ToolServer`, `SurfaceHub`, `SharedSession` (Tasks 8–10) ; `Session::{begin_turn, end_turn, agent_context, document_dir}`, `AgentUi` (Task 7).
- Produces :
  - `AgentConfig { pub command: String, pub args: Vec<String>, pub mcp_url: String, pub authorization: String, pub fallback_cwd: PathBuf }` (Debug sans `authorization`) ;
  - `AgentHandle::spawn(config, session: SharedSession, hub: Arc<SurfaceHub>, localizer: Arc<Localizer>) -> AgentHandle` (dans un runtime tokio) ; `prompt(&self, text: String, ui: AgentUi)`, `cancel(&self)`, `new_conversation(&self)`, `subscribe(&self) -> broadcast::Receiver<AgentEvent>` ;
  - `AgentEvent` (Serialize, `tag = "kind"`, snake_case) : `Status { state: AgentState }`, `UserText { text }`, `Chunk { text }`, `ToolCall { id, title, status }`, `ToolUpdate { id, title: Option<String>, status: Option<String> }`, `Plan { entries: Vec<PlanItem { content, status }> }`, `TurnEnded { stop: String }`, `Error { message }` (localisé), `Conversation` ; `AgentState { Idle, Starting, Working }` ;
  - `AgentError` (opaque, `Localize`).
  - Binaire de test `oku-slides-fake-agent --script <scénario.json> --log <journal.jsonl>`.
- Cette tâche livre le lancement, `initialize` et le refus sans HTTP ; les tours complets sont en Task 13.

- [ ] **Step 1 : le crate, le faux agent et les tests qui échouent.**

`crates/oku-slides-agent/Cargo.toml` :

```toml
[package]
name = "oku-slides-agent"
description = "Agent vocal : client ACP (agent en sous-processus), tours, et la pile agent + outils MCP."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
publish.workspace = true

[[bin]]
# Faux agent ACP pour les tests et le bout à bout ; jamais livré avec l'app.
name = "oku-slides-fake-agent"
path = "src/bin/fake_agent.rs"

[dependencies]
oku-slides-session.workspace = true
oku-slides-mcp.workspace = true
oku-slides-i18n.workspace = true
oku-slides-platform.workspace = true
oku-slides-stt.workspace = true
agent-client-protocol.workspace = true
rust-mcp-sdk.workspace = true
async-trait.workspace = true
tokio = { workspace = true, features = ["process", "sync", "macros", "rt", "time", "io-util", "io-std"] }
tokio-util.workspace = true
serde.workspace = true
serde_json.workspace = true
snafu.workspace = true
tracing.workspace = true

[dev-dependencies]
speculoos.workspace = true
insta.workspace = true

[lints]
workspace = true
```

`src/bin/fake_agent.rs` (outil de test : `std::fs` y est permis) :

```rust
//! Faux agent ACP : suit un scénario JSON (`--script <fichier>`), note chaque
//! message reçu (`--log <fichier>`, une ligne JSON par message) et peut
//! appeler les outils MCP que le client lui injecte.
//!
//! Scénario : `{ "http": true, "turns": [ { "plan": [{"content","status"}],
//! "tool_calls": [{"id","title"}], "mcp": [{"tool","args"}], "chunks": ["…"],
//! "wait_cancel": false, "crash": false } ] }` ; le n-ième prompt joue le
//! tour n (modulo le nombre de tours).

use std::collections::HashMap;
use std::io::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse, McpCapabilities,
    McpServer, NewSessionRequest, NewSessionResponse, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, PromptRequest,
    PromptResponse, SessionId, SessionNotification, SessionUpdate, StopReason, TextContent, ToolCall, ToolCallStatus,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Stdio};
use async_trait::async_trait;
use rust_mcp_sdk::mcp_client::{ClientHandler, client_runtime};
use rust_mcp_sdk::schema::{CallToolRequestParams, ClientCapabilities, Implementation, InitializeRequestParams, ProtocolVersion};
use rust_mcp_sdk::{McpClient, RequestOptions, StreamableTransportOptions};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Notify;

#[derive(Debug, Deserialize)]
struct Script {
    #[serde(default = "yes")]
    http: bool,
    #[serde(default)]
    turns: Vec<Turn>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Turn {
    #[serde(default)]
    plan: Vec<Step>,
    #[serde(default)]
    tool_calls: Vec<ToolStep>,
    #[serde(default)]
    mcp: Vec<McpStep>,
    #[serde(default)]
    chunks: Vec<String>,
    #[serde(default)]
    wait_cancel: bool,
    #[serde(default)]
    crash: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct Step {
    content: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ToolStep {
    id: String,
    title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct McpStep {
    tool: String,
    #[serde(default)]
    args: Value,
}

struct State {
    script: Script,
    log: Option<Mutex<std::fs::File>>,
    turn: AtomicUsize,
    sessions: AtomicUsize,
    /// URL et en-tête `Authorization` du serveur MCP reçu dans `session/new`.
    mcp: Mutex<Option<(String, String)>>,
    cancel: Notify,
}

impl State {
    fn log(&self, method: &str, params: Value) {
        if let Some(file) = &self.log
            && let Ok(mut f) = file.lock()
        {
            let _ = writeln!(f, "{}", serde_json::json!({ "method": method, "params": params }));
        }
    }
}

struct Noop;
#[async_trait]
impl ClientHandler for Noop {}

/// Appelle un outil MCP et renvoie son résultat en JSON (texte).
async fn call_tool(url: &str, auth: &str, tool: &str, args: &Value) -> String {
    let transport = StreamableTransportOptions {
        mcp_url: url.to_owned(),
        request_options: RequestOptions {
            custom_headers: Some(HashMap::from([("Authorization".to_owned(), auth.to_owned())])),
            ..RequestOptions::default()
        },
    };
    let details = InitializeRequestParams {
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "oku-slides-fake-agent".into(),
            version: "0.1.0".into(),
            title: None,
            description: None,
            icons: vec![],
            website_url: None,
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        meta: None,
    };
    let client = client_runtime::with_transport_options(details, transport, Noop, None, None, None);
    if let Err(e) = client.clone().start().await {
        return format!("mcp start failed: {e}");
    }
    let params = CallToolRequestParams::new(tool).with_arguments(args.as_object().cloned().unwrap_or_default());
    let out = match client.request_tool_call(params).await {
        Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
        Err(e) => format!("mcp call failed: {e}"),
    };
    let _ = client.shut_down().await;
    out
}

fn chunk(session: &SessionId, text: String) -> SessionNotification {
    SessionNotification::new(
        session.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(text)))),
    )
}

async fn play(state: &State, cx: &ConnectionTo<Client>, session: &SessionId, turn: Turn) -> agent_client_protocol::Result<StopReason> {
    if !turn.plan.is_empty() {
        let entries = turn
            .plan
            .iter()
            .map(|s| {
                let status = match s.status.as_str() {
                    "completed" => PlanEntryStatus::Completed,
                    "in_progress" => PlanEntryStatus::InProgress,
                    _ => PlanEntryStatus::Pending,
                };
                PlanEntry::new(s.content.clone(), PlanEntryPriority::Medium, status)
            })
            .collect();
        cx.send_notification(SessionNotification::new(session.clone(), SessionUpdate::Plan(Plan::new(entries))))?;
    }
    for t in &turn.tool_calls {
        let call = ToolCall::new(t.id.clone(), t.title.clone()).status(ToolCallStatus::Completed);
        cx.send_notification(SessionNotification::new(session.clone(), SessionUpdate::ToolCall(call)))?;
    }
    let mcp = state.mcp.lock().ok().and_then(|m| m.clone());
    for step in &turn.mcp {
        let out = match &mcp {
            Some((url, auth)) => call_tool(url, auth, &step.tool, &step.args).await,
            None => "no mcp server".to_owned(),
        };
        cx.send_notification(chunk(session, format!("tool:{} {out}", step.tool)))?;
    }
    for text in &turn.chunks {
        cx.send_notification(chunk(session, text.clone()))?;
    }
    if turn.wait_cancel {
        state.cancel.notified().await;
        return Ok(StopReason::Cancelled);
    }
    Ok(StopReason::EndTurn)
}

#[tokio::main]
async fn main() -> agent_client_protocol::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let script: Script = flag("--script")
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Script { http: true, turns: vec![Turn::default()] });
    let log = flag("--log")
        .and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok())
        .map(Mutex::new);
    let state = Arc::new(State {
        script,
        log,
        turn: AtomicUsize::new(0),
        sessions: AtomicUsize::new(0),
        mcp: Mutex::new(None),
        cancel: Notify::new(),
    });
    let (s1, s2, s3, s4) = (state.clone(), state.clone(), state.clone(), state.clone());
    Agent
        .builder()
        .name("oku-slides-fake-agent")
        .on_receive_request(
            async move |req: InitializeRequest, responder, _cx: ConnectionTo<Client>| {
                s1.log("initialize", serde_json::to_value(&req).unwrap_or_default());
                responder.respond(InitializeResponse::new(req.protocol_version).agent_capabilities(
                    AgentCapabilities::new().mcp_capabilities(McpCapabilities::new().http(s1.script.http)),
                ))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: NewSessionRequest, responder, _cx: ConnectionTo<Client>| {
                s2.log("session/new", serde_json::to_value(&req).unwrap_or_default());
                let http = req.mcp_servers.iter().find_map(|s| match s {
                    McpServer::Http(h) => {
                        let auth = h.headers.iter().find(|x| x.name == "Authorization").map(|x| x.value.clone())?;
                        Some((h.url.clone(), auth))
                    }
                    _ => None,
                });
                if let Ok(mut m) = s2.mcp.lock() {
                    *m = http;
                }
                let n = s2.sessions.fetch_add(1, Ordering::Relaxed) + 1;
                responder.respond(NewSessionResponse::new(format!("sess-{n}")))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: PromptRequest, responder, cx: ConnectionTo<Client>| {
                s3.log("session/prompt", serde_json::to_value(&req).unwrap_or_default());
                let turns = &s3.script.turns;
                let turn = match turns.len() {
                    0 => Turn::default(),
                    n => turns[s3.turn.fetch_add(1, Ordering::Relaxed) % n].clone(),
                };
                if turn.crash && turn.mcp.is_empty() {
                    std::process::exit(3);
                }
                let state = s3.clone();
                cx.spawn({
                    let cx = cx.clone();
                    async move {
                        let stop = play(&state, &cx, &req.session_id, turn.clone()).await?;
                        if turn.crash {
                            std::process::exit(3);
                        }
                        responder.respond(PromptResponse::new(stop))
                    }
                })
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |n: CancelNotification, _cx: ConnectionTo<Client>| {
                s4.log("session/cancel", serde_json::to_value(&n).unwrap_or_default());
                s4.cancel.notify_one();
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await
}
```

(Un tour `crash` sans `mcp` meurt tout de suite ; avec `mcp`, il appelle d'abord les outils puis meurt, pour tester le groupe refermé après des changements.)

`tests/common/mod.rs` :

```rust
//! Pile de test : session isolée, outils MCP réels, faux agent.
#![allow(dead_code, reason = "chaque fichier de test n'utilise qu'une partie de l'outillage")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_agent::{AgentConfig, AgentEvent, AgentHandle};
use oku_slides_mcp::{Applied, SettingsPort, SharedSession, SurfaceHub, ToolServer, Tools};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Default)]
pub struct NoSettings;

impl SettingsPort for NoSettings {
    fn read(&self) -> std::collections::BTreeMap<String, Value> {
        Default::default()
    }
    fn write(&self, _: &str, _: &Value) -> Result<Applied, String> {
        Err("read-only".into())
    }
}

pub struct Harness {
    pub agent: AgentHandle,
    pub hub: Arc<SurfaceHub>,
    pub session: SharedSession,
    pub server: ToolServer,
    pub log: PathBuf,
    pub cancel: CancellationToken,
}

fn unique() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!("{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Démarre la pile avec le faux agent et ce scénario ; `command` remplace le faux agent.
pub async fn start_with(script: Value, command: Option<&str>) -> Harness {
    let dir = std::env::temp_dir().join(format!("oku-slides-agent-{}", unique()));
    std::fs::create_dir_all(&dir).unwrap();
    let (script_path, log) = (dir.join("script.json"), dir.join("log.jsonl"));
    std::fs::write(&script_path, script.to_string()).unwrap();
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(dir.clone());
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap();
    let mut session = Session::new(Fonts::default(), &platform).unwrap();
    session.open(&PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))).unwrap();
    let session: SharedSession = Arc::new(Mutex::new(session));
    let hub = SurfaceHub::new(session.clone(), platform.localizer.clone());
    let cancel = CancellationToken::new();
    let tools = Tools::new(session.clone(), hub.clone(), Arc::new(NoSettings), &platform);
    let server = ToolServer::start(tools, cancel.clone()).await.unwrap();
    let config = AgentConfig {
        command: command.unwrap_or(env!("CARGO_BIN_EXE_oku-slides-fake-agent")).to_owned(),
        args: vec![
            "--script".into(),
            script_path.to_string_lossy().into_owned(),
            "--log".into(),
            log.to_string_lossy().into_owned(),
        ],
        mcp_url: server.url().to_owned(),
        authorization: server.authorization(),
        fallback_cwd: dir,
    };
    let agent = AgentHandle::spawn(config, session.clone(), hub.clone(), platform.localizer.clone());
    Harness { agent, hub, session, server, log, cancel }
}

pub async fn start(script: Value) -> Harness {
    start_with(script, None).await
}

/// Attend le premier évènement qui vérifie `pred` (20 s au plus).
pub async fn until(rx: &mut broadcast::Receiver<AgentEvent>, pred: impl Fn(&AgentEvent) -> bool) -> AgentEvent {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match rx.recv().await {
                Ok(e) if pred(&e) => return e,
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => panic!("évènements fermés"),
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("évènement attendu non reçu"))
}

/// Messages notés par le faux agent pour `method`.
pub fn logged(log: &PathBuf, method: &str) -> Vec<Value> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v["method"] == method)
        .map(|v| v["params"].clone())
        .collect()
}
```

`tests/launch.rs` :

```rust
mod common;

use common::*;
use oku_slides_agent::AgentEvent;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn first_prompt_launches_agent_without_fs_or_terminal() {
    let h = start(json!({ "turns": [{ "chunks": ["bonjour"] }] })).await;
    let mut rx = h.agent.subscribe();
    h.agent.prompt("bonjour".into(), Default::default());
    until(&mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    let init = &logged(&h.log, "initialize")[0];
    let caps = &init["clientCapabilities"];
    assert_that!(caps["fs"]["readTextFile"].as_bool()).is_equal_to(Some(false));
    assert_that!(caps["fs"]["writeTextFile"].as_bool()).is_equal_to(Some(false));
    assert_that!(caps["terminal"].as_bool()).is_equal_to(Some(false));
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_without_http_mcp_is_refused_with_localized_error() {
    let h = start(json!({ "http": false })).await;
    let mut rx = h.agent.subscribe();
    h.agent.prompt("bonjour".into(), Default::default());
    let e = until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await;
    let AgentEvent::Error { message } = e else { unreachable!() };
    assert_that!(message.contains("ne sait pas se connecter")).is_true();
    assert_that!(logged(&h.log, "session/new")).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_command_reports_not_found_with_command() {
    let h = start_with(json!({}), Some("/chemin/sans/agent")).await;
    let mut rx = h.agent.subscribe();
    h.agent.prompt("bonjour".into(), Default::default());
    let AgentEvent::Error { message } = until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await else {
        unreachable!()
    };
    assert_that!(message.contains("/chemin/sans/agent")).is_true();
}
```

`src/events_tests.rs` :

```rust
use agent_client_protocol::schema::v1::{
    ContentBlock, ContentChunk, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, SessionUpdate, TextContent, ToolCall,
    ToolCallStatus,
};
use speculoos::prelude::*;

use super::{AgentEvent, PlanItem, from_update};

#[test]
fn from_update_text_chunk_gives_chunk() {
    let u = SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new("salut"))));
    assert_that!(from_update(u)).is_equal_to(Some(AgentEvent::Chunk { text: "salut".into() }));
}

#[test]
fn from_update_tool_call_keeps_id_title_and_snake_case_status() {
    let u = SessionUpdate::ToolCall(ToolCall::new("t1", "Lire la slide").status(ToolCallStatus::InProgress));
    assert_that!(from_update(u)).is_equal_to(Some(AgentEvent::ToolCall {
        id: "t1".into(),
        title: "Lire la slide".into(),
        status: "in_progress".into(),
    }));
}

#[test]
fn from_update_plan_gives_items() {
    let u = SessionUpdate::Plan(Plan::new(vec![PlanEntry::new("Lire", PlanEntryPriority::High, PlanEntryStatus::Completed)]));
    assert_that!(from_update(u)).is_equal_to(Some(AgentEvent::Plan {
        entries: vec![PlanItem { content: "Lire".into(), status: "completed".into() }],
    }));
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-agent` → échec de compilation.

- [ ] **Step 3 : implémentation.**

`src/lib.rs` :

```rust
//! Agent vocal : Oku Slides lance un agent ACP en sous-processus (stdio),
//! lui injecte ses outils MCP (HTTP local, jeton) et lui relaie les
//! consignes dictées avec le contexte de l'interface.
//!
//! - [`AgentHandle`] : l'agent (lancé au premier prompt, relancé après un arrêt).
//! - [`AgentEvent`] : ce que l'interface affiche dans le fil.
//! - [`Stack`] : outils MCP + agent + réglages, démarrés par l'hôte.
//!
//! L'agent n'a ni fichiers ni terminal : il n'agit que par les outils.

mod actor;
mod config;
mod connection;
mod error;
mod events;

pub use actor::AgentHandle;
pub use config::AgentConfig;
pub use error::AgentError;
pub use events::{AgentEvent, AgentState, PlanItem};
```

`src/config.rs` :

```rust
//! Comment lancer l'agent et où sont ses outils.

use std::path::PathBuf;

/// Lancement de l'agent et accès à ses outils MCP.
#[derive(Clone)]
pub struct AgentConfig {
    pub command: String,
    pub args: Vec<String>,
    pub mcp_url: String,
    /// En-tête `Authorization` des outils ; jamais journalisé.
    pub authorization: String,
    /// Dossier de travail quand le document n'est pas enregistré.
    pub fallback_cwd: PathBuf,
}

impl std::fmt::Debug for AgentConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentConfig")
            .field("command", &self.command)
            .field("args", &self.args)
            .field("mcp_url", &self.mcp_url)
            .finish_non_exhaustive()
    }
}
```

`src/error.rs` :

```rust
//! Erreurs de l'agent : introuvable, sans HTTP, arrêté.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// L'agent n'a pas pu servir la consigne.
#[derive(Debug, Snafu)]
pub struct AgentError(pub(crate) AgentInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum AgentInner {
    #[snafu(display("agent command not found: {command}"))]
    NotFound { command: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("agent could not start: {command}"))]
    Spawn { command: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("the agent cannot reach MCP servers over HTTP"))]
    NoHttp { backtrace: Backtrace },
    #[snafu(display("the agent stopped: {message}"))]
    Stopped { message: String, backtrace: Backtrace },
}

impl AgentError {
    /// L'erreur et ses causes, sur une ligne (journaux).
    pub fn one_line(&self) -> String {
        oku_slides_platform::errors::one_line(self)
    }
}

impl Localize for AgentError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            AgentInner::NotFound { command, .. } | AgentInner::Spawn { command, .. } => {
                l.tr_with("agent-not-found", &[("command", command.as_str().into())])
            }
            AgentInner::NoHttp { .. } => l.tr("agent-no-http"),
            AgentInner::Stopped { .. } => l.tr("agent-stopped"),
        }
    }
}
```

Fluent (agent.ftl, 20 locales) :

| Clé | en | fr |
| --- | --- | --- |
| `agent-not-found` | `Agent not found: { $command }. Setting [agent] command.` | `Agent introuvable : { $command }. Réglage [agent] command.` |
| `agent-no-http` | `This agent cannot connect to Oku Slides' tools.` | `Cet agent ne sait pas se connecter aux outils d'Oku Slides.` |
| `agent-stopped` | `The agent stopped.` | `L'agent s'est arrêté.` |

`src/events.rs` :

```rust
//! Ce que l'agent fait, pour le fil de l'interface.

use agent_client_protocol::schema::v1::{ContentBlock, SessionUpdate};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Idle,
    Starting,
    Working,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanItem {
    pub content: String,
    /// `pending`, `in_progress`, `completed`.
    pub status: String,
}

/// Un évènement du fil (JSON `{"kind": …}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent {
    Status { state: AgentState },
    /// La consigne dite.
    UserText { text: String },
    /// Un morceau de la réponse de l'agent.
    Chunk { text: String },
    ToolCall { id: String, title: String, status: String },
    ToolUpdate { id: String, title: Option<String>, status: Option<String> },
    Plan { entries: Vec<PlanItem> },
    /// Fin du tour : `end_turn`, `cancelled`, `max_tokens`…
    TurnEnded { stop: String },
    /// Message dans la langue de l'utilisateur.
    Error { message: String },
    /// Nouvelle conversation : le fil repart de zéro.
    Conversation,
}

/// Une valeur ACP sérialisée en chaîne (`"in_progress"`, un identifiant…).
pub(crate) fn as_text<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

/// L'évènement d'une mise à jour de session ; `None` pour ce que le fil n'affiche pas.
pub(crate) fn from_update(update: SessionUpdate) -> Option<AgentEvent> {
    match update {
        SessionUpdate::AgentMessageChunk(chunk) => match chunk.content {
            ContentBlock::Text(t) => Some(AgentEvent::Chunk { text: t.text }),
            _ => None,
        },
        SessionUpdate::ToolCall(tc) => {
            Some(AgentEvent::ToolCall { id: as_text(&tc.tool_call_id), title: tc.title, status: as_text(&tc.status) })
        }
        SessionUpdate::ToolCallUpdate(u) => Some(AgentEvent::ToolUpdate {
            id: as_text(&u.tool_call_id),
            title: u.fields.title,
            status: u.fields.status.as_ref().map(as_text),
        }),
        SessionUpdate::Plan(p) => Some(AgentEvent::Plan {
            entries: p.entries.iter().map(|e| PlanItem { content: e.content.clone(), status: as_text(&e.status) }).collect(),
        }),
        _ => None,
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod events_tests;
```

`src/actor.rs` :

```rust
//! L'acteur de l'agent : une tâche tokio qui reçoit les commandes, lance
//! l'agent au premier prompt et le relance après un arrêt.

use std::sync::Arc;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::{SharedSession, SurfaceHub};
use oku_slides_session::agent::AgentUi;
use tokio::sync::{Mutex, broadcast, mpsc};

use crate::{AgentConfig, AgentEvent, AgentState, connection};

#[derive(Debug)]
pub(crate) enum Command {
    Prompt { text: String, ui: AgentUi },
    Cancel,
    NewConversation,
}

/// Ce que l'acteur et la connexion partagent.
#[derive(Debug, Clone)]
pub(crate) struct Ctx {
    pub config: AgentConfig,
    pub session: SharedSession,
    pub hub: Arc<SurfaceHub>,
    pub localizer: Arc<Localizer>,
    pub events: broadcast::Sender<AgentEvent>,
}

impl Ctx {
    pub fn emit(&self, event: AgentEvent) {
        let _ = self.events.send(event);
    }

    /// Ferme le tour d'annulation, quoi qu'il soit arrivé.
    pub fn end_turn(&self) {
        if let Ok(mut s) = self.session.lock() {
            s.end_turn();
        }
    }
}

pub(crate) type Commands = Arc<Mutex<mpsc::UnboundedReceiver<Command>>>;

/// L'agent vu par l'hôte ; chaque méthode rend la main tout de suite.
#[derive(Debug, Clone)]
pub struct AgentHandle {
    tx: mpsc::UnboundedSender<Command>,
    events: broadcast::Sender<AgentEvent>,
}

impl AgentHandle {
    /// Démarre l'acteur dans le runtime tokio courant ; l'agent sera lancé au premier prompt.
    pub fn spawn(config: AgentConfig, session: SharedSession, hub: Arc<SurfaceHub>, localizer: Arc<Localizer>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(256);
        let ctx = Ctx { config, session, hub, localizer, events: events.clone() };
        tokio::spawn(run(ctx, Arc::new(Mutex::new(rx))));
        Self { tx, events }
    }

    pub fn prompt(&self, text: String, ui: AgentUi) {
        let _ = self.tx.send(Command::Prompt { text, ui });
    }

    /// Arrête le tour en cours (`session/cancel`).
    pub fn cancel(&self) {
        let _ = self.tx.send(Command::Cancel);
    }

    /// Nouvelle session ACP au prochain prompt ; le fil repart de zéro.
    pub fn new_conversation(&self) {
        let _ = self.tx.send(Command::NewConversation);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.events.subscribe()
    }
}

async fn run(ctx: Ctx, rx: Commands) {
    loop {
        // Au repos, l'agent n'est pas lancé : on attend une consigne.
        let Some(first) = rx.lock().await.recv().await else { return };
        let first = match first {
            Command::Prompt { .. } => first,
            Command::NewConversation => {
                ctx.emit(AgentEvent::Conversation);
                continue;
            }
            Command::Cancel => continue,
        };
        ctx.emit(AgentEvent::Status { state: AgentState::Starting });
        let outcome = connection::run(&ctx, rx.clone(), first).await;
        ctx.end_turn();
        ctx.hub.cancel_asks();
        if let Err(e) = outcome {
            tracing::warn!(error = %e.one_line(), "agent arrêté");
            ctx.emit(AgentEvent::Error { message: e.localize(&ctx.localizer) });
        } else {
            tracing::info!("agent arrêté");
        }
        ctx.emit(AgentEvent::Status { state: AgentState::Idle });
    }
}
```

`src/connection.rs` (dans cette tâche : lancement, `initialize`, vérification HTTP ; la Task 13 remplace `conversation` par la vraie boucle) :

```rust
//! Une connexion ACP à l'agent : lancement du sous-processus, poignée de
//! main, puis la conversation jusqu'à l'arrêt de l'agent.

use std::process::Stdio;
use std::sync::Mutex as StdMutex;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{ClientCapabilities, FileSystemCapabilities, InitializeRequest, SessionNotification};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use snafu::IntoError;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::actor::{Command, Commands, Ctx};
use crate::error::{AgentError, NoHttpSnafu, NotFoundSnafu, SpawnSnafu, StoppedSnafu};
use crate::events::from_update;

/// Nom du client annoncé à l'agent.
const CLIENT_NAME: &str = "oku-slides";

pub(crate) async fn run(ctx: &Ctx, rx: Commands, first: Command) -> Result<(), AgentError> {
    let command = &ctx.config.command;
    let mut child = tokio::process::Command::new(command)
        .args(&ctx.config.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .map_err(|source| match source.kind() {
            std::io::ErrorKind::NotFound => NotFoundSnafu { command: command.clone() }.into_error(source),
            _ => SpawnSnafu { command: command.clone() }.into_error(source),
        })?;
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        return Err(StoppedSnafu { message: "no stdio".to_owned() }.build().into());
    };
    tracing::info!(command = %command, "agent lancé");
    let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());
    let failure: StdMutex<Option<AgentError>> = StdMutex::new(None);
    let events = ctx.events.clone();
    let result = Client
        .builder()
        .name(CLIENT_NAME)
        .on_receive_notification(
            async move |n: SessionNotification, _cx: ConnectionTo<Agent>| {
                tracing::trace!(?n, "acp");
                if let Some(event) = from_update(n.update) {
                    let _ = events.send(event);
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(transport, async |cx: ConnectionTo<Agent>| conversation(ctx, &cx, rx, first, &failure).await)
        .await;
    if let Some(e) = failure.lock().ok().and_then(|mut f| f.take()) {
        return Err(e);
    }
    result.map_err(|e| StoppedSnafu { message: e.to_string() }.build().into())
}

/// Poignée de main ; `false` (et `failure` rempli) si l'agent ne convient pas.
async fn handshake(cx: &ConnectionTo<Agent>, failure: &StdMutex<Option<AgentError>>) -> agent_client_protocol::Result<bool> {
    let caps = ClientCapabilities::new()
        .fs(FileSystemCapabilities::new().read_text_file(false).write_text_file(false))
        .terminal(false);
    let init = cx.send_request(InitializeRequest::new(ProtocolVersion::V1).client_capabilities(caps)).block_task().await?;
    if !init.agent_capabilities.mcp_capabilities.http {
        if let Ok(mut f) = failure.lock() {
            *f = Some(NoHttpSnafu.build().into());
        }
        return Ok(false);
    }
    Ok(true)
}

async fn conversation(
    ctx: &Ctx,
    cx: &ConnectionTo<Agent>,
    rx: Commands,
    first: Command,
    failure: &StdMutex<Option<AgentError>>,
) -> agent_client_protocol::Result<()> {
    if !handshake(cx, failure).await? {
        return Ok(());
    }
    let _ = (ctx, rx, first);
    Ok(())
}
```

(Si `NoHttpSnafu.build()` renvoie directement `AgentInner`, `.into()` le convertit en `AgentError` grâce à `#[derive(Snafu)]` sur la structure opaque ; idem pour `StoppedSnafu`.)

`ARCHITECTURE.md` (moins de 50 lignes) : rôle, modules, cycle de vie (repos → lancement au premier prompt → conversation → arrêt/relance), sécurité (ni `fs` ni `terminal`, jeton passé en en-tête), faux agent et son scénario, dépendances (`agent-client-protocol` 2.2, `oku-slides-mcp`).

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-agent -p oku-slides-i18n` → les tests de `launch.rs` et `events_tests.rs` passent (le premier attend `TurnEnded` : il passe à la Task 13 ; **le marquer `#[ignore = "tours : Task 13"]` ici et retirer l'`ignore` en Task 13**).
- [ ] **Step 5 : commit**

```bash
git add Cargo.lock crates/oku-slides-agent crates/oku-slides-i18n
git commit -m "feat(agent) : lancement de l'agent ACP, refus sans HTTP, faux agent de test (t-110)"
```

---

### Task 13 : Agent : sessions, tours et annulation

**Files:**
- Modify: `crates/oku-slides-agent/src/connection.rs`
- Create: `crates/oku-slides-agent/src/turn.rs`
- Modify: `crates/oku-slides-agent/src/lib.rs` (`mod turn;`), `tests/launch.rs` (retirer l'`ignore`)
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (`agent-instructions`)
- Test: `crates/oku-slides-agent/tests/turns.rs`

**Interfaces:**
- Consumes: `Session::{begin_turn, end_turn, agent_context, document_dir}`, `SurfaceHub::cancel_asks`.
- Produces: la conversation complète : `session/new` (cwd, `mcpServers = [Http { name: "oku-slides", url, headers: [Authorization] }]`), `session/prompt` = [instructions (premier prompt de la session seulement), contexte, consigne], mises à jour relayées, `session/cancel` sur `cancel()` ou sur une nouvelle commande pendant un tour, nouvelle session sur `new_conversation()`.

- [ ] **Step 1 : tests qui échouent** — `tests/turns.rs` :

```rust
mod common;

use common::*;
use oku_slides_agent::AgentEvent;
use oku_slides_session::agent::AgentUi;
use serde_json::json;
use speculoos::prelude::*;

fn ended(e: &AgentEvent) -> bool {
    matches!(e, AgentEvent::TurnEnded { .. })
}

#[tokio::test(flavor = "multi_thread")]
async fn new_session_injects_mcp_server_with_authorization_header() {
    let h = start(json!({ "turns": [{}] })).await;
    let mut rx = h.agent.subscribe();
    h.agent.prompt("bonjour".into(), AgentUi::default());
    until(&mut rx, ended).await;
    let new = &logged(&h.log, "session/new")[0];
    let server = &new["mcpServers"][0];
    assert_that!(server["url"].as_str()).is_equal_to(Some(h.server.url()));
    assert_that!(server["name"].as_str()).is_equal_to(Some("oku-slides"));
    assert_that!(server["headers"][0]["name"].as_str()).is_equal_to(Some("Authorization"));
    assert_that!(server["headers"][0]["value"].as_str().map(str::to_owned)).is_equal_to(Some(h.server.authorization()));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_sends_instructions_context_then_text_and_streams_chunks() {
    let h = start(json!({ "turns": [{ "chunks": ["Je regarde."] }] })).await;
    let mut rx = h.agent.subscribe();
    let ui = AgentUi { slide: Some("s-libre".into()), ..Default::default() };
    h.agent.prompt("rends le titre plus court".into(), ui);
    let chunk = until(&mut rx, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(chunk).is_equal_to(AgentEvent::Chunk { text: "Je regarde.".into() });
    until(&mut rx, ended).await;
    let blocks = logged(&h.log, "session/prompt")[0]["prompt"].as_array().cloned().unwrap();
    assert_that!(blocks.len()).is_equal_to(3);
    assert_that!(blocks[1]["text"].as_str().unwrap().starts_with("<oku-slides-context>")).is_true();
    assert_that!(blocks[1]["text"].as_str().unwrap().contains("slide: s-libre")).is_true();
    assert_that!(blocks[2]["text"].as_str()).is_equal_to(Some("rends le titre plus court"));
}

#[tokio::test(flavor = "multi_thread")]
async fn second_prompt_of_session_has_no_instructions() {
    let h = start(json!({ "turns": [{}] })).await;
    let mut rx = h.agent.subscribe();
    h.agent.prompt("un".into(), AgentUi::default());
    until(&mut rx, ended).await;
    h.agent.prompt("deux".into(), AgentUi::default());
    until(&mut rx, ended).await;
    let prompts = logged(&h.log, "session/prompt");
    assert_that!(prompts[1]["prompt"].as_array().map(Vec::len)).is_equal_to(Some(2));
    assert_that!(logged(&h.log, "session/new").len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_during_turn_sends_cancel_and_ends_cancelled() {
    let h = start(json!({ "turns": [{ "wait_cancel": true }] })).await;
    let mut rx = h.agent.subscribe();
    h.agent.prompt("long".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    h.agent.cancel();
    let e = until(&mut rx, ended).await;
    assert_that!(e).is_equal_to(AgentEvent::TurnEnded { stop: "cancelled".into() });
    assert_that!(logged(&h.log, "session/cancel")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_opens_a_new_session() {
    let h = start(json!({ "turns": [{}] })).await;
    let mut rx = h.agent.subscribe();
    h.agent.prompt("un".into(), AgentUi::default());
    until(&mut rx, ended).await;
    h.agent.new_conversation();
    until(&mut rx, |e| matches!(e, AgentEvent::Conversation)).await;
    h.agent.prompt("deux".into(), AgentUi::default());
    until(&mut rx, ended).await;
    assert_that!(logged(&h.log, "session/new")).has_length(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_calls_of_a_turn_form_one_undo_entry() {
    let apply = |t: &str| json!({ "tool": "apply", "args": { "ops": [{ "op": "set_theme", "theme": t }] } });
    let h = start(json!({ "turns": [{ "mcp": [apply("pro"), apply("papier")] }] })).await;
    let mut rx = h.agent.subscribe();
    h.agent.prompt("change le thème deux fois".into(), AgentUi::default());
    until(&mut rx, ended).await;
    let mut s = h.session.lock().unwrap();
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Agent : change le thème deux fois"));
    s.undo().unwrap();
    assert_that!(s.snapshot().can_undo).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn crash_mid_turn_reports_stopped_and_closes_group() {
    let apply = json!({ "tool": "apply", "args": { "ops": [{ "op": "set_theme", "theme": "pro" }] } });
    let h = start(json!({ "turns": [{ "mcp": [apply], "crash": true }] })).await;
    let mut rx = h.agent.subscribe();
    h.agent.prompt("puis plante".into(), AgentUi::default());
    let AgentEvent::Error { message } = until(&mut rx, |e| matches!(e, AgentEvent::Error { .. })).await else {
        unreachable!()
    };
    assert_that!(message.as_str()).is_equal_to("L'agent s'est arrêté.");
    let mut s = h.session.lock().unwrap();
    s.apply("Après", vec![oku_slides_core::Op::SetTheme { theme: Some("papier".into()) }]).unwrap();
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Après"));
}
```

Ajouter `oku-slides-core.workspace = true` aux `[dev-dependencies]` de l'agent.

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-agent --test turns` → échec (pas de `session/new`, pas d'évènements).

- [ ] **Step 3 : implémentation.**

Fluent `agent-instructions` (agent.ftl, 20 locales ; texte lu par l'agent, dans la langue de l'utilisateur) — en :

```
agent-instructions = You edit an Oku Slides presentation for the user, who speaks to you. Act only through the oku-slides tools. Read the deck with deck_outline, slide and render_slide before changing it; change it with apply (every change can be undone). When a decision belongs to the user, ask with ask_choice, ask_confirm or ask_rewrite; offer layouts with show_variants and live settings with show_params. Reply briefly, in the user's language. Each message starts with an <oku-slides-context> block describing what the user sees.
```

fr :

```
agent-instructions = Tu modifies une présentation Oku Slides pour l'utilisateur, qui te parle. N'agis que par les outils oku-slides. Lis le deck avec deck_outline, slide et render_slide avant de le changer ; change-le avec apply (tout s'annule). Quand une décision revient à l'utilisateur, demande avec ask_choice, ask_confirm ou ask_rewrite ; propose des mises en page avec show_variants et des réglages en direct avec show_params. Réponds brièvement, dans la langue de l'utilisateur. Chaque message commence par un bloc <oku-slides-context> qui décrit ce que voit l'utilisateur.
```

`src/turn.rs` :

```rust
//! Un tour : groupe d'annulation, contexte, `session/prompt`, et ce qui
//! arrive pendant qu'il tourne (annulation, nouvelle consigne).

use agent_client_protocol::schema::v1::{CancelNotification, ContentBlock, PromptRequest, SessionId, TextContent};
use agent_client_protocol::{Agent, ConnectionTo};
use oku_slides_session::agent::AgentUi;

use crate::actor::{Command, Commands, Ctx};
use crate::events::as_text;
use crate::{AgentEvent, AgentState};

/// Joue un tour ; renvoie la commande reçue pendant le tour (à traiter ensuite).
pub(crate) async fn run(
    ctx: &Ctx,
    cx: &ConnectionTo<Agent>,
    rx: &Commands,
    session: SessionId,
    text: String,
    ui: AgentUi,
    first: bool,
) -> agent_client_protocol::Result<Option<Command>> {
    let context = match ctx.session.lock() {
        Ok(mut s) => {
            s.begin_turn(&text);
            s.agent_context(&ui)
        }
        Err(_) => String::new(),
    };
    ctx.emit(AgentEvent::UserText { text: text.clone() });
    ctx.emit(AgentEvent::Status { state: AgentState::Working });
    let mut blocks = Vec::with_capacity(3);
    if first {
        blocks.push(ContentBlock::Text(TextContent::new(ctx.localizer.tr("agent-instructions"))));
    }
    blocks.push(ContentBlock::Text(TextContent::new(context)));
    blocks.push(ContentBlock::Text(TextContent::new(text)));
    let request = cx.send_request(PromptRequest::new(session.clone(), blocks)).block_task();
    tokio::pin!(request);
    let mut queued = None;
    let mut open = true;
    let response = loop {
        tokio::select! {
            r = &mut request => break r,
            cmd = async { rx.lock().await.recv().await }, if open => {
                // Toute commande pendant un tour l'arrête ; une consigne ou une
                // nouvelle conversation est traitée ensuite.
                ctx.hub.cancel_asks();
                cx.send_notification(CancelNotification::new(session.clone()))?;
                match cmd {
                    None => open = false,
                    Some(Command::Cancel) => {}
                    Some(other) => queued = Some(other),
                }
            }
        }
    };
    ctx.end_turn();
    let stop = as_text(&response?.stop_reason);
    ctx.emit(AgentEvent::TurnEnded { stop });
    ctx.emit(AgentEvent::Status { state: AgentState::Idle });
    Ok(queued)
}
```

`connection.rs` : remplacer `conversation` par :

```rust
/// Le serveur d'outils tel que l'agent doit le joindre.
fn tools_server(ctx: &Ctx) -> McpServer {
    McpServer::Http(
        McpServerHttp::new("oku-slides", ctx.config.mcp_url.clone())
            .headers(vec![HttpHeader::new("Authorization", ctx.config.authorization.clone())]),
    )
}

async fn conversation(
    ctx: &Ctx,
    cx: &ConnectionTo<Agent>,
    rx: Commands,
    first: Command,
    failure: &StdMutex<Option<AgentError>>,
) -> agent_client_protocol::Result<()> {
    if !handshake(cx, failure).await? {
        return Ok(());
    }
    ctx.emit(AgentEvent::Status { state: AgentState::Idle });
    let mut session: Option<SessionId> = None;
    let mut next = Some(first);
    loop {
        let cmd = match next.take() {
            Some(c) => c,
            None => match rx.lock().await.recv().await {
                Some(c) => c,
                None => return Ok(()),
            },
        };
        match cmd {
            Command::Cancel => ctx.hub.cancel_asks(),
            Command::NewConversation => {
                session = None;
                ctx.hub.cancel_asks();
                ctx.emit(AgentEvent::Conversation);
            }
            Command::Prompt { text, ui } => {
                let (id, fresh) = match &session {
                    Some(id) => (id.clone(), false),
                    None => {
                        let cwd = ctx
                            .session
                            .lock()
                            .ok()
                            .and_then(|s| s.document_dir())
                            .unwrap_or_else(|| ctx.config.fallback_cwd.clone());
                        let r = cx
                            .send_request(NewSessionRequest::new(cwd).mcp_servers(vec![tools_server(ctx)]))
                            .block_task()
                            .await?;
                        session = Some(r.session_id.clone());
                        (r.session_id, true)
                    }
                };
                next = crate::turn::run(ctx, cx, &rx, id, text, ui, fresh).await?;
            }
        }
    }
}
```

Imports de `connection.rs` à ajouter : `HttpHeader, McpServer, McpServerHttp, NewSessionRequest, SessionId` depuis `agent_client_protocol::schema::v1`, et `crate::{AgentEvent, AgentState}`. `lib.rs` : `mod turn;`. Retirer `#[ignore]` de `first_prompt_launches_agent_without_fs_or_terminal`.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-agent -p oku-slides-i18n` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-agent crates/oku-slides-i18n
git commit -m "feat(agent) : sessions ACP avec outils injectés, tours regroupés, annulation (t-111)"
```

---

### Task 14 : Agent : permissions, instructions et Stack

**Files:**
- Modify: `crates/oku-slides-agent/src/connection.rs` (permissions)
- Create: `crates/oku-slides-agent/src/stack.rs`, `crates/oku-slides-agent/src/settings_port.rs`, `crates/oku-slides-agent/src/settings_port_tests.rs`
- Create: `crates/oku-slides-agent/examples/smoke.rs`
- Modify: `crates/oku-slides-agent/src/lib.rs`, `justfile` (`agent-smoke`)
- Test: `crates/oku-slides-agent/tests/stack.rs`

**Interfaces:**
- Consumes: `SurfaceHub::{permission, answer_pending, has_pending_ask, cancel_asks, act, prune, subscribe}`, `config_write::write_value`, `Settings::config_file`, `oku_slides_stt::{Stt, SttRequest, Config}`.
- Produces :
  - `Stack::start(session: SharedSession, platform: &Platform, stt: Option<Stt>) -> Result<Stack, McpError>` (async) ;
  - `Stack::prompt(&self, text: String, ui: AgentUi)` (réponse dictée si une question attend, sinon nouveau tour) ; `cancel(&self)` ; `new_conversation(&self)` ; `surface_action(&self, SurfaceAction) -> Result<(), String>` ; `document_changed(&self)` (nouvelle conversation, surfaces fermées) ; `after_change(&self)` (post-its orphelins) ; `forward(&self, sink: impl Fn(&'static str, String) + Send + Sync + 'static)` (évènements `"mcp"` / `"agent"` en JSON, pour Tauri et le SSE) ; `Drop` arrête le serveur d'outils ;
  - `AppSettings` : `SettingsPort` de l'app (dictée en direct, `config.toml`).
- Permissions ACP : `session/request_permission` → `hub.permission(titre, noms des options)` dans une tâche détachée (`cx.spawn`), réponse `Selected(option_id)` ou `Cancelled`.

- [ ] **Step 1 : tests qui échouent.**

`settings_port_tests.rs` :

```rust
use std::sync::Arc;

use oku_slides_mcp::{Applied, SettingsPort};
use oku_slides_session::{Platform, Settings};
use serde_json::json;
use speculoos::prelude::*;

use super::AppSettings;

fn platform() -> Platform {
    let mut settings = Settings::defaults().unwrap();
    let dir = std::env::temp_dir().join(format!("oku-slides-settings-port-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::remove_file(dir.join("config.toml")).ok();
    settings.paths.data = Some(dir);
    Platform::with_settings(settings).unwrap()
}

#[test]
fn read_exposes_history_limit_and_vad_keys() {
    let p = platform();
    let port = AppSettings::new(&p, None);
    let v = port.read();
    assert_that!(v.get("history.limit").cloned()).is_equal_to(Some(json!(500)));
    assert_that!(v.contains_key("stt.vad.pause")).is_true();
}

#[test]
fn write_config_key_goes_to_config_toml_for_next_launch() {
    let p = platform();
    let port = AppSettings::new(&p, None);
    assert_that!(port.write("history.limit", &json!(42))).is_equal_to(Ok(Applied::NextLaunch));
    let text = std::fs::read_to_string(p.settings.config_file()).unwrap();
    assert_that!(text.contains("limit = 42")).is_true();
}

#[test]
fn write_dictation_key_without_dictation_is_an_error() {
    let p = platform();
    let port = Arc::new(AppSettings::new(&p, None));
    assert_that!(port.write("dictation.model", &json!("base"))).is_err();
}
```

`tests/stack.rs` :

```rust
mod common;

use std::sync::{Arc, Mutex};

use oku_slides_agent::{AgentEvent, Stack};
use oku_slides_mcp::SurfaceAction;
use oku_slides_session::agent::AgentUi;
use oku_slides_session::{Fonts, Platform, Session, Settings};
use serde_json::{Map, json};
use speculoos::prelude::*;

async fn stack(script: serde_json::Value) -> (Stack, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("oku-slides-stack-{}-{}", std::process::id(), rand_suffix()));
    std::fs::create_dir_all(&dir).unwrap();
    let (script_path, log) = (dir.join("script.json"), dir.join("log.jsonl"));
    std::fs::write(&script_path, script.to_string()).unwrap();
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(dir.clone());
    settings.i18n.locale = Some("fr".into());
    settings.agent.command = env!("CARGO_BIN_EXE_oku-slides-fake-agent").into();
    settings.agent.args = vec![
        "--script".into(),
        script_path.to_string_lossy().into_owned(),
        "--log".into(),
        log.to_string_lossy().into_owned(),
    ];
    let platform = Platform::with_settings(settings).unwrap();
    let mut session = Session::new(Fonts::default(), &platform).unwrap();
    session.open(&std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))).unwrap();
    let s = Stack::start(Arc::new(Mutex::new(session)), &platform, None).await.unwrap();
    (s, log)
}

fn rand_suffix() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_while_question_waits_answers_it_instead_of_new_turn() {
    let ask = json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } });
    let (s, log) = stack(json!({ "turns": [{ "mcp": [ask] }] })).await;
    let mut agent = s.agent().subscribe();
    let mut mcp = s.hub().subscribe();
    s.prompt("demande-moi".into(), AgentUi::default());
    common::wait_surface(&mut mcp).await;
    s.prompt("la deuxième".into(), AgentUi::default());
    let chunk = common::until(&mut agent, |e| matches!(e, AgentEvent::Chunk { text } if text.starts_with("tool:ask_choice"))).await;
    let AgentEvent::Chunk { text } = chunk else { unreachable!() };
    assert_that!(text.contains("la deuxième")).is_true();
    common::until(&mut agent, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    assert_that!(common::logged(&log, "session/prompt")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn surface_action_reaches_hub() {
    let (s, _) = stack(json!({ "turns": [{}] })).await;
    let r = s.surface_action(SurfaceAction { surface: "inconnue".into(), name: "dismiss".into(), context: Map::new() });
    assert_that!(r).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn forward_sends_named_json_events() {
    let (s, _) = stack(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    let seen = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let sink = seen.clone();
    s.forward(move |name, json| sink.lock().unwrap().push((name.to_owned(), json)));
    let mut agent = s.agent().subscribe();
    s.prompt("bonjour".into(), AgentUi::default());
    common::until(&mut agent, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let seen = seen.lock().unwrap();
    assert_that!(seen.iter().any(|(n, j)| n == "agent" && j.contains("\"kind\":\"chunk\""))).is_true();
}
```

Ajouter à `tests/common/mod.rs` :

```rust
/// Attend la prochaine surface poussée par le hub (20 s au plus).
pub async fn wait_surface(rx: &mut broadcast::Receiver<oku_slides_mcp::McpEvent>) -> oku_slides_session::agent::Surface {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Ok(oku_slides_mcp::McpEvent::Surface { surface }) = rx.recv().await {
                return surface;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("surface attendue non reçue"))
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-agent` → échec de compilation.

- [ ] **Step 3 : implémentation.**

`connection.rs`, dans le constructeur (après `on_receive_notification`) :

```rust
        .on_receive_request(
            {
                let hub = ctx.hub.clone();
                async move |req: RequestPermissionRequest, responder, cx: ConnectionTo<Agent>| {
                    let hub = hub.clone();
                    // Attendre l'utilisateur hors de la boucle de réception.
                    cx.spawn(async move {
                        let title = req.tool_call.fields.title.clone().unwrap_or_default();
                        let names: Vec<String> = req.options.iter().map(|o| o.name.clone()).collect();
                        let outcome = match hub.permission(&title, &names).await.and_then(|i| req.options.get(i)) {
                            Some(o) => RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(o.option_id.clone())),
                            None => RequestPermissionOutcome::Cancelled,
                        };
                        responder.respond(RequestPermissionResponse::new(outcome))
                    })
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
```

(Imports : `RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome`.)

`settings_port.rs` :

```rust
//! Les réglages de l'app pour l'agent : la dictée tout de suite, le reste
//! dans `config.toml` (au prochain lancement).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use oku_slides_mcp::{Applied, SettingsPort};
use oku_slides_platform::config_write::write_value;
use oku_slides_platform::storage::Storage;
use oku_slides_session::{Platform, Settings};
use oku_slides_stt::{Stt, SttRequest};
use serde_json::{Value, json};

/// Implémentation de [`SettingsPort`] pour l'app et `oku-slides serve`.
#[derive(Clone)]
pub struct AppSettings {
    settings: Arc<Settings>,
    storage: Storage,
    config_file: PathBuf,
    stt: Option<Stt>,
}

impl std::fmt::Debug for AppSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppSettings").field("config_file", &self.config_file).finish_non_exhaustive()
    }
}

impl AppSettings {
    /// `stt` : la dictée de l'hôte (absente dans les tests).
    pub fn new(platform: &Platform, stt: Option<Stt>) -> Self {
        Self {
            settings: platform.settings.clone(),
            storage: platform.storage.clone(),
            config_file: platform.settings.config_file(),
            stt,
        }
    }
}

impl SettingsPort for AppSettings {
    fn read(&self) -> BTreeMap<String, Value> {
        let mut out = BTreeMap::new();
        if let Some(stt) = &self.stt
            && let Ok(Value::Object(config)) = serde_json::to_value(stt.handle(SttRequest::Status).config)
        {
            for (k, v) in config {
                out.insert(format!("dictation.{k}"), v);
            }
        }
        out.insert("i18n.locale".into(), json!(self.settings.i18n.locale));
        out.insert("history.limit".into(), json!(self.settings.history.limit));
        let vad = &self.settings.stt.vad;
        for (k, v) in [
            ("threshold", json!(vad.threshold)),
            ("onset_frames", json!(vad.onset_frames)),
            ("pause", json!(vad.pause)),
            ("min_phrase", json!(vad.min_phrase)),
            ("long_pause", json!(vad.long_pause)),
            ("pre_roll", json!(vad.pre_roll)),
            ("post_roll", json!(vad.post_roll)),
            ("max_phrase", json!(vad.max_phrase)),
        ] {
            out.insert(format!("stt.vad.{k}"), v);
        }
        out
    }

    fn write(&self, key: &str, value: &Value) -> Result<Applied, String> {
        if let Some(field) = key.strip_prefix("dictation.") {
            let stt = self.stt.as_ref().ok_or_else(|| "dictation unavailable".to_owned())?;
            let mut config = serde_json::to_value(stt.handle(SttRequest::Status).config).map_err(|e| e.to_string())?;
            config[field] = value.clone();
            let config = serde_json::from_value(config).map_err(|e| format!("invalid {key}: {e}"))?;
            stt.handle(SttRequest::Configure(config));
            return Ok(Applied::Now);
        }
        write_value(&self.storage, &self.config_file, key, value).map_err(|e| e.to_string())?;
        Ok(Applied::NextLaunch)
    }
}

#[cfg(test)]
#[path = "settings_port_tests.rs"]
mod settings_port_tests;
```

(`oku_slides_stt::Config` doit dériver `Serialize` et `Deserialize` : c'est le cas s'il passe par `SttRequest::Configure` en JSON ; sinon ajouter les dérivations.)

`stack.rs` :

```rust
//! La pile de l'agent pour un hôte : outils MCP, agent, réglages, et le
//! relais des évènements vers l'interface.

use std::sync::Arc;

use oku_slides_mcp::{McpError, McpEvent, SharedSession, SurfaceAction, SurfaceHub, ToolServer, Tools};
use oku_slides_session::Platform;
use oku_slides_session::agent::AgentUi;
use oku_slides_stt::Stt;
use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;

use crate::settings_port::AppSettings;
use crate::{AgentConfig, AgentHandle};

/// Outils + agent, démarrés ensemble ; arrêtés quand la pile est lâchée.
#[derive(Debug)]
pub struct Stack {
    hub: Arc<SurfaceHub>,
    agent: AgentHandle,
    /// Gardé pour la durée de la pile (adresse et jeton des outils).
    _server: ToolServer,
    cancel: CancellationToken,
}

impl Stack {
    /// À appeler dans un runtime tokio. `stt` : la dictée de l'hôte, pour `set_setting`.
    pub async fn start(session: SharedSession, platform: &Platform, stt: Option<Stt>) -> Result<Self, McpError> {
        let hub = SurfaceHub::new(session.clone(), platform.localizer.clone());
        let cancel = CancellationToken::new();
        let settings = Arc::new(AppSettings::new(platform, stt));
        let tools = Tools::new(session.clone(), hub.clone(), settings, platform);
        let server = ToolServer::start(tools, cancel.clone()).await?;
        let a = &platform.settings.agent;
        let config = AgentConfig {
            command: a.command.clone(),
            args: a.args.clone(),
            mcp_url: server.url().to_owned(),
            authorization: server.authorization(),
            fallback_cwd: platform.settings.paths.data(),
        };
        let agent = AgentHandle::spawn(config, session, hub.clone(), platform.localizer.clone());
        Ok(Self { hub, agent, _server: server, cancel })
    }

    pub fn hub(&self) -> &Arc<SurfaceHub> {
        &self.hub
    }

    pub fn agent(&self) -> &AgentHandle {
        &self.agent
    }

    /// Consigne dictée : réponse à la question en attente, sinon nouveau tour.
    pub fn prompt(&self, text: String, ui: AgentUi) {
        if !self.hub.answer_pending(&text) {
            self.agent().prompt(text, ui);
        }
    }

    pub fn cancel(&self) {
        self.hub.cancel_asks();
        self.agent().cancel();
    }

    pub fn new_conversation(&self) {
        self.agent().new_conversation();
    }

    pub fn surface_action(&self, action: SurfaceAction) -> Result<(), String> {
        self.hub.act(action)
    }

    /// Un autre document est ouvert : nouvelle conversation.
    pub fn document_changed(&self) {
        self.hub.cancel_asks();
        self.new_conversation();
        self.hub.prune();
    }

    /// Le deck a changé depuis l'interface : fermer les post-its orphelins.
    pub fn after_change(&self) {
        self.hub.prune();
    }

    /// Relaie les évènements (`"mcp"`, `"agent"`) en JSON vers `sink`.
    /// Si l'interface a pris du retard, un `changed` la fait tout relire.
    pub fn forward(&self, sink: impl Fn(&'static str, String) + Send + Sync + 'static) {
        let sink = Arc::new(sink);
        let (mut mcp, mut agent) = (self.hub.subscribe(), self.agent().subscribe());
        let s = sink.clone();
        tokio::spawn(async move {
            loop {
                match mcp.recv().await {
                    Ok(e) => s("mcp", serde_json::to_string(&e).unwrap_or_default()),
                    Err(RecvError::Lagged(_)) => s("mcp", serde_json::to_string(&McpEvent::Changed).unwrap_or_default()),
                    Err(RecvError::Closed) => return,
                }
            }
        });
        tokio::spawn(async move {
            loop {
                match agent.recv().await {
                    Ok(e) => sink("agent", serde_json::to_string(&e).unwrap_or_default()),
                    Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => return,
                }
            }
        });
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
```

`lib.rs` : `mod settings_port; mod stack; pub use settings_port::AppSettings; pub use stack::Stack;`. `SurfaceAction` est réexporté par `oku_slides_mcp`.

`examples/smoke.rs` (vérification manuelle avec le vrai agent ; répond à toute question par la première option) :

```rust
//! `just agent-smoke <deck> "<consigne>"` : lance la vraie pile (agent
//! configuré, outils MCP) sur un deck, envoie la consigne, affiche le fil,
//! répond « oui » / première option à toute question, puis enregistre une
//! copie du deck en JSON à côté (`<deck>.agent.json`).

use std::sync::{Arc, Mutex};

use oku_slides_agent::{AgentEvent, Stack};
use oku_slides_mcp::{McpEvent, SurfaceAction};
use oku_slides_session::agent::AgentUi;
use oku_slides_session::{Fonts, Platform, Session};
use serde_json::{Map, Value};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    oku_slides_platform::telemetry::init();
    let mut args = std::env::args().skip(1);
    let (Some(deck), Some(prompt)) = (args.next(), args.next()) else {
        color_eyre::eyre::bail!("usage : smoke <deck.json|.oku-slides.pdf> \"<consigne>\"");
    };
    let platform = Platform::load()?;
    let mut session = Session::new(Fonts::default(), &platform)?;
    session.open(std::path::Path::new(&deck))?;
    let shared = Arc::new(Mutex::new(session));
    let stack = Stack::start(shared.clone(), &platform, None).await?;
    let mut mcp = stack.hub().subscribe();
    let mut agent = stack.agent().subscribe();
    stack.prompt(prompt, AgentUi::default());
    loop {
        tokio::select! {
            Ok(e) = agent.recv() => {
                println!("{}", serde_json::to_string(&e)?);
                if matches!(e, AgentEvent::TurnEnded { .. } | AgentEvent::Error { .. }) { break; }
            }
            Ok(McpEvent::Surface { surface }) = mcp.recv() => {
                println!("surface {} ({:?})", surface.id, surface.place);
                let (name, context) = first_answer(&serde_json::to_value(&surface)?);
                let _ = stack.surface_action(SurfaceAction { surface: surface.id, name, context });
            }
        }
    }
    let out = format!("{deck}.agent.json");
    let json = shared.lock().map_err(|_| color_eyre::eyre::eyre!("session"))?.deck().to_json()?;
    std::fs::write(&out, json)?;
    println!("deck : {out}");
    Ok(())
}

/// La première action proposée par la surface (bouton principal ou première option).
fn first_answer(surface: &Value) -> (String, Map<String, Value>) {
    let text = surface.to_string();
    let mut context = Map::new();
    if text.contains("\"confirm\"") {
        context.insert("confirmed".into(), Value::Bool(true));
        return ("confirm".into(), context);
    }
    if text.contains("\"permission\"") {
        context.insert("index".into(), Value::from(0));
        return ("permission".into(), context);
    }
    if text.contains("\"rewrite\"") {
        context.insert("kept".into(), Value::from(0));
        let first = surface.pointer("/messages/2/updateDataModel/value/draft").cloned().unwrap_or(Value::Null);
        context.insert("text".into(), first);
        return ("rewrite".into(), context);
    }
    if let Some(id) = surface.pointer("/messages/1/updateComponents/components").and_then(Value::as_array).and_then(|cs| {
        cs.iter().find_map(|c| c.get("options").and_then(|o| o.get(0)).and_then(|o| o.get("id")).cloned())
    }) {
        context.insert("choice".into(), id);
        return ("choose".into(), context);
    }
    if text.contains("\"variant\"") {
        context.insert("index".into(), Value::from(0));
        return ("variant".into(), context);
    }
    ("dismiss".into(), context)
}
```

`[dev-dependencies]` de l'agent : `color-eyre.workspace = true` (pour l'exemple). `justfile` :

```make
# Vérification manuelle avec le vrai agent (réseau, compte) : just agent-smoke fixtures/roadmap.json "Rends le titre de la slide 2 plus percutant"
agent-smoke deck prompt:
    cargo run -p oku-slides-agent --example smoke -- {{deck}} "{{prompt}}"
```

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-agent` → tout passe ; `cargo build -p oku-slides-agent --example smoke` compile.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-agent justfile
git commit -m "feat(agent) : permissions ACP, réglages de l'app, pile pour les hôtes, fumée avec le vrai agent (t-112)"
```

---
### Task 15 : Hôte Tauri : agent et évènements

**Files:**
- Modify: `crates/oku-slides-agent/src/stack.rs` (`AgentCommand`, `Stack::handle`), `crates/oku-slides-agent/src/lib.rs`
- Modify: `src-tauri/src/main.rs`, `src-tauri/Cargo.toml`
- Test: `crates/oku-slides-agent/tests/stack.rs`

**Interfaces:**
- Consumes: `Stack` (Task 14).
- Produces : `AgentCommand` (Deserialize, `tag = "cmd"`, snake_case) : `Prompt { text, #[serde(default)] ui: AgentUi }`, `Cancel`, `NewConversation`, `SurfaceAction { surface, name, #[serde(default)] context }` ; `Stack::handle(&self, cmd: AgentCommand) -> Result<(), String>` ; commande Tauri `agent(req: AgentCommand)` ; évènements Tauri `oku-slides:mcp` et `oku-slides:agent` (charge : chaîne JSON d'un `McpEvent` / `AgentEvent`).

- [ ] **Step 1 : test qui échoue** — dans `tests/stack.rs` :

```rust
#[tokio::test(flavor = "multi_thread")]
async fn handle_parses_ui_commands() {
    let (s, log) = stack(json!({ "turns": [{}] })).await;
    let mut agent = s.agent().subscribe();
    let cmd: oku_slides_agent::AgentCommand =
        serde_json::from_value(json!({ "cmd": "prompt", "text": "bonjour", "ui": { "slide": "s-libre" } })).unwrap();
    s.handle(cmd).unwrap();
    common::until(&mut agent, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    let prompt = &common::logged(&log, "session/prompt")[0]["prompt"];
    assert_that!(prompt[1]["text"].as_str().unwrap().contains("slide: s-libre")).is_true();
    let bad: oku_slides_agent::AgentCommand =
        serde_json::from_value(json!({ "cmd": "surface_action", "surface": "x", "name": "dismiss" })).unwrap();
    assert_that!(s.handle(bad)).is_err();
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-agent --test stack` → échec de compilation.

- [ ] **Step 3 : implémentation.**

`stack.rs` :

```rust
/// Commande de l'interface vers l'agent (JSON `{"cmd": …}`), la même pour Tauri et HTTP.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum AgentCommand {
    Prompt {
        text: String,
        #[serde(default)]
        ui: AgentUi,
    },
    Cancel,
    NewConversation,
    SurfaceAction {
        surface: String,
        name: String,
        #[serde(default)]
        context: serde_json::Map<String, serde_json::Value>,
    },
}

impl Stack {
    /// Traite une commande de l'interface ; l'erreur est lue par l'interface.
    pub fn handle(&self, cmd: AgentCommand) -> Result<(), String> {
        match cmd {
            AgentCommand::Prompt { text, ui } => self.prompt(text, ui),
            AgentCommand::Cancel => self.cancel(),
            AgentCommand::NewConversation => self.new_conversation(),
            AgentCommand::SurfaceAction { surface, name, context } => {
                return self.surface_action(SurfaceAction { surface, name, context });
            }
        }
        Ok(())
    }
}
```

`lib.rs` : `pub use stack::{AgentCommand, Stack};`.

`src-tauri/Cargo.toml` : `oku-slides-agent.workspace = true`.

`src-tauri/src/main.rs` (compléter la doc du module : « … session, dictée et agent (outils MCP + agent ACP). ») :

```rust
use std::sync::{Arc, Mutex};

use oku_slides_agent::{AgentCommand, Stack};
use tauri::{Emitter, Manager};

struct AppState {
    session: Arc<Mutex<Session>>,
    localizer: Arc<Localizer>,
    /// Absent si les outils de l'agent n'ont pas pu démarrer.
    stack: Option<Stack>,
}

/// La réponse est déjà du JSON : elle passe telle quelle à l'interface ; une
/// erreur y arrive dans la langue de l'utilisateur. Un autre document ouvert
/// commence une nouvelle conversation avec l'agent.
#[tauri::command]
fn request(state: tauri::State<'_, AppState>, req: Request) -> Result<Response, String> {
    let new_document = matches!(req, Request::New | Request::Open { .. });
    let reply = {
        let mut session = state.session.lock().map_err(|_| state.localizer.tr("session-busy"))?;
        session.handle(req).map_err(|e| {
            tracing::warn!(error = %e.one_line(), "requête refusée");
            e.localize(&state.localizer)
        })?
    };
    if let Some(stack) = &state.stack {
        if new_document { stack.document_changed() } else { stack.after_change() }
    }
    Ok(Response::new(reply))
}

/// Agent : consigne, annulation, nouvelle conversation, action d'une surface.
#[tauri::command]
fn agent(state: tauri::State<'_, AppState>, req: AgentCommand) -> Result<(), String> {
    match &state.stack {
        Some(stack) => stack.handle(req),
        None => Err(state.localizer.tr("agent-tools-unavailable")),
    }
}
```

Dans `setup`, après la création de `session` :

```rust
            let stt = Stt::new(&platform);
            let session = Arc::new(Mutex::new(session));
            let handle = app.handle().clone();
            // Les outils et l'agent vivent dans le runtime tokio de Tauri.
            let stack = tauri::async_runtime::block_on(async {
                match Stack::start(session.clone(), &platform, Some(stt.clone())).await {
                    Ok(stack) => {
                        stack.forward(move |name, json| {
                            let _ = handle.emit(&format!("oku-slides:{name}"), json);
                        });
                        Some(stack)
                    }
                    Err(e) => {
                        tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "outils de l'agent indisponibles");
                        None
                    }
                }
            });
            app.manage(AppState { session, localizer: platform.localizer.clone(), stack });
            app.manage(stt);
```

(supprimer l'ancien `app.manage(Stt::new(&platform))` et l'ancien `AppState`), et `invoke_handler(tauri::generate_handler![request, stt, agent])`. `std::sync::Mutex` remplace l'import existant. La capacité `core:default` couvre déjà l'écoute des évènements côté interface.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-agent` passe ; `cargo build -p oku-slides-app` compile (le nom du paquet de `src-tauri` est `oku-slides-app`, exclu de `just test`) ; `cargo clippy -p oku-slides-app -- -D warnings` propre.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-agent src-tauri
git commit -m "feat(app) : l'agent dans l'app Tauri, évènements vers l'interface (t-113)"
```

---

### Task 16 : Hôte serve : agent et évènements SSE

**Files:**
- Modify: `crates/oku-slides-cli/src/serve.rs`, `crates/oku-slides-cli/Cargo.toml`
- Create: `crates/oku-slides-cli/src/sse.rs`, `crates/oku-slides-cli/src/sse_tests.rs`
- Modify: `crates/oku-slides-cli/src/main.rs` (`mod sse;`)

**Interfaces:**
- Consumes: `Stack`, `AgentCommand` (Tasks 14–15).
- Produces : `POST /api/agent` (corps `AgentCommand`, réponse `{}` ou `{"error"}` en 400) ; `GET /api/events` : flux SSE, `event: mcp|agent`, `data: <json>` ; `sse::Clients` (diffusion) et `sse::SseReader` (corps de réponse bloquant).

- [ ] **Step 1 : tests qui échouent** — `sse_tests.rs` :

```rust
use std::io::Read;

use speculoos::prelude::*;

use super::Clients;

#[test]
fn reader_gets_formatted_event_then_eof_when_clients_drop() {
    let clients = Clients::default();
    let mut reader = clients.subscribe();
    clients.send("agent", r#"{"kind":"chunk","text":"a"}"#);
    drop(clients);
    let mut out = String::new();
    reader.read_to_string(&mut out).unwrap();
    assert_that!(out.as_str()).is_equal_to("event: agent\ndata: {\"kind\":\"chunk\",\"text\":\"a\"}\n\n");
}

#[test]
fn send_forgets_closed_readers() {
    let clients = Clients::default();
    drop(clients.subscribe());
    clients.send("mcp", "{}");
    assert_that!(clients.len()).is_equal_to(0);
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-cli sse` → échec de compilation.

- [ ] **Step 3 : implémentation.**

`crates/oku-slides-cli/Cargo.toml` : `oku-slides-agent.workspace = true`, `tokio = { workspace = true, features = ["rt-multi-thread"] }`.

`sse.rs` :

```rust
//! Évènements du serveur de développement en SSE : chaque client de
//! `GET /api/events` a son canal ; son corps de réponse bloque jusqu'au
//! prochain évènement.

use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

/// Les clients SSE connectés.
#[derive(Debug, Default, Clone)]
pub struct Clients(Arc<Mutex<Vec<Sender<String>>>>);

impl Clients {
    /// Un nouveau client : le corps de sa réponse.
    pub fn subscribe(&self) -> SseReader {
        let (tx, rx) = channel();
        if let Ok(mut list) = self.0.lock() {
            list.push(tx);
        }
        SseReader { rx, pending: Vec::new(), at: 0 }
    }

    /// Envoie `event: name` / `data: json` à chacun ; oublie ceux qui sont partis.
    pub fn send(&self, name: &str, json: &str) {
        let frame = format!("event: {name}\ndata: {json}\n\n");
        if let Ok(mut list) = self.0.lock() {
            list.retain(|tx| tx.send(frame.clone()).is_ok());
        }
    }

    pub fn len(&self) -> usize {
        self.0.lock().map(|l| l.len()).unwrap_or(0)
    }
}

/// Corps d'une réponse SSE : bloque jusqu'au prochain évènement ; fin quand
/// plus personne n'envoie.
#[derive(Debug)]
pub struct SseReader {
    rx: Receiver<String>,
    pending: Vec<u8>,
    at: usize,
}

impl Read for SseReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.at >= self.pending.len() {
            match self.rx.recv() {
                Ok(frame) => {
                    self.pending = frame.into_bytes();
                    self.at = 0;
                }
                Err(_) => return Ok(0),
            }
        }
        let n = buf.len().min(self.pending.len() - self.at);
        buf[..n].copy_from_slice(&self.pending[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

#[cfg(test)]
#[path = "sse_tests.rs"]
mod sse_tests;
```

`len` sans `is_empty` : clippy `len_without_is_empty` → ajouter `pub fn is_empty(&self) -> bool { self.len() == 0 }`.

`serve.rs` : la session devient partagée, l'agent démarre dans un runtime tokio, deux routes s'ajoutent.

```rust
use std::sync::Mutex;

use oku_slides_agent::{AgentCommand, Stack};
use tiny_http::StatusCode;

use crate::sse::Clients;
```

Dans `serve`, remplacer `let mut session = …` / `session.open(…)` par :

```rust
    let mut session = oku_slides_session::Session::new(fonts, platform)?;
    if let Some(path) = &input {
        session.open(path)?;
    }
    let session = Arc::new(Mutex::new(session));
```

Après la création de `stt` :

```rust
    // Outils MCP et agent : un runtime tokio à côté de la boucle HTTP.
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let clients = Clients::default();
    let stack = runtime.block_on(Stack::start(session.clone(), platform, Some(stt.clone())));
    let stack = match stack {
        Ok(stack) => {
            let _enter = runtime.enter();
            let c = clients.clone();
            stack.forward(move |name, json| c.send(name, &json));
            Some(stack)
        }
        Err(e) => {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "outils de l'agent indisponibles");
            None
        }
    };
```

Dans la boucle, avant le traitement de `/api` :

```rust
        if req.method() == &Method::Get && url == "/api/events" {
            let body = clients.subscribe();
            let no_cache = Header::from_bytes("Cache-Control", "no-cache")
                .map_err(|()| eyre!(l.tr_with("cli-bad-header", &[("value", "no-cache".into())])))?;
            let headers = vec![header("text/event-stream")?, no_cache];
            // Un flux sans fin : sa propre tâche, pour ne pas bloquer la boucle.
            std::thread::spawn(move || {
                if let Err(e) = req.respond(Response::new(StatusCode(200), headers, body, None, None)) {
                    tracing::debug!(error = %e, "flux d'évènements fermé");
                }
            });
            continue;
        }
```

La condition `POST` devient `url == "/api" || url == "/api/stt" || url == "/api/agent"` ; le bras `/api` :

```rust
            let (status, reply) = if url == "/api" {
                match serde_json::from_str::<oku_slides_session::Request>(&body) {
                    Ok(r) => {
                        let new_document = matches!(r, oku_slides_session::Request::New | oku_slides_session::Request::Open { .. });
                        let result = match session.lock() {
                            Ok(mut s) => s.handle(r).map_err(|e| {
                                tracing::warn!(error = %e.one_line(), "requête refusée");
                                e.localize(l)
                            }),
                            Err(_) => Err(l.tr("session-busy")),
                        };
                        match result {
                            Ok(json) => {
                                if let Some(stack) = &stack {
                                    if new_document { stack.document_changed() } else { stack.after_change() }
                                }
                                (200, json)
                            }
                            Err(message) => (400, serde_json::json!({ "error": message }).to_string()),
                        }
                    }
                    Err(e) => (400, bad_request(e)),
                }
            } else if url == "/api/agent" {
                match serde_json::from_str::<AgentCommand>(&body) {
                    Ok(cmd) => match stack.as_ref().map(|s| s.handle(cmd)) {
                        Some(Ok(())) => (200, "{}".to_owned()),
                        Some(Err(e)) => (400, serde_json::json!({ "error": e }).to_string()),
                        None => (400, serde_json::json!({ "error": l.tr("agent-tools-unavailable") }).to_string()),
                    },
                    Err(e) => (400, bad_request(e)),
                }
            } else {
                // `/api/stt`, inchangé.
```

À l'arrêt (après la boucle), `drop(stack); drop(runtime);` avant `out.line("cli-stopped", …)` : le serveur d'outils et l'agent s'arrêtent avec la pile.

`main.rs` : `mod sse;`.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-cli` passe. Vérification à la main : `OKU_SLIDES_AGENT__COMMAND=$PWD/target/debug/oku-slides-fake-agent cargo run -p oku-slides-cli -- serve fixtures/roadmap.json --ui ui/dist --port 4873`, puis dans un autre terminal `curl -N http://127.0.0.1:4873/api/events` et `curl -X POST http://127.0.0.1:4873/api/agent -d '{"cmd":"prompt","text":"bonjour"}'` : le premier affiche des `event: agent` (`status`, `user_text`, `turn_ended`).
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-cli Cargo.lock
git commit -m "feat(cli) : l'agent dans oku-slides serve, évènements en SSE (t-114)"
```

---
### Task 17 : UI : api de l'agent, fil et surfaces

**Files:**
- Modify: `ui/src/api.ts`
- Create: `ui/src/agent/events.ts`, `ui/src/agent/thread.ts`, `ui/src/agent/thread.test.ts`, `ui/src/agent/surfaces.ts`, `ui/src/agent/surfaces.test.ts`, `ui/src/agent/ui.ts`, `ui/src/agent/ui.test.ts`

**Interfaces:**
- Consumes: commandes `agent` (Tauri) / `POST /api/agent` ; évènements `oku-slides:mcp|agent` (Tauri) / `GET /api/events` (SSE).
- Produces (TS) :
  - `api.ts` : types `AgentEvent`, `McpEvent`, `Surface`, `Place`, `AgentUi` ; `agent = { prompt(text, ui), cancel(), newConversation(), surfaceAction(surface, name, context) }` ;
  - `agent/events.ts` : `subscribeHost(onMcp: (e: McpEvent) => void, onAgent: (e: AgentEvent) => void): () => void` ;
  - `agent/thread.ts` : `Entry`, `Thread`, `EMPTY_THREAD`, `threadReducer(t: Thread, e: AgentEvent): Thread` ;
  - `agent/surfaces.ts` : `surfacesReducer(list: Surface[], e: McpEvent): Surface[]`, `byPlace(list, place): Surface[]`, `isAsking(list): boolean` (une question attend une réponse : fiche, question dans le fil ou fenêtre) ;
  - `agent/ui.ts` : `agentUi(current: string | null, selected: string | null, editing: string | null): AgentUi`.

- [ ] **Step 1 : tests qui échouent.**

`ui/src/agent/thread.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { EMPTY_THREAD, threadReducer } from "./thread";
import type { AgentEvent } from "../api";

const run = (events: AgentEvent[]) => events.reduce(threadReducer, EMPTY_THREAD);

describe("fil de l'agent", () => {
  test("les morceaux de réponse s'ajoutent au même message", () => {
    const t = run([
      { kind: "user_text", text: "plus court" },
      { kind: "chunk", text: "Je " },
      { kind: "chunk", text: "regarde." },
    ]);
    expect(t.entries).toEqual([
      { kind: "user", text: "plus court" },
      { kind: "agent", text: "Je regarde." },
    ]);
  });

  test("une mise à jour d'outil change la ligne de l'outil", () => {
    const t = run([
      { kind: "tool_call", id: "t1", title: "Lire", status: "in_progress" },
      { kind: "tool_update", id: "t1", title: null, status: "completed" },
    ]);
    expect(t.entries).toEqual([{ kind: "tool", id: "t1", title: "Lire", status: "completed" }]);
  });

  test("le plan remplace le plan précédent", () => {
    const t = run([
      { kind: "plan", entries: [{ content: "Lire", status: "in_progress" }] },
      { kind: "plan", entries: [{ content: "Lire", status: "completed" }] },
    ]);
    expect(t.entries).toEqual([{ kind: "plan", entries: [{ content: "Lire", status: "completed" }] }]);
  });

  test("un tour annulé le dit ; une nouvelle conversation vide le fil", () => {
    const t = run([{ kind: "user_text", text: "a" }, { kind: "turn_ended", stop: "cancelled" }]);
    expect(t.entries.at(-1)).toEqual({ kind: "stop", stop: "cancelled" });
    expect(threadReducer(t, { kind: "conversation" }).entries).toEqual([]);
  });

  test("l'état suit les évènements de statut", () => {
    expect(run([{ kind: "status", state: "working" }]).state).toBe("working");
  });
});
```

`ui/src/agent/surfaces.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { byPlace, isAsking, surfacesReducer } from "./surfaces";
import type { Surface } from "../api";

const s = (id: string, place: Surface["place"]): Surface => ({ id, place, messages: [] });

describe("surfaces de l'agent", () => {
  test("une surface s'ajoute puis se ferme", () => {
    const list = surfacesReducer([], { kind: "surface", surface: s("ask-1", "sheet") });
    expect(list.map((x) => x.id)).toEqual(["ask-1"]);
    expect(surfacesReducer(list, { kind: "close_surface", id: "ask-1" })).toEqual([]);
  });

  test("une surface de même identifiant est remplacée", () => {
    const list = surfacesReducer([s("show-1", "postit")], { kind: "surface", surface: { ...s("show-1", "postit"), anchor: { slide: "a", element: "b" } } });
    expect(list).toHaveLength(1);
    expect(list[0].anchor).toEqual({ slide: "a", element: "b" });
  });

  test("byPlace garde l'ordre d'arrivée", () => {
    const list = [s("a", "postit"), s("b", "sheet"), s("c", "postit")];
    expect(byPlace(list, "postit").map((x) => x.id)).toEqual(["a", "c"]);
  });

  test("isAsking : une question ouverte, pas un post-it ni un tiroir", () => {
    expect(isAsking([s("a", "postit"), s("b", "drawer")])).toBe(false);
    expect(isAsking([s("a", "postit"), s("c", "sheet")])).toBe(true);
    expect(isAsking([s("m", "modal")])).toBe(true);
  });

  test("changed et focus ne touchent pas aux surfaces", () => {
    const list = [s("a", "postit")];
    expect(surfacesReducer(list, { kind: "changed" })).toBe(list);
  });
});
```

`ui/src/agent/ui.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { agentUi } from "./ui";

describe("contexte de la consigne", () => {
  test("un champ sélectionné (@title) devient field", () => {
    expect(agentUi("s-1", "@title", null)).toEqual({ slide: "s-1", element: null, field: "title", editing: false });
  });

  test("un élément en édition", () => {
    expect(agentUi("s-1", "e-2", "e-2")).toEqual({ slide: "s-1", element: "e-2", field: null, editing: true });
  });

  test("rien de sélectionné", () => {
    expect(agentUi(null, null, null)).toEqual({ slide: null, element: null, field: null, editing: false });
  });
});
```

- [ ] **Step 2 :** `cd ui && bun test src/agent` → échec (modules absents).

- [ ] **Step 3 : implémentation.**

`api.ts`, après les types de la dictée :

```ts
// --- Agent -----------------------------------------------------------------

/** Ce que voit l'utilisateur au moment de la consigne (bloc de contexte). */
export interface AgentUi {
  slide: string | null;
  element: string | null;
  field: string | null;
  editing: boolean;
}

export type AgentState = "idle" | "starting" | "working";

export interface PlanItem {
  content: string;
  status: string;
}

/** Évènements du fil (crate oku-slides-agent, `AgentEvent`). */
export type AgentEvent =
  | { kind: "status"; state: AgentState }
  | { kind: "user_text"; text: string }
  | { kind: "chunk"; text: string }
  | { kind: "tool_call"; id: string; title: string; status: string }
  | { kind: "tool_update"; id: string; title: string | null; status: string | null }
  | { kind: "plan"; entries: PlanItem[] }
  | { kind: "turn_ended"; stop: string }
  | { kind: "error"; message: string }
  | { kind: "conversation" };

export type Place = "sheet" | "thread" | "drawer" | "postit" | "modal";

/** Une surface de l'agent : ses messages A2UI et sa place (session, `agent::Surface`). */
export interface Surface {
  id: string;
  place: Place;
  anchor?: { slide: string; element: string };
  messages: A2uiMessage[];
}

/** Évènements des outils (crate oku-slides-mcp, `McpEvent`). */
export type McpEvent =
  | { kind: "changed" }
  | { kind: "surface"; surface: Surface }
  | { kind: "close_surface"; id: string }
  | { kind: "focus"; slide: string; element: string | null };

const agentCall = (req: object) => host<void>("agent", "/api/agent", req);

export const agent = {
  prompt: (text: string, ui: AgentUi) => agentCall({ cmd: "prompt", text, ui }),
  cancel: () => agentCall({ cmd: "cancel" }),
  newConversation: () => agentCall({ cmd: "new_conversation" }),
  surfaceAction: (surface: string, name: string, context: Record<string, unknown>) =>
    agentCall({ cmd: "surface_action", surface, name, context }),
};
```

`ui/src/agent/events.ts` :

```ts
// Évènements de l'hôte : outils (surfaces, changements) et fil de l'agent.
// Tauri : évènements `oku-slides:mcp` / `oku-slides:agent` (charge JSON) ;
// développement : SSE sur /api/events.

import { inTauri } from "../api";
import type { AgentEvent, McpEvent } from "../api";

/** S'abonne ; renvoie de quoi se désabonner. */
export function subscribeHost(onMcp: (e: McpEvent) => void, onAgent: (e: AgentEvent) => void): () => void {
  if (inTauri) {
    let closed = false;
    const stops: (() => void)[] = [];
    import("@tauri-apps/api/event").then(async ({ listen }) => {
      const a = await listen<string>("oku-slides:mcp", (e) => onMcp(JSON.parse(e.payload)));
      const b = await listen<string>("oku-slides:agent", (e) => onAgent(JSON.parse(e.payload)));
      if (closed) {
        a();
        b();
      } else stops.push(a, b);
    });
    return () => {
      closed = true;
      stops.forEach((stop) => stop());
    };
  }
  const source = new EventSource("/api/events");
  source.addEventListener("mcp", (e) => onMcp(JSON.parse((e as MessageEvent<string>).data)));
  source.addEventListener("agent", (e) => onAgent(JSON.parse((e as MessageEvent<string>).data)));
  return () => source.close();
}
```

`ui/src/agent/thread.ts` :

```ts
// Le fil de l'agent : ce qui a été dit, ce qu'il fait, ce qu'il répond.

import type { AgentEvent, AgentState, PlanItem } from "../api";

export type Entry =
  | { kind: "user"; text: string }
  | { kind: "agent"; text: string }
  | { kind: "tool"; id: string; title: string; status: string }
  | { kind: "plan"; entries: PlanItem[] }
  | { kind: "error"; text: string }
  | { kind: "stop"; stop: string };

export interface Thread {
  state: AgentState;
  entries: Entry[];
}

export const EMPTY_THREAD: Thread = { state: "idle", entries: [] };

export function threadReducer(t: Thread, e: AgentEvent): Thread {
  const entries = t.entries;
  switch (e.kind) {
    case "status":
      return { ...t, state: e.state };
    case "user_text":
      return { ...t, entries: [...entries, { kind: "user", text: e.text }] };
    case "chunk": {
      const last = entries.at(-1);
      if (last?.kind === "agent") return { ...t, entries: [...entries.slice(0, -1), { kind: "agent", text: last.text + e.text }] };
      return { ...t, entries: [...entries, { kind: "agent", text: e.text }] };
    }
    case "tool_call":
      return { ...t, entries: [...entries, { kind: "tool", id: e.id, title: e.title, status: e.status }] };
    case "tool_update":
      return {
        ...t,
        entries: entries.map((x) =>
          x.kind === "tool" && x.id === e.id ? { ...x, title: e.title ?? x.title, status: e.status ?? x.status } : x,
        ),
      };
    case "plan": {
      const i = entries.findIndex((x) => x.kind === "plan");
      const plan: Entry = { kind: "plan", entries: e.entries };
      return { ...t, entries: i < 0 ? [...entries, plan] : entries.map((x, j) => (j === i ? plan : x)) };
    }
    case "turn_ended":
      return e.stop === "end_turn" ? t : { ...t, entries: [...entries, { kind: "stop", stop: e.stop }] };
    case "error":
      return { ...t, entries: [...entries, { kind: "error", text: e.message }] };
    case "conversation":
      return { ...t, entries: [] };
  }
}
```

`ui/src/agent/surfaces.ts` :

```ts
// Surfaces ouvertes par l'agent, dans l'ordre d'arrivée.

import type { McpEvent, Place, Surface } from "../api";

export function surfacesReducer(list: Surface[], e: McpEvent): Surface[] {
  switch (e.kind) {
    case "surface": {
      const i = list.findIndex((s) => s.id === e.surface.id);
      return i < 0 ? [...list, e.surface] : list.map((s, j) => (j === i ? e.surface : s));
    }
    case "close_surface":
      return list.filter((s) => s.id !== e.id);
    default:
      return list;
  }
}

export const byPlace = (list: Surface[], place: Place) => list.filter((s) => s.place === place);

/** Une question de l'agent attend l'utilisateur : ⌘⇧D y répond au lieu d'arrêter le tour. */
export const isAsking = (list: Surface[]) => list.some((s) => s.place === "sheet" || s.place === "thread" || s.place === "modal");
```

`ui/src/agent/ui.ts` :

```ts
// Le contexte d'une consigne : slide, élément ou champ sélectionné, édition en cours.

import type { AgentUi } from "../api";

export function agentUi(current: string | null, selected: string | null, editing: string | null): AgentUi {
  const field = selected?.startsWith("@") ? selected.slice(1) : null;
  return {
    slide: current,
    element: selected && !field ? selected : null,
    field,
    editing: editing !== null && editing === selected,
  };
}
```

- [ ] **Step 4 :** `cd ui && bun test && bunx tsc -b` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add ui/src/api.ts ui/src/agent
git commit -m "feat(ui) : commandes et évènements de l'agent, fil et surfaces (t-115)"
```

---

### Task 18 : UI : catalogue v2

**Files:**
- Modify: `ui/src/a2ui/catalog.tsx`, `ui/src/a2ui/catalog.test.tsx`, `ui/src/styles.css`
- Create: `ui/src/a2ui/SurfaceView.tsx`
- Modify: `ui/src/a2ui/PanelSurface.tsx` (devient un appel à `SurfaceView`)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 locales)

**Interfaces:**
- Consumes: catalogue Rust v2 (Task 3), `Surface` (Task 17).
- Produces: `catalog` (au lieu de `panelCatalog`) avec Column, Row, Card, Text, TextField, Stepper, Button, ChoiceChips, Slider, ColorField, Select, SlideThumb, DiffText ; `SurfaceView({ messages, surfaceId, onAction, onError })`.

- [ ] **Step 1 : tests qui échouent** — dans `catalog.test.tsx`, renommer l'import `panelCatalog` → `catalog` et ajouter :

```ts
const surface = (components: unknown[], data: unknown = {}) => [
  { version: "v0.9", createSurface: { surfaceId: "s", catalogId: CATALOG_ID } },
  { version: "v0.9", updateComponents: { surfaceId: "s", components } },
  { version: "v0.9", updateDataModel: { surfaceId: "s", value: data } },
];

function accepts(components: unknown[], data: unknown = {}) {
  const processor = new MessageProcessor([catalog]);
  processor.processMessages(surface(components, data) as Parameters<typeof processor.processMessages>[0]);
  return processor.model.surfacesMap.get("s")?.catalog.id === CATALOG_ID;
}

describe("catalogue v2", () => {
  test("accepte les composants de l'agent", () => {
    expect(
      accepts([
        { id: "root", component: "Column", children: ["row", "chips", "diff", "thumb", "slider", "color", "select", "ok"] },
        { id: "row", component: "Row", children: [] },
        { id: "chips", component: "ChoiceChips", options: [{ id: "a", label: "A" }, { id: "b", label: "B", hint: "h" }], value: { path: "/choice" },
          action: { event: { name: "choose", context: { choice: { path: "/choice" } } } } },
        { id: "diff", component: "DiffText", segments: [{ kind: "keep", text: "Nos " }, { kind: "del", text: "objectifs" }, { kind: "ins", text: "paris" }] },
        { id: "thumb", component: "SlideThumb", svg: "<svg/>", label: "1 · A", action: { event: { name: "variant", context: { index: 0 } } } },
        { id: "slider", component: "Slider", label: "Espacement", value: { path: "/f0" }, min: 0, max: 64, step: 4 },
        { id: "color", component: "ColorField", label: "Accent", value: { path: "/f1" }, palette: ["#c2573a"] },
        { id: "select", component: "Select", label: "Thème", value: { path: "/f2" }, options: [{ id: "pro", label: "Pro" }] },
        { id: "ok", component: "Button", label: "OK", primary: true, action: { event: { name: "confirm", context: { confirmed: true } } } },
      ], { choice: "", f0: 16, f1: "#c2573a", f2: "pro" }),
    ).toBe(true);
  });

  test("refuse un segment de nature inconnue", () => {
    expect(() => accepts([{ id: "root", component: "DiffText", segments: [{ kind: "moved", text: "x" }] }])).toThrow();
  });

  test("un bouton sans action est refusé", () => {
    expect(() => accepts([{ id: "root", component: "Button", label: "OK" }])).toThrow();
  });
});
```

- [ ] **Step 2 :** `cd ui && bun test src/a2ui` → échec.

- [ ] **Step 3 : implémentation** — `catalog.tsx` (en-tête : « Catalogue A2UI d'Oku Slides (v2) : panneau et surfaces de l'agent. `Stepper`, `ChoiceChips`, `ColorField`, `SlideThumb`, `DiffText` sont propres à Oku Slides. ») ; garder Column, Card, Text, TextField, Stepper tels quels, ajouter :

```tsx
const Option = z.object({ id: z.string(), label: z.string(), hint: z.string().optional() }).strict();

const RowApi = { name: "Row", schema: z.object({ children: CommonSchemas.ChildList }).strict() };
const ButtonApi = {
  name: "Button",
  schema: z.object({ label: CommonSchemas.DynamicString, primary: z.boolean().optional(), action: CommonSchemas.Action }).strict(),
};
const ChoiceChipsApi = {
  name: "ChoiceChips",
  schema: z.object({ options: z.array(Option), value: CommonSchemas.DynamicString, action: CommonSchemas.Action.optional() }).strict(),
};
const SliderApi = {
  name: "Slider",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      value: CommonSchemas.DynamicNumber,
      min: z.number(),
      max: z.number(),
      step: z.number().optional(),
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};
const ColorFieldApi = {
  name: "ColorField",
  schema: z
    .object({ label: CommonSchemas.DynamicString, value: CommonSchemas.DynamicString, palette: z.array(z.string()), action: CommonSchemas.Action.optional() })
    .strict(),
};
const SelectApi = {
  name: "Select",
  schema: z
    .object({ label: CommonSchemas.DynamicString, value: CommonSchemas.DynamicString, options: z.array(Option), action: CommonSchemas.Action.optional() })
    .strict(),
};
const SlideThumbApi = {
  name: "SlideThumb",
  schema: z.object({ svg: z.string(), label: CommonSchemas.DynamicString, action: CommonSchemas.Action.optional() }).strict(),
};
const DiffTextApi = {
  name: "DiffText",
  schema: z.object({ segments: z.array(z.object({ kind: z.enum(["keep", "del", "ins"]), text: z.string() }).strict()) }).strict(),
};

/** Enfants côte à côte. */
const Row = createComponentImplementation(RowApi, ({ props, buildChild }) => (
  <div className="a2ui-row">
    {Array.isArray(props.children) &&
      props.children.map((ref) =>
        typeof ref === "string" ? <div key={ref}>{buildChild(ref)}</div> : <div key={`${ref.id}-${ref.basePath}`}>{buildChild(ref.id, ref.basePath)}</div>,
      )}
  </div>
));

/** Un bouton ; `primary` : l'action principale (Entrée la déclenche). */
const Button = createComponentImplementation(ButtonApi, ({ props }) => (
  <button className={`btn${props.primary ? " primary" : ""}`} data-primary={props.primary ? "" : undefined} onClick={() => props.action?.()}>
    {props.label}
  </button>
));

/** Des pastilles ; choisir écrit l'identifiant puis envoie l'action. */
const ChoiceChips = createComponentImplementation(ChoiceChipsApi, ({ props }) => (
  <div className="chips" role="listbox">
    {props.options.map((o, i) => (
      <button
        key={o.id}
        role="option"
        aria-selected={props.value === o.id}
        className={`chip${props.value === o.id ? " on" : ""}`}
        title={o.hint}
        onClick={() => {
          props.setValue(o.id);
          props.action?.();
        }}
      >
        <span className="chip-key">{i + 1}</span> {o.label}
      </button>
    ))}
  </div>
));

/** Un curseur ; l'action part quand on lâche. */
const Slider = createComponentImplementation(SliderApi, ({ props }) => {
  const [draft, setDraft] = useState(props.value ?? props.min);
  useEffect(() => setDraft(props.value ?? props.min), [props.value, props.min]);
  const commit = () => {
    if (draft === props.value) return;
    props.setValue(draft);
    props.action?.();
  };
  return (
    <label className="note-field">
      <span className="postit-hint">{props.label} · {draft}</span>
      <input type="range" min={props.min} max={props.max} step={props.step ?? 1} value={draft}
        onChange={(e) => setDraft(Number(e.target.value))} onPointerUp={commit} onKeyUp={commit} />
    </label>
  );
});

/** Une couleur : pastilles de la palette, ou sélecteur libre. */
const ColorField = createComponentImplementation(ColorFieldApi, ({ props }) => {
  const pick = (c: string) => {
    if (c === props.value) return;
    props.setValue(c);
    props.action?.();
  };
  return (
    <div className="note-field">
      <span className="postit-hint">{props.label}</span>
      <div className="swatches">
        {props.palette.map((c) => (
          <button key={c} className={`swatch${c === props.value ? " on" : ""}`} style={{ background: c }} aria-label={c} onClick={() => pick(c)} />
        ))}
        <input type="color" value={props.value ?? "#000000"} onChange={(e) => pick(e.target.value)} aria-label={t("ui-agent-color-other")} />
      </div>
    </div>
  );
});

/** Une liste déroulante. */
const Select = createComponentImplementation(SelectApi, ({ props }) => (
  <label className="note-field">
    <span className="postit-hint">{props.label}</span>
    <select className="select" value={props.value ?? ""} onChange={(e) => { props.setValue(e.target.value); props.action?.(); }}>
      {props.options.map((o) => <option key={o.id} value={o.id}>{o.label}</option>)}
    </select>
  </label>
));

/** Miniature d'une slide rendue par la session ; image (le SVG n'est pas exécuté). */
const SlideThumb = createComponentImplementation(SlideThumbApi, ({ props }) => (
  <button className="slide-thumb" onClick={() => props.action?.()}>
    <img src={`data:image/svg+xml;utf8,${encodeURIComponent(props.svg)}`} alt={props.label} />
    <span className="caption">{props.label}</span>
  </button>
));

/** Un texte et ses changements, mot à mot. */
const DiffText = createComponentImplementation(DiffTextApi, ({ props }) => (
  <p className="diff-text">
    {props.segments.map((s, i) =>
      s.kind === "del" ? <del key={i}>{s.text}</del> : s.kind === "ins" ? <ins key={i}>{s.text}</ins> : <span key={i}>{s.text}</span>,
    )}
  </p>
));

/** Le catalogue des surfaces d'Oku Slides. */
export const catalog = new Catalog(CATALOG_ID, [
  Column, Row, Card, Text, TextField, Stepper, Button, ChoiceChips, Slider, ColorField, Select, SlideThumb, DiffText,
]);
```

Le `setValue` d'un composant dont la valeur n'est pas liée à un chemin (littéral) est sans effet dans `@a2ui/react` : c'est voulu.

`SurfaceView.tsx` (le contenu de `PanelSurface.tsx`, paramétré) :

```tsx
// Une surface A2UI (panneau de droite ou agent), rendue avec le catalogue Papier.
// Chaque nouvelle liste de messages recrée la surface.

import { useMemo, useRef } from "react";
import { A2uiSurface } from "@a2ui/react/v0_9";
import type { A2uiClientAction, A2uiMessage } from "@a2ui/web_core/v0_9";
import { MessageProcessor } from "@a2ui/web_core/v0_9";
import { catalog } from "./catalog";

export function SurfaceView({
  messages,
  surfaceId,
  onAction,
  onError,
}: {
  messages: A2uiMessage[];
  surfaceId: string;
  onAction: (action: A2uiClientAction) => void;
  onError: (message: string) => void;
}) {
  const handlers = useRef({ onAction, onError });
  handlers.current = { onAction, onError };
  const surface = useMemo(() => {
    if (messages.length === 0) return null;
    const processor = new MessageProcessor([catalog], (action) => handlers.current.onAction(action));
    try {
      processor.processMessages(messages);
    } catch (e) {
      handlers.current.onError(String(e));
      return null;
    }
    return processor.model.surfacesMap.get(surfaceId) ?? null;
  }, [messages, surfaceId]);
  return surface ? <A2uiSurface surface={surface} /> : null;
}
```

`PanelSurface.tsx` : `export function PanelSurface(props) { return <SurfaceView surfaceId="panel" {...props} />; }` (types des props inchangés, `PANEL_SURFACE` gardé).

`styles.css` (à la fin, style Papier) :

```css
/* --- Surfaces de l'agent (catalogue v2) --------------------------------- */
.a2ui-row { display: flex; gap: 8px; flex-wrap: wrap; align-items: stretch; }
.chips { display: flex; flex-wrap: wrap; gap: 6px; }
.chip { border: 1px solid var(--accent-fill); color: var(--accent); background: transparent; border-radius: 999px; padding: 3px 10px; cursor: pointer; }
.chip.on { background: var(--accent-fill); color: var(--paper-50, #fff); }
.chip-key { opacity: .6; font-variant-numeric: tabular-nums; }
.swatches { display: flex; gap: 6px; align-items: center; }
.swatch { width: 20px; height: 20px; border-radius: 50%; border: 1px solid var(--ink-muted); cursor: pointer; }
.swatch.on { outline: 2px solid var(--accent-fill); outline-offset: 2px; }
.slide-thumb { display: flex; flex-direction: column; gap: 4px; border: 0; background: none; cursor: pointer; padding: 0; }
.slide-thumb img { width: 100%; aspect-ratio: 16 / 9; border-radius: var(--radius-sm, 3px); box-shadow: 0 1px 3px rgb(0 0 0 / .2); background: #fff; }
.slide-thumb:focus-visible img, .slide-thumb:hover img { outline: 2px solid var(--accent-fill); }
.diff-text { margin: 0; line-height: 1.5; }
.diff-text del { color: var(--danger, #a33); }
.diff-text ins { text-decoration: none; background: color-mix(in oklab, #2a7a3a 20%, transparent); }
```

Fluent `ui-agent-color-other` (ui.ftl, 20 locales) : en « Other colour », fr « Autre couleur ».

Remplacer les autres usages de `panelCatalog` (grep) par `catalog`.

- [ ] **Step 4 :** `cd ui && bun test && bunx tsc -b` ; `cargo nextest run -p oku-slides-i18n` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add ui/src/a2ui ui/src/styles.css crates/oku-slides-i18n
git commit -m "feat(ui) : catalogue A2UI v2 pour les surfaces de l'agent (t-116)"
```

---

### Task 19 : UI : micro en deux modes et fil de l'agent

**Files:**
- Modify: `ui/src/Dictation.tsx`, `ui/src/App.tsx`, `ui/src/styles.css`
- Create: `ui/src/agent/AgentThread.tsx`, `ui/src/agent/keys.ts`, `ui/src/agent/keys.test.ts`
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 locales)

**Interfaces:**
- Consumes: `agent`, `subscribeHost`, `threadReducer`, `surfacesReducer`, `byPlace`, `agentUi`, `SurfaceView`.
- Produces: `Dictation` : prop `onText(text: string, mode: DictationMode)` et `agentBusy: boolean`, `onAgentCancel()` ; `DictationMode = "text" | "agent"` ; `keys.ts` : `dictationShortcut(e: KeyboardEventLike, agentBusy: boolean): "text" | "agent" | "agent-cancel" | null` ; `AgentThread` (fil + surfaces `thread` et `postit`).

- [ ] **Step 1 : test qui échoue** — `ui/src/agent/keys.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { dictationShortcut } from "./keys";

const k = (key: string, mods: { meta?: boolean; ctrl?: boolean; shift?: boolean } = {}) => ({
  key, metaKey: !!mods.meta, ctrlKey: !!mods.ctrl, shiftKey: !!mods.shift,
});

describe("raccourcis de la dictée", () => {
  test("⌘D dicte du texte", () => expect(dictationShortcut(k("d", { meta: true }), false)).toBe("text"));
  test("⌘⇧D parle à l'agent", () => expect(dictationShortcut(k("D", { meta: true, shift: true }), false)).toBe("agent"));
  test("Ctrl⇧D aussi", () => expect(dictationShortcut(k("d", { ctrl: true, shift: true }), false)).toBe("agent"));
  test("⌘⇧D pendant un tour arrête l'agent", () =>
    expect(dictationShortcut(k("d", { meta: true, shift: true }), true)).toBe("agent-cancel"));
  test("D seul ne fait rien", () => expect(dictationShortcut(k("d"), false)).toBeNull());
});
```

- [ ] **Step 2 :** `cd ui && bun test src/agent/keys` → échec.

- [ ] **Step 3 : implémentation.**

`ui/src/agent/keys.ts` :

```ts
// Raccourcis de la dictée : ⌘D texte, ⌘⇧D agent (ou arrêt pendant un tour).

export type DictationMode = "text" | "agent";

export interface KeyboardEventLike {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
}

export function dictationShortcut(e: KeyboardEventLike, agentBusy: boolean): DictationMode | "agent-cancel" | null {
  if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== "d") return null;
  if (!e.shiftKey) return "text";
  return agentBusy ? "agent-cancel" : "agent";
}
```

`Dictation.tsx` :
- Props : `onText: (text: string, mode: DictationMode) => void` ; `agentBusy: boolean` ; `onAgentCancel: () => void`.
- Un `mode` d'état (`useState<DictationMode>("text")`) et une ref `modeRef` lue dans `receive` : `handlers.current.onText(s.text, modeRef.current)`.
- `toggle` devient `toggle(mode: DictationMode)` : au repos, `setMode(mode); modeRef.current = mode;` puis `stt.start(...)` ; pendant l'écoute, `stt.stop()`.
- Le gestionnaire clavier utilise `dictationShortcut(e, agentBusy)` : `"text"` / `"agent"` → `toggle(mode)` ; `"agent-cancel"` → `onAgentCancel()` ; toujours `preventDefault` + `stopPropagation` quand le résultat n'est pas `null`.
- Rendu : la classe du boîtier devient `` `device object dictation ${state}${mode === "agent" ? " agent" : ""}` `` ; à côté du bouton micro, un second bouton `className="mic agent-mic"`, `aria-label={t("ui-dictation-agent-start")}`, `title="⌘⇧D"`, `onClick={() => agentBusy ? onAgentCancel() : toggle("agent")}`, icône :

```tsx
const AgentIcon = (
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3l1.8 4.7L18.5 9.5l-4.7 1.8L12 16l-1.8-4.7L5.5 9.5l4.7-1.8z" /><path d="M18 15l.8 2.2L21 18l-2.2.8L18 21l-.8-2.2L15 18l2.2-.8z" /></svg>
);
```

- Au repos en mode agent, la ligne d'état dit `t("ui-dictation-agent-idle")` au lieu de la destination ; pendant un tour (`agentBusy`), `t("ui-dictation-agent-working")`.

`ui/src/agent/AgentThread.tsx` :

```tsx
// Le fil de l'agent dans le panneau de droite : consignes, outils, plan,
// réponses, et les surfaces qui y vivent (questions sans ancre, paramètres).

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { lazy, Suspense } from "react";
import type { Surface } from "../api";
import { t } from "../i18n";
import type { Thread } from "./thread";

const SurfaceView = lazy(() => import("../a2ui/SurfaceView").then((m) => ({ default: m.SurfaceView })));

const TOOL_MARK: Record<string, string> = { pending: "☐", in_progress: "◌", completed: "☑", failed: "✕" };

export function AgentThread({
  thread,
  surfaces,
  onAction,
  onNewConversation,
  onError,
}: {
  thread: Thread;
  surfaces: Surface[];
  onAction: (surface: Surface, action: A2uiClientAction) => void;
  onNewConversation: () => void;
  onError: (message: string) => void;
}) {
  if (thread.entries.length === 0 && surfaces.length === 0) return null;
  return (
    <section className="agent-thread" aria-label={t("ui-agent-thread")}>
      <header className="agent-thread-head">
        <span className="caption">{t(`ui-agent-state-${thread.state}`)}</span>
        <button className="linkish" onClick={onNewConversation}>{t("ui-agent-new-conversation")}</button>
      </header>
      <ol className="agent-entries" aria-live="polite">
        {thread.entries.map((e, i) => (
          <li key={i} className={`agent-entry ${e.kind}`}>
            {e.kind === "user" && <span className="you">« {e.text} »</span>}
            {e.kind === "agent" && <span className="msg">{e.text}</span>}
            {e.kind === "tool" && <span className="tool">{TOOL_MARK[e.status] ?? "·"} {e.title}</span>}
            {e.kind === "plan" && (
              <ul className="plan">
                {e.entries.map((p, j) => <li key={j} className={p.status}>{TOOL_MARK[p.status] ?? "☐"} {p.content}</li>)}
              </ul>
            )}
            {e.kind === "error" && <span className="error" role="alert">{e.text}</span>}
            {e.kind === "stop" && <span className="caption">{t(e.stop === "cancelled" ? "ui-agent-stopped" : "ui-agent-ended", { stop: e.stop })}</span>}
          </li>
        ))}
      </ol>
      <Suspense fallback={null}>
        {surfaces.map((s) => (
          <div key={s.id} className={`agent-surface ${s.place}`}>
            <SurfaceView surfaceId={s.id} messages={s.messages} onAction={(a) => onAction(s, a)} onError={onError} />
          </div>
        ))}
      </Suspense>
    </section>
  );
}
```

`App.tsx` :

```tsx
import { useReducer } from "react";
import { agent } from "./api";
import type { AgentEvent, McpEvent, Surface } from "./api";
import { subscribeHost } from "./agent/events";
import { EMPTY_THREAD, threadReducer } from "./agent/thread";
import { byPlace, isAsking, surfacesReducer } from "./agent/surfaces";
import { agentUi } from "./agent/ui";
import { AgentThread } from "./agent/AgentThread";
import type { DictationMode } from "./agent/keys";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
```

Dans le composant :

```tsx
  // --- Agent ------------------------------------------------------------------
  const [thread, onAgentEvent] = useReducer(threadReducer, EMPTY_THREAD);
  const [surfaces, onSurfaceEvent] = useReducer(surfacesReducer, []);
  // Pendant une question, ⌘⇧D répond (Stack::prompt) au lieu d'arrêter le tour.
  const agentBusy = thread.state !== "idle" && !isAsking(surfaces);
  useEffect(
    () =>
      subscribeHost(
        (e: McpEvent) => {
          onSurfaceEvent(e);
          if (e.kind === "changed") run(api.snapshot());
          if (e.kind === "focus") {
            setCurrent(e.slide);
            setSelected(e.element);
          }
        },
        (e: AgentEvent) => onAgentEvent(e),
      ),
    // S'abonner une fois ; `run` et les setters sont stables.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );
  const onSurfaceAction = (s: Surface, a: A2uiClientAction) => {
    agent.surfaceAction(s.id, a.name, (a.context ?? {}) as Record<string, unknown>).catch((err) => say(String(err), true));
  };
```

(`A2uiClientAction` expose le nom et le contexte déjà résolus ; si ses champs portent d'autres noms, s'aligner sur `panelActions.ts`, qui lit déjà ces actions.) `run` doit être une fonction stable : si elle ne l'est pas, passer par une ref (`const runRef = useRef(run); runRef.current = run;` et `runRef.current(api.snapshot())`).

`onDictated(text)` devient `onDictated(text, mode: DictationMode)` : si `mode === "agent"`, `agent.prompt(text, agentUi(current, selected, editing)).catch((err) => say(String(err), true)); return;` en tête ; le reste inchangé. Le rendu de `<Dictation>` passe `agentBusy={agentBusy}` et `onAgentCancel={() => agent.cancel().catch((err) => say(String(err), true))}`.

Dans `<aside className="notes">`, avant `PanelSurface` :

```tsx
          <AgentThread
            thread={thread}
            surfaces={[...byPlace(surfaces, "thread"), ...byPlace(surfaces, "postit")]}
            onAction={onSurfaceAction}
            onNewConversation={() => agent.newConversation().catch((err) => say(String(err), true))}
            onError={(m) => say(m, true)}
          />
```

`styles.css` :

```css
/* --- Fil de l'agent ------------------------------------------------------- */
.agent-thread { display: flex; flex-direction: column; gap: 6px; min-height: 0; overflow: auto; }
.agent-thread-head { display: flex; justify-content: space-between; align-items: baseline; }
.agent-entries { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
.agent-entry .you { color: var(--ink-muted); font-style: italic; }
.agent-entry .msg { display: block; background: var(--sheet, #fffdf7); border-inline-start: 3px solid var(--accent-fill); padding: 4px 6px; border-radius: 2px; }
.agent-entry .tool, .agent-entry .plan { color: var(--ink-muted); font-size: .9em; }
.agent-entry .plan { list-style: none; padding: 0; margin: 0; }
.agent-entry .error { color: var(--danger, #a33); }
.device.dictation.agent .mic, .mic.agent-mic { background: var(--accent-fill); color: var(--paper-50, #fff); }
```

Fluent (ui.ftl, 20 locales) :

| Clé | en | fr |
| --- | --- | --- |
| `ui-dictation-agent-start` | `Speak to the agent (⌘⇧D)` | `Parler à l'agent (⌘⇧D)` |
| `ui-dictation-agent-idle` | `Speak to the agent` | `Parler à l'agent` |
| `ui-dictation-agent-working` | `The agent is working… ⌘⇧D or Esc stops it` | `L'agent travaille… ⌘⇧D ou Échap l'arrête` |
| `ui-agent-thread` | `Agent` | `Agent` |
| `ui-agent-new-conversation` | `New conversation` | `Nouvelle conversation` |
| `ui-agent-state-idle` | `Agent` | `Agent` |
| `ui-agent-state-starting` | `Starting the agent…` | `L'agent démarre…` |
| `ui-agent-state-working` | `The agent is working…` | `L'agent travaille…` |
| `ui-agent-stopped` | `Stopped.` | `Arrêté.` |
| `ui-agent-ended` | `Turn ended ({ $stop }).` | `Tour terminé ({ $stop }).` |

`ui/src/i18n-keys.test.ts` exige que chaque clé `ui-…` soit écrite **en littéral** dans le code : `t(\`ui-agent-state-${thread.state}\`)` ne compte pas. Écrire à la place `const STATE_KEY = { idle: "ui-agent-state-idle", starting: "ui-agent-state-starting", working: "ui-agent-state-working" } as const;` et `t(STATE_KEY[thread.state])`.

- [ ] **Step 4 :** `cd ui && bun test && bunx tsc -b` ; `cargo nextest run -p oku-slides-i18n` → tout passe. Vérifier dans le navigateur (`just web` + `oku-slides serve` avec le faux agent, comme en Task 16) : ⌘⇧D passe le micro en couleur d'accent ; une consigne envoyée par `curl` s'affiche dans le fil avec la réponse du faux agent.
- [ ] **Step 5 : commit**

```bash
git add ui/src crates/oku-slides-i18n
git commit -m "feat(ui) : micro en deux modes et fil de l'agent dans le panneau (t-117)"
```

---

### Task 20 : UI : fiche, tiroir, post-it et fenêtre

**Files:**
- Create: `ui/src/agent/quick.ts`, `ui/src/agent/quick.test.ts`, `ui/src/agent/useSurfaceKeys.ts`, `ui/src/agent/AgentSheet.tsx`, `ui/src/agent/VariantDrawer.tsx`, `ui/src/agent/ConfirmModal.tsx`
- Modify: `ui/src/Stage.tsx` (prop `overlay`), `ui/src/App.tsx`, `ui/src/styles.css`

**Interfaces:**
- Consumes: `Surface`, `SurfaceView`, `agent.surfaceAction`, `byPlace`.
- Produces : `quick.ts` : `quickAction(surface: Surface, key: string): { name: string; context: Record<string, unknown> } | null` (`1`–`4`, `Enter`, `Escape`) ; `sheetPosition(target: { x: number; y: number; w: number; h: number }, scale: number, rtl: boolean): { left: number; top: number }` ; `Stage` prop `overlay?: { target: string; node: ReactNode }`.

- [ ] **Step 1 : tests qui échouent** — `ui/src/agent/quick.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { quickAction, sheetPosition } from "./quick";
import type { Surface } from "../api";

const surface = (components: unknown[]): Surface => ({
  id: "ask-1",
  place: "sheet",
  messages: [
    { version: "v0.9", createSurface: { surfaceId: "ask-1", catalogId: "x" } },
    { version: "v0.9", updateComponents: { surfaceId: "ask-1", components } },
  ] as Surface["messages"],
});

const choice = surface([
  { id: "options", component: "ChoiceChips", options: [{ id: "a", label: "A" }, { id: "b", label: "B" }], value: { path: "/choice" } },
]);

const rewrite = surface([
  { id: "pick-0", component: "Button", label: "Choisir 1", primary: true, action: { event: { name: "rewrite", context: { kept: 0, text: "Un" } } } },
  { id: "pick-1", component: "Button", label: "Choisir 2", action: { event: { name: "rewrite", context: { kept: 1, text: "Deux" } } } },
]);

const variants = surface([
  { id: "v0", component: "SlideThumb", svg: "", label: "1", action: { event: { name: "variant", context: { index: 0 } } } },
  { id: "v1", component: "SlideThumb", svg: "", label: "2", action: { event: { name: "variant", context: { index: 1 } } } },
]);

describe("touches des surfaces", () => {
  test("un chiffre choisit l'option correspondante", () => {
    expect(quickAction(choice, "2")).toEqual({ name: "choose", context: { choice: "b" } });
  });
  test("un chiffre choisit la proposition de reformulation", () => {
    expect(quickAction(rewrite, "2")).toEqual({ name: "rewrite", context: { kept: 1, text: "Deux" } });
  });
  test("un chiffre choisit la variante", () => {
    expect(quickAction(variants, "1")).toEqual({ name: "variant", context: { index: 0 } });
  });
  test("Entrée déclenche le bouton principal", () => {
    expect(quickAction(rewrite, "Enter")).toEqual({ name: "rewrite", context: { kept: 0, text: "Un" } });
  });
  test("Échap abandonne", () => expect(quickAction(choice, "Escape")).toEqual({ name: "dismiss", context: {} }));
  test("un chiffre hors des options ne fait rien", () => expect(quickAction(choice, "4")).toBeNull());
});

describe("position de la fiche", () => {
  test("sous la boîte, alignée à gauche, à l'échelle", () => {
    expect(sheetPosition({ x: 100, y: 200, w: 400, h: 80 }, 0.5, false)).toEqual({ left: 50, top: 148 });
  });
  test("en arabe, alignée sur le bord droit de la boîte", () => {
    expect(sheetPosition({ x: 100, y: 200, w: 400, h: 80 }, 0.5, true)).toEqual({ left: 250, top: 148 });
  });
});
```

- [ ] **Step 2 :** `cd ui && bun test src/agent/quick` → échec.

- [ ] **Step 3 : implémentation.**

`ui/src/agent/quick.ts` :

```ts
// Clavier des surfaces de l'agent : 1–4 choisissent, Entrée valide, Échap abandonne.
// Les actions sont lues dans les composants de la surface (littéraux seulement).

import type { Surface } from "../api";

type Action = { name: string; context: Record<string, unknown> };
type Component = { id: string; component: string; [k: string]: unknown };

/** Écart sous l'élément (px). */
const GAP = 8;

function components(s: Surface): Component[] {
  for (const m of s.messages as unknown as { updateComponents?: { components: Component[] } }[]) {
    if (m.updateComponents) return m.updateComponents.components;
  }
  return [];
}

function literal(c: Component | undefined): Action | null {
  const ev = (c?.action as { event?: { name: string; context?: Record<string, unknown> } } | undefined)?.event;
  if (!ev) return null;
  const context = Object.fromEntries(Object.entries(ev.context ?? {}).filter(([, v]) => !(v && typeof v === "object" && "path" in v)));
  return { name: ev.name, context };
}

export function quickAction(surface: Surface, key: string): Action | null {
  const cs = components(surface);
  if (key === "Escape") return { name: "dismiss", context: {} };
  if (key === "Enter") return literal(cs.find((c) => c.component === "Button" && c.primary === true));
  const n = Number(key);
  if (!Number.isInteger(n) || n < 1 || n > 4) return null;
  const chips = cs.find((c) => c.component === "ChoiceChips");
  if (chips) {
    const o = (chips.options as { id: string }[])[n - 1];
    return o ? { name: "choose", context: { choice: o.id } } : null;
  }
  return literal(cs.find((c) => c.id === `pick-${n - 1}`)) ?? literal(cs.find((c) => c.id === `v${n - 1}`));
}

/** Coin de la fiche sous `target` ; en droite à gauche, `left` est le bord droit (la fiche est posée avec `translateX(-100%)`). */
export function sheetPosition(target: { x: number; y: number; w: number; h: number }, scale: number, rtl: boolean) {
  return { left: (rtl ? target.x + target.w : target.x) * scale, top: (target.y + target.h) * scale + GAP };
}
```

En droite à gauche, la fiche reçoit la classe `rtl` (`transform: translateX(-100%)`).

`Stage.tsx` : prop `overlay?: { target: string; node: ReactNode }` ; dans le rendu, à l'intérieur du conteneur mis à l'échelle (là où les poignées des éléments sont posées en pixels écran), si `overlay` et la cible existe dans `view.targets` :

```tsx
        {overlay && (() => {
          const tg = view.targets.find((x) => x.id === overlay.target);
          if (!tg) return null;
          const rtl = document.documentElement.dir === "rtl";
          const pos = sheetPosition(tg, scale, rtl);
          return (
            <div className={`agent-sheet${rtl ? " rtl" : ""}`} style={{ left: pos.left, top: pos.top }}>
              {overlay.node}
            </div>
          );
        })()}
```

(placer ce bloc dans le même élément positionné que les cadres de sélection, pour partager l'origine ; importer `sheetPosition` depuis `./agent/quick`).

La gestion du focus et des touches est commune aux trois composants : `ui/src/agent/useSurfaceKeys.ts` :

```ts
// Focus gardé dans une surface de l'agent et touches rapides (1–4, Entrée, E, Échap).
import { useEffect, useRef } from "react";
import type { Surface } from "../api";
import { quickAction } from "./quick";

export type QuickAction = { name: string; context: Record<string, unknown> };

export function useSurfaceKeys(surface: Surface, onAction: (a: QuickAction) => void) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    return () => previous?.focus();
  }, []);
  const onKeyDown = (e: React.KeyboardEvent) => {
    if ((e.target as HTMLElement).tagName === "INPUT") return;
    if (e.key.toLowerCase() === "e") {
      const input = ref.current?.querySelector("input");
      if (input) {
        e.preventDefault();
        input.focus();
        return;
      }
    }
    const a = quickAction(surface, e.key);
    if (a) {
      e.preventDefault();
      e.stopPropagation();
      onAction(a);
    }
  };
  return { ref, onKeyDown, tabIndex: -1 };
}
```

`ui/src/agent/AgentSheet.tsx` :

```tsx
// Fiche d'une question ancrée (choix, reformulation).

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "../api";
import { SurfaceView } from "../a2ui/SurfaceView";
import { t } from "../i18n";
import { type QuickAction, useSurfaceKeys } from "./useSurfaceKeys";

export interface SurfaceProps {
  surface: Surface;
  onAction: (surface: Surface, action: QuickAction) => void;
  onError: (message: string) => void;
}

/** L'action A2UI d'un composant, au format des actions rapides. */
export const fromA2ui = (a: A2uiClientAction): QuickAction => ({ name: a.name, context: (a.context ?? {}) as Record<string, unknown> });

export function AgentSheet({ surface, onAction, onError }: SurfaceProps) {
  const keys = useSurfaceKeys(surface, (a) => onAction(surface, a));
  return (
    <div {...keys} className="agent-sheet-box object" role="dialog" aria-label={t("ui-agent-sheet")}>
      <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={(a) => onAction(surface, fromA2ui(a))} onError={onError} />
      <p className="caption">{t("ui-agent-sheet-keys")}</p>
    </div>
  );
}
```

`ui/src/agent/VariantDrawer.tsx` :

```tsx
// Tiroir des variantes, sous la scène.

import { SurfaceView } from "../a2ui/SurfaceView";
import { t } from "../i18n";
import { type SurfaceProps, fromA2ui } from "./AgentSheet";
import { useSurfaceKeys } from "./useSurfaceKeys";

export function VariantDrawer({ surface, onAction, onError }: SurfaceProps) {
  const keys = useSurfaceKeys(surface, (a) => onAction(surface, a));
  return (
    <div {...keys} className="variant-drawer object" role="dialog" aria-label={t("ui-agent-variants")}>
      <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={(a) => onAction(surface, fromA2ui(a))} onError={onError} />
      <p className="caption">{t("ui-agent-drawer-keys")}</p>
    </div>
  );
}
```

`ui/src/agent/ConfirmModal.tsx` :

```tsx
// Fenêtre de confirmation (ask_confirm, outils non annulables, permissions de l'agent).

import { SurfaceView } from "../a2ui/SurfaceView";
import { type SurfaceProps, fromA2ui } from "./AgentSheet";
import { useSurfaceKeys } from "./useSurfaceKeys";

export function ConfirmModal({ surface, onAction, onError }: SurfaceProps) {
  const keys = useSurfaceKeys(surface, (a) => onAction(surface, a));
  return (
    <div className="agent-veil">
      <div {...keys} className="agent-modal object" role="alertdialog" aria-modal="true">
        <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={(a) => onAction(surface, fromA2ui(a))} onError={onError} />
      </div>
    </div>
  );
}
```

`App.tsx` :

```tsx
  const sheet = byPlace(surfaces, "sheet").find((s) => s.anchor?.slide === current);
  const drawer = byPlace(surfaces, "drawer").at(-1);
  const modal = byPlace(surfaces, "modal").at(-1);
  const sendAction = (s: Surface, a: { name: string; context: Record<string, unknown> }) =>
    agent.surfaceAction(s.id, a.name, a.context).catch((err) => say(String(err), true));
```

- `<Stage … overlay={sheet?.anchor ? { target: sheet.anchor.element, node: <AgentSheet surface={sheet} onAction={sendAction} onError={(m) => say(m, true)} /> } : undefined} />` ;
- dans `<main className="sheet-area">`, après la scène : `{drawer && <VariantDrawer surface={drawer} onAction={sendAction} onError={(m) => say(m, true)} />}` ;
- à la fin du composant racine : `{modal && <ConfirmModal surface={modal} onAction={sendAction} onError={(m) => say(m, true)} />}` ;
- une fiche dont la slide n'est pas affichée apparaît dans le fil : `AgentThread` reçoit aussi `byPlace(surfaces, "sheet").filter((s) => s.anchor?.slide !== current)`.
- `onSurfaceAction` de la Task 19 devient `(s, a) => sendAction(s, fromA2ui(a))` (importer `fromA2ui` depuis `./agent/AgentSheet`).

`styles.css` :

```css
.agent-sheet { position: absolute; z-index: 5; width: min(320px, 60%); }
.agent-sheet.rtl { transform: translateX(-100%); }
.agent-sheet-box, .variant-drawer, .agent-modal { background: var(--sheet, #fffdf7); border-radius: var(--radius-md, 6px); box-shadow: 0 6px 20px rgb(0 0 0 / .25); padding: 10px; outline: none; }
.variant-drawer { position: absolute; inset-inline: 16px; bottom: 0; z-index: 4; border-end-start-radius: 0; border-end-end-radius: 0; }
.variant-drawer .a2ui-row > * { flex: 1 1 0; min-width: 0; }
.agent-veil { position: fixed; inset: 0; background: rgb(0 0 0 / .35); display: grid; place-items: center; z-index: 20; }
.agent-modal { width: min(420px, 90vw); }
```

Fluent (ui.ftl, 20 locales) : `ui-agent-sheet` en « Agent question », fr « Question de l'agent » ; `ui-agent-sheet-keys` en « 1–3 choose · Enter confirm · E edit · Esc cancel », fr « 1–3 choisir · Entrée valider · E retoucher · Échap abandonner » ; `ui-agent-variants` en « Variants », fr « Variantes » ; `ui-agent-drawer-keys` en « Click or 1–4 · Esc closes », fr « Clic ou 1–4 · Échap ferme ».

- [ ] **Step 4 :** `cd ui && bun test && bunx tsc -b` ; `cargo nextest run -p oku-slides-i18n` → tout passe. Dans le navigateur (serveur de Task 16 avec un scénario qui appelle `ask_rewrite` puis `show_variants`, par exemple `fixtures/agent/e2e.json` de la Task 21) : la fiche apparaît sous le texte visé, `2` choisit la seconde proposition, le tiroir montre les variantes rendues, `ask_confirm` ouvre la fenêtre modale ; capture d'écran à l'appui.
- [ ] **Step 5 : commit**

```bash
git add ui/src crates/oku-slides-i18n
git commit -m "feat(ui) : fiche ancrée, tiroir des variantes, post-its et fenêtre de confirmation (t-118)"
```

---
### Task 21 : Bout à bout, documentation et vérification

**Files:**
- Create: `fixtures/agent/e2e.json`, `ui/e2e/agent.cjs`
- Modify: `ui/package.json` (script `e2e`), `justfile` (`e2e`), `README.md`

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces: `just e2e` couvre l'agent (faux agent) ; README à jour.

- [ ] **Step 1 : le scénario et le test.**

`fixtures/agent/e2e.json` :

```json
{
  "http": true,
  "turns": [
    {
      "plan": [{ "content": "Lire la slide", "status": "completed" }, { "content": "Proposer des reformulations", "status": "in_progress" }],
      "tool_calls": [{ "id": "t1", "title": "Lire la slide" }],
      "mcp": [
        { "tool": "focus", "args": { "slide": "s-libre", "element": "e-texte" } },
        { "tool": "ask_rewrite", "args": { "slide": "s-libre", "element": "e-texte", "proposals": ["Un bloc bien placé", "Un bloc libre"] } }
      ],
      "chunks": ["Texte reformulé."]
    },
    {
      "mcp": [
        { "tool": "show_variants", "args": { "slide": "s-libre", "variants": [
          { "label": "Un", "ops": [{ "op": "set_text", "slide": "s-libre", "element": "e-texte", "text": [{ "runs": [{ "t": "Variante un" }] }] }] },
          { "label": "Deux", "ops": [{ "op": "set_text", "slide": "s-libre", "element": "e-texte", "text": [{ "runs": [{ "t": "Variante deux" }] }] }] }
        ] } }
      ],
      "chunks": ["Voici deux variantes."]
    }
  ]
}
```

`ui/e2e/agent.cjs` :

```js
// Agent (faux agent ACP) : reformulation dans une fiche, variantes dans le tiroir, puis annulation.
// Contre `oku-slides serve fixtures/roadmap.json` lancé avec OKU_SLIDES_AGENT__COMMAND = faux agent
// (just e2e s'en charge).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

const api = (p, req) => p.evaluate((r) => fetch('/api', { method: 'POST', body: JSON.stringify(r) }).then((x) => x.json()), req);
const prompt = (p, text) =>
  p.evaluate((t) => fetch('/api/agent', { method: 'POST', body: JSON.stringify({ cmd: 'prompt', text: t, ui: {} }) }), text);
const textOf = async (p) => (await api(p, { cmd: 'slide', id: 's-libre' })).targets.find((t) => t.id === 'e-texte').text;

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  const original = await textOf(p);

  // 1. Reformulation : la fiche s'ouvre sous le texte, « 2 » choisit la seconde proposition.
  await prompt(p, 'rends ce texte plus clair');
  await p.waitForSelector('.agent-sheet .diff-text');
  assert.equal(await p.locator('.agent-sheet .diff-text').count(), 2);
  await p.keyboard.press('2');
  await p.waitForSelector('.agent-sheet', { state: 'detached' });
  await p.getByText('Texte reformulé.').waitFor();
  assert.equal(await textOf(p), 'Un bloc libre');
  assert.equal((await api(p, { cmd: 'snapshot' })).undo_label, 'Agent : rends ce texte plus clair');

  // 2. Variantes : le tiroir montre deux rendus ; cliquer la seconde l'applique.
  await prompt(p, 'propose deux variantes');
  await p.waitForSelector('.variant-drawer .slide-thumb');
  assert.equal(await p.locator('.variant-drawer .slide-thumb').count(), 2);
  await p.locator('.variant-drawer .slide-thumb').nth(1).click();
  await p.waitForSelector('.variant-drawer', { state: 'detached' });
  assert.equal(await textOf(p), 'Variante deux');
  assert.equal((await api(p, { cmd: 'snapshot' })).undo_label, 'Variante : Deux');

  // 3. Deux annulations : la variante, puis tout le premier tour.
  await p.locator('.stage').click({ position: { x: 5, y: 5 } });
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(500);
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(700);
  assert.equal(await textOf(p), original);

  assert.deepEqual(errors, []);
  await b.close();
  console.log('agent : ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

(L'interface accepte ⌘Z et Ctrl+Z : `e.metaKey || e.ctrlKey`.)

`ui/package.json` : `"e2e": "bun e2e/toolbar.cjs && bun e2e/panel.cjs && bun e2e/agent.cjs"`.

`justfile`, recette `e2e` : construire le faux agent et le passer au serveur :

```make
    cargo build -q -p oku-slides-cli -p oku-slides-agent --bin oku-slides --bin oku-slides-fake-agent
    OKU_SLIDES_AGENT__COMMAND="$PWD/target/debug/oku-slides-fake-agent" \
    OKU_SLIDES_AGENT__ARGS="--script $PWD/fixtures/agent/e2e.json" \
      ./target/debug/oku-slides --fonts {{fonts}} serve "{{deck}}" --ui ui/dist --port 4872 &
```

(à la place des deux lignes `cargo build -q -p oku-slides-cli` et `./target/debug/oku-slides … &` actuelles ; le binaire de la CLI s'appelle `oku-slides`).

- [ ] **Step 2 :** `just e2e` → `agent.cjs` échoue tant que rien n'est branché ; si toutes les tâches précédentes sont faites, il doit passer. S'il échoue, diagnostiquer (G13–G15 : test qui reproduit, traces, une correction par diagnostic).

- [ ] **Step 3 : documentation.**

`README.md` : dans le tableau « Contenu du dépôt », ajouter

```markdown
| `crates/oku-slides-mcp` | Outils MCP de l'agent (HTTP local, jeton) : lecture, `apply`, surfaces, confirmations |
| `crates/oku-slides-agent` | Agent vocal : client ACP (agent en sous-processus), tours, pile pour l'app et le serveur ; faux agent de test |
```

et, après la section « Dictée », une section :

```markdown
### Agent (⌘⇧D)

⌘D dicte du texte ; **⌘⇧D parle à un agent** qui modifie la présentation.
La consigne est transcrite sur la machine (Whisper), puis envoyée par
[ACP](https://agentclientprotocol.com) à un agent lancé par Oku Slides. Il
n'a ni fichiers ni terminal : il n'agit que par les outils d'Oku Slides,
servis en MCP sur `127.0.0.1` avec un jeton propre à chaque lancement.

- **Tout s'annule** : un tour de l'agent est une seule entrée ⌘Z.
- **Ce qui ne s'annule pas** (enregistrer, ouvrir, supprimer un ensemble de
  modèles, changer un réglage) attend votre confirmation.
- **Décisions** : l'agent pose ses questions dans une fiche sous l'élément
  (choix, reformulations — `1`–`3`, Entrée, `E` pour retoucher, Échap), propose
  des variantes rendues dans un tiroir sous la scène, et des réglages en direct
  dans un post-it. On peut répondre à une question à la voix (⌘⇧D).
- **Conversation** : une par document, jamais enregistrée ; « Nouvelle
  conversation » repart de zéro.

| Réglage | Défaut |
| --- | --- |
| `[agent] command` / `OKU_SLIDES_AGENT__COMMAND` | `npx` |
| `[agent] args` / `OKU_SLIDES_AGENT__ARGS` (séparés par des espaces) | `-y @agentclientprotocol/claude-agent-acp@0.81.2` |

N'importe quel agent ACP qui accepte des serveurs MCP en HTTP convient.

```sh
just agent-smoke fixtures/roadmap.json "Rends le titre de la slide 2 plus percutant"   # vrai agent
```
```

- [ ] **Step 4 : vérification complète** (superpowers:verification-before-completion) :

```bash
just check
just test
just e2e
cargo build -p oku-slides-app
```

Tout passe ; noter les sorties. Passe de consolidation (G10) : chercher les doublons (`grep -rn "fn session()" crates/*/tests` : les helpers de test par crate sont attendus ; aucun helper de production en double), vérifier qu'aucun `#[allow(dead_code)]` hors `tests/common` n'a été ajouté, que `CATALOG_ID` n'a qu'une valeur côté Rust et côté TS.

- [ ] **Step 5 : commit et clôture**

```bash
git add fixtures/agent ui/e2e/agent.cjs ui/package.json justfile README.md
git commit -m "test(e2e) : l'agent de bout en bout avec le faux agent ; README (t-119)"
oku cli task done --project slider --id t-119
```

La tâche parente t-89 se ferme quand la branche est intégrée (superpowers:finishing-a-development-branch).
