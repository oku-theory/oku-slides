// The agent of a tab: its events, surfaces and close decisions, and the commands sent to it.

import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import { host } from "./hostCall";
import { locale } from "./i18n";
import type { PresentCommand, PresentMode } from "./present";

/** Ce que voit l'utilisateur au moment de la consigne (bloc de contexte). */
export interface AgentUi {
  slide: string | null;
  element: string | null;
  field: string | null;
  editing: boolean;
  widget: string | null;
}

/** `AgentUi` tel qu'il part : avec la langue affichée, lue à l'envoi pour suivre un changement à chaud. */
export type AgentUiSent = AgentUi & { locale: string };

/** L'agent au repos, en lancement, ou en plein tour. */
export type AgentState = "idle" | "starting" | "working";

/**
 * Évènements du canal `agent` : état de l'agent, brouillon de l'onglet, changement de la dictée
 * ou de l'agent (`voice`) et disponibilité de l'agent (le fil passe par les surfaces).
 */
export type AgentEvent =
  | { kind: "status"; state: AgentState }
  | { kind: "draft"; text: string | null; origin: string | null; images: PastedImage[] }
  | { kind: "voice" }
  | { kind: "availability"; unusable: string | null }
  /** The tab's turns run on the local agent standing in for the selected one (`reason`), or no longer (`null`). */
  | { kind: "fallback"; reason: string | null };

/** Une image collée dans le brouillon, tenue par l'hôte (`AttachmentRef`). */
export interface PastedImage {
  id: string;
  mime: string;
  width: number;
  height: number;
}

/** Où s'affiche une surface : fiche, fil, tiroir ou fenêtre. */
export type Place = "sheet" | "thread" | "drawer" | "modal" | "popover" | "inspector";

/**
 * Où l'agent pose une fiche (session, `agent::Anchor`) : l'élément `owner` de la slide, ou son
 * champ texte `field` ; `owner` est l'id de la slide pour un champ de la slide. Résolue par la
 * session contre le deck avant d'arriver ici.
 */
export interface Anchor {
  slide: string;
  owner: string;
  field?: string;
}

/** Une surface de l'agent : ses messages A2UI et sa place (session, `agent::Surface`). */
export interface Surface {
  id: string;
  place: Place;
  anchor?: Anchor;
  /** Le tour qui l'a créée (surfaces du fil). */
  turn?: number;
  /** Vivante : dans le dock ; figée : dans son tour. */
  live: boolean;
  /** Une question (`ask_*`, confirmation, permission) : elle attend la réponse de l'utilisateur. */
  question: boolean;
  messages: A2uiMessage[];
}

/** Évènements des outils (crate oku-slides-mcp, `McpEvent`). */
export type McpEvent =
  | { kind: "changed" }
  | { kind: "surface"; surface: Surface }
  | { kind: "close_surface"; id: string }
  | { kind: "focus"; slide: string | null }
  | { kind: "surfaces"; list: Surface[] }
  | { kind: "surface_patch"; id: string; messages: A2uiMessage[] }
  | { kind: "widget_touched"; name: string; removed: boolean }
  | { kind: "present"; command: PresentCommand }
  | { kind: "open_history" }
  | { kind: "interface"; change: InterfaceChange }
  | { kind: "close"; target: CloseTarget; decision: string };

/** What an agent's close covers (`hub::CloseTarget`): one tab, or a window. */
export type CloseTarget = { kind: "tab"; tab: string } | { kind: "window"; window: string };

/**
 * The user's spoken close decision (`hub::CloseDecision`), redeemed once by its window.
 *
 * `revisions`: each covered tab's dirty revision when the user confirmed; null: only while clean.
 */
export interface CloseDecision {
  discard: boolean;
  revisions: [string, number | null][];
}

/** Takes the agent's close decision `id` for window `window`; null: unknown, taken, expired or not its own. */
export const takeCloseDecision = (id: string, window: string) =>
  host<CloseDecision | null>("take_close_decision", { id }, "/api/close-decision", { id }, { "X-Oku-Window": window });

/** Ce que l'agent change dans l'interface (`set_interface`) ; `null` : inchangé. */
export interface InterfaceChange {
  scheme: "light" | "dark" | "system" | null;
  /** Code d'une langue livrée, ou `system`. */
  locale: string | null;
  guides: boolean | null;
  /** Pile sections to fold (`true`) or open (`false`); `null` in a section: unchanged. */
  folded: { slides: boolean | null; layouts: boolean | null } | null;
  /** Widths in px, within the static bounds; the page narrows them to the window. */
  pile: number | null;
  notes: number | null;
  /** A pack item's Code tab (`true`) or Settings tab. */
  code: boolean | null;
}

/** Commandes vers l'agent (crate oku-slides-agent, `AgentCommand`), un onglet par commande. */
type AgentCommand =
  | { cmd: "prompt"; tab: string; text: string; ui: AgentUiSent; answer: string | null; images: string[] }
  | { cmd: "cancel"; tab: string }
  | { cmd: "new_conversation"; tab: string }
  | { cmd: "surface_action"; tab: string; surface: string; name: string; context: Record<string, unknown> }
  | { cmd: "resync"; tab: string }
  | { cmd: "follow"; tab: string }
  | { cmd: "set_draft"; tab: string; text: string | null; origin: string }
  | { cmd: "paste_image"; tab: string; data: string }
  | { cmd: "remove_image"; tab: string; id: string }
  | { cmd: "show_packs"; tab: string }
  | { cmd: "show_themes"; tab: string; slide: string | null }
  | { cmd: "show_settings"; tab: string }
  | { cmd: "availability"; tab: string }
  | { cmd: "context"; tab: string; page: string; ui: AgentUiSent; seq: number; has_focus: boolean }
  | { cmd: "presenting"; tab: string; page: string; mode: PresentMode | null; seq: number };

/** `window`: the sending window's label, which the dev server checks against the tab's holder. */
const agentCall = (req: AgentCommand, window?: string) =>
  host<void>("agent", { req }, "/api/agent", req, window ? { "X-Oku-Window": window } : {});

/** Consigne, arrêt du tour, nouvelle conversation, réponse à une surface, liste des surfaces. */
export function agentApi(tab: string) {
  return {
    /**
     * `answer` : la question à laquelle la consigne répond (`answerTarget`) ; sans elle,
     * ou si elle n'attend plus, une consigne ordinaire.
     */
    prompt: (text: string, ui: AgentUi, answer: string | null, images: string[] = []) =>
      agentCall({ cmd: "prompt", tab, text, ui: { ...ui, locale: locale() }, answer, images }),
    /** Une image collée (octets en base64), ajoutée au brouillon par l'hôte. */
    pasteImage: (data: string) => agentCall({ cmd: "paste_image", tab, data }),
    /** Retire l'image `id` du brouillon. */
    removeImage: (id: string) => agentCall({ cmd: "remove_image", tab, id }),
    cancel: () => agentCall({ cmd: "cancel", tab }),
    newConversation: () => agentCall({ cmd: "new_conversation", tab }),
    surfaceAction: (surface: string, name: string, context: Record<string, unknown>) =>
      agentCall({ cmd: "surface_action", tab, surface, name, context }),
    /** Demande la liste entière des surfaces ouvertes (évènement `surfaces`). */
    resync: () => agentCall({ cmd: "resync", tab }),
    /** Une autre vue de l'onglet s'abonne : état, brouillon, puis le fil. */
    follow: () => agentCall({ cmd: "follow", tab }),
    /** Le brouillon de l'onglet, tenu par l'hôte ; `null` : fermé. `origin` : la vue qui l'écrit. */
    setDraft: (text: string | null, origin: string) => agentCall({ cmd: "set_draft", tab, text, origin }),
    /** Ouvre le panneau des packs du deck (surface `host-packs` du hub de l'onglet). */
    showPacks: () => agentCall({ cmd: "show_packs", tab }),
    /** Opens the theme picker on `slide` (`null`: the first one). */
    showThemes: (slide: string | null) => agentCall({ cmd: "show_themes", tab, slide }),
    /** Demande si l'agent est utilisable (évènement `availability` pour cet onglet). */
    availability: () => agentCall({ cmd: "availability", tab }),
    /** Ouvre la boîte des réglages de la dictée et de l'agent. */
    showSettings: () => agentCall({ cmd: "show_settings", tab }),
    /**
     * What window `window`'s page shows of the tab, for routing utterances: numbered `seq` in
     * page `page`; the host accepts it only from the window holding the tab.
     */
    context: (page: string, ui: AgentUi, seq: number, hasFocus: boolean, window: string) =>
      agentCall({ cmd: "context", tab, page, ui: { ...ui, locale: locale() }, seq, has_focus: hasFocus }, window),
    /** Where window `window`'s page presents the tab (`null`: it stopped), numbered like `context`. */
    presenting: (page: string, mode: PresentMode | null, seq: number, window: string) =>
      agentCall({ cmd: "presenting", tab, page, mode, seq }, window),
  };
}
