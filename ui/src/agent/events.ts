// Évènements de l'hôte : outils (surfaces, changements) et fil de l'agent.
// Tauri : évènements `oku-slides:mcp` / `oku-slides:agent` (charge JSON,
// enveloppe `{tab, event}`) ; développement : SSE sur /api/events.

import { inTauri } from "../api";
import type { AgentEvent, McpEvent } from "../api";

/** Les rappels d'un abonnement aux évènements de l'hôte. */
export interface HostHandlers {
  onMcp: (e: McpEvent) => void;
  onAgent: (e: AgentEvent) => void;
  /** Détail technique d'un échec : charge illisible, flux coupé, écoute impossible. */
  onError?: (message: string) => void;
  /**
   * L'écoute est en place ; en SSE, aussi après chaque reconnexion.
   * L'interface y demande `agent.resync()` et relit l'instantané du deck.
   */
  onReady?: () => void;
}

/** La forme de `listen` de `@tauri-apps/api/event`, réduite à ce qui sert ici. */
export type Listen = (event: string, cb: (e: { payload: string }) => void) => Promise<() => void>;

/** S'abonne aux évènements de l'onglet `tab` ; renvoie de quoi se désabonner. */
export function subscribeHost(tab: string, handlers: HostHandlers): () => void {
  if (inTauri) return subscribeTauri(async () => (await import("@tauri-apps/api/event")).listen, tab, handlers);
  return subscribeSse("/api/events", tab, handlers);
}

/** Lit une enveloppe `{tab, event}` ; `null` (et `onError`) si illisible, ou si `tab` ne correspond pas. */
function parse<T>(raw: string | undefined, tab: string, onError?: (message: string) => void): T | null {
  try {
    const value: unknown = JSON.parse(raw ?? "");
    if (typeof value === "object" && value !== null && typeof (value as { tab?: unknown }).tab === "string") {
      const event = (value as { event?: unknown }).event;
      if (typeof event === "object" && event !== null && typeof (event as { kind?: unknown }).kind === "string") {
        return (value as { tab: string }).tab === tab ? (event as T) : null;
      }
    }
    onError?.(`event without tab/kind: ${String(raw).slice(0, 80)}`);
  } catch (e) {
    onError?.(`unreadable event: ${String(e)}`);
  }
  return null;
}

/** Abonnement Tauri ; `load` fournit `listen` (import paresseux, ou un faux en test). */
export function subscribeTauri(load: () => Promise<Listen>, tab: string, h: HostHandlers): () => void {
  let closed = false;
  const stops: (() => void)[] = [];
  // Chaque écoute est gardée dès qu'elle existe : un échec de la suivante ne la perd pas.
  const keep = (stop: () => void) => (closed ? stop() : stops.push(stop));
  const deliver = <T>(cb: (e: T) => void) => (e: { payload: string }) => {
    if (closed) return;
    const value = parse<T>(e.payload, tab, h.onError);
    if (value) cb(value);
  };
  load()
    .then(async (listen) => {
      keep(await listen("oku-slides:mcp", deliver(h.onMcp)));
      keep(await listen("oku-slides:agent", deliver(h.onAgent)));
      if (!closed) h.onReady?.();
    })
    .catch((e) => h.onError?.(String(e)));
  return () => {
    closed = true;
    stops.splice(0).forEach((stop) => stop());
  };
}

/** Abonnement SSE du serveur de développement. */
export function subscribeSse(url: string, tab: string, h: HostHandlers): () => void {
  const source = new EventSource(url);
  const deliver = <T>(cb: (e: T) => void) => (e: Event) => {
    const value = parse<T>((e as MessageEvent<string>).data, tab, h.onError);
    if (value) cb(value);
  };
  source.addEventListener("mcp", deliver(h.onMcp));
  source.addEventListener("agent", deliver(h.onAgent));
  // Une reconnexion donne `error` puis `open` : `onReady` suit l'erreur signalée.
  source.addEventListener("open", () => h.onReady?.());
  source.addEventListener("error", () => h.onError?.(`event stream error: ${url}`));
  return () => source.close();
}
