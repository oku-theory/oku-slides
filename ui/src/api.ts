import type { Cell, ZoneView } from "./grid";

// Accès à la session Rust : `invoke` dans Tauri, `POST /api` en développement.

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
  kind: "text" | "image" | "shape" | "card" | "group";
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

async function sttCall(req: SttRequest): Promise<SttStatus> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<SttStatus>("stt", { req });
  }
  const res = await fetch("/api/stt", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  const body = await res.json();
  if (!res.ok) throw new Error(body.error ?? res.statusText);
  return body as SttStatus;
}

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
  | { cmd: "delete_template_set"; set: string };

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(req: Request): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>("request", { req });
  }
  const res = await fetch("/api", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  const body = await res.json();
  if (!res.ok) throw new Error(body.error ?? res.statusText);
  return body as T;
}

export const api = {
  snapshot: () => call<Snapshot>({ cmd: "snapshot" }),
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

/** Identifiant court et unique pour un nouvel élément ou une nouvelle slide. */
export function newId(prefix: string): string {
  const r = crypto.getRandomValues(new Uint32Array(2));
  return `${prefix}-${r[0].toString(36)}${r[1].toString(36).slice(0, 3)}`;
}

/** Élément par id, dans les blocs, les éléments libres et les groupes. */
export function findElement(slide: Slide, id: string): Element | undefined {
  const walk = (list: Element[] | undefined): Element | undefined => {
    for (const e of list ?? []) {
      if (e.id === id) return e;
      const found = walk(e.children);
      if (found) return found;
    }
    return undefined;
  };
  return walk(slide.blocks) ?? walk(slide.elements);
}

/** Texte éditable → paragraphes (même règle que côté Rust). */
export function textToParagraphs(text: string, previous: Paragraph[] = []): Paragraph[] {
  const plain = (p: Paragraph) => p.runs.map((r) => r.t).join("");
  return text.split("\n").map((line) => {
    let list: Paragraph["list"];
    let body = line;
    const num = /^(\d+)\. (.*)$/.exec(line);
    if (line.startsWith("- ")) {
      list = "bullet";
      body = line.slice(2);
    } else if (num) {
      list = "number";
      body = num[2];
    }
    const same = previous.find((p) => p.list === list && plain(p) === body);
    if (same) return same;
    return list ? { runs: [{ t: body }], list } : { runs: [{ t: body }] };
  });
}

/** Pile CSS pour des familles Typst : la version variable embarquée d'abord. */
function fontStack(families: string[], fallback: string): string {
  const names = families.flatMap((f) => [`"${f} Variable"`, `"${f}"`]);
  return [...names, fallback].join(", ");
}

const APP_COLORS = [
  "desk", "sheet", "piece", "ink", "ink-muted", "rule", "edge",
  "accent", "accent-text", "on-accent", "accent-soft", "danger", "shadow",
];

/** Accorde toute l'interface au thème des slides : couleurs, fonds, polices, formes. */
export async function applyAppearance(a: Appearance) {
  const root = document.documentElement;
  const style = root.style;
  const slide = (name: string) => a.slide[name];
  for (const name of ["bg", "ink", "ink-muted", "accent", "on-accent", "rule"]) {
    if (slide(name)) style.setProperty(`--t-${name}`, slide(name));
  }
  for (const name of APP_COLORS) {
    if (a.app[name]) style.setProperty(`--app-${name}`, a.app[name]);
    else style.removeProperty(`--app-${name}`);
  }
  for (const [name, value] of Object.entries(a.metrics)) {
    style.setProperty(`--app-${name}`, name.endsWith("elevation") ? String(value) : `${value}px`);
  }
  style.setProperty("--app-scheme", a.scheme);
  root.style.colorScheme = a.scheme;
  if (a.font_display.length) style.setProperty("--t-font-display", fontStack(a.font_display, "Georgia, serif"));
  if (a.font_body.length) style.setProperty("--t-font-body", fontStack(a.font_body, "system-ui, sans-serif"));

  // Le fond natif de la fenêtre suit aussi (visible au redimensionnement).
  if (inTauri) {
    const desk = getComputedStyle(root).getPropertyValue("--paper-000").trim();
    const hex = a.app.desk ?? (/^#[0-9a-f]{6}$/i.test(desk) ? desk : null);
    if (hex) {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const n = parseInt(hex.slice(1), 16);
        await getCurrentWindow().setBackgroundColor([(n >> 16) & 255, (n >> 8) & 255, n & 255, 255]);
      } catch {
        /* fond natif : facultatif */
      }
    }
  }
}

/** Copie d'une slide modèle avec de nouveaux identifiants (slide et éléments). */
export function instantiate(template: Slide): Slide {
  const copy: Slide = JSON.parse(JSON.stringify(template));
  copy.id = newId("s");
  const renew = (list?: Element[]) => {
    for (const e of list ?? []) {
      e.id = newId("e");
      renew(e.children);
    }
  };
  renew(copy.blocks);
  renew(copy.elements);
  return copy;
}
