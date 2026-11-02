// Surfaces ouvertes par l'agent, dans l'ordre d'arrivée.

import type { AgentState, Anchor, McpEvent, Place, Surface } from "../api";
import type { QuickAction } from "./quick";

/**
 * Applique un évènement des outils aux surfaces.
 * Une surface remplacée garde sa place ; `surfaces` (liste de l'hôte) remplace tout, une surface
 * inchangée gardant son objet.
 */
export function surfacesReducer(list: Surface[], e: McpEvent): Surface[] {
  switch (e.kind) {
    case "surface": {
      const i = list.findIndex((s) => s.id === e.surface.id);
      return i < 0 ? [...list, e.surface] : list.map((s, j) => (j === i ? e.surface : s));
    }
    case "close_surface":
      return list.filter((s) => s.id !== e.id);
    case "surfaces": {
      // Une surface inchangée garde son objet : sa vue (`SurfaceView`) n'est pas reconstruite,
      // et rien n'est relu (resynchronisation après une reconnexion).
      const before = new Map(list.map((s) => [s.id, s]));
      return e.list.map((s) => {
        const old = before.get(s.id);
        return old && JSON.stringify(old) === JSON.stringify(s) ? old : s;
      });
    }
    case "surface_patch":
      return list.map((s) => (s.id === e.id ? { ...s, messages: [...s.messages, ...e.messages] } : s));
    default:
      return list;
  }
}

/** Les surfaces d'une place, dans l'ordre d'arrivée. */
export const byPlace = (list: Surface[], place: Place) => list.filter((s) => s.place === place);

/** An agent question: the only live surfaces of the dock. */
export const isQuestion = (s: Surface) => s.question;
/** The host's theme picker, a popover next to the tools tray's Themes button. */
export const HOST_THEME_PICKER = "host-theme-picker";

/** The `data-popover-anchor` of a turn chip: its offer opens next to it. */
export const chipAnchor = (id: string) => `${id}@chip`;
/** The deck's packs panel, a popover next to the Packs button. */
export const HOST_PACKS = "host-packs";
/** The dictation and agent settings box, a popover next to the dictation gear. */
export const HOST_VOICE = "host-voice";
/** Le reçu d'un tour (outil `report`, ou reçu de repli) : identifiant `report-N` du hub. */
export const isReceipt = (s: Surface) => s.id.startsWith("report-");

/**
 * Un tour de l'agent est en cours et n'attend pas l'utilisateur : ⌘⇧D et Échap l'arrêtent.
 * `answer` : la question qui recevrait la consigne (`answerTarget`) ; avec elle, ⌘⇧D y répond.
 * Une question invisible (fiche hors de la slide affichée) n'attend donc pas : ⌘⇧D arrête le tour.
 */
export const agentBusy = (state: AgentState, answer: string | null) => state !== "idle" && answer === null;

/** Où s'affichent les surfaces ouvertes ; aucune n'est jamais invisible. */
export interface Routed {
  /** Fiche posée sous son élément, sur la scène. */
  stage?: Surface;
  /** Tiroir des variantes (le plus récent : l'hôte n'en garde qu'un). */
  drawer?: Surface;
  /** Fenêtre de confirmation (la plus ancienne : une question en attente n'est jamais remplacée sous le curseur). */
  modal?: Surface;
  /** Live agent questions, in arrival order, near the mic. */
  dock: Surface[];
  /** Live turnless popovers: boxes the user opened (button, spoken shortcut). */
  popovers: Surface[];
  /** Live popover or inspector surfaces an agent turn opened: a chip in that turn. */
  offers: { turn: number; surface: Surface }[];
  /** The turns, oldest first: the turn's surface, then its frozen ones and its live thread settings. */
  turns: { turn: number; surfaces: Surface[] }[];
  /** Off-stage sheets, and turnless thread surfaces other than questions. */
  loose: Surface[];
}

/**
 * Répartit les surfaces. `shown` : la slide affichée et ses cibles (`null`
 * pendant un changement de slide) ; une fiche n'y va que si son élément y est.
 * The dock holds live questions only; live thread settings (a pack, theme or help tab's
 * `show_params`) stay in their turn.
 */
export function routeSurfaces(list: Surface[], shown: { slide: string; has: (a: Anchor) => boolean } | null): Routed {
  const sheets = byPlace(list, "sheet");
  const stage = sheets.find((s) => !!shown && !!s.anchor && s.anchor.slide === shown.slide && shown.has(s.anchor));
  const thread = byPlace(list, "thread");
  const dock = thread.filter((s) => s.live && isQuestion(s));
  // Every other thread surface, live settings included, shows in its turn (or in loose).
  const inThread = thread.filter((s) => !(s.live && isQuestion(s)));
  // Popover and inspector surfaces: open (turnless), offered in their turn, or frozen like the thread's.
  const boxes = list.filter((s) => s.place === "popover" || s.place === "inspector");
  const offers = boxes.flatMap((s) => (s.live && s.turn !== undefined ? [{ turn: s.turn, surface: s }] : []));
  const popovers = boxes.filter((s) => s.live && s.turn === undefined && s.place === "popover");
  const byTurn = new Map<number, Surface[]>(offers.map((o) => [o.turn, []]));
  for (const s of [...inThread, ...boxes.filter((b) => !b.live)]) {
    if (s.turn === undefined) continue;
    const group = byTurn.get(s.turn);
    if (group) group.push(s);
    else byTurn.set(s.turn, [s]);
  }
  const turns = [...byTurn.entries()]
    .sort(([a], [b]) => a - b)
    .map(([turn, surfaces]) => ({
      turn,
      surfaces: [...surfaces].sort((a, b) => Number(b.id === `turn-${turn}`) - Number(a.id === `turn-${turn}`)),
    }));
  return {
    stage,
    drawer: byPlace(list, "drawer").at(-1),
    modal: byPlace(list, "modal")[0],
    dock,
    popovers,
    offers,
    turns,
    // Off-stage sheets, turnless thread surfaces and turnless inspector ones: none is ever invisible.
    loose: [
      ...sheets.filter((s) => s !== stage),
      ...inThread.filter((s) => s.turn === undefined),
      ...boxes.filter((s) => s.turn === undefined && !(s.live && s.place === "popover")),
    ],
  };
}

/** Le routage d'une vue sans scène : seule la fiche `staged` reste hors du fil. */
export function routeStaged(list: Surface[], staged: string | null): Routed {
  const routed = routeSurfaces(list, null);
  const stage = staged === null ? undefined : routed.loose.find((s) => s.id === staged);
  return stage ? { ...routed, stage, loose: routed.loose.filter((s) => s !== stage) } : routed;
}

/** La consigne d'une suite cliquée (`suggest {prompt}`), à envoyer à l'agent ; sinon `null`. */
export function suggestedPrompt(a: QuickAction): string | null {
  const prompt = a.name === "suggest" ? a.context.prompt : undefined;
  return typeof prompt === "string" && prompt.trim() ? prompt : null;
}

/**
 * Envoie l'action au hub, puis la consigne d'une suite à l'agent, seulement si le hub l'a acceptée
 * (une suite d'un tour passé est refusée). Une suite est une nouvelle consigne : elle ne répond à
 * aucune question en attente (`answer` nul).
 */
export async function sendThenPrompt(
  action: QuickAction,
  send: () => Promise<boolean>,
  prompt: (text: string, answer: null) => unknown,
): Promise<void> {
  const text = suggestedPrompt(action);
  const accepted = await send();
  if (accepted && text) prompt(text, null);
}
