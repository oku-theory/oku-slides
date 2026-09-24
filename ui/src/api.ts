// Accès à la session Rust : `invoke` dans Tauri, `POST /api` en développement.

export type SlideKind = "title" | "toc" | "chapter" | "content" | "end" | "blank";

export interface SlideSummary {
  id: string;
  kind: SlideKind;
  title: string | null;
  page: number;
  hash: string;
}

export interface Appearance {
  bg: string;
  surface: string;
  ink: string;
  ink_muted: string;
  accent: string;
  accent_text: string;
  on_accent: string;
  rule: string;
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
  slides: SlideSummary[];
  can_undo: boolean;
  undo_label: string | null;
  can_redo: boolean;
  redo_label: string | null;
  warnings: string[];
  notice: string | null;
  appearance: Appearance;
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
  text?: Paragraph[];
  children?: Element[];
  [key: string]: unknown;
}

export interface Slide {
  id: string;
  kind: SlideKind;
  title?: string;
  subtitle?: string;
  kicker?: string;
  layout?: string;
  text?: string;
  blocks?: Element[];
  elements?: Element[];
  notes?: string;
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
}

export interface SlideView {
  slide: Slide;
  page: number;
  hash: string;
  width: number;
  height: number;
  targets: Target[];
}

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
  | { cmd: "redo" };

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
};

/** Choix de fichier : dialogue natif dans Tauri, saisie du chemin en développement. */
export async function pickOpenPath(): Promise<string | null> {
  if (inTauri) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({ multiple: false, filters: [{ name: "Slider", extensions: ["pdf", "json"] }] });
    return typeof p === "string" ? p : null;
  }
  return window.prompt("Chemin du fichier à ouvrir (.slider.pdf ou .json)");
}

export async function pickSavePath(defaultName: string): Promise<string | null> {
  if (inTauri) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({ defaultPath: `${defaultName}.slider.pdf`, filters: [{ name: "Slider", extensions: ["pdf"] }] });
  }
  return window.prompt("Enregistrer sous (chemin complet)", `/tmp/${defaultName}.slider.pdf`);
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

/** Accorde l'interface au thème des slides (couleurs et polices). */
export function applyAppearance(a: Appearance) {
  if (!a.bg) return;
  const root = document.documentElement.style;
  const set = (name: string, value: string) => root.setProperty(`--t-${name}`, value);
  set("bg", a.bg);
  set("surface", a.surface);
  set("ink", a.ink);
  set("ink-muted", a.ink_muted);
  set("accent", a.accent);
  set("accent-text", a.accent_text);
  set("on-accent", a.on_accent);
  set("rule", a.rule);
  if (a.font_display.length) set("font-display", fontStack(a.font_display, "Georgia, serif"));
  if (a.font_body.length) set("font-body", fontStack(a.font_body, "system-ui, sans-serif"));
}
