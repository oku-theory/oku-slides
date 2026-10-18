// Mises à jour de l'app : commandes et événements de l'hôte Tauri (absents en développement web).

import { inTauri, listenTauri } from "./hostCall";

/** Installer, ou seulement prévenir (paquet à installer à la main). */
export type InstallMode = "install" | "notify_only";

/** Une version plus récente, telle que l'hôte la décrit. */
export interface UpdateOffer {
  version: string;
  mode: InstallMode;
}

/** État gardé par l'hôte, le même pour toutes les fenêtres. */
export interface UpdateStatus {
  offer: UpdateOffer | null;
  installing: boolean;
}

/** Avancée du téléchargement ; `total` inconnu : `null`. */
export interface UpdateProgress {
  received: number;
  total: number | null;
}

/** Réactions d'une fenêtre aux événements de mise à jour de l'hôte. */
export interface UpdateEvents {
  /** L'état a changé : le relire (`status`), jamais le deviner. */
  onChanged(): void;
  /** Avancée du téléchargement, pour la barre. */
  onProgress(p: UpdateProgress): void;
  /** Le menu de l'app demande une recherche (macOS). */
  onCheckRequested(): void;
}

/** Ce que l'interface demande à l'hôte ; remplacé par un faux dans les tests. */
export interface UpdateApi {
  status(): Promise<UpdateStatus>;
  check(): Promise<UpdateStatus>;
  install(): Promise<void>;
  openRelease(): Promise<void>;
  /** Abonne `h` ; la promesse se résout une fois tous les écouteurs posés. */
  subscribe(h: UpdateEvents): Promise<() => void>;
}

async function call<T>(command: string): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command);
}

/** Un événement de l'hôte diffusé à toutes les fenêtres ; résolu une fois l'écouteur posé. */
async function on<P>(name: string, cb: (payload: P) => void): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  return listen<P>(name, (e) => cb(e.payload));
}

/** Un nombre fini et non négatif, sinon `null` : les événements ne sont pas des données sûres. */
function count(v: unknown): number | null {
  return typeof v === "number" && Number.isFinite(v) && v >= 0 ? v : null;
}

/** L'hôte Tauri ; `null` hors de l'app (serveur de développement, e2e). */
export const tauriUpdates: UpdateApi | null = inTauri
  ? {
      status: () => call<UpdateStatus>("update_status"),
      check: () => call<UpdateStatus>("check_update"),
      install: () => call<void>("install_update"),
      openRelease: () => call<void>("open_release_page"),
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
        // Un écouteur refusé ne doit pas laisser les autres posés.
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

/** Abonnement suivant la vie d'un effet (StrictMode compris), puis `ready` une fois posé. */
export function subscribeUpdates(api: UpdateApi, h: UpdateEvents, ready: () => void): () => void {
  return listenTauri(async () => {
    const stop = await api.subscribe(h);
    ready();
    return stop;
  });
}
