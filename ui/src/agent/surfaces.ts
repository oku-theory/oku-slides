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

/** Une question de l'agent (les autres surfaces vivantes du fil sont des réglages). */
export const isQuestion = (s: Surface) => s.question;
/** Le sélecteur de thème de l'hôte : une boîte demandée, annoncée, jamais repliée. */
export const HOST_THEME_PICKER = "host-theme-picker";
/** Le panneau des packs du deck, ouvert par le bouton de la barre d'outils ou l'agent. */
export const HOST_PACKS = "host-packs";
/** La boîte des réglages de la dictée et de l'agent. Ouverte par le ⚙ ou l'agent. */
export const HOST_VOICE = "host-voice";
/** Une boîte de l'hôte demandée (thèmes, packs, réglages) : annoncée, jamais repliée. */
export const isRequestedBox = (s: Surface) => s.id === HOST_THEME_PICKER || s.id === HOST_PACKS || s.id === HOST_VOICE;
/** Une surface de réglages de l'hôte (`host-widget`, `host-data`, `host-theme`) : reconstruite à chaque relecture. */
export const isHostSettings = (s: Surface) => s.id.startsWith("host-") && !isRequestedBox(s);
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
  /** Surfaces vivantes du fil : réglages d'abord, la question en dernier (au plus près du micro). */
  dock: Surface[];
  /** Les tours, du plus ancien au plus récent : la surface du tour puis ses surfaces figées. */
  turns: { turn: number; surfaces: Surface[] }[];
  /** Fiches dont l'élément n'est pas à l'écran, et surfaces figées du fil sans tour. */
  loose: Surface[];
}

/**
 * Répartit les surfaces. `shown` : la slide affichée et ses cibles (`null`
 * pendant un changement de slide) ; une fiche n'y va que si son élément y est.
 * Au dock, les réglages d'abord et la question (`question`) en dernier, au plus près du micro.
 */
export function routeSurfaces(list: Surface[], shown: { slide: string; has: (a: Anchor) => boolean } | null): Routed {
  const sheets = byPlace(list, "sheet");
  const stage = sheets.find((s) => !!shown && !!s.anchor && s.anchor.slide === shown.slide && shown.has(s.anchor));
  const thread = byPlace(list, "thread");
  const dock = thread.filter((s) => s.live).sort((a, b) => Number(isQuestion(a)) - Number(isQuestion(b)));
  const byTurn = new Map<number, Surface[]>();
  for (const s of thread) {
    if (s.live || s.turn === undefined) continue;
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
    turns,
    // Fiches hors de la scène et surfaces figées du fil sans tour : aucune n'est jamais invisible.
    loose: [...sheets.filter((s) => s !== stage), ...thread.filter((s) => !s.live && s.turn === undefined)],
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
