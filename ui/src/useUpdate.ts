// App update in a window: the host's state ("Later" included), install, failed-install notice.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, subscribeUpdates, type UpdateApi, type UpdateOffer, type UpdateStatus } from "./api";

/** What a window shows and can do about an update. */
export interface UpdateState {
  /** Offer to show (hidden once put off with "Later", except while installing). */
  offer: UpdateOffer | null;
  installing: boolean;
  /** Download progress, `null` if unknown. */
  percent: number | null;
  /** Checks asked by the app menu, counted. */
  requested: number;
  /** A failed install told after the restart, until dismissed. */
  notice: string | null;
  /** Check; `null` without a host, rejects on a host error. */
  check(): Promise<UpdateStatus | null>;
  install(): void;
  later(): void;
  openRelease(): void;
  dismissNotice(): void;
}

/**
 * One notice request per host: a StrictMode remount reads the same answer. A failed request
 * is forgotten, so the next mount asks again.
 */
const notices = new WeakMap<UpdateApi, Promise<string | null>>();

function noticeOnce(api: UpdateApi): Promise<string | null> {
  let asked = notices.get(api);
  if (!asked) {
    asked = api.takeNotice();
    notices.set(api, asked);
    asked.catch(() => notices.delete(api));
  }
  return asked;
}

/** Update state for a window; no `api`: nothing to show. */
export function useUpdate(api: UpdateApi | null, onError: (text: string) => void): UpdateState {
  const [status, setStatus] = useState<UpdateStatus>({ offer: null, installing: false, dismissed: false });
  const [percent, setPercent] = useState<number | null>(null);
  const [requested, setRequested] = useState(0);
  const [pending, setPending] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  // Always the latest callback, without resubscribing the effect when it changes.
  const report = useRef(onError);
  report.current = onError;
  // Only the latest read asked for counts (replies out of order).
  const seq = useRef(0);

  // A status the host returns, numbered when asked: a read asked later wins.
  const settle = useCallback((request: Promise<UpdateStatus>) => {
    const n = ++seq.current;
    return request.then((s) => {
      if (n === seq.current) setStatus(s);
      return s;
    });
  }, []);

  const reread = useCallback(() => {
    if (!api) return;
    const n = ++seq.current;
    api.status().then(
      (s) => {
        if (n !== seq.current) return;
        setStatus(s);
        // The host finished (or never started): the pending click is over.
        if (!s.installing) setPending(false);
      },
      (e) => console.error("update", e),
    );
  }, [api]);

  useEffect(() => {
    if (!api) return;
    return subscribeUpdates(
      api,
      {
        onChanged: reread,
        onProgress: (p) => setPercent(p.total ? Math.min(100, Math.floor((p.received / p.total) * 100)) : null),
        onCheckRequested: () => setRequested((n) => n + 1),
      },
      reread,
    );
  }, [api, reread]);

  useEffect(() => {
    if (!api) return;
    noticeOnce(api).then(
      (text) => {
        if (text) setNotice(text);
      },
      (e) => console.error("update", e),
    );
  }, [api]);

  const check = useCallback(async () => {
    if (!api) return null;
    return settle(api.check());
  }, [api, settle]);

  const install = useCallback(() => {
    if (!api) return;
    setPending(true);
    // A previous attempt's percent must not stay on screen.
    setPercent(null);
    api.install().catch((e) => {
      setPending(false);
      report.current(errorText(e));
    });
  }, [api]);

  const version = status.offer?.version;
  const later = useCallback(() => {
    if (!api || !version) return;
    settle(api.later(version)).catch((e) => report.current(errorText(e)));
  }, [api, version, settle]);

  const openRelease = useCallback(() => {
    api?.openRelease().catch((e) => report.current(errorText(e)));
  }, [api]);

  const dismissNotice = useCallback(() => setNotice(null), []);

  const installing = status.installing || pending;
  const offer = status.offer && (installing || !status.dismissed) ? status.offer : null;
  return { offer, installing, percent, requested, notice, check, install, later, openRelease, dismissNotice };
}
