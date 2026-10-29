//! Deck-pack tabs: a deck-only pack edited in its own tab, its saves held for its owner.
//!
//! A pack that only one deck embeds (an import's) is edited in a pack tab
//! linked to its owner deck's tab (`TabLink`). Its saves never reach the
//! library: each one fills the tab's outbox (`fill_outbox`, from
//! `adopt_session_version` and `realign_pack`), which becomes due at a save
//! request, a flush or a turn end (`mark_due`). The workspace takes the due
//! send (`take_deck_pack_send`), hands it to the owner and brings its answer
//! back (`acknowledge_deck_pack`, `park_deck_pack`, `unpark_deck_pack`).
//! Until the owner acknowledges it, the tab is unsent (`unsent`).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};
use oku_slides_i18n::Localize;

use crate::pack_tab::PreviewSeed;
use crate::{Mode, Result, Session, SessionError};

/// Sequence numbers of deck-pack sends, unique for the process.
static DECK_PACK_SEQ: AtomicU64 = AtomicU64::new(1);

/// The pack id and parent a deck-pack save was built on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SendBase {
    /// The tab's pack before the save.
    pub pack: Uuid,
    /// The parent its saves carry.
    pub parent: Option<PackVersion>,
}

/// A deck-pack tab's saved pack, waiting for its owner deck.
#[derive(Debug)]
pub(crate) struct DeckPackSend {
    /// Unique per process; the owner ignores one below its last.
    pub seq: u64,
    /// What the save was built on.
    pub base: SendBase,
    /// The saved version's files; its closure resolves on the owner.
    pub files: PackFiles,
}

/// Where an outbox stands with its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutboxState {
    /// Saved, not yet asked to go.
    Idle,
    /// To forward at the next public call.
    Due,
    /// Taken by a forward that has not answered.
    InFlight,
    /// The owner was busy: waits for its pull.
    Parked,
    /// Refused: waits for the next save, flush or turn end.
    Refused,
}

/// The last save of a deck-pack tab not yet acknowledged.
#[derive(Debug, Clone)]
pub(crate) struct Outbox {
    /// What the oldest unacknowledged save was built on.
    base: SendBase,
    /// The latest saved files.
    files: PackFiles,
    /// The send built when it became due.
    send: Option<Arc<DeckPackSend>>,
    /// Where it stands.
    state: OutboxState,
    /// A save landed while in flight: send again after the answer.
    newer: bool,
    /// The owner pulled while in flight: a park goes straight to due.
    repull: bool,
}

impl Outbox {
    /// Makes the outbox due, building its send when there is none or its files changed.
    fn make_due(&mut self) {
        let current = self.send.as_ref().is_some_and(|send| send.files.version() == self.files.version());
        if !current {
            let seq = DECK_PACK_SEQ.fetch_add(1, Ordering::Relaxed);
            // No closure files (decision 8): the owner resolves them from what it holds.
            self.send = Some(Arc::new(DeckPackSend { seq, base: self.base.clone(), files: self.files.clone() }));
        }
        self.state = OutboxState::Due;
    }

    /// True when `seq` is this outbox's send.
    fn sent(&self, seq: u64) -> bool {
        self.send.as_ref().is_some_and(|send| send.seq == seq)
    }
}

/// What a deck-pack tab knows of its owner and its unsent work.
#[derive(Debug, Clone)]
pub(crate) struct TabLink {
    /// The owner deck's tab id.
    pub owner: String,
    /// The unacknowledged save, if any.
    outbox: Option<Outbox>,
}

impl TabLink {
    /// A link to `owner` with nothing unsent.
    #[must_use]
    pub(crate) fn new(owner: &str) -> Self {
        Self { owner: owner.to_owned(), outbox: None }
    }

    /// The pack the tab bar names: an unacknowledged save's base, else `edited`.
    #[must_use]
    pub(crate) fn reported_pack(&self, edited: Uuid) -> Uuid {
        self.outbox.as_ref().map_or(edited, |outbox| outbox.base.pack)
    }
}

impl Session {
    /// True when this session is a deck-pack tab.
    #[must_use]
    pub(crate) fn is_deck_pack_tab(&self) -> bool {
        matches!(&self.mode, Mode::Pack { link: Some(_), .. })
    }

    /// This deck-pack tab's outbox; `None` without a link or an unacknowledged save.
    fn outbox_mut(&mut self) -> Option<&mut Outbox> {
        let Mode::Pack { link: Some(link), .. } = &mut self.mode else { return None };
        link.outbox.as_mut()
    }

    /// Turns this fresh session into a deck-pack tab of `owner` on `pack`.
    ///
    /// The preview is installed from `seed` (never adopting), then pack mode
    /// entered with no lease and the link to `owner`;
    /// `widget` is selected. Resume is turned on by the caller.
    ///
    /// # Errors
    ///
    /// `PackTabNotFresh`, `Sources`, and the `enter_pack` and `select_item` errors.
    ///
    /// # Blocking
    ///
    /// Resolves the preview's widgets and renders the working deck.
    pub(crate) fn init_deck_pack_tab(
        &mut self,
        seed: Box<PreviewSeed>,
        owner: &str,
        pack: Uuid,
        widget: Option<&str>,
    ) -> Result<()> {
        self.install_preview(*seed)?;
        let link = Box::new(TabLink::new(owner));
        self.enter_pack(pack, None, Some(link))?;
        if let Some(w) = widget {
            self.select_item(w)?;
        }
        Ok(())
    }

    /// Takes the due send and marks it in flight.
    pub(crate) fn take_deck_pack_send(&mut self) -> Option<Arc<DeckPackSend>> {
        let outbox = self.outbox_mut().filter(|outbox| outbox.state == OutboxState::Due)?;
        outbox.state = OutboxState::InFlight;
        outbox.send.clone()
    }

    /// The owner's answer to send `seq`: clears the outbox, or notices the refusal.
    ///
    /// An answer to another send changes nothing. `Ok` with a save landed
    /// meanwhile makes it due again, based on what the owner now holds.
    pub(crate) fn acknowledge_deck_pack(&mut self, seq: u64, answer: Result<(), &SessionError>) {
        let current = match &self.mode {
            Mode::Pack { edit, .. } => SendBase { pack: edit.pack, parent: edit.parent.clone() },
            _ => return,
        };
        let Mode::Pack { link: Some(link), .. } = &mut self.mode else { return };
        let Some(outbox) = link.outbox.as_mut().filter(|outbox| outbox.sent(seq)) else { return };
        outbox.repull = false;
        match answer {
            Ok(()) if outbox.newer => {
                outbox.newer = false;
                outbox.base = current;
                outbox.send = None;
                outbox.make_due();
            }
            Ok(()) => link.outbox = None,
            Err(e) => {
                outbox.newer = false;
                outbox.state = OutboxState::Refused;
                tracing::warn!(seq, error = %e.one_line(), "deck-pack save refused by its owner deck");
                self.set_notice(e.localize(&self.localizer));
            }
        }
        self.persist(false);
    }

    /// Parks send `seq`: the owner was busy.
    ///
    /// An owner pull that came while it was in flight makes it due at once.
    pub(crate) fn park_deck_pack(&mut self, seq: u64) {
        let Some(outbox) = self.outbox_mut().filter(|outbox| outbox.sent(seq)) else { return };
        outbox.newer = false;
        if std::mem::take(&mut outbox.repull) {
            outbox.make_due();
        } else {
            outbox.state = OutboxState::Parked;
        }
        self.persist(false);
    }

    /// Releases send `seq` the owner found stale: idle until the next save, flush or turn end.
    ///
    /// A newer send was applied meanwhile; an answer to another send changes nothing.
    pub(crate) fn release_stale_deck_pack(&mut self, seq: u64) {
        let Some(outbox) = self.outbox_mut().filter(|outbox| outbox.sent(seq)) else { return };
        outbox.newer = false;
        outbox.repull = false;
        outbox.state = OutboxState::Idle;
    }

    /// Discards an unacknowledged save the user dropped, so no resume resends it.
    pub(crate) fn discard_unsent(&mut self) {
        let Mode::Pack { link: Some(link), .. } = &mut self.mode else { return };
        if link.outbox.take().is_some() {
            self.persist(false);
        }
    }

    /// Makes a parked send due again (the owner's pull).
    ///
    /// A send in flight remembers the pull: its park goes straight to due.
    pub(crate) fn unpark_deck_pack(&mut self) {
        let Some(outbox) = self.outbox_mut() else { return };
        match outbox.state {
            OutboxState::Parked => outbox.make_due(),
            OutboxState::InFlight => outbox.repull = true,
            OutboxState::Idle | OutboxState::Due | OutboxState::Refused => {}
        }
    }

    /// Saves when dirty, then marks the outbox due.
    ///
    /// # Errors
    ///
    /// The `save_pack` errors.
    ///
    /// # Blocking
    ///
    /// As `save_pack`: renders the working deck after a save.
    pub(crate) fn flush_deck_pack(&mut self) -> Result<()> {
        if !self.is_deck_pack_tab() {
            return Ok(());
        }
        if self.doc.dirty {
            self.save_pack(false)?;
        }
        self.mark_due();
        Ok(())
    }

    /// Makes an idle, refused or parked outbox due, building its send.
    pub(crate) fn mark_due(&mut self) {
        let Some(outbox) = self.outbox_mut() else { return };
        match outbox.state {
            OutboxState::Idle | OutboxState::Refused | OutboxState::Parked => outbox.make_due(),
            OutboxState::Due | OutboxState::InFlight => {}
        }
    }

    /// Takes the session version just adopted, keeping the oldest base.
    ///
    /// A save while a send is in flight leaves it in flight, to send again
    /// after its answer; otherwise the outbox waits, idle, to become due.
    pub(crate) fn fill_outbox(&mut self, base: SendBase, files: &PackFiles) {
        let Mode::Pack { link: Some(link), .. } = &mut self.mode else { return };
        match link.outbox.as_mut() {
            Some(outbox) => {
                outbox.files = files.clone();
                if outbox.state == OutboxState::InFlight {
                    outbox.newer = true;
                } else {
                    outbox.state = OutboxState::Idle;
                }
            }
            None => {
                link.outbox = Some(Outbox {
                    base,
                    files: files.clone(),
                    send: None,
                    state: OutboxState::Idle,
                    newer: false,
                    repull: false,
                });
            }
        }
        self.persist(false);
    }

    /// Rebuilds a resumed tab's unforwarded save, due at its first public call.
    pub(crate) fn resume_outbox(&mut self, base: SendBase, files: &PackFiles) {
        self.fill_outbox(base, files);
        self.mark_due();
    }

    /// Whether this tab holds a save its owner has not acknowledged.
    #[must_use]
    pub(crate) fn unsent(&self) -> bool {
        matches!(&self.mode, Mode::Pack { link: Some(link), .. } if link.outbox.is_some())
    }

    /// For the state file: the base of an unacknowledged save.
    #[must_use]
    pub(crate) fn deck_pack_unforwarded(&self) -> Option<SendBase> {
        let Mode::Pack { link: Some(link), .. } = &self.mode else { return None };
        link.outbox.as_ref().map(|outbox| outbox.base.clone())
    }
}

#[cfg(test)]
#[path = "deck_pack_tab_tests.rs"]
mod deck_pack_tab_tests;
