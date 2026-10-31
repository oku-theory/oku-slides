// App updates: commands and events of the Tauri host (absent in web development).

import { inTauri, listenTauri } from "./hostCall";

/** Install, or only tell (a package to install by hand). */
export type InstallMode = "install" | "notify_only";

/** A newer version, as the host describes it. */
export interface UpdateOffer {
  version: string;
  mode: InstallMode;
}

/** State kept by the host, the same for every window. */
export interface UpdateStatus {
  offer: UpdateOffer | null;
  installing: boolean;
  /** The user put this offer's version off with "Later". */
  dismissed: boolean;
}

/** Download progress; `total` unknown: `null`. */
export interface UpdateProgress {
  received: number;
  total: number | null;
}

/** A window's reactions to the host's update events. */
export interface UpdateEvents {
  /** The state changed: read it again (`status`), never guess it. */
  onChanged(): void;
  /** Download progress, for the bar. */
  onProgress(p: UpdateProgress): void;
  /** The app menu asks for a check (macOS). */
  onCheckRequested(): void;
}

/** What the UI asks of the host; replaced by a fake in tests. */
export interface UpdateApi {
  status(): Promise<UpdateStatus>;
  check(): Promise<UpdateStatus>;
  install(): Promise<void>;
  openRelease(): Promise<void>;
  /** Puts `version` off ("Later") if it is still the offer; the status that follows. */
  later(version: string): Promise<UpdateStatus>;
  /** The failed-install notice, for the first window that asks; then `null`. */
  takeNotice(): Promise<string | null>;
  /** Subscribes `h`; the promise resolves once every listener is in place. */
  subscribe(h: UpdateEvents): Promise<() => void>;
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

/** A host event sent to every window; resolves once the listener is in place. */
async function on<P>(name: string, cb: (payload: P) => void): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  return listen<P>(name, (e) => cb(e.payload));
}

/** A finite, non-negative number, else `null`: events are not trusted data. */
function count(v: unknown): number | null {
  return typeof v === "number" && Number.isFinite(v) && v >= 0 ? v : null;
}

/** The Tauri host; `null` outside the app (development server, e2e). */
export const tauriUpdates: UpdateApi | null = inTauri
  ? {
      status: () => call<UpdateStatus>("update_status"),
      check: () => call<UpdateStatus>("check_update"),
      install: () => call<void>("install_update"),
      openRelease: () => call<void>("open_release_page"),
      later: (version) => call<UpdateStatus>("later_update", { version }),
      takeNotice: () => call<string | null>("take_update_notice"),
      subscribe: async (h) => {
        const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
        const results = await Promise.allSettled([
          on<unknown>("update-changed", () => h.onChanged()),
          on<{ received?: unknown; total?: unknown } | null>("update-progress", (p) => {
            const received = count(p?.received);
            if (received !== null) h.onProgress({ received, total: count(p?.total) || null });
          }),
          getCurrentWebviewWindow().listen("update-check-requested", () => h.onCheckRequested()),
        ]);
        // A refused listener must not leave the others in place.
        const stops = results.flatMap((r) => (r.status === "fulfilled" ? [r.value] : []));
        const failed = results.find((r) => r.status === "rejected");
        if (failed) {
          stops.forEach((stop) => stop());
          throw failed.reason;
        }
        return () => stops.forEach((stop) => stop());
      },
    }
  : null;

/** A subscription that follows an effect's life (StrictMode included), then `ready` once in place. */
export function subscribeUpdates(api: UpdateApi, h: UpdateEvents, ready: () => void): () => void {
  return listenTauri(async () => {
    const stop = await api.subscribe(h);
    ready();
    return stop;
  });
}
