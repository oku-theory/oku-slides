// Outils des tests de rendu du panneau Widgets (`WidgetWorkbench*.render.test.tsx`).

import "./dom";
import type { TabApi, WidgetChange, WidgetStatus } from "../api";

// Voir ./dom.ts : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const WidgetWorkbench = (await import("../WidgetWorkbench")).default;

type Root = ReturnType<typeof createRoot>;
type Props = Parameters<typeof WidgetWorkbench>[0];

let mounted: { root: Root; host: HTMLElement } | null = null;

/** Démonte le panneau monté, s'il y en a un (à appeler dans `afterEach`). */
export async function unmount() {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
}

/** Monte le panneau avec `over` en plus des props par défaut ; rend de nouvelles props par `rerender`. */
export async function mountWith(fake: TabApi, over: Partial<Props> = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const props = (p: Partial<Props>): Props => ({
    api: fake,
    revision: "r",
    onChanged: () => {},
    onClose: () => {},
    onError: () => {},
    ...over,
    ...p,
  });
  await act(async () => root.render(createElement(WidgetWorkbench, props({}))));
  const rerender = (p: Partial<Props>) => act(async () => root.render(createElement(WidgetWorkbench, props(p))));
  return { host, rerender };
}

/** Choisit le fichier `label` dans la liste et attend sa source. */
export async function pick(host: HTMLElement, label: string) {
  const button = [...host.querySelectorAll<HTMLButtonElement>(".workbench-file")].find((b) => b.textContent?.includes(label));
  await act(async () => button?.click());
}

/** Remplace le texte de l'éditeur, comme une frappe. */
export async function type(host: HTMLElement, text: string) {
  const cm = EditorView.findFromDOM(host.querySelector(".typst-editor .cm-content") as HTMLElement);
  await act(async () => {
    cm?.dispatch({ changes: { from: 0, to: cm.state.doc.length, insert: text }, userEvent: "input.type" });
  });
}

export const editorText = (host: HTMLElement) => host.querySelector(".typst-editor .cm-content")?.textContent;

export const wait = (ms: number) =>
  act(async () => {
    await new Promise((r) => setTimeout(r, ms));
  });

/** Une promesse tenue de l'extérieur. */
export function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((ok, ko) => {
    resolve = ok;
    reject = ko;
  });
  return { promise, resolve, reject };
}

/** Saisit `value` dans le champ du nom puis le valide. */
export async function submitName(host: HTMLElement, value: string) {
  const input = host.querySelector<HTMLInputElement>(".workbench-name input");
  await act(async () => {
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
    set?.call(input, value);
    input?.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () =>
    host.querySelector("form.workbench-name")?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })),
  );
  return input;
}

/** Le bouton d'action `index` (Nouveau, Renommer, Supprimer). */
export const action = (host: HTMLElement, index: number) =>
  [...host.querySelectorAll<HTMLButtonElement>(".workbench-actions .btn")][index];

/**
 * Une api en mémoire : sources par nom ; écritures, suppressions et
 * renommages notés dans `log`, ainsi que chaque appel (`calls`).
 */
export function memory(files: Record<string, string>) {
  const log: string[] = [];
  const calls: string[] = [];
  const statuses = (): WidgetStatus[] => Object.keys(files).map((name) => ({ name, schema: null, diagnostics: [] }));
  const fake = {
    widgetStatus: async () => {
      calls.push("status");
      return statuses();
    },
    widgetSource: async (name: string) => {
      calls.push("source");
      return files[name] ?? "";
    },
    setWidget: async (name: string, source: string): Promise<WidgetChange> => {
      log.push(`set ${name}:${source}`);
      files[name] = source;
      return { status: { name, schema: null, diagnostics: [] }, broken: [], saved: true };
    },
    removeWidget: async (name: string) => {
      log.push(`remove ${name}`);
      delete files[name];
      return statuses();
    },
    renameWidget: async (from: string, to: string) => {
      log.push(`rename ${from}>${to}`);
      // Comme la session : le nom déclaré suit.
      files[to] = (files[from] ?? "").replace(`widget("${from}"`, `widget("${to}"`);
      delete files[from];
      return statuses();
    },
    previewWidget: async () => {
      calls.push("preview");
      return { svg: null, diagnostics: [] };
    },
  } as unknown as TabApi;
  return { fake, log, calls };
}

/** Remplace la méthode `key` de l'api `fake`. */
export function stub(fake: TabApi, key: keyof TabApi, f: (...args: never[]) => unknown) {
  (fake as unknown as Record<string, unknown>)[key] = f;
}
