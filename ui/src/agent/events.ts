// Évènements de l'hôte : outils (surfaces, changements) et fil de l'agent.
// Tauri : évènements `oku-slides:mcp` / `oku-slides:agent` (charge JSON,
// enveloppe `{tab, event}`) ; développement : SSE sur /api/events.

import { z } from "zod";
import { inTauri } from "../api";
import type { AgentEvent, McpEvent } from "../api";
import { NOTES_MIN, PILE_MAX, PILE_MIN } from "../columns";

/** The longest slide id a present `show` command may carry. */
export const PRESENT_SLIDE_MAX = 64;

const step = (action: "next" | "previous" | "first" | "last" | "exit") => z.object({ action: z.literal(action) });

/** A present event as the host sends it (`McpEvent::Present`). */
const presentEvent = z.object({
  kind: z.literal("present"),
  command: z.discriminatedUnion("action", [
    z.object({ action: z.literal("start"), mode: z.enum(["screen", "window"]) }),
    z.object({ action: z.literal("show"), slide: z.string().min(1).max(PRESENT_SLIDE_MAX) }),
    step("next"),
    step("previous"),
    step("first"),
    step("last"),
    step("exit"),
  ]),
});

/** A focus event (`McpEvent::Focus`): the slide to show, or none to deselect; the agent never selects. */
const focusEvent = z.object({ kind: z.literal("focus"), slide: z.string().min(1).max(PRESENT_SLIDE_MAX).nullable() });

/** The longest interface locale: a shipped code (`zh-Hant`) or `system`. */
const LOCALE_MAX = 16;

/** An interface change (`McpEvent::Interface`): widths within the static bounds of columns.ts. */
const interfaceEvent = z
  .object({
    kind: z.literal("interface"),
    change: z
      .object({
        scheme: z.enum(["light", "dark", "system"]).nullable(),
        locale: z.string().min(1).max(LOCALE_MAX).nullable(),
        guides: z.boolean().nullable(),
        folded: z.object({ slides: z.boolean().nullable(), layouts: z.boolean().nullable() }).strict().nullable(),
        pile: z.number().int().min(PILE_MIN).max(PILE_MAX).nullable(),
        notes: z.number().int().min(NOTES_MIN).nullable(),
        code: z.boolean().nullable(),
      })
      .strict(),
  })
  .strict();

/** A tab id, `t-<n>`, as the host numbers them: at most 18 digits, as `tab_number` reads. */
const TAB_ID = /^t-[0-9]{1,18}$/;
/** A window label, `w-<n>`. */
const WINDOW_ID = /^w-[0-9]{1,18}$/;
/** A close decision id: a UUID in its hyphenated form. */
const DECISION_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** An agent's close (`McpEvent::Close`): its target and the decision the page redeems. */
const closeEvent = z
  .object({
    kind: z.literal("close"),
    target: z.discriminatedUnion("kind", [
      z.object({ kind: z.literal("tab"), tab: z.string().regex(TAB_ID) }).strict(),
      z.object({ kind: z.literal("window"), window: z.string().regex(WINDOW_ID) }).strict(),
    ]),
    decision: z.string().regex(DECISION_ID),
  })
  .strict();

/** The schema of each mcp event kind checked here; the others pass as they are. */
const MCP_SCHEMAS: Partial<Record<McpEvent["kind"], z.ZodType>> = {
  present: presentEvent,
  focus: focusEvent,
  interface: interfaceEvent,
  close: closeEvent,
};

/** False for an mcp event its schema refuses: it is dropped, with a warning naming only its kind. */
function validMcp(e: McpEvent): boolean {
  const schema = MCP_SCHEMAS[e.kind];
  if (!schema || schema.safeParse(e).success) return true;
  console.warn("mcp event dropped: invalid", e.kind);
  return false;
}

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
  const deliver = <T>(cb: (e: T) => void, valid: (e: T) => boolean = () => true) => (e: { payload: string }) => {
    if (closed) return;
    const value = parse<T>(e.payload, tab, h.onError);
    if (value && valid(value)) cb(value);
  };
  load()
    .then(async (listen) => {
      keep(await listen("oku-slides:mcp", deliver(h.onMcp, validMcp)));
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
  const deliver = <T>(cb: (e: T) => void, valid: (e: T) => boolean = () => true) => (e: Event) => {
    const value = parse<T>((e as MessageEvent<string>).data, tab, h.onError);
    if (value && valid(value)) cb(value);
  };
  source.addEventListener("mcp", deliver(h.onMcp, validMcp));
  source.addEventListener("agent", deliver(h.onAgent));
  // Une reconnexion donne `error` puis `open` : `onReady` suit l'erreur signalée.
  source.addEventListener("open", () => h.onReady?.());
  source.addEventListener("error", () => h.onError?.(`event stream error: ${url}`));
  return () => source.close();
}
