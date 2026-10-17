// Requête à l'hôte : `invoke` dans Tauri, `POST` au serveur de développement ailleurs.

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Requête à l'hôte : commande Tauri, ou `POST` en développement. */
export async function host<T>(command: string, args: Record<string, unknown>, endpoint: string, body: unknown): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, args);
  }
  const res = await fetch(endpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  // Un refus (403) ou une page d'erreur du proxy peuvent arriver sans corps
  // JSON : le statut reste lisible.
  const text = await res.text();
  let json: { error?: string } = {};
  try {
    json = text ? JSON.parse(text) : {};
  } catch {
    throw new Error(`${res.status} ${res.statusText}`);
  }
  if (!res.ok) throw new Error(json.error ?? `${res.status} ${res.statusText}`);
  return json as unknown as T;
}

/** Le texte d'une erreur. */
export function errorText(e: unknown): string {
  return String(e instanceof Error ? e.message : e);
}

/** Sur macOS : ⌘ et barre de titre intégrée. */
export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);

/** Écoute un événement Tauri ; l'abonnement suit la vie de l'effet (StrictMode compris). */
export function listenTauri(subscribe: () => Promise<() => void>): () => void {
  let stop: (() => void) | undefined;
  let dead = false;
  subscribe()
    .then((un) => (dead ? un() : (stop = un)))
    .catch((e) => console.error("événement de l'hôte", e));
  return () => {
    dead = true;
    stop?.();
  };
}
