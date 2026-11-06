//! The agent's working folder: `work/<uuid>` in the tab's resume folder, one per conversation.
//!
//! Its ACP session runs there, never in the deck's folder or the data folder:
//! nothing next to a deck (`CLAUDE.md`, `.claude/`…) configures the agent. Empty
//! when made, kept across restarts (agents file their sessions by folder, the
//! uuid is saved with the conversation), removed when a new conversation starts
//! and deleted with the tab's folder. The agent
//! may write there: before each new, resumed or loaded session the names agents
//! read as config ([`AGENT_CONFIG_NAMES`]) are removed, in that folder and in `work/`
//! (agents also read their parents'), so a turn steered by deck text cannot configure
//! the next session. When a conversation's uuid is minted, whatever else lies in `work/`
//! (the flat cwd of earlier versions) is removed too. The session lock only finds the
//! folder ([`Session::agent_work_dir`]); [`AgentWorkDir`] does the disk work after.

use std::path::{Path, PathBuf};

use oku_slides_platform::storage::Storage;
use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use crate::agent::SavedConversation;
use crate::error::{NoWorkDirSnafu, WriteSnafu};
use crate::resume_agent::AGENT_WORK_DIR;
use crate::{Result, Session};

/// Names coding agents read as project config from their working folder.
///
/// Claude Code (`.claude/`, `CLAUDE.md`, `CLAUDE.local.md`, `.mcp.json`), Codex
/// (`.codex/`, `AGENTS.md`, `AGENTS.override.md`) and Gemini (`.gemini/`, `GEMINI.md`).
/// The only list: what [`AgentWorkDir::prepare`] removes.
pub const AGENT_CONFIG_NAMES: &[&str] = &[
    ".claude",
    ".codex",
    ".gemini",
    ".mcp.json",
    "AGENTS.md",
    "AGENTS.override.md",
    "CLAUDE.md",
    "CLAUDE.local.md",
    "GEMINI.md",
];

/// A conversation's agent working folder, found under the session lock, used outside it.
#[derive(Debug, Clone)]
pub struct AgentWorkDir {
    path: PathBuf,
    storage: Storage,
    /// The conversation's uuid was just minted: `work/` drops what is not a conversation.
    minted: bool,
}

impl AgentWorkDir {
    /// The folder's path; nothing is created.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Creates the folder if missing and removes the agent config names in it and in `work/`.
    ///
    /// Everything else stays: the agents' own session files are kept for a resume. Right
    /// after the uuid was minted, `work/` also loses every entry that is not a conversation's
    /// folder: an upgraded tab's old flat working folder.
    ///
    /// # Errors
    ///
    /// `File` if the folder cannot be created or an entry removed; never another folder.
    pub fn prepare(&self) -> Result<PathBuf> {
        self.storage.create_dir_all(&self.path).context(WriteSnafu { path: &self.path })?;
        if let Some(works) = self.path.parent() {
            if self.minted {
                self.drop_legacy(works)?;
            }
            self.scrub(works)?;
        }
        self.scrub(&self.path)?;
        Ok(self.path.clone())
    }

    /// Removes the agent config names in `dir`.
    fn scrub(&self, dir: &Path) -> Result<()> {
        for name in AGENT_CONFIG_NAMES {
            let entry = dir.join(name);
            let removed = match self.storage.is_file(&entry) {
                Ok(true) => self.storage.delete(&entry),
                // A folder, or a link to one: removed without following it.
                Ok(false) => self.storage.delete_dir(&entry),
                Err(e) => Err(e),
            };
            removed.context(WriteSnafu { path: &entry })?;
        }
        Ok(())
    }

    /// Removes from `works` every file, and every folder not named by a uuid.
    fn drop_legacy(&self, works: &Path) -> Result<()> {
        for file in self.storage.list_files(works).context(WriteSnafu { path: works })? {
            self.storage.delete(&file).context(WriteSnafu { path: &file })?;
        }
        for dir in self.storage.list_dirs(works).context(WriteSnafu { path: works })? {
            let conversation = dir.file_name().is_some_and(|n| Uuid::parse_str(&n.to_string_lossy()).is_ok());
            if !conversation {
                self.storage.delete_dir(&dir).context(WriteSnafu { path: &dir })?;
            }
        }
        Ok(())
    }

    /// Removes the folder of a forgotten conversation: its files belonged to it.
    ///
    /// The next conversation gets a folder of its own; a missing folder is no error.
    ///
    /// # Errors
    ///
    /// `File` if the folder cannot be deleted.
    pub fn remove(&self) -> Result<()> {
        self.storage.delete_dir(&self.path).context(WriteSnafu { path: &self.path })?;
        Ok(())
    }
}

impl Session {
    /// The current conversation's private working folder, without touching the disk.
    ///
    /// `<resume folder>/work/<uuid>`; the first call of a conversation mints its uuid and saves
    /// it with the conversation. Call [`AgentWorkDir::prepare`] once the session lock is released.
    ///
    /// # Errors
    ///
    /// `NoWorkDir` for a session without a resume folder, never another folder instead.
    pub fn agent_work_dir(&mut self) -> Result<AgentWorkDir> {
        let works = self.resume.as_ref().context(NoWorkDirSnafu)?.dir().join(AGENT_WORK_DIR);
        let conversation = self.conversation.get_or_insert_with(SavedConversation::default);
        let (id, minted) = if let Some(id) = conversation.work {
            (id, false)
        } else {
            let id = Uuid::new_v4();
            conversation.work = Some(id);
            self.persist_conversation();
            (id, true)
        };
        Ok(AgentWorkDir { path: works.join(id.to_string()), storage: self.storage.clone(), minted })
    }

    /// The current conversation's working folder, if one was minted; mints nothing.
    #[must_use]
    pub fn conversation_work_dir(&self) -> Option<AgentWorkDir> {
        let id = self.conversation.as_ref()?.work?;
        let path = self.resume.as_ref()?.dir().join(AGENT_WORK_DIR).join(id.to_string());
        Some(AgentWorkDir { path, storage: self.storage.clone(), minted: false })
    }
}

#[cfg(test)]
#[path = "work_dir_tests.rs"]
mod work_dir_tests;
