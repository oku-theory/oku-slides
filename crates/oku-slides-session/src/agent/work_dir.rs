//! The agent's working folder: `work/` in the tab's resume folder.
//!
//! Its ACP session runs there, never in the deck's folder or the data folder:
//! nothing next to a deck (`CLAUDE.md`, `.claude/`…) configures the agent. Empty
//! when made, kept across restarts (agents file their sessions by folder),
//! emptied by a new conversation and deleted with the tab's folder. The agent
//! may write there: before each new, resumed or loaded session the names agents
//! read as config ([`AGENT_CONFIG_NAMES`]) are removed, so a turn steered by deck
//! text cannot configure the next session. The session lock only finds the
//! folder ([`Session::agent_work_dir`]); [`AgentWorkDir`] does the disk work after.

use std::path::{Path, PathBuf};

use oku_slides_platform::storage::Storage;
use snafu::{OptionExt, ResultExt};

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

/// A tab's agent working folder, found under the session lock, used outside it.
#[derive(Debug, Clone)]
pub struct AgentWorkDir {
    path: PathBuf,
    storage: Storage,
}

impl AgentWorkDir {
    /// The folder's path; nothing is created.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Creates the folder if missing and removes the agent config names in it.
    ///
    /// Everything else stays: the agents' own session files are kept for a resume.
    ///
    /// # Errors
    ///
    /// `File` if the folder cannot be created or a config entry removed; never another folder.
    pub fn prepare(&self) -> Result<PathBuf> {
        self.storage.create_dir_all(&self.path).context(WriteSnafu { path: &self.path })?;
        for name in AGENT_CONFIG_NAMES {
            let entry = self.path.join(name);
            let removed = match self.storage.is_file(&entry) {
                Ok(true) => self.storage.delete(&entry),
                // A folder, or a link to one: removed without following it.
                Ok(false) => self.storage.delete_dir(&entry),
                Err(e) => Err(e),
            };
            removed.context(WriteSnafu { path: &entry })?;
        }
        Ok(self.path.clone())
    }

    /// Empties the folder for a new conversation: its files belonged to the old one.
    ///
    /// # Errors
    ///
    /// `File` if the folder cannot be deleted or made again.
    pub fn empty(&self) -> Result<()> {
        self.storage
            .delete_dir(&self.path)
            .and_then(|()| self.storage.create_dir_all(&self.path))
            .context(WriteSnafu { path: &self.path })?;
        Ok(())
    }
}

impl Session {
    /// The tab's private working folder for its agent, without touching the disk.
    ///
    /// `<resume folder>/work`; call [`AgentWorkDir::prepare`] once the session lock is released.
    ///
    /// # Errors
    ///
    /// `NoWorkDir` for a session without a resume folder, never another folder instead.
    pub fn agent_work_dir(&self) -> Result<AgentWorkDir> {
        let path = self.resume.as_ref().context(NoWorkDirSnafu)?.dir().join(AGENT_WORK_DIR);
        Ok(AgentWorkDir { path, storage: self.storage.clone() })
    }
}

#[cfg(test)]
#[path = "work_dir_tests.rs"]
mod work_dir_tests;
