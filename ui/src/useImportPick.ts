// A .pptx given by the OS opens its window's import dialog. The view carries the queue of picks.

import { useEffect, useRef } from "react";
import type { ImportPick, WorkspaceView } from "./api";
import type { ImportJob } from "./useImportJob";

/**
 * Opens the import dialog for each pick aimed at window `label`, one after another.
 *
 * The pick with the lowest `id` above the last one taken (the queue head) waits while the dialog
 * is shown. Taken (`pick` returns `true`), `ack` asks the host to forget it, so a reload does not
 * reopen it; views that arrive before its reply still list it, and the `id` mark skips it. While
 * an import runs with its dialog closed, `pick` is called once per queue head: it shows that
 * import's dialog and refuses the file, so the pick stays queued and is offered when the import
 * ends. Closing that dialog again does not reshow it for the same head.
 */
export function useImportPick(
  view: WorkspaceView | null,
  label: string | null,
  dialog: Pick<ImportJob, "shown" | "busy"> | null,
  pick: (path: string) => boolean,
  ack: (id: number) => void,
) {
  const handled = useRef(0);
  const reshown = useRef(0);
  const latest = useRef({ pick, ack });
  latest.current = { pick, ack };
  // Read during render: the mark only moves in the effect below, which a new view re-runs.
  const next =
    label === null
      ? undefined
      : view?.import_picks
          .filter((p) => p.window === label && p.id > handled.current)
          .reduce<ImportPick | undefined>((low, p) => (low === undefined || p.id < low.id ? p : low), undefined);
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
}
