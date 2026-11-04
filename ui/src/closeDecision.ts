// The agent's close decision: redeemed once from the host, it answers only a discard question
// about a tab whose live dirty revision still equals the one the user confirmed by voice.
//
// The close itself always runs closeTabFlow or closeWindowFlow (guards, saves, the opener
// offer); without a decision, or for a tab changed since, the flow asks as it would on a click.

import { createContext } from "react";
import { errorText } from "./api";
import type { CloseDecision, CloseTarget, WorkspaceView } from "./api";
import type { Confirm } from "./useConfirm";

/** Whether the agent's decision answers the discard question about tab `id`. */
export type Decide = (id: string) => Promise<boolean>;

/** The deciding side of `decision`: yes only for a confirmed discard of a tab unchanged since. */
export function decider(decision: CloseDecision, freshView: () => Promise<WorkspaceView>): Decide {
  return async (id) => {
    if (!decision.discard) return false;
    const confirmed = decision.revisions.find(([tab]) => tab === id)?.[1] ?? null;
    // A tab confirmed clean that is dirty now: its unsaved work was never mentioned.
    if (confirmed === null) return false;
    const live = (await freshView()).windows.flatMap((w) => w.tabs).find((t) => t.id === id);
    return live?.dirty_revision === confirmed;
  };
}

/** Redeems decision `id` with `take`; undefined (the flow then asks) when the host has none. */
export async function redeem(
  take: (id: string) => Promise<CloseDecision | null>,
  id: string,
  freshView: () => Promise<WorkspaceView>,
): Promise<Decide | undefined> {
  const decision = await take(id).catch((e: unknown) => {
    console.warn("close decision not taken:", errorText(e));
    return null;
  });
  return decision ? decider(decision, freshView) : undefined;
}

/** `confirm`, except that a danger question about tab `id` the decision answers is not asked. */
export function confirmFor(confirm: Confirm, decide: Decide | undefined, id: string): Confirm {
  if (!decide) return confirm;
  const decided = (tab: string) =>
    decide(tab).catch((e: unknown) => {
      console.warn("close decision not read:", errorText(e));
      return false;
    });
  return async (q) => (q.danger && (await decided(id))) || confirm(q);
}

/** Runs an agent's close event in this window: its target and decision id, as received. */
export type AgentClose = (target: CloseTarget, decision: string) => void;

/** The window's runner of agent close events; null where no workspace window holds the panel. */
export const AgentCloseContext = createContext<AgentClose | null>(null);
