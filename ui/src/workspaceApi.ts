// L'espace de travail de l'hôte : fenêtres, onglets, import d'un .pptx, glisser d'un onglet.

import type { Element, Messages, SchemeSetting } from "./api";
import { host } from "./hostCall";

/** Un onglet, tel que la barre le montre. */
export interface TabInfo {
  id: string;
  title: string;
  dirty: boolean;
  /** L'onglet est en mode pack (l'atelier). */
  editing_pack: boolean;
  path: string | null;
  pristine: boolean;
  /** Identité du deck : une autre copie du même deck la partage. */
  deck_uid: string | null;
}

/** Une fenêtre : ses onglets et l'onglet actif. */
export interface WindowView {
  label: string;
  active: string;
  tabs: TabInfo[];
}

/** Un .pptx reçu de l'OS, pour le dialogue d'import de sa fenêtre. Il l'ouvre une fois par `id`. */
export interface ImportPick {
  window: string;
  path: string;
  /** Numéro croissant : un choix plus récent a un numéro plus grand. */
  id: number;
}

/** Les fenêtres et leurs onglets ; `focused` : la dernière fenêtre active. */
export interface WorkspaceView {
  /** Numéro croissant de la vue : une vue plus petite est plus ancienne. */
  generation: number;
  focused: string;
  /** L'agent est dans sa propre fenêtre. */
  agent_detached: boolean;
  /** Le dernier .pptx reçu de l'OS (argument, seconde instance, macOS). */
  import_pick: ImportPick | null;
  windows: WindowView[];
}

/**
 * La cible d'un import PowerPoint : retrofit sur des packs de layouts (ids, dans l'ordre ;
 * vide : `essentiel`) et un thème, ou hi-fi.
 */
export type ImportTarget = { mode: "retrofit"; layouts: string[]; theme: string | null } | { mode: "hifi" };

type WorkspaceRequest =
  | { cmd: "view" }
  | { cmd: "messages" }
  | { cmd: "set_locale"; locale: string | null }
  | { cmd: "scheme" }
  | { cmd: "set_scheme"; scheme: "light" | "dark" | null }
  | { cmd: "new_tab"; window: string }
  | { cmd: "open_tab"; window: string; path: string }
  | ({ cmd: "import_tab"; window: string; path: string; tune?: boolean } & ImportTarget)
  | { cmd: "open_help"; window: string }
  | { cmd: "same_deck"; path: string }
  | { cmd: "carry_assets"; from: string; to: string; element: Element }
  | { cmd: "activate"; tab: string }
  | { cmd: "close"; tab: string; discard: boolean }
  | { cmd: "reorder"; tab: string; index: number }
  | { cmd: "move_tab"; tab: string; window: string | null; index: number | null }
  | { cmd: "new_window" }
  | { cmd: "close_window"; window: string; discard: string[] }
  | { cmd: "detach_agent" }
  | { cmd: "attach_agent" }
  | { cmd: "import_picked"; id: number };

const ws = <T>(req: WorkspaceRequest) => host<T>("workspace", { req }, "/api/workspace", req);

/** L'espace de travail : fenêtres et onglets. */
export const workspace = {
  view: () => ws<WorkspaceView>({ cmd: "view" }),
  messages: () => ws<Messages>({ cmd: "messages" }),
  /** Choisit la langue de l'interface (`null` : celle du système) ; ses messages. */
  setLocale: (locale: string | null) => ws<Messages>({ cmd: "set_locale", locale }),
  /** Schéma de l'interface choisi (`null` : celui du système). */
  scheme: () => ws<SchemeSetting>({ cmd: "scheme" }),
  /** Choisit le schéma de l'interface (`null` : celui du système). */
  setScheme: (scheme: "light" | "dark" | null) => ws<SchemeSetting>({ cmd: "set_scheme", scheme }),
  newTab: (window: string) => ws<WorkspaceView>({ cmd: "new_tab", window }),
  openTab: (window: string, path: string) => ws<WorkspaceView>({ cmd: "open_tab", window, path }),
  /** Importe un `.pptx` dans un nouvel onglet ; `tune` : l'agent le peaufine ensuite. Attend la fin de l'import. */
  importTab: (window: string, path: string, target: ImportTarget, tune: boolean) =>
    ws<WorkspaceView>({ cmd: "import_tab", window, path, tune, ...target }),
  /** L'aide de l'app, en lecture seule : son onglet s'il est ouvert, sinon un nouveau. */
  openHelp: (window: string) => ws<WorkspaceView>({ cmd: "open_help", window }),
  /** L'onglet qui tient déjà le deck de `path` sous un autre fichier. Ne change rien. */
  sameDeck: (path: string) => ws<{ tab: string | null; title: string | null }>({ cmd: "same_deck", path }),
  /** Avant un collage venu de `from` : copie ses assets vers `to`. */
  carryAssets: (from: string, to: string, element: Element) =>
    ws<{ carried: number }>({ cmd: "carry_assets", from, to, element }),
  activate: (tab: string) => ws<WorkspaceView>({ cmd: "activate", tab }),
  /** `discard` : l'utilisateur a choisi de perdre le travail non enregistré. */
  close: (tab: string, discard = false) => ws<WorkspaceView>({ cmd: "close", tab, discard }),
  reorder: (tab: string, index: number) => ws<WorkspaceView>({ cmd: "reorder", tab, index }),
  moveTab: (tab: string, window: string | null, index: number | null = null) =>
    ws<WorkspaceView>({ cmd: "move_tab", tab, window, index }),
  newWindow: () => ws<WorkspaceView>({ cmd: "new_window" }),
  closeWindow: (window: string, discard: string[]) => ws<WorkspaceView>({ cmd: "close_window", window, discard }),
  /** L'agent passe dans sa propre fenêtre. */
  detachAgent: () => ws<WorkspaceView>({ cmd: "detach_agent" }),
  /** L'agent revient dans la colonne de droite. */
  attachAgent: () => ws<WorkspaceView>({ cmd: "attach_agent" }),
  /** Le dialogue d'import du choix `id` est ouvert : l'hôte l'oublie (un choix plus récent reste). */
  importPicked: (id: number) => ws<WorkspaceView>({ cmd: "import_picked", id }),
};

/**
 * Glisser un onglet hors de sa fenêtre (Tauri) : l'hôte lit le curseur lui-même.
 *
 * Jamais `screenX/screenY` : WKWebView ne les donne pas dans le repère des
 * fenêtres (écran au-dessus du principal). La fenêtre visée reçoit
 * `tab-drag` (fantôme, place de dépôt) puis `tab-drop`.
 */
export const tabDrag = {
  over: async (tab: string) => (await import("@tauri-apps/api/core")).invoke<void>("drag_tab_over", { tab }),
  leave: async () => (await import("@tauri-apps/api/core")).invoke<void>("drag_tab_leave"),
  drop: async (tab: string) => (await import("@tauri-apps/api/core")).invoke<WorkspaceView>("drop_tab", { tab }),
};

/** Onglet d'une autre fenêtre sous le pointeur, en coordonnées de cette page. */
export interface TabAt {
  tab: string;
  x: number;
  y: number;
}
