// Accès à l'hôte : requêtes par onglet, espace de travail (fenêtres et
// onglets), dictée et dialogues de fichier. `invoke` dans Tauri, `POST` au
// serveur de développement ailleurs. Les types décrivent le JSON de l'hôte.

import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import type { Cell, ZoneView } from "./grid";
import { t } from "./i18n";

export type SlideKind = string;

export interface SlideSummary {
  id: string;
  kind: SlideKind;
  title: string | null;
  /** Nom de la slide quand elle sert de modèle. */
  name?: string | null;
  page: number;
  hash: string;
}

export interface Appearance {
  scheme: "light" | "dark" | string;
  /** Couleurs des slides (bg, ink, accent…), en hex. */
  slide: Record<string, string>;
  /** Couleurs de l'interface (desk, sheet, piece, ink…) ; absentes = dérivées. */
  app: Record<string, string>;
  /** radius-sm, radius-md, radius-lg (px), elevation. */
  metrics: Record<string, number>;
  font_display: string[];
  font_body: string[];
  /** Palette des graphiques du thème (jetons chart.palette), en hex. */
  chart_palette: string[];
}

export interface Snapshot {
  path: string | null;
  name: string;
  dirty: boolean;
  theme: string | null;
  themes: string[];
  title: string | null;
  title_source: string | null;
  slides: SlideSummary[];
  can_undo: boolean;
  undo_label: string | null;
  can_redo: boolean;
  redo_label: string | null;
  warnings: string[];
  notice: string | null;
  /** Le dernier enregistrement n'a rien écrit : le fichier est devenu un autre deck (vrai une fois). */
  disk_replaced: boolean;
  appearance: Appearance;
  /** Ensemble de modèles du deck. */
  templates: string;
  /** Édition d'un ensemble de modèles en cours. */
  editing: TemplateMode | null;
  /** Historique ouvert : la version montrée (le reste du snapshot la décrit). */
  history: HistoryView | null;
  ui: UiState;
}

/** Une version du deck (un commit), de la plus ancienne à la plus récente. */
export interface Version {
  /** Nom de la session qui l'a faite ; `null` : anonyme. */
  author: string | null;
  /** Secondes Unix. */
  timestamp: number;
  label: string | null;
  /** Première version d'une session. */
  session_start: boolean;
  merge: boolean;
}

/** Slides du deck courant comparées à la version montrée (identifiants). */
export interface SlideChanges {
  added_since: string[];
  removed_since: string[];
  changed_since: string[];
}

/** Le cran montré : `index` de 0 à `count - 1` (la plus récente). */
export interface HistoryView {
  index: number;
  count: number;
  version: Version;
  changes: SlideChanges;
}

/** Slide courante et grille, gardées par l'hôte d'un lancement à l'autre. */
export interface UiState {
  slide: string | null;
  grid: boolean;
}

export interface TemplateMode {
  set: string;
  name: string;
  /** Ensemble intégré : l'enregistrer en crée une copie. */
  builtin: boolean;
}

export interface SetInfo {
  id: string;
  name: string;
  description: string;
  builtin: boolean;
  /** L'ensemble d'un deck qui n'en choisit pas. */
  default: boolean;
  count: number;
}

export interface TemplatesView {
  set: SetInfo;
  sets: SetInfo[];
  templates: TemplateView[];
}

export interface Frame {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Run {
  t: string;
  bold?: boolean;
  italic?: boolean;
  color?: string;
  /** Cible du lien (`http`, `https` ou `mailto`). */
  link?: string;
  /** Pastille : numéro (dès 1) de la couleur de fond dans la palette du thème. */
  highlight?: number;
}

export interface Paragraph {
  runs: Run[];
  list?: "bullet" | "number";
}

/** Paramètres d'une slide ou d'un élément : texte, slots d'éléments, valeurs, selon le widget. */
export type Params = Record<string, unknown>;

export interface Element {
  id: string;
  kind: string;
  frame?: Frame;
  /** Accroche à la grille de la slide (prioritaire sur `frame`). */
  cell?: Cell;
  style?: Record<string, unknown>;
  params?: Params;
}

export interface Slide {
  id: string;
  kind: SlideKind;
  params?: Params;
  /** Nom et description d'une slide modèle. */
  name?: string;
  description?: string;
  notes?: string;
}

/** Typographie du rendu Typst (pt), pour caler l'éditeur sur le texte. */
export interface TextStyle {
  size: number;
  leading: number;
  tracking: number;
  font: string[];
  weight: number | string;
  fill: string | null;
  align: "left" | "center" | "right" | null;
  case: "upper" | "lower" | null;
  bg: string | null;
}

/** Cible éditable de la vue (case de sélection à l'écran) ; nom distinct de `Target` (adressage des ops). */
export interface EditTarget {
  id: string;
  target: "element" | "field";
  kind: string;
  /** Premier paramètre riche du widget (élément) ; absent : pas éditable en place. */
  param?: string;
  free: boolean;
  text: string | null;
  x: number;
  y: number;
  w: number;
  h: number;
  /** Zone disponible pour le texte : début et largeur (contient la boîte). */
  ax: number;
  aw: number;
  style: TextStyle | null;
}

export interface SlideView {
  slide: Slide;
  page: number;
  hash: string;
  width: number;
  height: number;
  targets: EditTarget[];
  /** Zone des éléments libres (pt, page) et sa grille résolue. */
  zone: ZoneView | null;
}

/** Adressage d'une slide ou d'un élément par les ops. */
export interface Target {
  slide: string;
  element?: string;
}

/** Adressage d'un slot (paramètre qui contient des éléments) par les ops. */
export interface SlotRef {
  owner: Target;
  param: string;
}

export interface TemplateView {
  id: string;
  name: string;
  description: string;
  slide: Slide;
  svg: string;
}

// --- Dictée ---------------------------------------------------------------

export type SttModel = "turbo" | "large" | "base";
export type SttState = "idle" | "downloading" | "recording" | "transcribing";

export interface SttConfig {
  model: SttModel;
  language: string | null;
  /** Arrêter l'écoute seule après un silence de `silence` secondes. */
  auto_stop: boolean;
  silence: number;
}

export interface SttStatus {
  state: SttState;
  config: SttConfig;
  models: { id: SttModel; label: string; size: number; installed: boolean }[];
  download: { model: SttModel; received: number; total: number } | null;
  level: number;
  /** Voix détectée en ce moment. */
  speaking: boolean;
  /** Phrases déjà transcrites (définitives) et phrase en cours (aperçu). */
  committed: string;
  partial: string;
  detected: string | null;
  /** Secondes avant l'arrêt automatique, pendant un silence. */
  stop_in: number | null;
  text: string | null;
  seq: number;
  seconds: number;
  error: string | null;
}

type SttRequest =
  | { cmd: "status" }
  | ({ cmd: "configure" } & SttConfig)
  | { cmd: "start"; prompt: string | null }
  | { cmd: "stop" }
  | { cmd: "cancel" }
  | { cmd: "download"; model: SttModel };

const sttCall = (req: SttRequest) => host<SttStatus>("stt", { req }, "/api/stt", req);

export const stt = {
  status: () => sttCall({ cmd: "status" }),
  configure: (config: SttConfig) => sttCall({ cmd: "configure", ...config }),
  start: (prompt: string | null) => sttCall({ cmd: "start", prompt }),
  stop: () => sttCall({ cmd: "stop" }),
  cancel: () => sttCall({ cmd: "cancel" }),
  download: (model: SttModel) => sttCall({ cmd: "download", model }),
};

export type Op = { op: string } & Record<string, unknown>;

type Request =
  | { cmd: "snapshot" }
  | { cmd: "save"; path: string | null; overwrite: boolean }
  | { cmd: "merge"; path: string }
  | { cmd: "move"; path: string }
  | { cmd: "save_copy"; path: string }
  | { cmd: "slide"; id: string }
  | { cmd: "svg"; id: string }
  | { cmd: "apply"; label: string; ops: Op[] }
  | { cmd: "undo" }
  | { cmd: "redo" }
  | { cmd: "templates"; set?: string | null }
  | { cmd: "template_sets" }
  | { cmd: "edit_templates"; set: string }
  | { cmd: "save_templates"; as_new: boolean }
  | { cmd: "close_templates" }
  | { cmd: "delete_template_set"; set: string }
  | { cmd: "chart_kind"; label: string; slide: string; element: string; kind: string }
  | { cmd: "panel"; slide: string }
  | { cmd: "ui"; slide: string | null; grid: boolean }
  | { cmd: "history_open" }
  | { cmd: "history_seek"; index: number }
  | { cmd: "history_restore" }
  | { cmd: "history_close" }
  | { cmd: "history_versions" };

/** Langue de l'utilisateur et ressources Fluent de l'interface (anglais d'abord). */
export interface Messages {
  locale: string;
  resources: string[];
  /** Langues livrées, l'anglais d'abord. */
  available: string[];
  /** Langue choisie ; `null` : celle du système. */
  chosen: string | null;
}

// --- Agent -----------------------------------------------------------------

/** Ce que voit l'utilisateur au moment de la consigne (bloc de contexte). */
export interface AgentUi {
  slide: string | null;
  element: string | null;
  field: string | null;
  editing: boolean;
}

/** L'agent au repos, en lancement, ou en plein tour. */
export type AgentState = "idle" | "starting" | "working";

/** Valeurs ACP connues ; une valeur future reste une chaîne acceptée. */
type Open<T extends string> = T | (string & {});

/** Statut d'une étape du plan (ACP `PlanEntryStatus`). */
export type PlanStatus = Open<"pending" | "in_progress" | "completed">;

/** Statut d'un appel d'outil (ACP `ToolCallStatus`). */
export type ToolStatus = Open<"pending" | "in_progress" | "completed" | "failed">;

/** Raison de fin d'un tour (ACP `StopReason`). */
export type StopReason = Open<"end_turn" | "max_tokens" | "max_turn_requests" | "refusal" | "cancelled">;

/** Une étape du plan de l'agent. */
export interface PlanItem {
  content: string;
  status: PlanStatus;
}

/** Évènements du fil (crate oku-slides-agent, `AgentEvent`). */
export type AgentEvent =
  | { kind: "status"; state: AgentState }
  | { kind: "user_text"; text: string }
  | { kind: "chunk"; text: string }
  | { kind: "tool_call"; id: string; title: string; status: ToolStatus }
  | { kind: "tool_update"; id: string; title: string | null; status: ToolStatus | null }
  | { kind: "plan"; entries: PlanItem[] }
  | { kind: "turn_ended"; stop: StopReason }
  | { kind: "error"; message: string }
  | { kind: "conversation" }
  /** Le fil gardé de l'onglet, rejoué à l'interface (ré)abonnée. */
  | { kind: "thread"; entries: AgentEvent[] };

/** Où s'affiche une surface : fiche, fil, tiroir, post-it ou fenêtre. */
export type Place = "sheet" | "thread" | "drawer" | "postit" | "modal" | "suggestions";

/** Une surface de l'agent : ses messages A2UI et sa place (session, `agent::Surface`). */
export interface Surface {
  id: string;
  place: Place;
  anchor?: { slide: string; element: string };
  messages: A2uiMessage[];
}

/** Évènements des outils (crate oku-slides-mcp, `McpEvent`). */
export type McpEvent =
  | { kind: "changed" }
  | { kind: "surface"; surface: Surface }
  | { kind: "close_surface"; id: string }
  | { kind: "focus"; slide: string; element: string | null }
  | { kind: "surfaces"; list: Surface[] };

/** Commandes vers l'agent (crate oku-slides-agent, `AgentCommand`), un onglet par commande. */
type AgentCommand =
  | { cmd: "prompt"; tab: string; text: string; ui: AgentUi }
  | { cmd: "cancel"; tab: string }
  | { cmd: "new_conversation"; tab: string }
  | { cmd: "surface_action"; tab: string; surface: string; name: string; context: Record<string, unknown> }
  | { cmd: "resync"; tab: string };

const agentCall = (req: AgentCommand) => host<void>("agent", { req }, "/api/agent", req);

/** Consigne, arrêt du tour, nouvelle conversation, réponse à une surface, liste des surfaces. */
export function agentApi(tab: string) {
  return {
    prompt: (text: string, ui: AgentUi) => agentCall({ cmd: "prompt", tab, text, ui }),
    cancel: () => agentCall({ cmd: "cancel", tab }),
    newConversation: () => agentCall({ cmd: "new_conversation", tab }),
    surfaceAction: (surface: string, name: string, context: Record<string, unknown>) =>
      agentCall({ cmd: "surface_action", tab, surface, name, context }),
    /** Demande la liste entière des surfaces ouvertes (évènement `surfaces`). */
    resync: () => agentCall({ cmd: "resync", tab }),
  };
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Requête à l'hôte : commande Tauri, ou `POST` en développement. */
async function host<T>(command: string, args: Record<string, unknown>, endpoint: string, body: unknown): Promise<T> {
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

/** Les requêtes d'un onglet : chacune ne touche que sa session. */
export function tabApi(tab: string) {
  const call = <T>(req: Request) => host<T>("request", { tab, req }, "/api", { tab, req });
  return {
    snapshot: () => call<Snapshot>({ cmd: "snapshot" }),
    chartKind: (label: string, slide: string, element: string, kind: string) =>
      call<Snapshot>({ cmd: "chart_kind", label, slide, element, kind }),
    save: (path: string | null, overwrite = false) => call<Snapshot>({ cmd: "save", path, overwrite }),
    /** Fusionne un autre PDF du même deck ; l'avis de fusion arrive par `notice`. */
    merge: (path: string) => call<Snapshot>({ cmd: "merge", path }),
    /** Déplace le fichier du deck (l'ancien est supprimé). */
    move: (path: string) => call<Snapshot>({ cmd: "move", path }),
    /** Écrit une copie ailleurs ; le deck garde son fichier. */
    saveCopy: (path: string) => call<Snapshot>({ cmd: "save_copy", path }),
    slide: (id: string) => call<SlideView>({ cmd: "slide", id }),
    svg: (id: string) => call<string>({ cmd: "svg", id }),
    /** Surface A2UI du panneau de droite pour une slide (vide hors des modèles). */
    panel: (slide: string) => call<A2uiMessage[]>({ cmd: "panel", slide }),
    apply: (label: string, ops: Op[]) => call<Snapshot>({ cmd: "apply", label, ops }),
    undo: () => call<Snapshot>({ cmd: "undo" }),
    redo: () => call<Snapshot>({ cmd: "redo" }),
    templates: (set?: string | null) => call<TemplatesView>({ cmd: "templates", set: set ?? null }),
    templateSets: () => call<SetInfo[]>({ cmd: "template_sets" }),
    editTemplates: (set: string) => call<Snapshot>({ cmd: "edit_templates", set }),
    saveTemplates: (asNew = false) => call<Snapshot>({ cmd: "save_templates", as_new: asNew }),
    closeTemplates: () => call<Snapshot>({ cmd: "close_templates" }),
    deleteTemplateSet: (set: string) => call<SetInfo[]>({ cmd: "delete_template_set", set }),
    ui: (slide: string | null, grid: boolean) => call<Snapshot>({ cmd: "ui", slide, grid }),
    /** Ouvre l'historique (lecture seule) sur la version la plus récente. */
    historyOpen: () => call<Snapshot>({ cmd: "history_open" }),
    historySeek: (index: number) => call<Snapshot>({ cmd: "history_seek", index }),
    /** Restaure la version montrée : nouveau commit, annulable, retour au deck. */
    historyRestore: () => call<Snapshot>({ cmd: "history_restore" }),
    /** Referme l'historique : le deck courant, intact. */
    historyClose: () => call<Snapshot>({ cmd: "history_close" }),
    historyVersions: () => call<Version[]>({ cmd: "history_versions" }),
  };
}

/** Les requêtes d'un onglet (voir `tabApi`). */
export type TabApi = ReturnType<typeof tabApi>;

/** Un onglet, tel que la barre le montre. */
export interface TabInfo {
  id: string;
  title: string;
  dirty: boolean;
  editing_templates: boolean;
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

/** Les fenêtres et leurs onglets ; `focused` : la dernière fenêtre active. */
export interface WorkspaceView {
  /** Numéro croissant de la vue : une vue plus petite est plus ancienne. */
  generation: number;
  focused: string;
  windows: WindowView[];
}

type WorkspaceRequest =
  | { cmd: "view" }
  | { cmd: "messages" }
  | { cmd: "set_locale"; locale: string | null }
  | { cmd: "new_tab"; window: string }
  | { cmd: "open_tab"; window: string; path: string }
  | { cmd: "same_deck"; path: string }
  | { cmd: "activate"; tab: string }
  | { cmd: "close"; tab: string; discard: boolean }
  | { cmd: "reorder"; tab: string; index: number }
  | { cmd: "move_tab"; tab: string; window: string | null; index: number | null }
  | { cmd: "new_window" }
  | { cmd: "close_window"; window: string; discard: string[] };

const ws = <T>(req: WorkspaceRequest) => host<T>("workspace", { req }, "/api/workspace", req);

/** L'espace de travail : fenêtres et onglets. */
export const workspace = {
  view: () => ws<WorkspaceView>({ cmd: "view" }),
  messages: () => ws<Messages>({ cmd: "messages" }),
  /** Choisit la langue de l'interface (`null` : celle du système) ; ses messages. */
  setLocale: (locale: string | null) => ws<Messages>({ cmd: "set_locale", locale }),
  newTab: (window: string) => ws<WorkspaceView>({ cmd: "new_tab", window }),
  openTab: (window: string, path: string) => ws<WorkspaceView>({ cmd: "open_tab", window, path }),
  /** L'onglet qui tient déjà le deck de `path` sous un autre fichier ; ne change rien. */
  sameDeck: (path: string) => ws<{ tab: string | null; title: string | null }>({ cmd: "same_deck", path }),
  activate: (tab: string) => ws<WorkspaceView>({ cmd: "activate", tab }),
  /** `discard` : l'utilisateur a choisi de perdre le travail non enregistré. */
  close: (tab: string, discard = false) => ws<WorkspaceView>({ cmd: "close", tab, discard }),
  reorder: (tab: string, index: number) => ws<WorkspaceView>({ cmd: "reorder", tab, index }),
  moveTab: (tab: string, window: string | null, index: number | null = null) =>
    ws<WorkspaceView>({ cmd: "move_tab", tab, window, index }),
  newWindow: () => ws<WorkspaceView>({ cmd: "new_window" }),
  closeWindow: (window: string, discard: string[]) => ws<WorkspaceView>({ cmd: "close_window", window, discard }),
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

/** Choix de fichier : dialogue natif dans Tauri, saisie du chemin en développement. */
export async function pickOpenPath(): Promise<string | null> {
  if (inTauri) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({ multiple: false, filters: [{ name: t("ui-brand"), extensions: ["pdf", "json"] }] });
    return typeof p === "string" ? p : null;
  }
  return window.prompt(t("ui-file-open-prompt"));
}

export async function pickSavePath(defaultName: string): Promise<string | null> {
  if (inTauri) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({ defaultPath: `${defaultName}.oku-slides.pdf`, filters: [{ name: t("ui-brand"), extensions: ["pdf"] }] });
  }
  return window.prompt(t("ui-file-save-prompt"), `/tmp/${defaultName}.oku-slides.pdf`);
}
