// Canal entre la fenêtre principale et la fenêtre de l'agent : ce qui n'existe que dans la
// fenêtre principale (où écrire, contexte de l'interface). Tout le reste passe par l'hôte.
// Tauri : `emit` livre à toutes les fenêtres, l'émettrice comprise ; `onlyFor` trie.

import { z } from "zod";
import type { AgentUi } from "../api";
import { inTauri, listenTauri } from "../api";
import { REAL_TIMERS, type Timers } from "../debounce";
import { randomId } from "../randomId";

export const AGENT_LABEL = "agent";

export const INSERT_ACK_MS = 3000;
const EVENT = "agent-link";

/** Ce que la fenêtre principale dit de l'onglet affiché. */
export interface LinkContext {
  /** Une édition de texte est en cours : la dictée va au curseur. */
  editing: boolean;
  /** Ce que voit l'utilisateur, joint aux consignes. */
  ui: AgentUi;
  /** La fiche posée sur la scène, sinon null. */
  staged: string | null;
  /** Touches rapides coupées (édition, pointage, widget actif). */
  keysOff: boolean;
  /** Empreinte de l'apparence du deck (`appearanceLook`) : changée, la fenêtre de l'agent relit l'onglet. */
  look: string;
}

/** Raccourci brut relayé : la fenêtre de l'agent l'interprète avec son propre état. */
export type LinkShortcut = "auto" | "draft" | "type";

export type LinkMessage =
  | { to: "agent"; kind: "context"; tab: string; context: LinkContext }
  | { to: "main"; kind: "hello"; tab: string }
  | { to: "main"; kind: "insert"; tab: string; id: string; text: string }
  | { to: "main"; kind: "insert-cancel"; tab: string; id: string }
  | { to: "agent"; kind: "insert-ack"; tab: string; id: string; ok: boolean }
  | { to: "agent"; kind: "shortcut"; tab: string; key: LinkShortcut };

/** Un transport : diffuse à toutes les pages, celle qui envoie comprise. */
export interface LinkTransport {
  send(m: LinkMessage): void;
  /** Écoute ; renvoie de quoi arrêter. */
  listen(cb: (m: LinkMessage) => void): () => void;
  /** Résolue une fois l'écoute de l'hôte en place (Tauri : `listen` est asynchrone). */
  ready: Promise<void>;
  /** Arrête l'écoute de l'hôte (ferme le `BroadcastChannel`). */
  close(): void;
}

/** Plus long texte accepté dans un message (`insert`), en caractères : la limite du brouillon. */
export const MAX_LINK_TEXT = 256 * 1024;

// Bornes : un identifiant (onglet, envoi, élément) reste court ; un libellé, une ligne.
const Id = z.string().max(64);
const Label = z.string().max(256);
const Ui = z.object({ slide: Id.nullable(), element: Id.nullable(), field: Label.nullable(), editing: z.boolean(), widget: Label.nullable() });
const Context = z.object({ editing: z.boolean(), ui: Ui, staged: Id.nullable(), keysOff: z.boolean(), look: Id });
const Message = z.discriminatedUnion("kind", [
  z.object({ to: z.literal("agent"), kind: z.literal("context"), tab: Id, context: Context }),
  z.object({ to: z.literal("main"), kind: z.literal("hello"), tab: Id }),
  z.object({ to: z.literal("main"), kind: z.literal("insert"), tab: Id, id: Id, text: z.string().max(MAX_LINK_TEXT) }),
  z.object({ to: z.literal("main"), kind: z.literal("insert-cancel"), tab: Id, id: Id }),
  z.object({ to: z.literal("agent"), kind: z.literal("insert-ack"), tab: Id, id: Id, ok: z.boolean() }),
  z.object({ to: z.literal("agent"), kind: z.literal("shortcut"), tab: Id, key: z.enum(["auto", "draft", "type"]) }),
]);

/** Le message, s'il a la forme d'un `LinkMessage` (schéma zod) ; sinon null. */
export function parseLink(raw: unknown): LinkMessage | null {
  const r = Message.safeParse(raw);
  if (r.success) return r.data as LinkMessage;
  // Les chemins fautifs seulement : la charge peut contenir du texte dicté.
  console.debug("agent-link: message écarté", r.error.issues.map((i) => i.path.join(".")));
  return null;
}

/** Les écouteurs d'une page, servis par une seule écoute de l'hôte. */
function fanOut() {
  const listeners = new Set<(m: LinkMessage) => void>();
  return {
    deliver: (raw: unknown) => {
      const m = parseLink(raw);
      if (m) [...listeners].forEach((cb) => cb(m));
    },
    listen: (cb: (m: LinkMessage) => void) => {
      listeners.add(cb);
      return () => void listeners.delete(cb);
    },
  };
}

/** Évènement Tauri « agent-link », émis à toutes les fenêtres (l'émettrice comprise). */
export function tauriTransport(): LinkTransport {
  const out = fanOut();
  let resolveReady: () => void = () => {};
  const ready = new Promise<void>((r) => (resolveReady = r));
  const stopListening = listenTauri(async () => {
    const { listen } = await import("@tauri-apps/api/event");
    const stop = await listen(EVENT, (e) => out.deliver(e.payload));
    resolveReady();
    return stop;
  });
  return {
    ready,
    listen: out.listen,
    close: stopListening,
    send: (m) => void import("@tauri-apps/api/event").then(({ emit }) => emit(EVENT, m)).catch((e) => console.warn(EVENT, String(e))),
  };
}

/** `BroadcastChannel` « oku-slides-agent-link » (hors Tauri, e2e) ; livre aussi à la page elle-même. */
export function channelTransport(): LinkTransport {
  const out = fanOut();
  const channel = typeof BroadcastChannel === "undefined" ? null : new BroadcastChannel("oku-slides-agent-link");
  if (channel) channel.onmessage = (e: MessageEvent) => out.deliver(e.data);
  return {
    ready: Promise.resolve(),
    listen: out.listen,
    close: () => channel?.close(),
    // Un BroadcastChannel ne se livre pas à lui-même : la page reçoit aussi, comme avec Tauri.
    send: (m) => {
      channel?.postMessage(m);
      out.deliver(m);
    },
  };
}

let shared: LinkTransport | null = null;
/** Le transport de cette page, créé une fois. */
export function linkTransport(): LinkTransport {
  shared ??= inTauri ? tauriTransport() : channelTransport();
  return shared;
}

/** Les messages pour `to` et `tab` seulement. */
export function onlyFor(to: "agent" | "main", tab: string, cb: (m: LinkMessage) => void) {
  return (m: LinkMessage) => {
    if (m.to === to && m.tab === tab) cb(m);
  };
}

/** Envoie `insert` et attend l'accusé ; refusé ou sans réponse : false (sans réponse : `insert-cancel`). */
export function insertWithAck(t: LinkTransport, tab: string, text: string, ms = INSERT_ACK_MS, timers: Timers = REAL_TIMERS): Promise<boolean> {
  // L'écoute de l'hôte est en place depuis le `hello` de la fenêtre (attendu sur `ready`) :
  // un `insert` ne part qu'ensuite, sur un geste de l'utilisateur.
  const id = randomId("i");
  return new Promise((resolve) => {
    let done = false;
    let handle: unknown = null;
    let stop: () => void = () => {};
    const finish = (ok: boolean) => {
      if (done) return;
      done = true;
      if (handle !== null) timers.clear(handle);
      stop();
      resolve(ok);
    };
    stop = t.listen(onlyFor("agent", tab, (m) => {
      if (m.kind === "insert-ack" && m.id === id) finish(m.ok);
    }));
    handle = timers.set(() => {
      t.send({ to: "main", kind: "insert-cancel", tab, id });
      finish(false);
    }, ms);
    t.send({ to: "main", kind: "insert", tab, id, text });
  });
}
