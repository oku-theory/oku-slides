// Accès à l'hôte Rust : la session et la dictée (`invoke` dans Tauri, `POST`
// en développement), et les dialogues de fichier. Les types décrivent le JSON
// échangé.

import type { Cell, ZoneView } from "./grid";

export type SlideKind = "title" | "toc" | "chapter" | "content" | "end" | "blank";

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
  appearance: Appearance;
  /** Ensemble de modèles du deck. */
  templates: string;
  /** Édition d'un ensemble de modèles en cours. */
  editing: TemplateMode | null;
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
}

export interface Paragraph {
  runs: Run[];
  list?: "bullet" | "number";
}

export interface Element {
  id: string;
  kind: "text" | "image" | "shape" | "card" | "group" | "chart";
  frame?: Frame;
  /** Accroche à la grille de la slide (prioritaire sur `frame`). */
  cell?: Cell;
  text?: Paragraph[];
  children?: Element[];
  [key: string]: unknown;
}

export interface Slide {
  id: string;
  kind: SlideKind;
  /** Nom et description d'une slide modèle. */
  name?: string;
  description?: string;
  title?: string;
  subtitle?: string;
  kicker?: string;
  layout?: string;
  text?: string;
  blocks?: Element[];
  elements?: Element[];
  /** Grille de la zone de corps ; gouttière : jeton ou pt. */
  grid?: { columns: number; rows: number; gap?: string | number };
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

export interface Target {
  id: string;
  target: "element" | "field";
  kind: string;
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
  targets: Target[];
  /** Zone des éléments libres (pt, page) et sa grille résolue. */
  zone: ZoneView | null;
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

const sttCall = (req: SttRequest) => host<SttStatus>("stt", "/api/stt", req);

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
  | { cmd: "new" }
  | { cmd: "open"; path: string }
  | { cmd: "save"; path: string | null }
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
  | { cmd: "messages" };

/** Langue de l'utilisateur et ressources Fluent de l'interface (anglais d'abord). */
export interface Messages {
  locale: string;
  resources: string[];
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Une requête à l'hôte : la commande Tauri `command`, ou `POST endpoint` en développement. */
async function host<T>(command: string, endpoint: string, req: object): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, { req });
  }
  const res = await fetch(endpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  const body = await res.json();
  if (!res.ok) throw new Error(body.error ?? res.statusText);
  return body as T;
}

const call = <T>(req: Request) => host<T>("request", "/api", req);

export const api = {
  snapshot: () => call<Snapshot>({ cmd: "snapshot" }),
  messages: () => call<Messages>({ cmd: "messages" }),
  newDeck: () => call<Snapshot>({ cmd: "new" }),
  open: (path: string) => call<Snapshot>({ cmd: "open", path }),
  save: (path: string | null) => call<Snapshot>({ cmd: "save", path }),
  slide: (id: string) => call<SlideView>({ cmd: "slide", id }),
  svg: (id: string) => call<string>({ cmd: "svg", id }),
  apply: (label: string, ops: Op[]) => call<Snapshot>({ cmd: "apply", label, ops }),
  undo: () => call<Snapshot>({ cmd: "undo" }),
  redo: () => call<Snapshot>({ cmd: "redo" }),
  templates: (set?: string | null) => call<TemplatesView>({ cmd: "templates", set: set ?? null }),
  templateSets: () => call<SetInfo[]>({ cmd: "template_sets" }),
  editTemplates: (set: string) => call<Snapshot>({ cmd: "edit_templates", set }),
  saveTemplates: (asNew = false) => call<Snapshot>({ cmd: "save_templates", as_new: asNew }),
  closeTemplates: () => call<Snapshot>({ cmd: "close_templates" }),
  deleteTemplateSet: (set: string) => call<SetInfo[]>({ cmd: "delete_template_set", set }),
};

/** Choix de fichier : dialogue natif dans Tauri, saisie du chemin en développement. */
export async function pickOpenPath(): Promise<string | null> {
  if (inTauri) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({ multiple: false, filters: [{ name: "Écho", extensions: ["pdf", "json"] }] });
    return typeof p === "string" ? p : null;
  }
  return window.prompt("Chemin du fichier à ouvrir (.echo.pdf ou .json)");
}

export async function pickSavePath(defaultName: string): Promise<string | null> {
  if (inTauri) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({ defaultPath: `${defaultName}.echo.pdf`, filters: [{ name: "Écho", extensions: ["pdf"] }] });
  }
  return window.prompt("Enregistrer sous (chemin complet)", `/tmp/${defaultName}.echo.pdf`);
}
