//! `update` and `pack_update`: the app's updates through `AppPort`, a pinned pack's (t-999).
//!
//! `update` checks or puts off through the host's [`AppPort`]; `install` asks the user, then
//! records [`AfterTurn::Install`], which the agent stack runs once the turn has ended, never
//! inside the tool. `pack_update` sends the packs panel's own event for a pinned pack, refused
//! when the panel offers no such update.

use oku_slides_session::agent::{AppRefusal, UpdateAction, UpdateState};
use oku_slides_session::{PackUpdate, packs_event_for};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::IntoError;

use crate::tool_error::{
    AfterTurnTakenSnafu, AppSnafu, NoPackUpdateSnafu, NoTurnSnafu, PacksDeckModeSnafu, ToolError, TurnOverSnafu,
};
use crate::tools::Tools;
use crate::{AfterTurn, ConfirmAsk, ConfirmVerb};

/// What `update` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UpdateChoice {
    /// Check for a newer version now.
    Check,
    /// Put the current offer off ("Later").
    Later,
    /// Install the offer and restart, once this reply ends; the user confirms first.
    Install,
}

/// `update`: the action.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateArgs {
    /// check, later or install.
    pub action: UpdateChoice,
}

/// `pack_update`: the pinned pack and the panel's update.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackUpdateArgs {
    /// The pack: the namespace the deck pins it under (see `deck_outline`), or its id.
    pub pack: String,
    /// `use_mine`: move the deck to my current version; `make_current`: make the deck's version mine.
    pub action: PackUpdate,
}

/// The tool's answer to an update state; `Unavailable` is the host's refusal.
pub(crate) fn state_json(state: UpdateState) -> Result<Value, AppRefusal> {
    match state {
        UpdateState::UpToDate => Ok(json!({ "state": "up_to_date" })),
        UpdateState::Offer { version, installable } => {
            Ok(json!({ "state": "offer", "version": version, "installable": installable }))
        }
        UpdateState::Unavailable => Err(AppRefusal::unavailable()),
    }
}

/// The version an install would bring, from a fresh check; refused without an installable one.
pub(crate) fn installable(state: UpdateState) -> Result<String, AppRefusal> {
    match state {
        UpdateState::Offer { version, installable: true } => Ok(version),
        UpdateState::Offer { installable: false, .. } | UpdateState::UpToDate => Err(AppRefusal::no_offer()),
        UpdateState::Unavailable => Err(AppRefusal::unavailable()),
    }
}

/// What `pack_update` found in the session.
enum PackUpdated {
    /// The update was applied.
    Done,
    /// The tab is not in deck mode.
    NotDeck,
    /// The panel offers no such update on that pack.
    NotOffered,
}

impl Tools {
    /// `update`: check, later, or install after the turn.
    pub(crate) async fn update(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let UpdateArgs { action } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let asked = match action {
            UpdateChoice::Check => UpdateAction::Check,
            UpdateChoice::Later => UpdateAction::Later,
            UpdateChoice::Install => return self.install_update(turn).await,
        };
        match self.app.update(asked).await.and_then(state_json) {
            Ok(v) => self.json_result(&v),
            Err(e) => self.fail_with(&AppSnafu.into_error(e)),
        }
    }

    /// `update {install}`: a fresh check, the user's confirmation, then the install once the turn ends.
    ///
    /// The check forgets a version put off, as the user's own check does.
    async fn install_update(&self, turn: Option<u32>) -> CallToolResult {
        let Some(turn) = turn else { return self.fail_with(&NoTurnSnafu { tool: "update" }.build()) };
        if self.hub.ends_app_after(turn) {
            return self.fail_with(&AfterTurnTakenSnafu.build());
        }
        let version = match self.app.update(UpdateAction::Check).await.and_then(installable) {
            Ok(v) => v,
            Err(e) => return self.fail_with(&AppSnafu.into_error(e)),
        };
        let message =
            self.localizer.current().tr_with("agent-confirm-update-install", &[("version", version.as_str().into())]);
        if !self.hub.confirm(message, ConfirmAsk::Danger(ConfirmVerb::Install)).await {
            return self.denied();
        }
        if let Err(e) = self.record_after_turn(turn, AfterTurn::Install) {
            return self.fail_with(&e);
        }
        self.json_result(&json!({ "install": "after_turn", "version": version }))
    }

    /// Records `action` for once turn `turn` has ended, or why it cannot be.
    ///
    /// # Errors
    ///
    /// `AfterTurnTaken` once the turn ends the app another way; `TurnOver` once it has ended
    /// (the user may confirm after it).
    pub(crate) fn record_after_turn(&self, turn: u32, action: AfterTurn) -> Result<(), ToolError> {
        if self.hub.after_turn(turn, action) {
            return Ok(());
        }
        if self.hub.ends_app_after(turn) {
            return AfterTurnTakenSnafu.fail();
        }
        TurnOverSnafu.fail()
    }

    /// `pack_update`: the packs panel's update of a pinned pack, then `prune`.
    pub(crate) async fn pack_update(&self, args: Value) -> CallToolResult {
        let PackUpdateArgs { pack, action } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let done = self
            .on_packs(move |s| {
                if !s.in_deck_mode() {
                    return Ok(PackUpdated::NotDeck);
                }
                let Some(event) = packs_event_for(&s.deck_packs()?, &pack, action) else {
                    return Ok(PackUpdated::NotOffered);
                };
                s.packs_action(event).map(|()| PackUpdated::Done)
            })
            .await;
        match done {
            Ok(PackUpdated::Done) => self.packs_changed().await,
            Ok(PackUpdated::NotDeck) => self.fail_with(&PacksDeckModeSnafu.build()),
            Ok(PackUpdated::NotOffered) => self.fail_with(&NoPackUpdateSnafu.build()),
            Err(e) => self.fail_with(&e),
        }
    }
}

#[cfg(test)]
#[path = "host_tools_tests.rs"]
mod host_tools_tests;
