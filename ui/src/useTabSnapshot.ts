// A tab's snapshot: the last accepted one, `run` to apply a reply, a refetch on each broadcast view.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, type Snapshot, type TabApi } from "./api";
import { Latest } from "./latest";

/** What the pipeline needs from its tab. */
export interface TabSnapshotInput {
  /** Changes on each broadcast view: the snapshot is read again (a notice may have arrived). */
  nonce: number;
  /** A notice from elsewhere, said once. */
  notice?: string | null;
  /** A snapshot was accepted (the tab bar reads again, in development). */
  onChange?: () => void;
  /** Shows a message; `true`: sticky error. */
  say: (message: string, sticky?: boolean) => void;
}

/** The tab's snapshot, a ref current as soon as one is accepted, and `run` to apply a reply. */
export function useTabSnapshot(api: TabApi, { nonce, notice, onChange, say }: TabSnapshotInput) {
  // Latest `onChange` received, without changing `run`'s identity.
  const changed = useRef(onChange);
  changed.current = onChange;
  const [snap, setSnap] = useState<Snapshot | null>(null);
  // A reread (`nonce`) sometimes crosses an edit: the older reply, arriving last,
  // never takes the deck back.
  const latest = useRef(new Latest());
  // The last accepted snapshot, current as soon as it arrives: read after an await,
  // before the render that follows.
  const snapRef = useRef(snap);
  snapRef.current = snap;
  // `null`: nothing was done (dialog abandoned), the state does not change.
  const run = useCallback(async (p: Promise<Snapshot | null>) => {
    const n = latest.current.begin();
    try {
      const s = await p;
      if (!s) return null;
      if (latest.current.accept(n)) {
        snapRef.current = s;
        setSnap(s);
        changed.current?.();
      }
      // The host sends a notice only once: even from an outdated reply, it is shown.
      if (s.notice) say(s.notice, true);
      return s;
    } catch (e) {
      say(errorText(e), true);
      return null;
    }
  }, []);

  useEffect(() => {
    if (notice) say(notice, true);
  }, [notice]);

  // On mount, then on each broadcast view: a notice set outside this tab (a received
  // file that could not open) does not wait for the next edit.
  useEffect(() => {
    run(api.snapshot());
  }, [run, nonce]);

  return { snap, snapRef, run };
}
