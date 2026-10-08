# Sessions de l'agent par perspective — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** une session de l'agent (fil + session ACP) par perspective — slides,
atelier d'un pack, chaque widget d'un pack — rappelée quand on change de
perspective et restaurée quand l'application est rouverte.

**Architecture :** une `AgentScope` (session) accompagne chaque consigne.
Le hub (`oku-slides-mcp`) range chaque tour dans sa portée et n'envoie que le
fil vu ; il ne touche jamais à la session. La session (`oku-slides-session`)
garde une conversation par portée (fil + id de session ACP + empreinte de
l'agent) dans `agent/<portée>.json` du dossier de reprise. Dans
`oku-slides-agent`, un **gardien** par onglet est le seul à écrire ces
conversations (file d'ordres, appliqués un par un) ; l'**acteur** ne garde
que l'état vivant des sessions ACP ; il demande au gardien la session
d'une portée, puis l'ouvre, la reprend (`session/resume`, `session/load`) ou
en crée une. L'interface calcule la portée
et la donne à chaque commande.

**Tech Stack :** Rust (tokio, serde, snafu, speculoos, nextest),
`agent-client-protocol` 2.2 (schéma 1.9.1, `LoadSession*`/`ResumeSession*`
stables), TypeScript/React (bun test, happy-dom), Playwright (e2e), Fluent.

**Spec :** `docs/superpowers/specs/2026-10-08-sessions-par-perspective-design.md`
(tâche t-598, révisée après la revue d'équipe du plan).

## Global Constraints

- Commits : Conventional Commits en français, `(t-598)` en fin de titre, **jamais**
  de ligne `Co-Authored-By`. `git add` **fichier par fichier** (d'autres
  sessions modifient le dépôt : jamais `git add ui/src` ni un dossier entier),
  puis `git diff --cached --name-only` juste avant `git commit`.
- **Chaque commit compile tout le workspace** : l'étape de vérification de
  chaque tâche lance `cargo build --workspace --all-targets` en plus des tests.
- TDD : test rouge, minimum, vert. Tests Rust côte à côte (`foo_tests.rs`,
  `#[path]`), speculoos ; nom `<unité>_<scénario>_<résultat>` ; tests TS
  `"<unité> <scénario> <résultat>"`.
- Pas de `.unwrap()`/`.expect()` hors tests ; erreurs Snafu ; un échec
  d'écriture ou de lecture de conversation se journalise en `warn!`, jamais
  avalé (`let _ =` interdit sur un `Result` ou un `JoinHandle` attendu).
- Doc : première phrase < 15 mots ; tout élément `pub` documenté.
- Fichiers : 500 lignes visées, 1000 au plus (`App.tsx` fait 971 lignes :
  la logique nouvelle va dans `ui/src/agent/useAgentScope.ts`).
- Chaînes visibles : Fluent, **20** langues
  (`crates/oku-slides-i18n/src/locales/{ar,bn,de,en,es,fr,hi,it,ja,ko,nl,pl,pt,ru,th,tr,uk,vi,zh-Hans,zh-Hant}`),
  clés au même endroit et dans le même ordre dans chaque fichier.
- App ou e2e : `OKU_SLIDES_PATHS__DATA` / `OKU_SLIDES_PATHS__DECKS` vers un
  dossier temporaire ; e2e sur le port 4872, ou un port libre s'il est pris.
- JSON de portée (Rust ↔ UI) : `{"kind":"slides"}`,
  `{"kind":"atelier","pack":"<uuid>"}`, `{"kind":"widget","pack":"<uuid>","name":"<nom>"}`.

## Fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-a2ui/src/message.rs` | `Deserialize` sur `Version`, `ServerMessage`, `ServerBody` et ses messages |
| `crates/oku-slides-session/src/agent/scope.rs` (nouveau) | `AgentScope`, `file_name` |
| `crates/oku-slides-session/src/agent/conversation.rs` (nouveau) | `SavedConversation`, `ScopeMove`, `Conversations`, méthodes de `Session` |
| `crates/oku-slides-session/src/resume_agent.rs` (nouveau) | fichiers `agent/*.json` de `Resume` |
| `crates/oku-slides-session/src/agent/forms.rs`, `mod.rs` | `Deserialize` sur `Surface`/`Place`, `KEPT_TURNS` |
| `crates/oku-slides-session/src/resume.rs`, `resume_writer.rs` | `persist_with`, relecture, nettoyage, `Job.agent`, `merge` |
| `crates/oku-slides-session/src/widgets_edit.rs` | renommage → conversation |
| `crates/oku-slides-mcp/src/hub.rs`, `hub_turns.rs`, `hub_scopes.rs` (nouveau) | portées du fil, sans entrées-sorties |
| `crates/oku-slides-agent/src/events.rs`, `turns.rs` | portée dans les évènements, éviction par portée |
| `crates/oku-slides-agent/src/link.rs`, `bin/fake_agent.rs` | capacités, empreinte d'agent, faux `load`/`resume` |
| `crates/oku-slides-agent/src/keeper.rs` (nouveau) | gardien de l'onglet |
| `crates/oku-slides-agent/src/tab_agent.rs`, `turn.rs`, `stack.rs`, `tab_entry.rs`, `examples/smoke.rs` | portées de bout en bout |
| `ui/src/agent/scope.ts`, `useAgentScope.ts` (nouveaux), `ui/src/api.ts`, `ui/src/App.tsx`, `ui/src/DictationDraft.tsx` | portée côté interface |
| `ui/e2e/http.cjs`, `ui/e2e/agent-scopes.cjs` (nouveaux), `fixtures/agent/e2e.json` | e2e |

---

### Task 1 : Portée et surfaces relisibles

**Files :**
- Modify : `crates/oku-slides-a2ui/src/message.rs:12-70` ; Test : `message_tests.rs`
- Create : `crates/oku-slides-session/src/agent/scope.rs`, `scope_tests.rs`
- Modify : `crates/oku-slides-session/src/agent/mod.rs`, `forms.rs:22-52`, `crates/oku-slides-session/Cargo.toml`
- Modify : `crates/oku-slides-agent/src/turns.rs:27` (`KEPT_TURNS` importé)
- Test : `crates/oku-slides-session/src/agent/forms_tests.rs`

**Interfaces :**
- Produces : `oku_slides_session::agent::AgentScope { Slides, Atelier { pack: Uuid }, Widget { pack: Uuid, name: String } }`
  (`Default` = `Slides`, `Ord`, `Hash`, serde `tag = "kind"`), `AgentScope::file_name(&self) -> String` ;
  `oku_slides_session::agent::KEPT_TURNS: usize = 50` ; `Surface: Deserialize`.

- [ ] **Step 1 : tests rouges**

`crates/oku-slides-a2ui/src/message_tests.rs` (ajouter) :

```rust
#[test]
fn server_message_json_round_trips() {
    let m = ServerMessage {
        version: Version::V0_9,
        body: ServerBody::UpdateDataModel(UpdateDataModel {
            surface_id: "s-1".into(),
            path: Some("/narration".into()),
            value: Some(serde_json::json!("bonjour")),
        }),
    };
    let back: ServerMessage = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_that!(back).is_equal_to(&m);
}
```

`crates/oku-slides-session/src/agent/scope_tests.rs` :

```rust
use speculoos::prelude::*;
use uuid::Uuid;

use super::AgentScope;

const PACK: Uuid = Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);

#[test]
fn scope_json_matches_the_interface_shape() {
    assert_that!(serde_json::to_string(&AgentScope::Slides).unwrap()).is_equal_to(r#"{"kind":"slides"}"#.to_owned());
    let widget = AgentScope::Widget { pack: PACK, name: "frise".into() };
    assert_that!(serde_json::to_string(&widget).unwrap()).is_equal_to(
        r#"{"kind":"widget","pack":"01234567-89ab-cdef-0123-456789abcdef","name":"frise"}"#.to_owned(),
    );
    let back: AgentScope = serde_json::from_str(r#"{"kind":"atelier","pack":"01234567-89ab-cdef-0123-456789abcdef"}"#).unwrap();
    assert_that!(back).is_equal_to(AgentScope::Atelier { pack: PACK });
}

#[test]
fn scope_file_name_of_a_widget_has_a_fixed_safe_length() {
    let long = AgentScope::Widget { pack: PACK, name: format!("../{}", "é".repeat(300)) }.file_name();
    let short = AgentScope::Widget { pack: PACK, name: "a".into() }.file_name();
    assert_that!(long.len()).is_equal_to(short.len());
    assert_that!(long.chars().all(|c| c.is_ascii_alphanumeric() || c == '.')).is_true();
    assert_that!(long.len()).is_less_than(100);
}

#[test]
fn scope_file_name_differs_per_pack_and_kind() {
    let other = Uuid::from_u128(7);
    assert_that!(AgentScope::Slides.file_name()).is_equal_to("slides.json".to_owned());
    assert_that!(AgentScope::Atelier { pack: PACK }.file_name()).is_not_equal_to(AgentScope::Atelier { pack: other }.file_name());
    assert_that!(AgentScope::Widget { pack: PACK, name: "a".into() }.file_name())
        .is_not_equal_to(AgentScope::Widget { pack: other, name: "a".into() }.file_name());
}
```

`forms_tests.rs` (ajouter ; `confirm`, `AskConfirm`, `Surface`, `Localizer` sont
déjà importés par ce fichier, sinon les ajouter au `use super::…` existant) :

```rust
#[test]
fn surface_json_round_trips() {
    let l = Localizer::new(Some("fr"));
    let s = confirm("c-1", &AskConfirm { message: "Sûr ?".into(), danger: false }, &l);
    let back: Surface = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_that!(back).is_equal_to(&s);
}
```

- [ ] **Step 2 : lancer, constater l'échec**

Run : `cargo nextest run -p oku-slides-a2ui -p oku-slides-session -E 'test(round_trips) | test(scope_)'`
Expected : erreurs de compilation (`Deserialize`, module `scope` absents).

- [ ] **Step 3 : implémenter**

`message.rs` : ajouter `Deserialize` aux dérivés de `Version`, `ServerMessage`,
`ServerBody`, `CreateSurface`, `UpdateComponents`, `UpdateDataModel`,
`DeleteSurface` (`use serde::{Deserialize, Serialize};`). Sur
`UpdateDataModel::value`, ajouter le commentaire : `// Relu : un `null` explicite
devient `None` (effacer) ; l'hôte n'envoie jamais `Some(Null)`.`

`forms.rs` : `Deserialize` sur `Place` (`#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]`)
et sur `Surface`.

`crates/oku-slides-session/Cargo.toml`, `[dependencies]` : `hex.workspace = true`, `sha2.workspace = true`.

`crates/oku-slides-session/src/agent/scope.rs` :

```rust
//! La portée d'une conversation de l'agent.
//!
//! Les slides, l'atelier d'un pack, ou un widget d'un pack : chacune a son fil et sa session ACP.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Où l'utilisateur parle à l'agent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentScope {
    /// Le deck (et son historique).
    #[default]
    Slides,
    /// Le mode modèles du pack `pack` : « Modèles » et « Thème ».
    Atelier { pack: Uuid },
    /// Le sous-onglet du widget `name` du pack `pack`.
    Widget { pack: Uuid, name: String },
}

impl AgentScope {
    /// Nom du fichier de reprise ; longueur fixe, quel que soit le nom du widget.
    pub fn file_name(&self) -> String {
        match self {
            Self::Slides => "slides.json".to_owned(),
            Self::Atelier { pack } => format!("atelier.{}.json", pack.simple()),
            Self::Widget { pack, name } => {
                let digest = hex::encode(Sha256::digest(name.as_bytes()));
                format!("widget.{}.{}.json", pack.simple(), &digest[..32])
            }
        }
    }
}

#[cfg(test)]
#[path = "scope_tests.rs"]
mod scope_tests;
```

`agent/mod.rs` :

```rust
pub mod scope;
pub use scope::AgentScope;

/// Tours gardés au plus par fil (réducteur des tours, hub, reprise).
pub const KEPT_TURNS: usize = 50;
```

`crates/oku-slides-agent/src/turns.rs:27` : remplacer la définition locale par
`pub(crate) use oku_slides_session::agent::KEPT_TURNS;` (garder le commentaire).

- [ ] **Step 4 : lancer, constater le succès**

Run : `cargo nextest run -p oku-slides-a2ui -p oku-slides-session -p oku-slides-agent --lib && cargo build --workspace --all-targets`
Expected : PASS, build vert.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-a2ui/src/message.rs crates/oku-slides-a2ui/src/message_tests.rs crates/oku-slides-session/Cargo.toml crates/oku-slides-session/src/agent/scope.rs crates/oku-slides-session/src/agent/scope_tests.rs crates/oku-slides-session/src/agent/mod.rs crates/oku-slides-session/src/agent/forms.rs crates/oku-slides-session/src/agent/forms_tests.rs crates/oku-slides-agent/src/turns.rs
git diff --cached --name-only
git commit -m "feat(session) : portée de l'agent et surfaces relisibles (t-598)"
```

---

### Task 2 : Conversations gardées par la session, écrites et relues

**Files :**
- Create : `crates/oku-slides-session/src/agent/conversation.rs`, `conversation_tests.rs`
- Create : `crates/oku-slides-session/src/resume_agent.rs` (fichiers `agent/*.json` et `persist_agent`)
- Modify : `crates/oku-slides-session/src/lib.rs` (un champ `conversations` + constructeur ~l.356 ; `mod resume_agent;`)
- Modify : `crates/oku-slides-session/src/resume.rs` (`persist` → `persist_with` ; `Result`, `Resume::storage`,
  `AccessSnafu` en `pub(crate)` ; `restore`, `resume`, `resume_fresh`)
- Modify : `crates/oku-slides-session/src/resume_writer.rs` (`Job.agent`, `merge`, `write`) ; Test : `resume_writer_tests.rs`

**Interfaces :**
- Consumes : `AgentScope`, `Surface: Deserialize` (Task 1).
- Produces :
  ```rust
  pub struct SavedConversation { pub scope: AgentScope, pub agent: Option<String>, pub session: Option<String>, pub thread: Vec<Surface> }
  pub enum ScopeMove { Renamed { from: AgentScope, to: AgentScope } }
  impl Session {
      pub fn conversations(&self) -> Vec<SavedConversation>;
      pub fn conversation(&self, scope: &AgentScope) -> Option<&SavedConversation>;
      pub fn keep_thread(&mut self, scope: AgentScope, thread: Vec<Surface>);
      pub fn keep_agent_session(&mut self, scope: AgentScope, session: String, agent: String);
      pub fn forget_conversation(&mut self, scope: &AgentScope) -> Option<String>;   // l'id de session oublié
      pub fn take_scope_moves(&mut self) -> Vec<ScopeMove>;
  }
  pub(crate) fn resume_writer::merge(prev: Job, next: Job) -> Job;
  ```

Stockage (`crates/oku-slides-platform/src/storage.rs`) : `write_atomic` crée les
dossiers parents (OpenDAL), `list_files` d'un dossier absent rend une liste vide,
`delete` d'un fichier absent réussit. Aucun `create_dir_all` ni `exists` n'est donc utile.

- [ ] **Step 1 : tests rouges**

`conversation_tests.rs` :

```rust
use speculoos::prelude::*;

use super::{AgentScope, SavedConversation};
use crate::agent::forms::frozen_choice;
use crate::test_support::{fresh_dir, session_on};

/// Une surface du fil, figée, rangée dans le tour `turn`.
fn frozen(id: &str, turn: u32) -> crate::agent::Surface {
    let mut s = frozen_choice(id, "Laquelle ?", Some("A"), &oku_slides_i18n::Localizer::new(Some("fr")));
    s.turn = Some(turn);
    s
}

#[test]
fn conversations_survive_a_restart() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_thread(AgentScope::Slides, vec![frozen("report-3", 1)]);
    a.keep_agent_session(AgentScope::Slides, "sess-9".into(), "empreinte".into());
    drop(a);

    let mut b = session_on(data.path());
    assert!(b.resume(dir));
    assert_that!(b.conversation(&AgentScope::Slides).cloned()).is_equal_to(Some(SavedConversation {
        scope: AgentScope::Slides,
        agent: Some("empreinte".into()),
        session: Some("sess-9".into()),
        thread: vec![frozen("report-3", 1)],
    }));
}

#[test]
fn forget_conversation_removes_its_file_and_returns_its_session() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_agent_session(AgentScope::Slides, "sess-9".into(), "empreinte".into());
    a.flush_resume();
    assert_that!(dir.join("agent/slides.json").exists()).is_true();
    assert_that!(a.forget_conversation(&AgentScope::Slides)).is_equal_to(Some("sess-9".to_owned()));
    a.flush_resume();
    assert_that!(dir.join("agent/slides.json").exists()).is_false();
    assert_that!(a.conversations()).is_empty();
}

#[test]
fn unreadable_conversation_file_is_skipped_and_the_others_kept() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_thread(AgentScope::Slides, vec![frozen("report-1", 1)]);
    drop(a);
    std::fs::write(dir.join("agent/atelier.0123.json"), b"{pas du json").unwrap();

    let mut b = session_on(data.path());
    assert!(b.resume(dir));
    let scopes: Vec<_> = b.conversations().into_iter().map(|c| c.scope).collect();
    assert_that!(scopes).is_equal_to(vec![AgentScope::Slides]);
}

#[test]
fn conversation_file_naming_another_scope_is_skipped() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_thread(AgentScope::Slides, vec![frozen("report-1", 1)]);
    a.flush_resume();
    drop(a);
    // Le contenu de `slides.json` sous un autre nom : il ne prend pas la place des slides.
    std::fs::rename(dir.join("agent/slides.json"), dir.join("agent/widget.x.json")).unwrap();

    let mut b = session_on(data.path());
    assert!(b.resume(dir));
    assert_that!(b.conversations()).is_empty();
}

#[test]
fn saved_conversation_without_session_reads_as_none() {
    let c: SavedConversation = serde_json::from_str(r#"{"scope":{"kind":"slides"}}"#).unwrap();
    assert_that!(c.session).is_none();
    assert_that!(c.thread).is_empty();
}
```

`resume_writer_tests.rs` : le helper `job()` gagne `agent: BTreeMap::new()` ;
ajouter (fonction pure, sans fil) :

```rust
fn conv(scope: AgentScope) -> Option<SavedConversation> {
    Some(SavedConversation { scope, agent: None, session: Some("s".into()), thread: Vec::new() })
}

#[test]
fn merge_with_documents_keeps_the_previous_conversations() {
    let mut prev = job(None, "s-1");
    prev.agent.insert(AgentScope::Slides, conv(AgentScope::Slides));
    let atelier = AgentScope::Atelier { pack: uuid::Uuid::nil() };
    let mut next = job(Some("pro"), "s-2");
    next.agent.insert(atelier.clone(), conv(atelier.clone()));
    let merged = merge(prev, next);
    assert_that!(merged.agent.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![AgentScope::Slides, atelier]);
}

#[test]
fn merge_later_delete_wins_over_earlier_write() {
    let mut prev = job(None, "s-1");
    prev.agent.insert(AgentScope::Slides, conv(AgentScope::Slides));
    let mut next = job(None, "s-2");
    next.agent.insert(AgentScope::Slides, None);
    assert_that!(merge(prev, next).agent.get(&AgentScope::Slides)).is_equal_to(Some(&None));
}

#[test]
fn merge_later_write_wins_over_earlier_delete() {
    let mut prev = job(None, "s-1");
    prev.agent.insert(AgentScope::Slides, None);
    let mut next = job(None, "s-2");
    next.agent.insert(AgentScope::Slides, conv(AgentScope::Slides));
    assert_that!(merge(prev, next).agent.get(&AgentScope::Slides)).is_equal_to(Some(&conv(AgentScope::Slides)));
}

#[test]
fn merge_without_documents_keeps_the_previous_documents() {
    let merged = merge(job(Some("pro"), "s-1"), job(None, "s-2"));
    assert_that!(merged.docs.is_empty()).is_false();
    assert_that!(merged.state.ui.slide.as_deref()).is_equal_to(Some("s-2"));
}
```

(lire le `job(theme, slide)` existant : `Some(theme)` donne des documents.)

- [ ] **Step 2 : lancer, constater l'échec**

Run : `cargo nextest run -p oku-slides-session -E 'test(conversation) | test(merge_)'`
Expected : erreurs de compilation.

- [ ] **Step 3 : implémenter**

`agent/conversation.rs` :

```rust
//! Les conversations de l'agent gardées par la session.
//!
//! Une par portée, écrite dans `agent/<portée>.json` du dossier de reprise. La
//! session est le seul écrivain ; dans l'agent, le gardien de l'onglet le seul appelant.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{AgentScope, Surface};
use crate::Session;

/// Une conversation : son fil et sa session ACP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedConversation {
    /// La portée de la conversation.
    pub scope: AgentScope,
    /// Empreinte de l'agent qui tient la session (`Link::agent_id`) ; opaque ici.
    #[serde(default)]
    pub agent: Option<String>,
    /// Identifiant de la session ACP.
    #[serde(default)]
    pub session: Option<String>,
    /// Les surfaces figées du fil, sans actions.
    #[serde(default)]
    pub thread: Vec<Surface>,
}

impl SavedConversation {
    fn empty(scope: AgentScope) -> Self {
        Self { scope, agent: None, session: None, thread: Vec::new() }
    }
}

/// Une conversation déplacée par la session ; le gardien la fait suivre au hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeMove {
    /// Le widget a été renommé.
    Renamed {
        /// L'ancienne portée.
        from: AgentScope,
        /// La nouvelle portée.
        to: AgentScope,
    },
}

/// Les conversations d'une session et les déplacements pas encore lus.
#[derive(Debug, Default)]
pub(crate) struct Conversations {
    pub(crate) by_scope: BTreeMap<AgentScope, SavedConversation>,
    pub(crate) moves: Vec<ScopeMove>,
}

impl Session {
    /// Les conversations gardées.
    pub fn conversations(&self) -> Vec<SavedConversation> {
        self.conversations.by_scope.values().cloned().collect()
    }

    /// La conversation de `scope`, si elle existe.
    pub fn conversation(&self, scope: &AgentScope) -> Option<&SavedConversation> {
        self.conversations.by_scope.get(scope)
    }

    /// Le fil de `scope` (déjà sans surface vivante ni action) ; écrit pour la reprise.
    pub fn keep_thread(&mut self, scope: AgentScope, thread: Vec<Surface>) {
        self.conversation_mut(&scope).thread = thread;
        self.persist_conversation(&scope);
    }

    /// La session ACP de `scope` et l'empreinte de son agent ; écrites pour la reprise.
    pub fn keep_agent_session(&mut self, scope: AgentScope, session: String, agent: String) {
        let c = self.conversation_mut(&scope);
        c.session = Some(session);
        c.agent = Some(agent);
        self.persist_conversation(&scope);
    }

    /// Nouvelle conversation dans `scope` : oubliée, son fichier effacé ; rend sa session.
    pub fn forget_conversation(&mut self, scope: &AgentScope) -> Option<String> {
        let gone = self.conversations.by_scope.remove(scope)?;
        self.persist_agent(scope.clone(), None);
        gone.session
    }

    /// Les déplacements de conversations depuis le dernier appel.
    pub fn take_scope_moves(&mut self) -> Vec<ScopeMove> {
        std::mem::take(&mut self.conversations.moves)
    }

    fn conversation_mut(&mut self, scope: &AgentScope) -> &mut SavedConversation {
        self.conversations.by_scope.entry(scope.clone()).or_insert_with(|| SavedConversation::empty(scope.clone()))
    }

    pub(crate) fn persist_conversation(&mut self, scope: &AgentScope) {
        let saved = self.conversations.by_scope.get(scope).cloned();
        self.persist_agent(scope.clone(), saved);
    }
}

#[cfg(test)]
#[path = "conversation_tests.rs"]
mod conversation_tests;
```

`agent/mod.rs` : `pub mod conversation;` et `pub use conversation::{SavedConversation, ScopeMove};`.

`lib.rs`, struct `Session` (à côté de `agent_notes`) — `lib.rs` fait 969 lignes,
n'y ajouter que ce champ et la ligne `mod` :

```rust
    /// Conversations de l'agent, une par portée (`agent::conversation`).
    conversations: agent::conversation::Conversations,
```

constructeur : `conversations: Default::default(),`. `mod resume_agent;` à côté de `mod resume;`.

`resume_writer.rs` :

```rust
pub(crate) struct Job {
    /// Vide : l'état seul (les documents n'ont pas changé).
    pub docs: Vec<(&'static str, DocSnapshot)>,
    pub state: State,
    /// Conversations à écrire (`Some`) ou effacer (`None`), par portée.
    pub agent: BTreeMap<AgentScope, Option<SavedConversation>>,
}

/// Fusionne deux dépôts : le plus récent l'emporte, sans perdre de conversation.
pub(crate) fn merge(prev: Job, next: Job) -> Job {
    let docs = if next.docs.is_empty() { prev.docs } else { next.docs };
    let mut agent = prev.agent;
    agent.extend(next.agent);
    Job { docs, state: next.state, agent }
}
```

dans `send` : `slot.job = Some(match slot.job.take() { Some(prev) => merge(prev, job), None => job });`
(remplace l'ancienne fusion). Dans `write`, **après et hors** de la chaîne
`written` (un état non écrit n'empêche pas les conversations) :

```rust
    for (scope, saved) in &job.agent {
        let done = match saved {
            Some(c) => resume.write_conversation(c),
            None => resume.delete_conversation(scope),
        };
        if let Err(e) = done {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "conversation de l'agent non enregistrée");
        }
    }
```

`resume_agent.rs` :

```rust
//! Les conversations de l'agent dans le dossier de reprise : `agent/<portée>.json`.

use std::collections::BTreeMap;

use serde::Serialize;
use snafu::ResultExt;

use crate::agent::{AgentScope, SavedConversation};
use crate::resume::{AccessSnafu, Result, Resume};
use crate::Session;

/// Sous-dossier des conversations, dans le dossier de reprise de l'onglet.
pub(crate) const AGENT_DIR: &str = "agent";
/// Version des fichiers `agent/*.json`.
pub(crate) const CONVERSATION_VERSION: u32 = 1;

#[derive(Serialize)]
struct ConversationOut<'a> {
    version: u32,
    #[serde(flatten)]
    conversation: &'a SavedConversation,
}

impl Resume {
    pub(crate) fn write_conversation(&self, c: &SavedConversation) -> Result<()> {
        let out = ConversationOut { version: CONVERSATION_VERSION, conversation: c };
        self.write_json(&format!("{AGENT_DIR}/{}", c.scope.file_name()), &out)
    }

    pub(crate) fn delete_conversation(&self, scope: &AgentScope) -> Result<()> {
        let path = self.dir.join(AGENT_DIR).join(scope.file_name());
        self.storage.delete(&path).context(AccessSnafu { path: &path })
    }

    /// Les conversations relues ; un fichier illisible ou mal nommé est journalisé et sauté.
    pub(crate) fn read_conversations(&self) -> BTreeMap<AgentScope, SavedConversation> {
        let dir = self.dir.join(AGENT_DIR);
        let files = match self.storage.list_files(&dir) {
            Ok(files) => files,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "conversations de l'agent illisibles");
                Vec::new()
            }
        };
        let mut out = BTreeMap::new();
        for path in files {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if name.starts_with('.') || !name.ends_with(".json") {
                continue;
            }
            // `read_json` vérifie la version à part ; le champ `version` est ignoré ici.
            match self.read_json::<SavedConversation>(&format!("{AGENT_DIR}/{name}"), CONVERSATION_VERSION) {
                Ok(Some(c)) if c.scope.file_name() == name => {
                    out.insert(c.scope.clone(), c);
                }
                Ok(Some(_)) => tracing::warn!(file = name, "conversation sous un autre nom : ignorée"),
                Ok(None) => tracing::debug!(file = name, "conversation disparue"),
                Err(e) => tracing::warn!(file = name, error = %oku_slides_platform::errors::one_line(&e), "conversation illisible : ignorée"),
            }
        }
        out
    }
}

impl Session {
    /// Une conversation à écrire (`Some`) ou effacer (`None`), avec l'état courant.
    pub(crate) fn persist_agent(&mut self, scope: AgentScope, saved: Option<SavedConversation>) {
        self.persist_with(false, BTreeMap::from([(scope, saved)]));
    }
}
```

(si `list_files` rend des chemins relatifs au dossier, `file_name()` les lit pareil.)

`resume.rs` : `type Result`, `Resume::storage` et `AccessSnafu` deviennent
`pub(crate)` ; `persist` devient

```rust
    pub(crate) fn persist(&mut self, documents: bool) {
        self.persist_with(documents, BTreeMap::new());
    }
```

et `persist_with(documents, agent: BTreeMap<AgentScope, Option<SavedConversation>>)`
reprend l'ancien corps ; la branche de l'aide passe `agent` à son `Job` :
`writer.send(Job { docs: Vec::new(), state, agent }); return;` ; la fin :
`writer.send(Job { docs, state, agent });`.

Relecture : au début de `restore`, `self.conversations.by_scope = resume.read_conversations();` ;
dans la branche d'échec de `resume` (avant `self.resume = None; return false;`) et
au début de `resume_fresh` : `self.conversations = Default::default();`.

- [ ] **Step 4 : lancer, constater le succès**

Run : `cargo nextest run -p oku-slides-session && cargo build --workspace --all-targets`
Expected : PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session/src/agent/conversation.rs crates/oku-slides-session/src/agent/conversation_tests.rs crates/oku-slides-session/src/agent/mod.rs crates/oku-slides-session/src/resume_agent.rs crates/oku-slides-session/src/lib.rs crates/oku-slides-session/src/resume.rs crates/oku-slides-session/src/resume_writer.rs crates/oku-slides-session/src/resume_writer_tests.rs
git diff --cached --name-only
git commit -m "feat(session) : conversations de l'agent par portée, gardées pour la reprise (t-598)"
```

---

### Task 3 : Renommage et nettoyage des conversations de widgets

**Files :**
- Modify : `crates/oku-slides-session/src/agent/conversation.rs`, `crates/oku-slides-session/src/widgets_edit.rs` (`rename_widget`, nouveaux `pack_files_known`, `widget_known`), `crates/oku-slides-session/src/resume.rs` (`resume`)
- Test : `crates/oku-slides-session/src/agent/conversation_tests.rs`

**Interfaces :**
- Consumes : Task 2 ; `Session::{edit_templates, set_widget, rename_widget, remove_widget, undo, deck_pack, user_blobs}`,
  `crate::pack::DEFAULT_PACK`, `library.current_version(id) -> Option<StoredVersion>` (`.files`).
- Produces : `ScopeMove::Renamed { from, to }` après `rename_widget` ; `Session::prune_conversations()` (crate)
  appelé à la reprise ; `keep_thread`/`keep_agent_session` refusent une portée `Widget` inconnue.

Décision 3 de la spec : un renommage déplace la conversation ; une suppression
ne touche à rien ; la reprise nettoie **prudemment** ; une portée inventée
n'écrit rien.

- [ ] **Step 1 : tests rouges** (`conversation_tests.rs`, ajouter ; `ScopeMove` au `use super::…`)

```rust
use crate::pack::DEFAULT_PACK;

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

fn widget(name: &str) -> AgentScope {
    AgentScope::Widget { pack: DEFAULT_PACK, name: name.into() }
}

/// Une session reprise dans `resume/`, en édition du pack par défaut, avec le widget `kpi`.
fn editing_with_kpi(data: &std::path::Path) -> crate::Session {
    let mut s = session_on(data);
    assert!(s.resume(data.join("resume")));
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_widget("kpi", KPI.into()).unwrap();
    s
}

#[test]
fn rename_widget_moves_its_conversation_and_notes_the_move() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = editing_with_kpi(data.path());
    s.keep_thread(widget("kpi"), vec![frozen("report-1", 1)]);
    s.rename_widget("kpi", "indicateur").unwrap();
    s.flush_resume();
    let agent_dir = data.path().join("resume/agent");
    assert_that!(agent_dir.join(widget("kpi").file_name()).exists()).is_false();
    assert_that!(agent_dir.join(widget("indicateur").file_name()).exists()).is_true();
    assert_that!(s.conversation(&widget("indicateur")).map(|c| c.thread.len())).is_equal_to(Some(1));
    assert_that!(s.take_scope_moves()).is_equal_to(vec![ScopeMove::Renamed { from: widget("kpi"), to: widget("indicateur") }]);
}

#[test]
fn renamed_conversation_is_read_back_under_its_new_name() {
    let data = fresh_dir("oku-slides-conv-");
    let mut a = editing_with_kpi(data.path());
    a.keep_thread(widget("kpi"), vec![frozen("report-1", 1)]);
    a.rename_widget("kpi", "indicateur").unwrap();
    drop(a);
    let mut b = session_on(data.path());
    assert!(b.resume(data.path().join("resume")));
    assert_that!(b.conversation(&widget("indicateur"))).is_some();
}

#[test]
fn remove_widget_then_undo_keeps_its_conversation() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = editing_with_kpi(data.path());
    s.keep_thread(widget("kpi"), vec![frozen("report-1", 1)]);
    s.remove_widget("kpi").unwrap();
    assert_that!(s.conversation(&widget("kpi"))).is_some();
    s.undo().unwrap();
    assert_that!(s.conversation(&widget("kpi"))).is_some();
}

#[test]
fn keep_thread_of_an_unknown_widget_writes_nothing() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = editing_with_kpi(data.path());
    s.keep_thread(widget("invente"), vec![frozen("report-1", 1)]);
    s.flush_resume();
    assert_that!(s.conversation(&widget("invente"))).is_none();
    assert_that!(data.path().join("resume/agent").join(widget("invente").file_name()).exists()).is_false();
}

#[test]
fn resume_prunes_only_conversations_of_widgets_missing_from_known_pack_files() {
    let data = fresh_dir("oku-slides-conv-");
    let mut a = editing_with_kpi(data.path());
    a.set_widget("disparu", KPI.replace("kpi", "disparu")).unwrap();
    a.keep_thread(widget("kpi"), vec![frozen("report-1", 1)]);
    a.keep_thread(widget("disparu"), vec![frozen("report-2", 2)]);
    let foreign = AgentScope::Widget { pack: uuid::Uuid::from_u128(42), name: "kpi".into() };
    a.keep_thread(AgentScope::Atelier { pack: DEFAULT_PACK }, vec![frozen("report-4", 4)]);
    a.set_widget("disparu", String::new()).unwrap(); // supprimé : sa conversation reste jusqu'à la reprise
    a.flush_resume();
    drop(a);
    // Une conversation d'un pack inconnu ici (deck d'une autre machine) : écrite à la main.
    let path = data.path().join("resume/agent").join(foreign.file_name());
    let json = serde_json::json!({ "version": 1, "scope": foreign, "thread": [] });
    std::fs::write(&path, json.to_string()).unwrap();

    let mut b = session_on(data.path());
    assert!(b.resume(data.path().join("resume")));
    let mut scopes: Vec<_> = b.conversations().into_iter().map(|c| c.scope).collect();
    scopes.sort();
    let mut expected = vec![AgentScope::Atelier { pack: DEFAULT_PACK }, foreign, widget("kpi")];
    expected.sort();
    assert_that!(scopes).is_equal_to(expected);
    b.flush_resume();
    assert_that!(data.path().join("resume/agent").join(widget("disparu").file_name()).exists()).is_false();
}
```

- [ ] **Step 2 : lancer, constater l'échec**

Run : `cargo nextest run -p oku-slides-session -E 'test(rename_widget_moves) | test(renamed_conversation) | test(remove_widget_then_undo) | test(unknown_widget) | test(resume_prunes)'`
Expected : FAIL (pas de déplacement, pas de refus, pas de nettoyage).

- [ ] **Step 3 : implémenter**

`widgets_edit.rs` (accès au mode, au deck et à la bibliothèque) :

```rust
    /// Fichiers connus du pack `pack` : version éditée, pack du deck, version courante.
    pub(crate) fn pack_files_known(&self, pack: uuid::Uuid) -> Vec<PackFiles> {
        let mut out = Vec::new();
        if let Mode::Templates { edit, .. } = &self.mode
            && edit.set == pack
        {
            out.extend(self.doc.sources.packs.get(&edit.base).cloned());
        }
        if let Some(r) = self.deck_pack().filter(|r| r.id == pack) {
            out.extend(self.user_blobs().packs.get(&r.version).cloned());
        }
        out.extend(self.library.current_version(pack).map(|v| v.files));
        out
    }

    /// Le widget `name` est-il dans un fichier connu du pack `pack` ?
    pub(crate) fn widget_known(&self, pack: uuid::Uuid, name: &str) -> bool {
        let Ok(file) = parse_name(name) else { return false };
        self.pack_files_known(pack).iter().any(|f| f.widget(&file).is_some())
    }
```

`conversation.rs`, dans `impl Session` :

```rust
    /// Une portée qu'on peut écrire : les slides, un atelier, ou un widget connu de son pack.
    fn writable(&self, scope: &AgentScope) -> bool {
        match scope {
            AgentScope::Slides | AgentScope::Atelier { .. } => true,
            AgentScope::Widget { pack, name } => self.widget_known(*pack, name),
        }
    }

    /// Le widget `from` du pack `pack` s'appelle désormais `to` : sa conversation le suit.
    pub(crate) fn conversation_renamed(&mut self, pack: uuid::Uuid, from: &str, to: &str) {
        let old = AgentScope::Widget { pack, name: from.to_owned() };
        let new = AgentScope::Widget { pack, name: to.to_owned() };
        if let Some(mut c) = self.conversations.by_scope.remove(&old) {
            c.scope = new.clone();
            self.conversations.by_scope.insert(new.clone(), c);
            self.persist_agent(old.clone(), None);
            self.persist_conversation(&new);
        }
        self.conversations.moves.push(ScopeMove::Renamed { from: old, to: new });
    }

    /// Efface les conversations dont le widget manque à tous les fichiers trouvés de son pack.
    ///
    /// Prudent : un pack dont aucun fichier n'est trouvé ne fait rien effacer.
    pub(crate) fn prune_conversations(&mut self) {
        let gone: Vec<AgentScope> = self
            .conversations
            .by_scope
            .keys()
            .filter(|scope| match scope {
                AgentScope::Widget { pack, name } => {
                    let files = self.pack_files_known(*pack);
                    !files.is_empty() && !self.widget_known(*pack, name)
                }
                AgentScope::Slides | AgentScope::Atelier { .. } => false,
            })
            .cloned()
            .collect();
        for scope in gone {
            tracing::debug!(?scope, "conversation d'un widget disparu : effacée");
            self.forget_conversation(&scope);
        }
    }
```

`keep_thread` et `keep_agent_session` commencent par :

```rust
        if !self.writable(&scope) {
            tracing::warn!(?scope, "portée inconnue : conversation non enregistrée");
            return;
        }
```

`rename_widget`, à la fin, après le `if let Some(scratch)` et avant `Ok(statuses)` :
`self.conversation_renamed(t.id, from.as_str(), to.as_str());`.

`resume.rs`, dans `resume` (chemin de succès), juste après
`self.resume = Some(Writer::new(resume));` : `self.prune_conversations();`.

- [ ] **Step 4 : lancer, constater le succès**

Run : `cargo nextest run -p oku-slides-session && cargo build --workspace --all-targets`
Expected : PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session/src/agent/conversation.rs crates/oku-slides-session/src/agent/conversation_tests.rs crates/oku-slides-session/src/widgets_edit.rs crates/oku-slides-session/src/resume.rs
git diff --cached --name-only
git commit -m "feat(session) : la conversation d'un widget suit son renommage, nettoyée prudemment à la reprise (t-598)"
```

---

### Task 4 : Hub — fils par portée, restauration, image du fil

**Files :**
- Create : `crates/oku-slides-mcp/src/hub_scopes.rs`, `hub_scopes_tests.rs`
- Modify : `crates/oku-slides-mcp/src/hub.rs` (`HubState`, `record_and_send`, `resync`, `take_asks_where`, `cancel_asks_where`), `hub_turns.rs` (`begin_turn`, `close_turn`, `clear_thread`)
- Modify (appelants) : `crates/oku-slides-mcp/tests/surfaces.rs`, `receipts.rs`, `turns.rs` ;
  `crates/oku-slides-agent/src/tab_agent.rs`, `tab_entry.rs`, `stack.rs`
- Modify : `crates/oku-slides-mcp/ARCHITECTURE.md:58` (`KEPT_TURNS` vient de `oku-slides-session::agent`)

**Interfaces :**
- Consumes : `AgentScope`, `SavedConversation`, `KEPT_TURNS` (Tasks 1–2).
- Produces (hub sans accès à la session ; méthodes pilotées par le gardien de l'onglet) :
  ```rust
  pub const THREAD_BYTES: usize = 768 * 1024;
  impl SurfaceHub {
      pub fn begin_turn(&self, scope: AgentScope) -> u32;
      pub fn resync(&self, scope: AgentScope);
      pub fn clear_thread(&self, scope: &AgentScope);
      pub fn visible_surfaces(&self) -> Vec<Surface>;
      pub fn restore(&self, saved: Vec<SavedConversation>);
      pub fn thread_snapshot(&self, scope: &AgentScope) -> Option<Vec<Surface>>;
      pub fn turn_scope(&self, turn: u32) -> Option<AgentScope>;
      pub fn rename_scope(&self, from: &AgentScope, to: &AgentScope);
  }
  ```
  Ponts temporaires dans `oku-slides-agent` : `AgentScope::Slides` (remplacés en Task 7).

- [ ] **Step 1 : tests rouges** — `hub_scopes_tests.rs` :

```rust
use std::sync::Arc;

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::forms::{self, AskChoice};
use oku_slides_session::agent::turn_view::{TurnState, TurnView, turn_surface};
use oku_slides_session::agent::{AgentScope, KEPT_TURNS, SavedConversation, Sessions, Surface, TabSession};
use speculoos::prelude::*;
use uuid::Uuid;

use super::{McpEvent, SurfaceHub, THREAD_BYTES};

#[derive(Debug)]
struct NoSession;
impl Sessions for NoSession {
    fn with(&self, _tab: &str, _f: &mut dyn FnMut(&mut oku_slides_session::Session)) -> oku_slides_session::Result<()> {
        Ok(())
    }
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn hub() -> Arc<SurfaceHub> {
    SurfaceHub::new(TabSession::new("t-1", Arc::new(NoSession)), Arc::new(fr()))
}

fn atelier() -> AgentScope {
    AgentScope::Atelier { pack: Uuid::from_u128(1) }
}

/// La surface du tour `n`, clos.
fn turn(n: u32) -> Surface {
    let view = TurnView { n, prompt: format!("consigne {n}"), plan: vec![], steps: vec![], tools: 0, notes: 0,
        narration: String::new(), state: TurnState::Done { secs: 1 } };
    turn_surface(&view, &fr())
}

/// Une question du fil, vivante, posée dans le tour `n`.
fn question(id: &str, n: u32) -> Surface {
    let option = |id: &str| oku_slides_a2ui::ChoiceOption { id: id.into(), label: id.into(), hint: None };
    let mut s = forms::choice(id, &AskChoice { question: "?".into(), options: vec![option("a"), option("b")], anchor: None }, &fr());
    s.turn = Some(n);
    s
}

/// La même question, répondue et figée dans son tour.
fn answered(id: &str, n: u32) -> Surface {
    let mut s = forms::frozen_choice(id, "?", Some("a"), &fr());
    s.turn = Some(n);
    s
}

fn ids(list: &[Surface]) -> Vec<String> {
    list.iter().map(|s| s.id.clone()).collect()
}

fn last_surfaces(rx: &mut tokio::sync::broadcast::Receiver<McpEvent>) -> Vec<String> {
    let mut found = None;
    while let Ok(e) = rx.try_recv() {
        if let McpEvent::Surfaces { list } = e {
            found = Some(ids(&list));
        }
    }
    found.unwrap_or_else(|| panic!("liste attendue"))
}

/// Ouvre puis clôt un tour de `scope` avec sa surface.
fn closed_turn(hub: &SurfaceHub, scope: AgentScope) -> u32 {
    let n = hub.begin_turn(scope);
    hub.emit(McpEvent::Surface { surface: turn(n) });
    hub.end_turn(n, "");
    n
}

#[test]
fn resync_sends_only_the_thread_of_the_viewed_scope() {
    let hub = hub();
    let one = closed_turn(&hub, AgentScope::Slides);
    let two = closed_turn(&hub, atelier());
    let mut rx = hub.subscribe();
    hub.resync(AgentScope::Slides);
    assert_that!(last_surfaces(&mut rx)).is_equal_to(vec![format!("turn-{one}")]);
    hub.resync(atelier());
    assert_that!(last_surfaces(&mut rx)).is_equal_to(vec![format!("turn-{two}")]);
    assert_that!(hub.surfaces()).has_length(2);
}

#[test]
fn surface_of_a_turn_in_another_scope_is_kept_but_not_sent() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let n = hub.begin_turn(atelier());
    let mut rx = hub.subscribe();
    hub.emit(McpEvent::Surface { surface: turn(n) });
    assert_that!(rx.try_recv().is_err()).is_true();
    assert_that!(ids(&hub.surfaces())).is_equal_to(vec![format!("turn-{n}")]);
}

#[test]
fn live_surface_of_another_scope_is_sent_then_closed_once_hidden() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let n = hub.begin_turn(atelier());
    let mut rx = hub.subscribe();
    hub.emit(McpEvent::Surface { surface: question("ask-1", n) });
    assert_that!(matches!(rx.try_recv(), Ok(McpEvent::Surface { .. }))).is_true();
    hub.emit(McpEvent::Surface { surface: answered("ask-1", n) });
    assert_that!(matches!(rx.try_recv(), Ok(McpEvent::CloseSurface { id }) if id == "ask-1")).is_true();
}

#[test]
fn hidden_surface_never_shown_sends_no_close() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let n = hub.begin_turn(atelier());
    let mut rx = hub.subscribe();
    hub.emit(McpEvent::Surface { surface: answered("ask-1", n) });
    hub.emit(McpEvent::Surface { surface: answered("ask-1", n) });
    assert_that!(rx.try_recv().is_err()).is_true();
}

#[test]
fn patch_of_a_hidden_surface_is_not_sent() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let n = hub.begin_turn(atelier());
    hub.emit(McpEvent::Surface { surface: turn(n) });
    let mut rx = hub.subscribe();
    hub.emit(McpEvent::SurfacePatch { id: format!("turn-{n}"), messages: vec![] });
    assert_that!(rx.try_recv().is_err()).is_true();
}

async fn pending_ask(hub: &Arc<SurfaceHub>, id: &'static str, n: u32) -> tokio::task::JoinHandle<super::Answer> {
    let task = tokio::spawn({
        let hub = hub.clone();
        async move { hub.ask_then(Some(n), question(id, n), Box::new(move |_| answered(id, n))).await }
    });
    while !hub.surfaces().iter().any(|s| s.id == id) {
        tokio::task::yield_now().await;
    }
    task
}

#[tokio::test]
async fn clear_thread_keeps_the_questions_of_other_scopes() {
    let hub = hub();
    let n = hub.begin_turn(AgentScope::Slides);
    let waiting = pending_ask(&hub, "ask-1", n).await;
    hub.clear_thread(&atelier());
    tokio::task::yield_now().await;
    assert_that!(waiting.is_finished()).is_false();
    assert_that!(hub.has_pending_ask()).is_true();
    hub.cancel_asks();
}

#[tokio::test]
async fn clear_thread_releases_the_questions_of_its_scope() {
    let hub = hub();
    let n = hub.begin_turn(AgentScope::Slides);
    let waiting = pending_ask(&hub, "ask-1", n).await;
    hub.clear_thread(&AgentScope::Slides);
    assert_that!(matches!(waiting.await.unwrap(), super::Answer::Cancelled)).is_true();
    assert_that!(hub.has_pending_ask()).is_false();
}

#[test]
fn clear_thread_leaves_the_other_scopes() {
    let hub = hub();
    let a = closed_turn(&hub, AgentScope::Slides);
    let b = closed_turn(&hub, atelier());
    hub.clear_thread(&atelier());
    assert_that!(ids(&hub.surfaces())).is_equal_to(vec![format!("turn-{a}")]);
    assert_that!(hub.turn_scope(b)).is_none();
}

#[test]
fn begin_turn_closes_the_oldest_turns_of_its_scope_only() {
    let hub = hub();
    let other = closed_turn(&hub, atelier());
    for _ in 0..=KEPT_TURNS {
        closed_turn(&hub, AgentScope::Slides);
    }
    hub.resync(AgentScope::Slides);
    assert_that!(hub.visible_surfaces()).has_length(KEPT_TURNS);
    assert_that!(ids(&hub.surfaces())).contains(format!("turn-{other}"));
}

#[test]
fn restore_puts_threads_back_and_numbers_new_turns_after_them() {
    let hub = hub();
    hub.restore(vec![SavedConversation { scope: atelier(), agent: None, session: None, thread: vec![turn(7), answered("report-12", 7)] }]);
    hub.resync(atelier());
    assert_that!(ids(&hub.visible_surfaces())).is_equal_to(vec!["turn-7".to_owned(), "report-12".to_owned()]);
    hub.resync(AgentScope::Slides);
    assert_that!(hub.visible_surfaces()).is_empty();
    assert_that!(hub.begin_turn(AgentScope::Slides)).is_equal_to(8);
    assert_that!(hub.next_id("report")).is_equal_to("report-13".to_owned());
}

#[test]
fn restore_cleans_what_it_reads() {
    let hub = hub();
    let mut tampered = answered("report-3", 3);
    tampered.live = true;
    tampered.question = true;
    tampered.messages = question("report-3", 3).messages; // avec ses actions
    let mut huge = turn(4);
    huge.turn = Some(u32::MAX);
    let mut no_turn = turn(5);
    no_turn.turn = None;
    hub.restore(vec![SavedConversation { scope: atelier(), agent: None, session: None, thread: vec![tampered, huge, no_turn] }]);
    hub.resync(atelier());
    let kept = hub.visible_surfaces();
    assert_that!(ids(&kept)).is_equal_to(vec!["report-3".to_owned()]);
    assert_that!(kept[0].live || kept[0].question || kept[0].has_actions()).is_false();
    assert_that!(hub.begin_turn(AgentScope::Slides)).is_equal_to(4);
}

#[test]
fn restore_ignores_a_turn_already_taken() {
    let hub = hub();
    let open = hub.begin_turn(AgentScope::Slides);
    hub.restore(vec![
        SavedConversation { scope: atelier(), agent: None, session: None, thread: vec![turn(open), turn(9)] },
        SavedConversation { scope: AgentScope::Widget { pack: Uuid::from_u128(1), name: "kpi".into() }, agent: None,
            session: None, thread: vec![answered("report-20", 9)] },
    ]);
    assert_that!(hub.turn_scope(open)).is_equal_to(Some(AgentScope::Slides));
    assert_that!(hub.turn_scope(9)).is_equal_to(Some(atelier()));
    assert_that!(ids(&hub.surfaces())).does_not_contain("report-20".to_owned());
}

#[test]
fn thread_snapshot_keeps_closed_frozen_surfaces_of_the_scope_without_actions() {
    let hub = hub();
    let a = hub.begin_turn(AgentScope::Slides);
    hub.emit(McpEvent::Surface { surface: turn(a) });
    hub.emit(McpEvent::Surface { surface: question("ask-1", a) });
    hub.emit(McpEvent::Surface { surface: answered("report-2", a) });
    hub.end_turn(a, "");
    closed_turn(&hub, atelier());
    let open = hub.begin_turn(AgentScope::Slides);
    hub.emit(McpEvent::Surface { surface: turn(open) });
    let kept = hub.thread_snapshot(&AgentScope::Slides).unwrap_or_default();
    assert_that!(ids(&kept)).is_equal_to(vec![format!("turn-{a}"), "report-2".to_owned()]);
    assert_that!(kept.iter().any(Surface::has_actions)).is_false();
}

fn big_turn(hub: &SurfaceHub, kib: usize) -> u32 {
    let n = hub.begin_turn(AgentScope::Slides);
    let mut big = turn(n);
    big.set_value("narration", serde_json::Value::String("x".repeat(kib * 1024)));
    hub.emit(McpEvent::Surface { surface: big });
    hub.end_turn(n, "");
    n
}

#[test]
fn thread_snapshot_keeps_a_contiguous_suffix_within_its_byte_budget() {
    let hub = hub();
    let mut last = 0;
    for _ in 0..KEPT_TURNS {
        last = big_turn(&hub, 40);
    }
    let kept = hub.thread_snapshot(&AgentScope::Slides).unwrap_or_default();
    let bytes: usize = kept.iter().map(|s| serde_json::to_vec(s).map_or(0, |b| b.len())).sum();
    assert_that!(bytes).is_less_than_or_equal_to(THREAD_BYTES);
    let turns: Vec<u32> = kept.iter().filter_map(|s| s.turn).collect();
    assert_that!(turns.len()).is_greater_than(1);
    assert_that!(turns.last().copied()).is_equal_to(Some(last));
    assert_that!(turns.windows(2).all(|w| w[1] == w[0] + 1)).is_true();
}

#[test]
fn thread_snapshot_with_an_oversized_last_turn_writes_nothing() {
    let hub = hub();
    big_turn(&hub, 900);
    assert_that!(hub.thread_snapshot(&AgentScope::Slides)).is_none();
}

#[test]
fn rename_scope_moves_turns_and_the_viewed_scope() {
    let hub = hub();
    let from = AgentScope::Widget { pack: Uuid::from_u128(1), name: "kpi".into() };
    let to = AgentScope::Widget { pack: Uuid::from_u128(1), name: "indicateur".into() };
    let n = closed_turn(&hub, from.clone());
    hub.resync(from.clone());
    hub.rename_scope(&from, &to);
    assert_that!(hub.turn_scope(n)).is_equal_to(Some(to.clone()));
    hub.resync(to);
    assert_that!(hub.visible_surfaces()).has_length(1);
}
```

Le relais en retard (`tab_entry.rs:140`) passe à `visible_surfaces()` ; le
reproduire demande un canal plein : couvert par les tests de `visible_surfaces`
ci-dessus, et vérifié en revue.

- [ ] **Step 2 : lancer, constater l'échec**

Run : `cargo nextest run -p oku-slides-mcp`
Expected : erreurs de compilation.

- [ ] **Step 3 : implémenter**

`hub.rs`, `HubState` (ajouts) :

```rust
    /// Portée de chaque tour encore dans le fil.
    turn_scopes: HashMap<u32, AgentScope>,
    /// La portée que l'interface montre : seul son fil lui est envoyé.
    viewing: AgentScope,
```

Questions, avec un filtre (garder la doc existante de `cancel_asks`) :

```rust
    fn take_asks_where(&mut self, which: impl Fn(&str) -> bool) -> Vec<PendingAsk> {
        let (taken, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.asks).into_iter().partition(|a| which(&a.id));
        self.asks = kept;
        self.open.retain(|s| !taken.iter().any(|a| !a.frozen && a.id == s.id));
        taken
    }
```

`take_asks()` devient `take_asks_where(|_| true)` ; ajouter

```rust
    /// Libère les questions en attente que `which` désigne.
    pub(crate) fn cancel_asks_where(&self, which: impl Fn(&str) -> bool) {
        let asks = self.state().take_asks_where(which);
        Self::release(asks);
    }
```

et `cancel_asks` appelle `self.cancel_asks_where(|_| true)`.

`record_and_send` n'envoie que ce qui est montré ; une surface qui devient
cachée part en `CloseSurface` :

```rust
    fn record_and_send(&self, state: &mut HubState, mut event: McpEvent) {
        let send = match &mut event {
            McpEvent::Surface { surface } if surface.turn.is_some_and(|t| !state.live_turns.contains(&t)) => {
                tracing::debug!(surface = %surface.id, "surface d'un tour effacé : ignorée");
                return;
            }
            McpEvent::Surface { surface } => {
                let was = state.open.iter().find(|s| s.id == surface.id).map(|s| (s.live, s.turn));
                let was = was.is_some_and(|(live, turn)| Self::shown(&*state, live, turn));
                match state.open.iter_mut().find(|s| s.id == surface.id) {
                    Some(open) => *open = surface.clone(),
                    None => state.open.push(surface.clone()),
                }
                let now = Self::shown(&*state, surface.live, surface.turn);
                if was && !now {
                    // Montrée, maintenant cachée (question d'une autre portée figée) : l'interface la ferme.
                    event = McpEvent::CloseSurface { id: surface.id.clone() };
                    true
                } else {
                    now
                }
            }
            McpEvent::CloseSurface { id } => {
                state.open.retain(|s| &s.id != id);
                true
            }
            McpEvent::Surfaces { list } => {
                *list = Self::visible(&*state);
                true
            }
            McpEvent::SurfacePatch { id, messages } => {
                let seen = state.open.iter_mut().find(|s| &s.id == id).map(|open| {
                    open.apply_patch(messages);
                    (open.live, open.turn)
                });
                match seen {
                    Some((live, turn)) => Self::shown(&*state, live, turn),
                    None => {
                        tracing::debug!(surface = %id, "mise à jour d'une surface fermée : ignorée");
                        true
                    }
                }
            }
            McpEvent::Changed | McpEvent::Focus { .. } | McpEvent::WidgetTouched { .. } => true,
        };
        if send {
            let _ = self.events.send(event); // personne à l'écoute n'est pas une erreur (comme avant)
        }
    }
```

`resync(&self, scope: AgentScope)` : `self.state().viewing = scope;` puis
`self.emit(McpEvent::Surfaces { list: Vec::new() })` (doc : « Montre le fil de
`scope` ; piloté par le gardien de l'onglet »).

`hub_turns.rs` (`use oku_slides_session::agent::{AgentScope, KEPT_TURNS};`) :

```rust
    /// Ouvre un tour dans `scope` ; ferme les plus anciens tours de la portée au-delà de `KEPT_TURNS`.
    ///
    /// Une seule éviction fait foi, celle-ci (tours relus compris) ; celle du réducteur des
    /// tours de l'agent ne libère que sa propre mémoire.
    pub fn begin_turn(&self, scope: AgentScope) -> u32 {
        let (n, old) = {
            let mut state = self.state();
            state.turns = state.turns.saturating_add(1);
            let n = state.turns;
            state.turn = Some(n);
            state.live_turns.insert(n);
            state.turn_scopes.insert(n, scope.clone());
            let mut same: Vec<u32> = state.turn_scopes.iter().filter(|(_, s)| **s == scope).map(|(t, _)| *t).collect();
            same.sort_unstable();
            let excess = same.len().saturating_sub(KEPT_TURNS);
            (n, same.into_iter().take(excess).collect::<Vec<_>>())
        };
        for t in old {
            self.close_turn(t);
        }
        self.expire_actions();
        n
    }
```

`close_turn` retire aussi `state.turn_scopes.remove(&n)`. `clear_thread` :

```rust
    /// Nouvelle conversation dans `scope` : ses questions relâchées, ses tours et surfaces fermés.
    pub fn clear_thread(&self, scope: &AgentScope) {
        let ids = {
            let mut state = self.state();
            let turns: Vec<u32> = state.turn_scopes.iter().filter(|(_, s)| *s == scope).map(|(t, _)| *t).collect();
            let ids = Self::thread_ids(&state, |t| turns.contains(&t));
            if state.turn.is_some_and(|t| turns.contains(&t)) {
                state.turn = None;
            }
            for t in &turns {
                state.live_turns.remove(t);
                state.reported.remove(t);
                state.turn_scopes.remove(t);
            }
            ids
        };
        self.cancel_asks_where(|id| ids.iter().any(|x| x == id));
        self.close_all(ids);
    }
```

`hub_scopes.rs` (`#[path = "hub_scopes.rs"] mod scopes;` dans `hub.rs`,
`pub use scopes::THREAD_BYTES;` ; export dans `lib.rs`) :

```rust
//! Portées du fil : surfaces montrées, restauration et image du fil.
//!
//! Le hub ne touche jamais à la session : le gardien de l'onglet (agent) lit et écrit.

use std::collections::{BTreeSet, HashSet};

use oku_slides_session::agent::{AgentScope, KEPT_TURNS, Place, SavedConversation, Surface};

use super::{HubState, McpEvent, SurfaceHub};

/// Octets au plus du fil d'une portée enregistré (sous la limite de lecture de 1 Mio).
pub const THREAD_BYTES: usize = 768 * 1024;
/// Plus grand numéro de tour ou d'identifiant accepté à la relecture.
const MAX_RESTORED: u64 = 1_000_000;

/// Le numéro d'un identifiant `préfixe-N`.
fn id_number(id: &str) -> Option<u64> {
    id.rsplit_once('-').and_then(|(_, n)| n.parse().ok())
}

/// Une surface relue, prête à remettre : du fil, numérotée, figée, sans actions.
fn cleaned(s: Surface) -> Option<Surface> {
    let ok = s.place == Place::Thread
        && s.turn.is_some_and(|t| u64::from(t) <= MAX_RESTORED)
        && id_number(&s.id).is_some_and(|n| n <= MAX_RESTORED);
    ok.then(|| {
        let mut s = s.without_actions();
        s.live = false;
        s.question = false;
        s
    })
}

impl SurfaceHub {
    /// Montrée : vivante, hors tour, ou dans le fil vu.
    pub(super) fn shown(state: &HubState, live: bool, turn: Option<u32>) -> bool {
        live || turn.is_none_or(|t| state.turn_scopes.get(&t) == Some(&state.viewing))
    }

    pub(super) fn visible(state: &HubState) -> Vec<Surface> {
        state.open.iter().filter(|s| Self::shown(state, s.live, s.turn)).cloned().collect()
    }

    /// Les surfaces que l'interface montre, dans l'ordre d'ouverture.
    pub fn visible_surfaces(&self) -> Vec<Surface> {
        Self::visible(&self.state())
    }

    /// La portée du tour `turn`, s'il est encore dans le fil.
    pub fn turn_scope(&self, turn: u32) -> Option<AgentScope> {
        self.state().turn_scopes.get(&turn).cloned()
    }

    /// Le widget renommé : ses tours, et la portée vue, le suivent ; piloté par le gardien.
    ///
    /// Aucune liste n'est renvoyée : la portée vue suit, ce que l'interface montre ne change pas.
    pub fn rename_scope(&self, from: &AgentScope, to: &AgentScope) {
        let mut state = self.state();
        for s in state.turn_scopes.values_mut().filter(|s| *s == from) {
            *s = to.clone();
        }
        if state.viewing == *from {
            state.viewing = to.clone();
        }
    }

    /// Remet les fils relus, nettoyés ; piloté par le gardien, avant tout tour.
    ///
    /// Un tour déjà pris (tour ouvert, autre portée) est ignoré ; les suivants sont numérotés après.
    pub fn restore(&self, saved: Vec<SavedConversation>) {
        {
            let mut state = self.state();
            let mut seen: HashSet<String> = state.open.iter().map(|s| s.id.clone()).collect();
            for c in saved {
                let mut thread: Vec<Surface> = c.thread.into_iter().filter_map(cleaned).collect();
                let turns: BTreeSet<u32> = thread.iter().filter_map(|s| s.turn).collect();
                let first = turns.iter().rev().nth(KEPT_TURNS - 1).copied().unwrap_or(0);
                thread.retain(|s| s.turn.is_some_and(|t| t >= first));
                for s in thread {
                    let Some(t) = s.turn else { continue };
                    let taken = state.turn_scopes.get(&t).is_some_and(|x| *x != c.scope)
                        || (state.live_turns.contains(&t) && !state.turn_scopes.contains_key(&t))
                        || state.turn == Some(t);
                    if taken || !seen.insert(s.id.clone()) {
                        tracing::warn!(surface = %s.id, turn = t, "surface relue en conflit : ignorée");
                        continue;
                    }
                    state.turn_scopes.insert(t, c.scope.clone());
                    state.live_turns.insert(t);
                    state.turns = state.turns.max(t);
                    state.next = state.next.max(id_number(&s.id).unwrap_or(0));
                    state.open.push(s);
                }
            }
        }
        self.emit(McpEvent::Surfaces { list: Vec::new() });
    }

    /// Le fil de `scope` à enregistrer : derniers tours clos, figés, sans actions, dans le budget.
    ///
    /// `None` si rien ne tient dans le budget : l'historique enregistré reste tel quel.
    pub fn thread_snapshot(&self, scope: &AgentScope) -> Option<Vec<Surface>> {
        let state = self.state();
        let ours = |s: &&Surface| {
            !s.live && !s.question && s.place == Place::Thread
                && s.turn.is_some_and(|t| state.turn != Some(t))
                && s.turn.and_then(|t| state.turn_scopes.get(&t)).is_some_and(|x| x == scope)
        };
        let turns: BTreeSet<u32> = state.open.iter().filter(ours).filter_map(|s| s.turn).collect();
        if turns.is_empty() {
            return Some(Vec::new());
        }
        let size = |s: &Surface| serde_json::to_vec(&s.without_actions()).map_or(0, |b| b.len());
        let (mut first, mut bytes) = (None, 0usize);
        for &t in turns.iter().rev().take(KEPT_TURNS) {
            let turn_bytes: usize = state.open.iter().filter(ours).filter(|s| s.turn == Some(t)).map(size).sum();
            if bytes + turn_bytes > THREAD_BYTES {
                break;
            }
            bytes += turn_bytes;
            first = Some(t);
        }
        let first = first?;
        Some(state.open.iter().filter(ours).filter(|s| s.turn.is_some_and(|t| t >= first)).map(Surface::without_actions).collect())
    }
}

#[cfg(test)]
#[path = "hub_scopes_tests.rs"]
mod hub_scopes_tests;
```

Appelants :
- tests du crate mcp (`tests/surfaces.rs`, `receipts.rs`, `turns.rs`) :
  `begin_turn()` → `begin_turn(AgentScope::Slides)` ; `resync()` → `resync(AgentScope::Slides)` ;
  `clear_thread()` → `clear_thread(&AgentScope::Slides)`.
- `oku-slides-agent` (ponts jusqu'à Task 7) : `tab_agent.rs` `begin_turn(AgentScope::Slides)`,
  `clear_thread(&AgentScope::Slides)` ; `tab_entry.rs:226` `begin_turn(AgentScope::Slides)` ;
  `stack.rs` `t.hub.resync(AgentScope::Slides)` ; **`tab_entry.rs:140`** :
  `let list = hub.visible_surfaces();` (définitif : le relais en retard n'envoie que le fil vu).
- `crates/oku-slides-mcp/ARCHITECTURE.md:58` : `KEPT_TURNS` (`oku-slides-session::agent`).

- [ ] **Step 4 : lancer, constater le succès**

Run : `cargo nextest run -p oku-slides-mcp -p oku-slides-agent && cargo build --workspace --all-targets`
Expected : PASS (anciens tests compris).

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-mcp/src/hub.rs crates/oku-slides-mcp/src/hub_turns.rs crates/oku-slides-mcp/src/hub_scopes.rs crates/oku-slides-mcp/src/hub_scopes_tests.rs crates/oku-slides-mcp/src/lib.rs crates/oku-slides-mcp/ARCHITECTURE.md
git add crates/oku-slides-mcp/tests/surfaces.rs crates/oku-slides-mcp/tests/receipts.rs crates/oku-slides-mcp/tests/turns.rs
git add crates/oku-slides-agent/src/tab_agent.rs crates/oku-slides-agent/src/tab_entry.rs crates/oku-slides-agent/src/stack.rs
git status --short crates/oku-slides-mcp crates/oku-slides-agent   # rien d'autre de modifié par cette tâche
git diff --cached --name-only
git commit -m "feat(mcp) : un fil par portée dans le hub, restauration nettoyée et image du fil (t-598)"
```

(si le compilateur a désigné un autre fichier de test, l'ajouter par son nom.)

---

### Task 5 : Évènements et tours portent la portée

**Files :**
- Modify : `crates/oku-slides-agent/Cargo.toml` (`[dev-dependencies]` : `uuid.workspace = true`)
- Modify : `crates/oku-slides-agent/src/events.rs`, `turns.rs` ; Test : `turns_tests.rs`, `events_tests.rs`
- Modify (ponts, Slides/None) : `src/tab_agent.rs`, `src/turn.rs`, `src/tab_entry.rs`, `src/tab_entry_tests.rs` (l.26, `Conversation`),
  `src/stack.rs`, `tests/common/mod.rs` (`wait_status`), `tests/turns.rs` (l.103, 134)

**Interfaces :**
- Produces :
  ```rust
  AgentEvent::Status { state: AgentState, #[serde(skip_serializing_if = "Option::is_none")] scope: Option<AgentScope> }
  AgentEvent::UserText { turn: u32, scope: AgentScope, text: String }
  AgentEvent::Conversation { scope: AgentScope }
  impl AgentEvent { pub fn idle() -> Self }
  ```

- [ ] **Step 1 : tests rouges** — `turns_tests.rs` : `user(turn)` passe
`scope: AgentScope::Slides` ; ajouter :

```rust
fn user_in(turn: u32, scope: AgentScope) -> AgentEvent {
    AgentEvent::UserText { turn, scope, text: "Change le fond".into() }
}

fn atelier() -> AgentScope {
    AgentScope::Atelier { pack: uuid::Uuid::nil() }
}

#[test]
fn turns_evict_only_the_oldest_turn_of_the_same_scope() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user_in(1, atelier()), now, &fr());
    t.apply(&AgentEvent::TurnEnded { turn: 1, stop: "end_turn".into() }, now, &fr());
    let mut evicted = Vec::new();
    for n in 2..=(KEPT_TURNS as u32 + 2) {
        for out in t.apply(&user_in(n, AgentScope::Slides), now, &fr()) {
            if let TurnOut::Evict(k) = out {
                evicted.push(k);
            }
        }
    }
    assert_that!(evicted).is_equal_to(vec![2]);
}

#[test]
fn turns_conversation_forgets_only_its_scope() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user_in(1, atelier()), now, &fr());
    t.apply(&user_in(2, AgentScope::Slides), now, &fr());
    t.apply(&AgentEvent::Conversation { scope: AgentScope::Slides }, now, &fr());
    assert_that!(t.turns.iter().map(|x| x.view.n).collect::<Vec<_>>()).is_equal_to(vec![1]);
}
```

`events_tests.rs` :

```rust
#[test]
fn status_json_carries_the_scope_only_when_set() {
    let busy = AgentEvent::Status { state: AgentState::Working, scope: Some(AgentScope::Slides) };
    assert_that!(serde_json::to_string(&busy).unwrap())
        .is_equal_to(r#"{"kind":"status","state":"working","scope":{"kind":"slides"}}"#.to_owned());
    assert_that!(serde_json::to_string(&AgentEvent::idle()).unwrap()).is_equal_to(r#"{"kind":"status","state":"idle"}"#.to_owned());
}
```

- [ ] **Step 2 : lancer, constater l'échec**

Run : `cargo nextest run -p oku-slides-agent --lib`
Expected : erreurs de compilation.

- [ ] **Step 3 : implémenter**

`events.rs` : les trois variantes (champs documentés) et

```rust
impl AgentEvent {
    /// L'agent au repos, sans portée.
    pub fn idle() -> Self {
        Self::Status { state: AgentState::Idle, scope: None }
    }
}
```

`turns.rs` : `Turn` gagne `scope: AgentScope` ; `open(n, scope, prompt, now, l)` ;
un tour ouvert sans consigne (évènement d'un tour inconnu) prend la portée du
dernier tour, ou `Slides`. Éviction (mémoire du réducteur seulement ; le hub
fait foi) :

```rust
        let same = self.turns.iter().filter(|t| t.scope == scope).count();
        if same > KEPT_TURNS
            && let Some(i) = self.turns.iter().position(|t| t.scope == scope)
            && let Some(old) = self.turns.remove(i)
        {
            out.push(TurnOut::Evict(old.view.n));
        }
```

`AgentEvent::Conversation { scope }` : `self.turns.retain(|t| t.scope != *scope); return Vec::new();`.
Motifs `AgentEvent::Status { state: AgentState::Idle }` → `{ state: AgentState::Idle, .. }`.

Ponts (définitifs en Task 7) : chaque `AgentEvent::Status { state: AgentState::Idle }`
construit → `AgentEvent::idle()` ; `Starting`/`Working` → `scope: Some(AgentScope::Slides)` ;
`UserText { .., scope: AgentScope::Slides }` ; `Conversation { scope: AgentScope::Slides }`
(dont `src/tab_entry_tests.rs:26`, `tests/turns.rs:103,134`) ; `tests/common/mod.rs:191`
`wait_status` : motif `Status { state: s, .. }` ; `tab_entry.rs`
`status: Arc<Mutex<(AgentState, Option<AgentScope>)>>`, `replay` émet
`Status { state, scope: scope.clone() }`, `Relay::emit` garde les deux.

- [ ] **Step 4 : lancer, constater le succès**

Run : `cargo nextest run -p oku-slides-agent && cargo build --workspace --all-targets`
Expected : PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-agent/Cargo.toml crates/oku-slides-agent/src/events.rs crates/oku-slides-agent/src/events_tests.rs crates/oku-slides-agent/src/turns.rs crates/oku-slides-agent/src/turns_tests.rs
git add crates/oku-slides-agent/src/tab_agent.rs crates/oku-slides-agent/src/turn.rs crates/oku-slides-agent/src/tab_entry.rs crates/oku-slides-agent/src/tab_entry_tests.rs crates/oku-slides-agent/src/stack.rs
git add crates/oku-slides-agent/tests/common/mod.rs crates/oku-slides-agent/tests/turns.rs
git status --short crates/oku-slides-agent
git diff --cached --name-only
git commit -m "feat(agent) : les évènements et les tours portent leur portée (t-598)"
```

---

### Task 6 : Connexion — capacités de reprise, empreinte de l'agent, faux agent

**Files :**
- Modify : `crates/oku-slides-agent/Cargo.toml` (`hex`, `sha2` du workspace), `src/link.rs`, `src/lib.rs`, `src/bin/fake_agent.rs`
- Test : `crates/oku-slides-agent/src/link_tests.rs` (nouveau), `tests/launch.rs`

**Interfaces :**
- Produces :
  ```rust
  pub(crate) struct SessionCaps { pub load: bool, pub resume: bool }   // Live.caps
  pub fn agent_fingerprint(command: &str, args: &[String]) -> String;   // sha256 hex ; pub : utilisé par tests/common
  impl Link { pub fn agent_id(&self) -> String; pub(crate) fn current_generation(&self) -> Option<u64>; }
  ```
  Faux agent : scénario `load_session`, `resume`, `unknown_sessions` (bool),
  `replay: Vec<String>`, `by_prompt: { "<texte>": Turn }` ; journal `session/load`, `session/resume`.

- [ ] **Step 1 : tests rouges**

`link_tests.rs` (`#[cfg(test)] #[path = "link_tests.rs"] mod link_tests;` en bas de `link.rs`) :

```rust
use speculoos::prelude::*;

use super::agent_fingerprint;

#[test]
fn agent_fingerprint_hides_the_arguments() {
    let id = agent_fingerprint("npx", &["--api-key".into(), "secret".into()]);
    assert_that!(id.contains("secret")).is_false();
    assert_that!(id.len()).is_equal_to(64);
}

#[test]
fn agent_fingerprint_separates_command_and_arguments() {
    assert_that!(agent_fingerprint("ab", &["c".into()])).is_not_equal_to(agent_fingerprint("a", &["bc".into()]));
}
```

`tests/launch.rs` (ajouter) :

```rust
#[tokio::test(flavor = "multi_thread")]
async fn agent_id_is_the_fingerprint_of_the_launch() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    let id = h.link.agent_id();
    assert_that!(id.len()).is_equal_to(64);
    assert_that!(id.contains("script")).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn fake_agent_by_prompt_plays_the_matching_turn() {
    let h = start(json!({ "by_prompt": { "bonjour": { "chunks": ["salut"] } }, "turns": [{ "chunks": ["autre"] }] })).await;
    let mut rx = h.tab("t-1").agent.subscribe();
    h.tab("t-1").agent.prompt("bonjour".into(), AgentUi::default());
    let chunk = until(&mut rx, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(chunk).is_equal_to(AgentEvent::Chunk { turn: 1, text: "salut".into() });
}
```

- [ ] **Step 2 : lancer, constater l'échec**

Run : `cargo nextest run -p oku-slides-agent -E 'test(agent_fingerprint) | test(agent_id_is) | test(by_prompt)'`
Expected : erreurs de compilation / FAIL.

- [ ] **Step 3 : implémenter**

`link.rs` :

```rust
/// Ce que l'agent sait faire d'une session existante.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SessionCaps {
    /// `session/load` (rejoue l'historique).
    pub load: bool,
    /// `session/resume` (sans rejeu).
    pub resume: bool,
}

/// Empreinte d'un lancement d'agent : ses arguments ne sont jamais écrits en clair.
pub fn agent_fingerprint(command: &str, args: &[String]) -> String {
    let mut h = Sha256::new();
    h.update(command.as_bytes());
    for a in args {
        h.update([0u8]);
        h.update(a.as_bytes());
    }
    hex::encode(h.finalize())
}
```

`Live` gagne `pub caps: SessionCaps`. `handshake` rend
`agent_client_protocol::Result<Option<SessionCaps>>` :

```rust
    let caps = &init.agent_capabilities;
    Ok(caps.mcp_capabilities.http.then(|| SessionCaps {
        load: caps.load_session,
        resume: caps.session_capabilities.resume.is_some(),
    }))
```

`Serving::run` : `Ok(Some(caps))` → `Live { cx, generation, closed, caps }`,
`Ok(None)` → `NoHttp`. `Link` :

```rust
    /// L'empreinte de l'agent lancé : une session ne se reprend qu'avec le même.
    pub fn agent_id(&self) -> String {
        agent_fingerprint(&self.inner.launch.command, &self.inner.launch.args)
    }

    /// Le lancement vivant, s'il y en a un ; `None` aussi pendant un lancement (verrou pris).
    pub(crate) fn current_generation(&self) -> Option<u64> {
        let state = self.inner.state.try_lock().ok()?;
        state.live.as_ref().filter(|l| !l.closed.is_cancelled()).map(|l| l.generation)
    }
```

`lib.rs` : `pub use link::{Link, agent_fingerprint};`.

`fake_agent.rs` : champs du `Script` (`#[serde(default)]`) `load_session: bool`,
`resume: bool`, `unknown_sessions: bool`, `replay: Vec<String>`,
`by_prompt: HashMap<String, Turn>` (mettre à jour le `Script { … }` par défaut de `main`).
Extraire de `session/new` :

```rust
impl State {
    /// Range le serveur MCP HTTP de `session` (URL et `Authorization`).
    fn remember_mcp(&self, session: &str, servers: &[McpServer]) {
        let http = servers.iter().find_map(|s| match s {
            McpServer::Http(h) => {
                let auth = h.headers.iter().find(|x| x.name == "Authorization").map(|x| x.value.clone())?;
                Some((h.url.clone(), auth))
            }
            _ => None,
        });
        if let Some(http) = http {
            self.mcp.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(session.to_owned(), http);
        }
    }
}
```

`initialize` :

```rust
let sessions = if s1.script.resume {
    SessionCapabilities::new().resume(SessionResumeCapabilities::new())
} else {
    SessionCapabilities::new()
};
responder.respond(InitializeResponse::new(req.protocol_version).agent_capabilities(
    AgentCapabilities::new()
        .mcp_capabilities(McpCapabilities::new().http(s1.script.http))
        .load_session(s1.script.load_session)
        .session_capabilities(sessions),
))
```

Deux gestionnaires (`let (s5, s6) = (state.clone(), state.clone());`) :

```rust
        .on_receive_request(
            async move |req: LoadSessionRequest, responder, cx: ConnectionTo<Client>| {
                s5.log("session/load", serde_json::to_value(&req).unwrap_or_default());
                if s5.script.unknown_sessions {
                    return responder.respond_with_internal_error("unknown session");
                }
                s5.remember_mcp(&req.session_id.0, &req.mcp_servers);
                for text in &s5.script.replay {
                    cx.send_notification(chunk(&req.session_id, text.clone()))?;
                }
                responder.respond(LoadSessionResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |req: ResumeSessionRequest, responder, _cx: ConnectionTo<Client>| {
                s6.log("session/resume", serde_json::to_value(&req).unwrap_or_default());
                if s6.script.unknown_sessions {
                    return responder.respond_with_internal_error("unknown session");
                }
                s6.remember_mcp(&req.session_id.0, &req.mcp_servers);
                responder.respond(ResumeSessionResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
```

Dans `session/prompt`, avant le choix par numéro de session :

```rust
let said = req.prompt.iter().rev().find_map(|b| match b {
    ContentBlock::Text(t) => Some(t.text.as_str()),
    _ => None,
});
let by_prompt = said.and_then(|t| s3.script.by_prompt.get(t)).cloned();
let turns_script = s3.script.sessions.get(&rank.to_string()).unwrap_or(&s3.script.turns);
let turn = match by_prompt {
    Some(t) => t,
    None => match turns_script.len() {
        0 => Turn::default(),
        n => turns_script[index_in_session % n].clone(),
    },
};
```

Mettre à jour le commentaire de tête (scénario) avec les nouveaux champs.

- [ ] **Step 4 : lancer, constater le succès**

Run : `cargo nextest run -p oku-slides-agent && cargo build --workspace --all-targets`
Expected : PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-agent/Cargo.toml crates/oku-slides-agent/src/link.rs crates/oku-slides-agent/src/link_tests.rs crates/oku-slides-agent/src/lib.rs crates/oku-slides-agent/src/bin/fake_agent.rs crates/oku-slides-agent/tests/launch.rs
git diff --cached --name-only
git commit -m "feat(agent) : capacités de reprise de session, empreinte de l'agent, faux load/resume (t-598)"
```

---

### Task 7 : Gardien et acteur — une session ACP par portée, reprise, note

**Files :**
- Create : `crates/oku-slides-agent/src/keeper.rs`
- Modify : `crates/oku-slides-agent/src/tab_agent.rs`, `turn.rs`, `stack.rs`, `tab_entry.rs`, `lib.rs`, `examples/smoke.rs`
- Modify : `crates/oku-slides-i18n/src/locales/*/agent.ftl` (20 fichiers)
- Test : `crates/oku-slides-agent/tests/scopes.rs` (nouveau), `tests/stack.rs`, `tests/common/mod.rs`, `tests/turns.rs`, `tests/launch.rs`

**Interfaces :**
- Consumes : Tasks 2–6.
- Produces :
  ```rust
  pub enum Keep {
      Thread(u32),
      Session { scope: AgentScope, id: String, agent: String },
      Forget(AgentScope, oneshot::Sender<Option<String>>),        // rend l'id oublié
      Lookup(AgentScope, oneshot::Sender<Option<(String, Option<String>)>>),  // id, empreinte
      Show(AgentScope),
      #[doc(hidden)] Flush(oneshot::Sender<()>),
  }
  #[derive(Clone)] pub struct Keeper;   // spawn(tab, hub) ; send ; restored().await ; flush().await ; lookup(scope).await ; forget(scope).await
  pub(crate) enum Command { Prompt { scope: AgentScope, text: String, ui: AgentUi }, Cancel, NewConversation { scope: AgentScope } }
  impl AgentHandle {
      pub fn spawn(tab: TabSession, hub: Arc<SurfaceHub>, tools: ToolsAccess, link: Link, localizer: Arc<Localizer>, keeper: Keeper) -> Self;
      pub fn prompt(&self, text: String, ui: AgentUi);                       // Slides
      pub fn prompt_in(&self, scope: AgentScope, text: String, ui: AgentUi);
      pub fn new_conversation(&self, scope: AgentScope);
  }
  AgentCommand::Prompt { tab, #[serde(default)] scope, text, ui, answer }
  AgentCommand::NewConversation { tab, #[serde(default)] scope }
  AgentCommand::Resync { tab, #[serde(default)] scope }
  ```
  `Keeper`/`Keep` sont `pub` parce que `tests/common` construit des onglets sans `Stack` ;
  leur doc le dit (« piloté par la pile ; public pour les tests d'intégration »). Clé Fluent `agent-memory-lost`.

- [ ] **Step 1 : tests rouges**

`tests/common/mod.rs` — ajouts (helpers complets) :

```rust
use oku_slides_agent::{Keep, Keeper, agent_fingerprint};
use oku_slides_session::agent::turn_view::{TurnState, TurnView, turn_surface};
use oku_slides_session::agent::{AgentScope, Surface};

/// La surface du tour `n`, clos (pour remplir un fil).
pub fn frozen_turn(n: u32) -> Surface {
    let view = TurnView { n, prompt: format!("consigne {n}"), plan: vec![], steps: vec![], tools: 0, notes: 0,
        narration: String::new(), state: TurnState::Done { secs: 1 } };
    turn_surface(&view, &oku_slides_i18n::Localizer::new(Some("fr")))
}

/// La session de `t-1` connaît déjà `scope` → `session` (relue au lancement, par l'agent `agent`).
pub async fn seed_session(h: &Harness, scope: AgentScope, session: &str, agent: &str) {
    h.session("t-1").lock().unwrap().keep_agent_session(scope, session.into(), agent.into());
    h.tab("t-1").keeper.flush().await;
}

/// Les `sessionId` des messages `method` du journal du faux agent.
pub fn sessions_in(log: &std::path::Path, method: &str) -> Vec<String> {
    logged(log, method).iter().map(|m| m["params"]["sessionId"].as_str().unwrap_or_default().to_owned()).collect()
}
```

Dans `start_tabs_ready_within`, pour chaque onglet :

```rust
        let keeper = Keeper::spawn(tab.clone(), hub.clone());
        keeper.restored().await;
        let agent = AgentHandle::spawn(tab, hub.clone(), access, link.clone(), platform.localizer.clone(), keeper.clone());
        // … `Tab { …, keeper }` (nouveau champ `pub keeper: Keeper`)
```

Variante de la pile reprise d'un dossier (à côté de `stack_with_sink_and_log`,
même corps ; seule la création de la session de `t-1` change) :

```rust
/// Une pile dont la session `t-1` a été reprise d'un dossier où `seed` a déjà écrit.
///
/// `seed` reçoit la session et l'empreinte de l'agent de la pile ; la session est ensuite
/// lâchée puis reprise du même dossier, comme après un redémarrage.
pub async fn stack_resumed(script: Value, seed: impl FnOnce(&mut Session, &str) + Send + 'static) -> (TestStack, Relayed, PathBuf, PathBuf) {
    // `dir`, `script_path`, `log`, `args`, `data` comme `stack_with_sink_and_log`.
    let resume = data.join("resume-t-1");
    let fingerprint = agent_fingerprint(env!("CARGO_BIN_EXE_oku-slides-fake-agent"), &args);
    // Dans le `spawn_blocking` qui crée les sessions, pour `t-1` :
    let mut first = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {e}"));
    assert!(first.resume(resume.clone()));
    first.open(&fixture).unwrap_or_else(|e| panic!("ouverture : {e}"));
    seed(&mut first, &fingerprint);
    first.flush_resume();
    drop(first);
    let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {e}"));
    assert!(session.resume(resume.clone()));
    // … insérer `session` sous "t-1", puis démarrer la pile comme `stack_with_sink_and_log`.
    // Rendre aussi `resume` : un test peut reprendre la même session une seconde fois.
}
```

(factoriser avec `stack_with_sink_and_log` : un paramètre `prepare: Option<…>`
plutôt qu'une copie ; garder le nom public `stack_resumed`.)

`tests/scopes.rs` :

```rust
//! Une session ACP par portée ; reprise d'une session connue ; gardien.

mod common;

use common::*;
use oku_slides_agent::{AgentEvent, AgentState, Keep};
use oku_slides_session::agent::{AgentScope, AgentUi};
use oku_slides_session::pack::DEFAULT_PACK;
use serde_json::json;
use speculoos::prelude::*;

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

fn atelier() -> AgentScope {
    AgentScope::Atelier { pack: DEFAULT_PACK }
}

fn widget(name: &str) -> AgentScope {
    AgentScope::Widget { pack: DEFAULT_PACK, name: name.into() }
}

/// `t-1` en édition du pack par défaut, avec le widget `kpi`.
fn editing_with_kpi(h: &Harness) {
    let mut s = h.session("t-1").lock().unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_widget("kpi", KPI.into()).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn each_scope_gets_its_own_session_and_keeps_it() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    let t = h.tab("t-1");
    for (scope, text) in [(AgentScope::Slides, "un"), (atelier(), "deux"), (AgentScope::Slides, "trois")] {
        t.agent.prompt_in(scope, text.into(), AgentUi::default());
        wait_turn_ended(t).await;
    }
    assert_that!(logged(&h.log, "session/new")).has_length(2);
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec!["sess-1".into(), "sess-2".into(), "sess-1".into()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn status_carries_the_scope_of_the_turn() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt_in(atelier(), "un".into(), AgentUi::default());
    let working = until(&mut rx, |e| matches!(e, AgentEvent::Status { state: AgentState::Working, .. })).await;
    assert_that!(working).is_equal_to(AgentEvent::Status { state: AgentState::Working, scope: Some(atelier()) });
    let idle = until(&mut rx, |e| matches!(e, AgentEvent::Status { state: AgentState::Idle, .. })).await;
    assert_that!(idle).is_equal_to(AgentEvent::idle());
}

#[tokio::test(flavor = "multi_thread")]
async fn known_session_is_resumed_at_the_first_prompt_only() {
    let h = start(json!({ "resume": true, "load_session": true, "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, AgentScope::Slides, "sess-42", &h.link.agent_id()).await;
    let t = h.tab("t-1");
    // Rien n'est repris avant un prompt (contrôle faible : l'absence au bout de 100 ms).
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_that!(logged(&h.log, "session/resume")).is_empty();
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/resume")).is_equal_to(vec!["sess-42".to_owned()]);
    assert_that!(logged(&h.log, "session/load")).is_empty();
    assert_that!(logged(&h.log, "session/new")).is_empty();
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec!["sess-42".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn loaded_session_replay_stays_out_of_events_and_surfaces() {
    let h = start(json!({ "load_session": true, "replay": ["ancien texte"], "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, AgentScope::Slides, "sess-42", &h.link.agent_id()).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/load")).is_equal_to(vec!["sess-42".to_owned()]);
    let mut chunks = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let AgentEvent::Chunk { text, .. } = e {
            chunks.push(text);
        }
    }
    assert_that!(chunks).is_equal_to(vec!["ok".to_owned()]);
    assert_that!(serde_json::to_string(&t.hub.surfaces()).unwrap()).does_not_contain("ancien texte");
}

#[tokio::test(flavor = "multi_thread")]
async fn refused_resume_opens_a_new_session_with_a_card_and_instructions() {
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    let lost: String = l.tr("agent-memory-lost").chars().take(30).collect();
    let instructions: String = l.tr("agent-instructions").chars().take(40).collect();
    for caps in [json!({ "resume": true }), json!({ "load_session": true }), json!({})] {
        let mut script = json!({ "unknown_sessions": true, "turns": [{ "chunks": ["ok"] }] });
        script.as_object_mut().unwrap().extend(caps.as_object().unwrap().clone());
        let h = start(script).await;
        seed_session(&h, AgentScope::Slides, "sess-42", &h.link.agent_id()).await;
        let t = h.tab("t-1");
        t.agent.prompt("encore".into(), AgentUi::default());
        wait_turn_ended(t).await;
        assert_that!(logged(&h.log, "session/new")).has_length(1);
        assert_that!(serde_json::to_string(&t.hub.surfaces()).unwrap()).contains(lost.as_str());
        assert_that!(logged(&h.log, "session/prompt")[0].to_string()).contains(instructions.as_str());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn session_of_another_agent_is_not_resumed() {
    let h = start(json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] })).await;
    seed_session(&h, AgentScope::Slides, "sess-42", "autre-agent").await;
    let t = h.tab("t-1");
    t.agent.prompt("encore".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(logged(&h.log, "session/resume")).is_empty();
    assert_that!(logged(&h.log, "session/new")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn session_is_resumed_after_the_agent_stops() {
    let h = start(json!({ "resume": true, "turns": [{ "chunks": ["ok"] }, { "crash": true }, { "chunks": ["ok"] }] })).await;
    let t = h.tab("t-1");
    t.agent.prompt("un".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    {
        let mut rx = t.events.lock().await;
        wait_error(&mut rx).await;
    }
    t.agent.prompt("trois".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/resume")).is_equal_to(vec!["sess-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_of_another_scope_during_a_turn_runs_next_in_its_own_session() {
    let h = start(json!({ "by_prompt": { "long": { "delay_ms": 600, "chunks": ["lent"] }, "court": { "chunks": ["vite"] } } })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt_in(atelier(), "long".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::Status { state: AgentState::Working, .. })).await;
    t.agent.prompt_in(AgentScope::Slides, "court".into(), AgentUi::default());
    // Le tour de l'atelier est annulé puis la consigne des slides part (le faux agent
    // répond `end_turn` même annulé, sans `wait_cancel` : seule la suite compte ici).
    let vite = until(&mut rx, |e| matches!(e, AgentEvent::Chunk { .. })).await;
    assert_that!(vite).is_equal_to(AgentEvent::Chunk { turn: 2, text: "vite".into() });
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec!["sess-1".into(), "sess-2".into()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_conversation_then_prompt_never_resumes_the_forgotten_session() {
    let h = start(json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] })).await;
    let t = h.tab("t-1");
    t.agent.prompt_in(atelier(), "un".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.prompt_in(AgentScope::Slides, "deux".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.new_conversation(atelier());
    t.agent.prompt_in(atelier(), "trois".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.prompt_in(AgentScope::Slides, "quatre".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(logged(&h.log, "session/resume")).is_empty();
    assert_that!(sessions_in(&h.log, "session/prompt"))
        .is_equal_to(vec!["sess-1".into(), "sess-2".into(), "sess-3".into(), "sess-2".into()]);
    assert_that!(h.session("t-1").lock().unwrap().conversation(&atelier()).and_then(|c| c.session.clone()))
        .is_equal_to(Some("sess-3".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn renamed_widget_keeps_its_session_and_a_new_widget_of_the_old_name_gets_a_new_one() {
    let h = start(json!({ "turns": [{ "chunks": ["ok"] }] })).await;
    editing_with_kpi(&h);
    let t = h.tab("t-1");
    t.agent.prompt_in(widget("kpi"), "un".into(), AgentUi::default());
    wait_turn_ended(t).await;
    h.session("t-1").lock().unwrap().rename_widget("kpi", "indicateur").unwrap();
    h.session("t-1").lock().unwrap().set_widget("kpi", KPI.into()).unwrap();
    t.agent.prompt_in(widget("indicateur"), "deux".into(), AgentUi::default());
    wait_turn_ended(t).await;
    t.agent.prompt_in(widget("kpi"), "trois".into(), AgentUi::default());
    wait_turn_ended(t).await;
    assert_that!(sessions_in(&h.log, "session/prompt")).is_equal_to(vec!["sess-1".into(), "sess-1".into(), "sess-2".into()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn keeper_writes_the_thread_of_a_turn_under_its_renamed_scope() {
    let h = start(json!({ "turns": [] })).await;
    editing_with_kpi(&h);
    let t = h.tab("t-1");
    let n = t.hub.begin_turn(widget("kpi"));
    t.hub.emit(oku_slides_mcp::McpEvent::Surface { surface: frozen_turn(n) });
    t.hub.end_turn(n, "");
    h.session("t-1").lock().unwrap().rename_widget("kpi", "indicateur").unwrap();
    t.keeper.send(Keep::Thread(n));
    t.keeper.flush().await;
    let s = h.session("t-1").lock().unwrap();
    assert_that!(s.conversation(&widget("indicateur")).map(|c| c.thread.len())).is_equal_to(Some(1));
    assert_that!(s.conversation(&widget("kpi"))).is_none();
}

#[tokio::test(flavor = "multi_thread")]
async fn keeper_forget_after_a_queued_write_leaves_nothing() {
    let h = start(json!({ "turns": [] })).await;
    let t = h.tab("t-1");
    let n = t.hub.begin_turn(AgentScope::Slides);
    t.hub.emit(oku_slides_mcp::McpEvent::Surface { surface: frozen_turn(n) });
    t.hub.end_turn(n, "");
    t.keeper.send(Keep::Thread(n));
    assert_that!(t.keeper.forget(AgentScope::Slides).await).is_none();
    assert_that!(h.session("t-1").lock().unwrap().conversation(&AgentScope::Slides)).is_none();
}
```

`tests/stack.rs` (ajouter ; `stack_resumed` et `frozen_turn` viennent de `common`) :

```rust
#[tokio::test(flavor = "multi_thread")]
async fn restart_shows_the_saved_thread_and_resumes_its_session() {
    let script = json!({ "resume": true, "turns": [{ "chunks": ["ok"] }] });
    let (stack, rx, log, _resume) = stack_resumed(script, |s, agent| {
        s.keep_thread(AgentScope::Slides, vec![frozen_turn(3)]);
        s.keep_agent_session(AgentScope::Slides, "sess-42".into(), agent.into());
    })
    .await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1", "scope": { "kind": "slides" } }))).unwrap();
    let surfaces = wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "surfaces"
        && v["event"]["list"].as_array().is_some_and(|l| !l.is_empty())).await;
    assert_that!(surfaces.1["event"]["list"][0]["id"].clone()).is_equal_to(json!("turn-3"));
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "encore" }))).unwrap();
    wait_turn_done(&rx, "t-1", 4).await;
    assert_that!(sessions_in(&log, "session/resume")).is_equal_to(vec!["sess-42".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_turn_ended_before_shutdown_is_read_back_after_a_restart() {
    let script = json!({ "turns": [{ "chunks": ["ok"] }] });
    let (stack, rx, _log, resume) = stack_resumed(script.clone(), |_, _| {}).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    stack.shutdown().await; // vide la file du gardien
    drop(stack);
    // Second lancement sur le même dossier de reprise : le fil du tour 1 y est.
    let (stack, rx, _log, _) = stack_resumed_from(script, resume).await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" }))).unwrap();
    let surfaces = wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "surfaces"
        && v["event"]["list"].as_array().is_some_and(|l| !l.is_empty())).await;
    assert_that!(surfaces.1["event"]["list"][0]["id"].clone()).is_equal_to(json!("turn-1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn command_without_scope_reads_as_slides() {
    match cmd(json!({ "cmd": "resync", "tab": "t-1" })) {
        AgentCommand::Resync { scope, .. } => assert_that!(scope).is_equal_to(AgentScope::Slides),
        other => panic!("resync attendu : {other:?}"),
    }
}
```

(`stack_resumed_from(script, resume_dir)` : la même pile, sans `seed`, sur un
dossier de reprise déjà rempli — même factorisation que `stack_resumed`.)
Renommer `resync_of_an_unused_tab_sends_idle_and_no_surfaces` →
`resync_of_a_tab_without_conversation_sends_idle_and_no_surfaces`, et
`new_conversation_on_a_tab_without_agent_starts_nothing` →
`new_conversation_on_a_tab_without_agent_starts_no_agent_process`.

- [ ] **Step 2 : lancer, constater l'échec**

Run : `cargo nextest run -p oku-slides-agent`
Expected : erreurs de compilation.

- [ ] **Step 3 : implémenter**

`keeper.rs` :

```rust
//! Le gardien d'un onglet : seul passage vers les conversations de la session.
//!
//! Une tâche qui applique ses ordres un par un, hors du fil async : l'ordre de la
//! file règle les courses (écriture puis oubli, renommage puis écriture, oubli puis lecture).
//! Piloté par la pile ; public pour les tests d'intégration.

use std::sync::Arc;

use oku_slides_mcp::SurfaceHub;
use oku_slides_session::agent::{AgentScope, ScopeMove, TabSession};
use tokio::sync::{mpsc, oneshot, watch};

/// Un ordre au gardien.
#[derive(Debug)]
pub enum Keep {
    /// Fin du tour `n` : écrire le fil de sa portée (lue au moment d'écrire).
    Thread(u32),
    /// La session ACP de `scope` et l'empreinte de son agent.
    Session {
        /// La portée (réécrite selon un renommage appliqué entre-temps).
        scope: AgentScope,
        /// L'identifiant de la session ACP.
        id: String,
        /// L'empreinte de l'agent.
        agent: String,
    },
    /// Nouvelle conversation dans `scope` ; rend l'id de session oublié.
    Forget(AgentScope, oneshot::Sender<Option<String>>),
    /// La session connue de `scope` et l'empreinte de son agent, dans l'ordre de la file.
    Lookup(AgentScope, oneshot::Sender<Option<(String, Option<String>)>>),
    /// Montrer le fil de `scope` à l'interface.
    Show(AgentScope),
    /// Attendre que les ordres précédents soient appliqués.
    #[doc(hidden)]
    Flush(oneshot::Sender<()>),
}

/// Le gardien d'un onglet, vu par l'acteur, le relais et la pile.
#[derive(Debug, Clone)]
pub struct Keeper {
    tx: mpsc::UnboundedSender<Keep>,
    restored: watch::Receiver<bool>,
}

impl Keeper {
    /// Démarre le gardien : il relit d'abord les conversations et les rend au hub.
    pub fn spawn(tab: TabSession, hub: Arc<SurfaceHub>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (done, restored) = watch::channel(false);
        tokio::spawn(run(tab, hub, rx, done));
        Self { tx, restored }
    }

    /// Envoie un ordre ; un gardien arrêté (onglet fermé) l'ignore.
    pub fn send(&self, k: Keep) {
        if self.tx.send(k).is_err() {
            tracing::debug!("gardien arrêté : ordre ignoré");
        }
    }

    /// Attend la relecture des conversations.
    pub async fn restored(&self) {
        let mut rx = self.restored.clone();
        if rx.wait_for(|done| *done).await.is_err() {
            tracing::debug!("gardien arrêté avant la relecture");
        }
    }

    /// Attend que les ordres déjà envoyés soient appliqués.
    pub async fn flush(&self) {
        let (tx, rx) = oneshot::channel();
        self.send(Keep::Flush(tx));
        if rx.await.is_err() {
            tracing::debug!("gardien arrêté pendant l'attente");
        }
    }

    /// La session connue de `scope` (id, empreinte de l'agent), après les ordres déjà envoyés.
    pub async fn lookup(&self, scope: AgentScope) -> Option<(String, Option<String>)> {
        let (tx, rx) = oneshot::channel();
        self.send(Keep::Lookup(scope, tx));
        rx.await.ok().flatten()
    }

    /// Oublie la conversation de `scope` ; rend l'id de session oublié.
    pub async fn forget(&self, scope: AgentScope) -> Option<String> {
        let (tx, rx) = oneshot::channel();
        self.send(Keep::Forget(scope, tx));
        rx.await.ok().flatten()
    }
}

async fn run(tab: TabSession, hub: Arc<SurfaceHub>, mut rx: mpsc::UnboundedReceiver<Keep>, done: watch::Sender<bool>) {
    let (t, h) = (tab.clone(), hub.clone());
    match tokio::task::spawn_blocking(move || t.with(|s| s.conversations())).await {
        Ok(Ok(saved)) => h.restore(saved),
        Ok(Err(e)) => tracing::warn!(error = %e.one_line(), "conversations de l'agent non relues"),
        Err(e) => tracing::warn!(error = %e, "relecture des conversations perdue"),
    }
    done.send_replace(true);
    while let Some(k) = rx.recv().await {
        let (t, h) = (tab.clone(), hub.clone());
        if let Err(e) = tokio::task::spawn_blocking(move || apply(&t, &h, k)).await {
            tracing::warn!(error = %e, "ordre du gardien perdu");
        }
    }
}

/// Applique un ordre : d'abord les renommages notés par la session, au hub et à l'ordre.
fn apply(tab: &TabSession, hub: &SurfaceHub, k: Keep) {
    let moves = match tab.with(|s| s.take_scope_moves()) {
        Ok(moves) => moves,
        Err(e) => {
            tracing::warn!(error = %e.one_line(), "renommages des conversations non lus");
            Vec::new()
        }
    };
    let follow = |mut scope: AgentScope| {
        for ScopeMove::Renamed { from, to } in &moves {
            if scope == *from {
                scope = to.clone();
            }
        }
        scope
    };
    for ScopeMove::Renamed { from, to } in &moves {
        hub.rename_scope(from, to);
    }
    let written = match k {
        Keep::Thread(turn) => match hub.turn_scope(turn).map(|scope| (hub.thread_snapshot(&scope), scope)) {
            Some((Some(thread), scope)) => tab.with(|s| s.keep_thread(scope, thread)),
            Some((None, scope)) => {
                tracing::warn!(?scope, "fil trop lourd pour le budget : enregistrement sauté");
                Ok(())
            }
            None => Ok(()), // tour effacé entre-temps (nouvelle conversation) : rien à écrire
        },
        Keep::Session { scope, id, agent } => tab.with(|s| s.keep_agent_session(follow(scope), id, agent)),
        Keep::Forget(scope, reply) => tab.with(|s| {
            let gone = s.forget_conversation(&follow(scope));
            let _ = reply.send(gone); // l'acteur a pu partir (onglet fermé)
        }),
        Keep::Lookup(scope, reply) => tab.with(|s| {
            let found = s.conversation(&follow(scope)).and_then(|c| Some((c.session.clone()?, c.agent.clone())));
            let _ = reply.send(found); // idem
        }),
        Keep::Show(scope) => {
            hub.resync(follow(scope));
            Ok(())
        }
        Keep::Flush(ack) => {
            let _ = ack.send(()); // l'attente a pu être abandonnée
            Ok(())
        }
    };
    if let Err(e) = written {
        tracing::warn!(error = %e.one_line(), "conversation de l'agent non enregistrée");
    }
}
```

`tab_agent.rs` — `Ctx` gagne `keeper: Keeper` ; `TabAgent` :

```rust
struct TabAgent {
    ctx: Ctx,
    tools: ToolsAccess,
    link: Link,
    /// Sessions ACP vivantes : id → lancement de l'agent.
    live: HashMap<String, u64>,
}
```

```rust
    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Command>) {
        self.ctx.keeper.restored().await;
        let mut next = None;
        loop {
            let cmd = match next.take() { Some(c) => c, None => match rx.recv().await { Some(c) => c, None => break } };
            match cmd {
                Command::Cancel => self.ctx.hub.cancel_asks(),
                Command::NewConversation { scope } => self.new_conversation(scope).await,
                Command::Prompt { scope, text, ui } => next = self.prompt(&mut rx, scope, text, ui).await,
            }
        }
        for (id, generation) in self.live.drain() {
            self.link.forget(generation, SessionId::new(id));
        }
        self.ctx.hub.cancel_asks();
    }

    /// Nouvelle conversation dans `scope` : oubliée par le gardien, sa route et son fil aussi.
    async fn new_conversation(&mut self, scope: AgentScope) {
        if let Some(id) = self.ctx.keeper.forget(scope.clone()).await
            && let Some(generation) = self.live.remove(&id)
        {
            self.link.forget(generation, SessionId::new(id));
        }
        self.ctx.hub.clear_thread(&scope);
        self.ctx.emit(AgentEvent::Conversation { scope });
    }

    async fn prompt(&mut self, rx: &mut mpsc::UnboundedReceiver<Command>, scope: AgentScope, text: String, ui: AgentUi) -> Option<Command> {
        let n = self.ctx.hub.begin_turn(scope.clone());
        self.ctx.emit(AgentEvent::UserText { turn: n, scope: scope.clone(), text: text.clone() });
        let known = self.ctx.keeper.lookup(scope.clone()).await;
        let alive = known.as_ref().and_then(|(id, _)| self.live.get(id).copied());
        if alive.is_none() || alive != self.link.current_generation() {
            self.ctx.emit(AgentEvent::Status { state: AgentState::Starting, scope: Some(scope.clone()) });
        }
        let live = match self.link.connect().await {
            Ok(live) => live,
            Err(e) => {
                self.failed(n, &e);
                return None;
            }
        };
        let opened = match known {
            Some((id, _)) if self.live.get(&id) == Some(&live.generation) => Ok((SessionId::new(id), false)),
            Some((id, agent)) => match self.reopen(&live, &scope, &id, agent.as_deref()).await {
                Ok(Some(id)) => Ok((id, false)),
                Ok(None) => {
                    self.memory_lost(n);
                    self.open(&live, &scope).await.map(|id| (id, true))
                }
                Err(e) => Err(e),
            },
            None => self.open(&live, &scope).await.map(|id| (id, true)),
        };
        let (id, fresh) = match opened {
            Ok(x) => x,
            Err(e) => {
                self.request_failed(n, &live, e).await;
                return None;
            }
        };
        match crate::turn::run(&self.ctx, &live.cx, rx, n, &scope, id, text, ui, fresh).await {
            Ok(pending) => {
                for scope in pending.new_conversations {
                    self.new_conversation(scope).await;
                }
                pending.prompt.map(|(scope, text, ui)| Command::Prompt { scope, text, ui })
            }
            Err(e) => {
                self.request_failed(n, &live, e).await;
                None
            }
        }
    }

    /// Dossier du document et serveur d'outils de l'onglet.
    async fn setup(&self) -> (PathBuf, McpServer) {
        let tab = self.ctx.tab.clone();
        let dir = match tokio::task::spawn_blocking(move || tab.with(|s| s.document_dir())).await {
            Ok(Ok(dir)) => dir,
            Ok(Err(e)) => {
                tracing::warn!(error = %e.one_line(), "dossier du document illu : dossier par défaut");
                None
            }
            Err(e) => {
                tracing::warn!(error = %e, "dossier du document perdu : dossier par défaut");
                None
            }
        };
        let cwd = dir.unwrap_or_else(|| self.link.fallback_cwd().to_path_buf());
        let tools = McpServer::Http(
            McpServerHttp::new("oku-slides", self.tools.url.clone())
                .headers(vec![HttpHeader::new("Authorization", self.tools.authorization.clone())]),
        );
        (cwd, tools)
    }

    /// Nouvelle session ACP pour `scope`.
    async fn open(&mut self, live: &Live, scope: &AgentScope) -> agent_client_protocol::Result<SessionId> {
        let (cwd, tools) = self.setup().await;
        let r = live.cx.send_request(NewSessionRequest::new(cwd).mcp_servers(vec![tools])).block_task().await?;
        self.adopt(live, scope, r.session_id.clone());
        Ok(r.session_id)
    }

    /// Reprend la session `id` ; `None` si l'agent ne le peut pas.
    ///
    /// Autre agent, aucune capacité, ou session inconnue de l'agent : le tour en ouvre une autre.
    async fn reopen(&mut self, live: &Live, scope: &AgentScope, id: &str, agent: Option<&str>) -> agent_client_protocol::Result<Option<SessionId>> {
        if agent != Some(self.link.agent_id().as_str()) || !(live.caps.resume || live.caps.load) {
            return Ok(None);
        }
        let id = SessionId::new(id.to_owned());
        let (cwd, tools) = self.setup().await;
        // La route n'est posée qu'après la réponse : le rejeu de `session/load` est ignoré.
        let asked = if live.caps.resume {
            live.cx.send_request(ResumeSessionRequest::new(id.clone(), cwd).mcp_servers(vec![tools])).block_task().await.map(drop)
        } else {
            live.cx.send_request(LoadSessionRequest::new(id.clone(), cwd).mcp_servers(vec![tools])).block_task().await.map(drop)
        };
        match asked {
            Ok(()) => {
                self.adopt(live, scope, id.clone());
                Ok(Some(id))
            }
            Err(e) if agent_client_protocol::is_incoming_transport_closed(&e) || live.closed.is_cancelled() => Err(e),
            Err(e) => {
                tracing::info!(tab = %self.ctx.tab.tab(), error = %e, "session de l'agent non reprise");
                Ok(None)
            }
        }
    }

    /// La session `id` sert `scope` : routée, vivante, transmise au gardien.
    fn adopt(&mut self, live: &Live, scope: &AgentScope, id: SessionId) {
        self.link.route(live.generation, id.clone(), Route { events: self.ctx.events.clone(), hub: self.ctx.hub.clone() });
        let key = id.0.to_string();
        self.live.insert(key.clone(), live.generation);
        self.ctx.keeper.send(Keep::Session { scope: scope.clone(), id: key, agent: self.link.agent_id() });
    }

    /// L'agent a oublié la conversation : une carte le dit dans le tour.
    fn memory_lost(&self, turn: u32) {
        let id = self.ctx.hub.next_id("note");
        let text = self.ctx.localizer.tr("agent-memory-lost");
        match oku_slides_session::agent::receipt::fallback(&id, &text, &self.ctx.localizer) {
            Some(card) => {
                self.ctx.hub.show_static(turn, card);
            }
            None => tracing::debug!("carte « mémoire perdue » vide : non montrée"),
        }
    }
```

`request_failed(turn, live, e)` : sur connexion finie, **ne rien oublier**
(supprimer l'ancien `self.forget()` de cette branche : la session reste connue
du gardien, ses routes partent avec le lancement, elle sera reprise). `failed`
émet `AgentEvent::idle()`. `AgentHandle::spawn(…, keeper)` ;
`prompt` → `Command::Prompt { scope: AgentScope::Slides, .. }` ; `prompt_in` ;
`new_conversation(scope)`.

`turn.rs` : `run(ctx, cx, rx, turn, scope: &AgentScope, session, text, ui, first)` ;
`Working` → `Status { state: Working, scope: Some(scope.clone()) }` ; fin → `AgentEvent::idle()` ;

```rust
pub(crate) struct Pending {
    /// Portées dont la conversation repart de zéro après le tour.
    pub new_conversations: BTreeSet<AgentScope>,
    /// La dernière consigne reçue pendant le tour, à jouer ensuite.
    pub prompt: Option<(AgentScope, String, AgentUi)>,
}
```

et dans la boucle : `Some(Command::NewConversation { scope }) => { pending.new_conversations.insert(scope); }`,
`Some(Command::Prompt { scope, text, ui }) => pending.prompt = Some((scope, text, ui)),`.

`stack.rs` : champs `scope` (`#[serde(default)]`) sur `Prompt`, `NewConversation`,
`Resync` ; `AgentCommand::tab()` : motifs `Self::NewConversation { tab, .. }`,
`Self::Resync { tab, .. }`. `handle` :

```rust
            AgentCommand::Prompt { tab, scope, text, ui, answer } => {
                let Some(t) = self.open(&tab) else { return Ok(()) };
                if !answer.is_some_and(|id| t.hub.answer(&id, &text)) {
                    t.send(Command::Prompt { scope, text, ui });
                }
            }
            AgentCommand::NewConversation { tab, scope } => {
                if let Some(t) = self.tab(&tab) {
                    t.send(Command::NewConversation { scope });
                }
            }
            // Démarre l'onglet (outils, gardien ; pas l'agent ACP) : un onglet rouvert retrouve ses fils.
            AgentCommand::Resync { tab, scope } => match self.open(&tab) {
                Some(t) => {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Show(scope));
                }
                None => {
                    emit(&self.out_tx, "agent", &tab, &AgentEvent::idle());
                    emit(&self.out_tx, "mcp", &tab, &McpEvent::Surfaces { list: Vec::new() });
                }
            },
```

Supprimer l'ancienne méthode `resync` ; doc de `Stack::start` : « les outils et
le gardien d'un onglet démarrent à sa première commande, `resync` compris ;
l'agent ACP au premier prompt ». `shutdown` vide les gardiens avant d'arrêter
l'agent :

```rust
    pub async fn shutdown(&self) {
        let gone = {
            let mut table = self.table();
            table.shut = true;
            std::mem::take(&mut table.tabs)
        };
        let keepers: Vec<Keeper> = gone.values().map(|e| e.link.keeper.clone()).collect();
        drop(gone);
        // Les fins de tour encore en file sont écrites avant l'arrêt (bornées, comme l'agent).
        let flushed = futures::future::join_all(keepers.iter().map(Keeper::flush));
        if tokio::time::timeout(SHUTDOWN_WITHIN, flushed).await.is_err() {
            tracing::warn!("conversations de l'agent pas toutes écrites à temps");
        }
        self.link.shutdown(SHUTDOWN_WITHIN).await;
    }
```

(si `futures` n'est pas une dépendance du crate, attendre les gardiens l'un
après l'autre dans le même `timeout`.)

`tab_entry.rs` : `TabLink` gagne `pub keeper: Keeper` ; `TabEntry::start` crée
`let keeper = Keeper::spawn(session.clone(), hub.clone());` avant `Agent` ;
`Relay` garde un `keeper` et, sur `TurnOut::Ended { turn, narration }`, appelle
`self.hub.end_turn(turn, &narration)` puis `self.keeper.send(Keep::Thread(turn))` ;
`Agent::run` passe `keeper` à `AgentHandle::spawn` ; la branche « outils
indisponibles » commence par `keeper.restored().await`, puis ouvre le tour avec
`begin_turn(scope)` et `UserText { turn, scope, text }` ; la boucle relaie
`Command::Prompt { scope, text, ui } => agent.prompt_in(scope, text, ui)`,
`Command::NewConversation { scope } => agent.new_conversation(scope)`.

`examples/smoke.rs:48` : `AgentCommand::Prompt { tab: TAB.to_owned(), scope: AgentScope::Slides, text: prompt, ui: AgentUi::default(), answer: None }`.

`lib.rs` : `mod keeper;`, `pub use keeper::{Keep, Keeper};`, et `[`Keeper`]`
dans la table des matières du module.

i18n — `agent.ftl`, juste après `agent-turn-interrupted`, dans les 20 langues
(`fr` et `en` ci-dessous, traduire les 18 autres) :

```
# fr
agent-memory-lost = L'agent ne se souvient plus de cette conversation : il repart de zéro.
# en
agent-memory-lost = The agent no longer remembers this conversation: it starts over.
```

Tests existants à adapter :
- `tests/turns.rs:99,133` : `new_conversation()` → `new_conversation(AgentScope::Slides)`.
- `tests/launch.rs` : lancer `grep -n "crash" crates/oku-slides-agent/tests/launch.rs` ;
  dans chaque test qui prompte **après** un arrêt de l'agent, le tour suivant
  contient désormais une carte `note-N` (`agent-memory-lost` : le faux agent
  n'a pas de `resume`/`load` par défaut) avant `session/new`. Là où un test
  compte les surfaces de ce tour ou compare son JSON, ajouter cette carte à
  l'attendu ; ne rien changer d'autre à ce qu'il vérifie. Noter dans le
  rapport de la tâche les tests touchés et leur nouvel attendu.

- [ ] **Step 4 : lancer, constater le succès**

Run : `cargo nextest run -p oku-slides-agent -p oku-slides-mcp -p oku-slides-session -p oku-slides-i18n && cargo build --workspace --all-targets`
Expected : PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-agent/src/keeper.rs crates/oku-slides-agent/src/tab_agent.rs crates/oku-slides-agent/src/turn.rs crates/oku-slides-agent/src/stack.rs crates/oku-slides-agent/src/tab_entry.rs crates/oku-slides-agent/src/lib.rs crates/oku-slides-agent/examples/smoke.rs
git add crates/oku-slides-agent/tests/scopes.rs crates/oku-slides-agent/tests/stack.rs crates/oku-slides-agent/tests/common/mod.rs crates/oku-slides-agent/tests/turns.rs crates/oku-slides-agent/tests/launch.rs
git add crates/oku-slides-i18n/src/locales/*/agent.ftl
git status --short crates/oku-slides-agent crates/oku-slides-i18n
git diff --cached --name-only
git commit -m "feat(agent) : gardien d'onglet et une session ACP par portée, reprise des sessions connues (t-598)"
```

---

### Task 8 : Interface — portée calculée, fil rappelé, saisie bloquée ailleurs

**Files :**
- Create : `ui/src/agent/scope.ts`, `scope.test.ts`, `ui/src/agent/useAgentScope.ts`, `useAgentScope.render.test.tsx`
- Modify : `ui/src/api.ts:565-632`, `ui/src/App.tsx` (section agent ~l.364-440, Échap ~l.553, ~l.905-932), `ui/src/DictationDraft.tsx`
- Test : `ui/src/DictationDraft.render.test.tsx`
- Modify : `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 fichiers)

**Interfaces :**
- Consumes : JSON des Tasks 1, 5, 7.
- Produces :
  ```ts
  // api.ts
  export type AgentScope = { kind: "slides" } | { kind: "atelier"; pack: string } | { kind: "widget"; pack: string; name: string };
  export type AgentEvent = { kind: "status"; state: AgentState; scope?: AgentScope };
  // scope.ts
  export interface ScopeView { templates: boolean; history: boolean; pack: string | null; widget: string | null }
  export interface AgentStatus { state: AgentState; scope?: AgentScope }
  export function agentScope(v: ScopeView): AgentScope;
  export function scopeKey(s: AgentScope): string;
  export function sameScope(a: AgentScope, b: AgentScope): boolean;
  export function busyElsewhere(status: AgentStatus, scope: AgentScope): AgentScope | null;
  export function scopePlace(s: AgentScope, tr: (key: string) => string): string;
  // useAgentScope.ts
  export interface AgentScopeState { scope: AgentScope; status: AgentStatus; elsewhere: AgentScope | null; notice: string | null;
    onAgentEvent: (e: AgentEvent) => void; onReady: () => void }
  export function useAgentScope(agent: { resync: (s: AgentScope) => Promise<void> }, view: ScopeView | null, clearSurfaces: (e: McpEvent) => void): AgentScopeState;
  ```

- [ ] **Step 1 : tests rouges**

`ui/src/agent/scope.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { agentScope, busyElsewhere, sameScope, scopeKey, scopePlace } from "./scope";

const view = { templates: true, history: false, pack: "p-1", widget: null };
const slides = { kind: "slides" } as const;
const atelier = { kind: "atelier", pack: "p-1" } as const;
const kpi = { kind: "widget", pack: "p-1", name: "kpi" } as const;

describe("agentScope", () => {
  test("agentScope on the deck gives slides", () => expect(agentScope({ ...view, templates: false })).toEqual(slides));
  test("agentScope in the history gives slides", () => expect(agentScope({ ...view, history: true, widget: "kpi" })).toEqual(slides));
  test("agentScope in templates without widget gives the pack atelier", () => expect(agentScope(view)).toEqual(atelier));
  test("agentScope on a widget tab gives that widget of the pack", () => expect(agentScope({ ...view, widget: "kpi" })).toEqual(kpi));
  test("agentScope in templates without pack gives slides", () => expect(agentScope({ ...view, pack: null })).toEqual(slides));
});

describe("scopeKey", () => {
  test("scopeKey of equal scopes is equal", () => expect(scopeKey(kpi)).toBe(scopeKey({ ...kpi })));
  test("scopeKey of another pack differs", () => expect(scopeKey(atelier)).not.toBe(scopeKey({ ...atelier, pack: "p-2" })));
});

describe("sameScope", () => {
  test("sameScope with the same widget is true", () => expect(sameScope(kpi, { ...kpi })).toBe(true));
  test("sameScope with another widget name is false", () => expect(sameScope(kpi, { ...kpi, name: "frise" })).toBe(false));
  test("sameScope of atelier and widget is false", () => expect(sameScope(atelier, kpi)).toBe(false));
});

describe("busyElsewhere", () => {
  test("busyElsewhere when idle is null", () => expect(busyElsewhere({ state: "idle" }, slides)).toBeNull());
  test("busyElsewhere without scope is null", () => expect(busyElsewhere({ state: "starting" }, slides)).toBeNull());
  test("busyElsewhere in the same scope is null", () => expect(busyElsewhere({ state: "working", scope: slides }, slides)).toBeNull());
  test("busyElsewhere in another scope names it", () =>
    expect(busyElsewhere({ state: "working", scope: atelier }, slides)).toEqual(atelier));
});

describe("scopePlace", () => {
  const tr = (key: string) => `[${key}]`;
  test("scopePlace of a widget is its name", () => expect(scopePlace(kpi, tr)).toBe("kpi"));
  test("scopePlace of the atelier is its label", () => expect(scopePlace(atelier, tr)).toBe("[ui-agent-scope-atelier]"));
  test("scopePlace of the slides is their label", () => expect(scopePlace(slides, tr)).toBe("[ui-agent-scope-slides]"));
});
```

`DictationDraft.render.test.tsx` : `mount` gagne une option `notice` passée au
composant (`{ busy = false, insertTo = …, typing = 0, notice = null as string | null } = {}`,
puis `notice` dans `createElement(DictationDraft, { … })`) ; ajouter :

```tsx
  test("DictationDraft with a notice disables send", async () => {
    const { send } = await mount("change le fond", { notice: "L'agent répond dans l'atelier…" });
    expect(send().disabled).toBe(true);
  });

  test("DictationDraft with a notice shows it", async () => {
    const { host } = await mount("change le fond", { notice: "L'agent répond dans l'atelier…" });
    expect(host.querySelector(".draft-notice")?.textContent).toBe("L'agent répond dans l'atelier…");
  });

  test("DictationDraft with a notice ignores ⌘↩", async () => {
    const { area, sent } = await mount("change le fond", { notice: "L'agent répond dans l'atelier…", typing: 1 });
    key(area(), { key: "Enter", metaKey: true });
    expect(sent).toEqual([]);
  });
```

`ui/src/agent/useAgentScope.render.test.tsx` (mise en place comme
`AgentThread.render.test.tsx` : `../test/dom` d'abord, puis React) :

```tsx
// Rappel du fil de l'agent quand la portée change.

import { afterEach, expect, test } from "bun:test";
import "../test/dom";
import type { AgentScope, McpEvent } from "../api";
import type { ScopeView } from "./scope";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useAgentScope } = await import("./useAgentScope");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

/** Monte le hook ; rend ses appels à `resync`, ses vidages et de quoi changer la vue. */
async function mountHook(view: ScopeView | null) {
  const resyncs: AgentScope[] = [];
  const cleared: McpEvent[] = [];
  const agent = { resync: async (s: AgentScope) => void resyncs.push(s) };
  const clear = (e: McpEvent) => void cleared.push(e);
  let state: ReturnType<typeof useAgentScope> | null = null;
  const Probe = ({ v }: { v: ScopeView | null }) => {
    state = useAgentScope(agent, v, clear);
    return null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe, { v: view })));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  const rerender = (v: ScopeView | null) => act(async () => root.render(createElement(Probe, { v })));
  return { resyncs, cleared, rerender, state: () => state! };
}

const deck: ScopeView = { templates: false, history: false, pack: null, widget: null };
const atelier: ScopeView = { templates: true, history: false, pack: "p-1", widget: null };

test("useAgentScope before the subscription is ready does not resync", async () => {
  const h = await mountHook(deck);
  expect(h.resyncs).toEqual([]);
});

test("useAgentScope at the first ready resyncs once the view is known", async () => {
  const h = await mountHook(deck);
  await act(async () => h.state().onReady());
  expect(h.resyncs).toEqual([{ kind: "slides" }]);
});

test("useAgentScope on a scope change clears then resyncs", async () => {
  const h = await mountHook(deck);
  await act(async () => h.state().onReady());
  await h.rerender(atelier);
  expect(h.cleared.at(-1)).toEqual({ kind: "surfaces", list: [] });
  expect(h.resyncs.at(-1)).toEqual({ kind: "atelier", pack: "p-1" });
});

test("useAgentScope at a reconnection resyncs without clearing", async () => {
  const h = await mountHook(deck);
  await act(async () => h.state().onReady());
  const cleared = h.cleared.length;
  await act(async () => h.state().onReady());
  expect(h.resyncs).toHaveLength(2);
  expect(h.cleared).toHaveLength(cleared);
});
```

- [ ] **Step 2 : lancer, constater l'échec**

Run : `cd ui && bun test src/agent/scope.test.ts src/agent/useAgentScope.render.test.tsx src/DictationDraft.render.test.tsx`
Expected : FAIL (modules absents, prop absente).

- [ ] **Step 3 : implémenter**

`ui/src/agent/scope.ts` :

```ts
// La portée de l'agent : les slides, l'atelier d'un pack, ou un widget.

import type { AgentScope, AgentState } from "../api";

/** Ce qui décide de la portée : mode, historique, pack édité, widget actif. */
export interface ScopeView {
  templates: boolean;
  history: boolean;
  pack: string | null;
  widget: string | null;
}

/** L'état de l'agent et la portée de son tour (absente au repos). */
export interface AgentStatus {
  state: AgentState;
  scope?: AgentScope;
}

/** La portée de ce que l'utilisateur regarde. */
export function agentScope(v: ScopeView): AgentScope {
  if (!v.templates || v.history || v.pack === null) return { kind: "slides" };
  return v.widget === null ? { kind: "atelier", pack: v.pack } : { kind: "widget", pack: v.pack, name: v.widget };
}

/** Une clé stable : deux portées égales ont la même. */
export function scopeKey(s: AgentScope): string {
  return s.kind === "slides" ? "slides" : s.kind === "atelier" ? `atelier:${s.pack}` : `widget:${s.pack}:${s.name}`;
}

/** Deux portées désignent-elles la même conversation ? */
export function sameScope(a: AgentScope, b: AgentScope): boolean {
  return scopeKey(a) === scopeKey(b);
}

/** La portée d'un tour en cours ailleurs que dans `scope` ; `null` sinon. */
export function busyElsewhere(status: AgentStatus, scope: AgentScope): AgentScope | null {
  if (status.state === "idle" || !status.scope) return null;
  return sameScope(status.scope, scope) ? null : status.scope;
}

/** Le nom d'une portée dans une phrase : « les slides », « l'atelier », ou le widget. */
export function scopePlace(s: AgentScope, tr: (key: string) => string): string {
  if (s.kind === "widget") return s.name;
  return tr(s.kind === "slides" ? "ui-agent-scope-slides" : "ui-agent-scope-atelier");
}
```

`ui/src/agent/useAgentScope.ts` :

```ts
// La portée de l'agent dans l'interface et le rappel de son fil.

import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import type { AgentEvent, AgentScope, McpEvent } from "../api";
import { t } from "../i18n";
import { type AgentStatus, type ScopeView, agentScope, busyElsewhere, scopeKey, scopePlace } from "./scope";

const IDLE: AgentStatus = { state: "idle" };
const NO_VIEW: ScopeView = { templates: false, history: false, pack: null, widget: null };

/** Ce que l'interface lit de la portée de l'agent. */
export interface AgentScopeState {
  scope: AgentScope;
  status: AgentStatus;
  /** La portée d'un tour en cours ailleurs, sinon `null`. */
  elsewhere: AgentScope | null;
  /** « L'agent répond dans … », sinon `null`. */
  notice: string | null;
  onAgentEvent: (e: AgentEvent) => void;
  /** Abonnement prêt ; stable d'un rendu à l'autre (lu une fois par `subscribeHost`). */
  onReady: () => void;
}

/**
 * Portée, état de l'agent, note « répond ailleurs » et rappel du fil.
 *
 * Le fil est demandé une fois l'abonnement prêt et la vue connue, puis à chaque
 * changement de portée (surfaces vidées d'abord) ; une reconnexion le redemande.
 */
export function useAgentScope(
  agent: { resync: (scope: AgentScope) => Promise<void> },
  view: ScopeView | null,
  clearSurfaces: (e: McpEvent) => void,
): AgentScopeState {
  const scope = agentScope(view ?? NO_VIEW);
  const key = scopeKey(scope);
  const scopeRef = useRef(scope);
  scopeRef.current = scope;
  const [ready, setReady] = useState(false);
  const readyRef = useRef(false);
  const [status, onAgentEvent] = useReducer((_: AgentStatus, e: AgentEvent): AgentStatus => ({ state: e.state, scope: e.scope }), IDLE);
  const loaded = view !== null;
  useEffect(() => {
    if (!ready || !loaded) return;
    clearSurfaces({ kind: "surfaces", list: [] });
    agent.resync(scopeRef.current).catch((err) => console.warn("resync", err));
  }, [agent, key, ready, loaded, clearSurfaces]);
  // Ne lit que des références : `subscribeHost` garde la première version.
  const onReady = useCallback(() => {
    if (!readyRef.current) {
      readyRef.current = true;
      setReady(true);
      return;
    }
    agent.resync(scopeRef.current).catch((err) => console.warn("resync", err));
  }, [agent]);
  const elsewhere = busyElsewhere(status, scope);
  const notice = elsewhere ? t("ui-agent-busy-elsewhere", { place: scopePlace(elsewhere, t) }) : null;
  return { scope, status, elsewhere, notice, onAgentEvent, onReady };
}
```

(`clearSurfaces` est le `dispatch` d'un `useReducer` et `agent` un `useMemo` par
onglet : les dépendances de l'effet ne changent pas d'un rendu à l'autre.)

`api.ts` : `AgentScope`, `AgentEvent` avec `scope?`, commandes `prompt`,
`new_conversation`, `resync` avec `scope: AgentScope`, et

```ts
    prompt: (scope: AgentScope, text: string, ui: AgentUi, answer: string | null) =>
      agentCall({ cmd: "prompt", tab, scope, text, ui, answer }),
    newConversation: (scope: AgentScope) => agentCall({ cmd: "new_conversation", tab, scope }),
    resync: (scope: AgentScope) => agentCall({ cmd: "resync", tab, scope }),
```

`App.tsx`, section agent (remplace le `useReducer` d'état de l'agent ; à placer
**après** `const [surfaces, onSurfaceEvent] = useReducer(surfacesReducer, …)`) :

```tsx
  const scopeView = snap
    ? { templates: !!snap.editing, history: !!snap.history, pack: snap.editing?.set ?? null, widget: widgets.activeName }
    : null;
  const agentScopeState = useAgentScope(agent, scopeView, onSurfaceEvent);
  const { scope, status: agentStatus, elsewhere, notice: elsewhereNotice } = agentScopeState;
  const agentState = agentStatus.state;
```

Dans `subscribeHost` : `onAgent: agentScopeState.onAgentEvent`, et
`onReady: () => { agentScopeState.onReady(); run(api.snapshot()); }` (l'ancien
`agent.resync()` disparaît ; le comptage `countResync` sur `surfaces` reste :
la liste rappelée est comptée, donc pas relue à voix haute).

`promptAgent` :

```tsx
  const promptAgent = (text: string, answer = answerTarget(keysTarget)) => {
    // Pendant un tour ailleurs, seule une réponse à une question part.
    if (elsewhereNotice && answer === null) {
      say(elsewhereNotice, true);
      return Promise.resolve();
    }
    return agent
      .prompt(scope, text, agentUi(current, selected, editField, widgets.activeName), answer)
      .catch((err) => say(String(err), true));
  };
```

`onSurfaceAction` : avant `sendThenPrompt`,
`if (elsewhereNotice && suggestedPrompt(action) !== null) { say(elsewhereNotice, true); return; }`
(`suggestedPrompt` importé de `./agent/surfaces`).
`AgentThread` : `state={elsewhere ? "idle" : agentState}` ;
`onNewConversation={() => agent.newConversation(scope).catch((err) => say(String(err), true))}`.
`DictationDraft` : `notice={answerTarget(keysTarget) === null ? elsewhereNotice : null}`.
`busy` (⌘⇧D, `Dictation`) reste sur `agentState` : ⌘⇧D annule où qu'il soit.
Échap (~l.553) : `escapeAction({ selected: selected !== null, agentBusy: busy && !elsewhere })`
— Échap n'annule que le tour de la portée vue.

`DictationDraft.tsx` : prop `notice?: string | null` ;
`const canSend = said !== "" && !busy && !notice;` ; juste après la zone de
texte : `{notice && <p className="caption draft-notice">{notice}</p>}`
(`caption` est déjà stylée ; aucune règle CSS nouvelle).

i18n — `ui.ftl`, juste après `ui-agent-new-conversation`, dans cet ordre, dans
les 20 langues (`fr`, `en` ci-dessous ; traduire les autres) :

```
# fr
ui-agent-busy-elsewhere = L'agent répond dans { $place }…
ui-agent-scope-slides = les slides
ui-agent-scope-atelier = l'atelier
# en
ui-agent-busy-elsewhere = The agent is answering in { $place }…
ui-agent-scope-slides = the slides
ui-agent-scope-atelier = the workshop
```

- [ ] **Step 4 : lancer, constater le succès**

Run : `cd ui && bun test && bunx tsc --noEmit && wc -l src/App.tsx`
Expected : PASS (dont `i18n-keys`) ; `App.tsx` sous 1000 lignes.

- [ ] **Step 5 : commit**

```bash
git add ui/src/agent/scope.ts ui/src/agent/scope.test.ts ui/src/agent/useAgentScope.ts ui/src/agent/useAgentScope.render.test.tsx ui/src/api.ts ui/src/App.tsx ui/src/DictationDraft.tsx ui/src/DictationDraft.render.test.tsx
git add crates/oku-slides-i18n/src/locales/*/ui.ftl
git diff --cached --name-only
git commit -m "feat(ui) : le fil de l'agent suit la perspective, saisie bloquée pendant un tour ailleurs (t-598)"
```

---

### Task 9 : e2e, documentation, vérification finale

**Files :**
- Create : `ui/e2e/http.cjs`, `ui/e2e/agent-scopes.cjs`
- Modify : `ui/package.json` (script `e2e`), `fixtures/agent/e2e.json` (`by_prompt`)
- Modify : `crates/oku-slides-agent/ARCHITECTURE.md`, `crates/oku-slides-mcp/ARCHITECTURE.md`, `crates/oku-slides-session/ARCHITECTURE.md` (une ligne chacun),
  `docs/adr/0001-espace-de-travail-session-par-onglet.md` (une ligne de conséquence)

- [ ] **Step 1 : e2e rouge**

`ui/e2e/http.cjs` (le `post` copié dans cinq fichiers ; ceux-ci migreront à part) :

```js
// Requêtes JSON vers `oku-slides serve` depuis la page (garde anti-CSRF : JSON déclaré).
const post = (p, endpoint, req) =>
  p.evaluate(
    ([e, r]) => fetch(e, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(r) }).then((x) => x.json()),
    [endpoint, req],
  );
module.exports = { post };
```

`fixtures/agent/e2e.json`, au niveau racine :

```json
"by_prompt": {
  "portée slides": { "chunks": ["Réponse des slides."] },
  "portée atelier": { "chunks": ["Réponse de l'atelier."] }
}
```

`ui/e2e/agent-scopes.cjs` :

```js
// Agent : un fil par perspective. Les slides et l'atelier gardent chacun le leur.
// Contre `oku-slides serve` lancé avec le faux agent (just e2e s'en charge).
// Précondition : t-1 sans tour en cours ni question en attente (agent.cjs répond aux siennes).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

const turn = (p, text) => p.locator('.agent-turn', { hasText: text });
/** Vide la conversation de la perspective affichée (les fils sont gardés d'un e2e à l'autre). */
const newConversation = async (p) => {
  const button = p.locator('.agent-new');
  if (await button.count()) await button.click();
  await p.waitForSelector('.agent-thread.vacant');
};
const openAtelier = async (p) => {
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Atelier' }).click();
};

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  try {
    await p.goto(URL);
    await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
    await p.reload();
    await p.waitForSelector('.stage img');
    await newConversation(p);

    // 1. Slides : un tour.
    await post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text: 'portée slides', ui: {} });
    await turn(p, 'portée slides').waitFor();

    // 2. Atelier : son fil à lui (vidé d'un e2e précédent), puis un tour envoyé par le brouillon.
    await openAtelier(p);
    await newConversation(p);
    assert.equal(await turn(p, 'portée slides').count(), 0);
    await p.keyboard.press('ControlOrMeta+k');
    await p.locator('.draft-text').fill('portée atelier');
    await p.locator('.draft-send').click();
    await turn(p, 'portée atelier').waitFor();
    assert.equal(await p.locator('.agent-turn').count(), 1);

    // 3. Retour aux slides, sans rien vider : seul leur fil.
    await p.getByRole('button', { name: 'Terminer' }).click();
    await turn(p, 'portée slides').waitFor();
    assert.equal(await turn(p, 'portée atelier').count(), 0);
    assert.equal(await p.locator('.agent-turn').count(), 1);

    // 4. Rangement : les deux conversations repartent de zéro pour les e2e suivants.
    await newConversation(p);
    await openAtelier(p);
    await newConversation(p);
    await p.getByRole('button', { name: 'Terminer' }).click();

    assert.deepEqual(errors, []);
    console.log('agent-scopes : ok');
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

`ui/package.json` : ajouter `&& bun e2e/agent-scopes.cjs` juste après `bun e2e/agent.cjs`.

- [ ] **Step 2 : lancer l'e2e**

Run : `OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) just e2e`
Expected : `agent-scopes : ok`, toute la suite verte. En cas d'échec :
diagnostiquer (G13–G15) avant de toucher au code.

- [ ] **Step 3 : documentation**

Une ligne par crate (le détail est dans les docs de module) :
- `oku-slides-agent` : « Une session ACP par portée (`AgentScope`) ; le gardien
  (`keeper.rs`) relit, écrit et retrouve les conversations ; `Resync` démarre l'onglet. »
- `oku-slides-mcp` : « Le hub range chaque tour dans sa portée et n'envoie que
  le fil vu (`hub_scopes.rs`) ; il ne touche pas à la session. »
- `oku-slides-session` : « Conversations de l'agent par portée dans
  `agent/<portée>.json` (`agent/conversation.rs`, `resume_agent.rs`). »
- ADR-0001, section conséquences : « `Resync` (l'interface s'abonne à un
  onglet affiché) réveille la session de cet onglet pour relire ses
  conversations : un onglet n'est plus relu à son premier prompt mais à son
  premier affichage. »

- [ ] **Step 4 : vérification complète**

Run : `just check && just test`
Expected : tout vert. Passe de consolidation (G10) :
`rg -n "const KEPT_TURNS|fn shown|fn visible|fn agent_fingerprint|const post" crates ui/e2e`
(une définition de chacun, plus les cinq anciennes copies de `post` notées en suivi) ;
code mort : `rg -n "conversation: Option<\(u64" crates/oku-slides-agent` vide.

- [ ] **Step 5 : commit**

```bash
git add ui/e2e/http.cjs ui/e2e/agent-scopes.cjs ui/package.json fixtures/agent/e2e.json crates/oku-slides-agent/ARCHITECTURE.md crates/oku-slides-mcp/ARCHITECTURE.md crates/oku-slides-session/ARCHITECTURE.md docs/adr/0001-espace-de-travail-session-par-onglet.md
git diff --cached --name-only
git commit -m "test(e2e) : un fil par perspective ; docs des portées de l'agent (t-598)"
```

---

## Suivis (hors plan, au backlog)

- Contrôle d'`Origin`/`Host` du serveur de développement (`oku-slides-cli/src/serve.rs`).
- Carte « mémoire perdue » : seulement quand une reprise a été tentée ?
- Agent qui enverrait le rejeu de `session/load` après sa réponse.
- Annuler un renommage de widget : ramener sa conversation sous l'ancien nom.
- Migrer les cinq copies du helper `post` des e2e vers `ui/e2e/http.cjs`.
- `oku-slides-session/src/lib.rs` (~971 lignes) : à découper.
