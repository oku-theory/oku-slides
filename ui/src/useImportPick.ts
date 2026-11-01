// A .pptx given by the OS opens its window's import dialog. The view carries the queue of picks.

import { useCallback, useEffect, useRef, useState } from "react";
import type { ImportPick, WorkspaceView } from "./api";
import type { ImportJob } from "./useImportJob";

/** The picks waiting behind the import dialog, and how to dismiss them all. */
export interface WaitingPicks {
  /** How many of this window's picks wait. */
  count: number;
  /** Asks the host to forget them; none is offered afterwards. */
  onDismiss: () => void;
}

/**
 * Opens the import dialog for each pick aimed at window `label`, one after another.
 *
 * The pick with the lowest `id` above the last one taken (the queue head) waits while the dialog
 * is shown. Taken (`pick` returns `true`), `ack` asks the host to forget it, so a reload does not
 * reopen it; views that arrive before its reply still list it, and the `id` mark skips it. While
 * an import runs with its dialog closed, `pick` is called once per queue head: it shows that
 * import's dialog and refuses the file, so the pick stays queued and is offered when the import
 * ends. Closing that dialog again does not reshow it for the same head.
 *
 * Returns the picks still waiting for this window (`null`: none) and a way to dismiss them all:
 * `forget` asks the host to drop them, through the highest id shown, and none is offered again.
 */
export function useImportPick(
  view: WorkspaceView | null,
  label: string | null,
  dialog: Pick<ImportJob, "shown" | "busy"> | null,
  pick: (path: string) => boolean,
  ack: (id: number) => void,
  forget: (through: number) => void,
): WaitingPicks | null {
  const handled = useRef(0);
  const reshown = useRef(0);
  // Picks dismissed by the user, through this id: skipped before the host's reply removes them.
  const [dismissed, setDismissed] = useState(0);
  const latest = useRef({ pick, ack, forget });
  latest.current = { pick, ack, forget };
  // Read during render: `handled` moves only in the effect below, which a new view re-runs;
  // `dismissed` is state, so its move in `onDismiss` re-renders.
  const mark = Math.max(handled.current, dismissed);
  const mine = label === null ? [] : (view?.import_picks ?? []).filter((p) => p.window === label && p.id > mark);
  const next = mine.reduce<ImportPick | undefined>((low, p) => (low === undefined || p.id < low.id ? p : low), undefined);
  const through = mine.reduce((high, p) => Math.max(high, p.id), 0);
  const id = next?.id ?? 0;
  const path = next?.path ?? "";
  const shown = dialog?.shown ?? false;
  const busy = dialog?.busy ?? false;
  useEffect(() => {
    if (id <= handled.current || shown) return;
    if (busy) {
      if (reshown.current === id) return;
      reshown.current = id;
    }
    if (!latest.current.pick(path)) return;
    handled.current = id;
    latest.current.ack(id);
  }, [id, path, busy, shown]);
  const onDismiss = useCallback(() => {
    if (through === 0) return;
    setDismissed((d) => Math.max(d, through));
    latest.current.forget(through);
  }, [through]);
  return mine.length > 0 ? { count: mine.length, onDismiss } : null;
}
