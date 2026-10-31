//! Each session's cumulative cost, turned into each turn's share, and the turn's record.
//!
//! ACP sends the session's cost so far in `usage_update`; a turn's share is the growth since the
//! previous turn. [`tells`] decides whether a session-state update counts (not replayed, from a
//! routed session). [`record_turn`] builds the [`UsageRecord`] when a reply arrives and files it
//! in the tab's deck through `TabSession::record_usage`.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use agent_client_protocol::schema::v1::{Cost, SessionId, Usage};
use oku_slides_core::{UsageCost, UsageRecord, UsageTab, UsageTokens};
use oku_slides_session::DocumentKind;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::link::Link;
use crate::tab_agent::Ctx;

/// What one session told about its cost.
#[derive(Debug, Default)]
struct Seen {
    /// The cumulative cost at the end of the previous turn that saw one.
    base: Option<UsageCost>,
    /// The latest cumulative cost, if one arrived since the previous turn ended.
    fresh: Option<UsageCost>,
    /// Reopened by `session/load` or `session/resume`: its first cost covers earlier turns.
    resumed: bool,
}

/// The cumulative cost each live session told, by launch and session.
#[derive(Debug, Default)]
pub(crate) struct UsageBoard(Mutex<HashMap<(u64, SessionId), Seen>>);

impl UsageBoard {
    /// Notes a `usage_update` of a routed session.
    ///
    /// No cost, or an implausible one, changes nothing.
    pub(crate) fn told(&self, generation: u64, id: &SessionId, cost: Option<&Cost>) {
        let Some(cost) = cost else { return };
        let cost = UsageCost { amount: cost.amount, currency: cost.currency.clone() };
        if !cost.is_valid() {
            tracing::debug!(session = ?id, "agent cost not finite, out of 0..=1e9 or without ISO code: ignored");
            return;
        }
        let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        map.entry((generation, id.clone())).or_default().fresh = Some(cost);
    }

    /// Session `id` was reopened: its first cost sets the base and charges no turn.
    pub(crate) fn resumed(&self, generation: u64, id: &SessionId) {
        let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        map.entry((generation, id.clone())).or_default().resumed = true;
    }

    /// The ending turn's cost.
    ///
    /// `None` when no cost arrived during it, or it is a resumed base.
    pub(crate) fn take_turn_cost(&self, generation: u64, id: &SessionId) -> Option<UsageCost> {
        let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let seen = map.get_mut(&(generation, id.clone()))?;
        let now = seen.fresh.take()?;
        let share = (seen.base.is_some() || !seen.resumed).then(|| turn_cost(seen.base.as_ref(), &now));
        seen.base = Some(now);
        share
    }

    /// Drops session `id` of launch `generation` (new conversation, tab closed).
    pub(crate) fn forget(&self, generation: u64, id: &SessionId) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).remove(&(generation, id.clone()));
    }

    /// Drops the sessions of an agent process that ended.
    pub(crate) fn forget_generation(&self, generation: u64) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).retain(|(g, _), _| *g != generation);
    }
}

/// Whether a session-state update counts: from a routed session, not replayed.
///
/// `replaying`: the route's replay flag, `None` without a route. A late replay of
/// `session/load` tells past state (t-612).
pub(crate) fn tells(replaying: Option<bool>, session: &SessionId, what: &'static str) -> bool {
    match replaying {
        Some(false) => true,
        Some(true) => {
            tracing::debug!(?session, what, "replayed session state: ignored");
            false
        }
        None => {
            tracing::debug!(?session, what, "state of an unknown session: ignored");
            false
        }
    }
}

/// A turn's share of a cumulative cost.
///
/// The growth since `base`, or `now` whole after a reset.
/// A smaller total or another currency means the agent started counting again.
pub(crate) fn turn_cost(base: Option<&UsageCost>, now: &UsageCost) -> UsageCost {
    match base {
        Some(b) if b.currency == now.currency && now.amount >= b.amount => {
            UsageCost { amount: now.amount - b.amount, currency: now.currency.clone() }
        }
        _ => now.clone(),
    }
}

/// The record's token counts from ACP's end-of-turn usage.
pub(crate) fn tokens_of(u: &Usage) -> UsageTokens {
    UsageTokens {
        total: u.total_tokens,
        input: u.input_tokens,
        output: u.output_tokens,
        thought: u.thought_tokens,
        cache_read: u.cached_read_tokens,
        cache_write: u.cached_write_tokens,
    }
}

/// The record's tab for a tab of `kind`; a help tab records nothing.
pub(crate) fn usage_tab(kind: DocumentKind) -> Option<UsageTab> {
    match kind {
        DocumentKind::Deck => Some(UsageTab::Deck),
        DocumentKind::Pack => Some(UsageTab::Pack),
        DocumentKind::Theme => Some(UsageTab::Theme),
        DocumentKind::Help => None,
    }
}

/// Files the ended turn's usage in the tab's deck; it never fails the turn.
///
/// The turn's cost is taken even when nothing is recorded, so the next turn's share starts
/// here. Nothing is recorded for a help tab, nor by a `Link` without preset (tests).
pub(crate) async fn record_turn(ctx: &Ctx, link: &Link, generation: u64, id: &SessionId, usage: Option<&Usage>) {
    let cost = link.usage().take_turn_cost(generation, id);
    let Some(tab) = usage_tab(ctx.kind) else { return };
    let Some(agent) = link.preset() else {
        tracing::debug!(tab = %ctx.tab.tab(), "link without preset: usage not recorded");
        return;
    };
    let record = UsageRecord {
        model: link.models().session(generation, id).map(|o| o.current),
        tokens: usage.map(tokens_of),
        cost,
        ..UsageRecord::new(Uuid::new_v4(), OffsetDateTime::now_utc(), tab, agent)
    };
    // The agent's values are untrusted: cut to the ledger's checks, never written unreadable.
    let Some(record) = record.sanitized() else {
        tracing::warn!(tab = %ctx.tab.tab(), "usage record without agent id: not recorded");
        return;
    };
    let session = ctx.tab.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || session.record_usage(record)).await {
        tracing::warn!(tab = %ctx.tab.tab(), error = %e, "usage not recorded: blocking task lost");
    }
}

#[cfg(test)]
#[path = "usage_board_tests.rs"]
mod usage_board_tests;
