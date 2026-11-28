// The active knowledge tab of a window: KnowledgeTab over its owner deck tab's session.
//
// The view owns the tab's toast. Whether the deck browses its history comes from the owner's
// knowledge revision in the workspace view, never from a snapshot read: a snapshot hands out the
// session's one-shot notice, which belongs to the deck tab. `run` applies the knowledge dialog's
// approvals and says a notice their reply carries.

import { useCallback, useMemo } from "react";
import { errorText, tabApi } from "../api";
import type { Run } from "../GuidesSection";
import { useToast } from "../useToast";
import { KnowledgeTab } from "./KnowledgeTab";
import type { KnowledgeRevision } from "./types";

interface Props {
  /** This knowledge tab: its sub-tabs are kept under it across remounts. */
  tab: string;
  /** The owner deck tab: every read goes to its session. */
  owner: string;
  /** The owner's knowledge revision, from the view; `null` when unknown. */
  revision: KnowledgeRevision | null;
}

/** The knowledge tab `tab` of deck tab `owner`, with its toast. */
export function KnowledgeView({ tab, owner, revision }: Props) {
  const api = useMemo(() => tabApi(owner), [owner]);
  const { say, node } = useToast();
  const run = useCallback<Run>(async (p) => {
    try {
      const s = await p;
      if (s.notice) say(s.notice, true);
      return s;
    } catch (e) {
      say(errorText(e), true);
      return null;
    }
  }, [say]);
  return (
    <div className="knowledge-view">
      <KnowledgeTab tab={tab} api={api} revision={revision} deckInHistory={revision?.in_history ?? false} say={say} run={run} />
      {node}
    </div>
  );
}
