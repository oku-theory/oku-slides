// Outils de test partagés : api en mémoire, promesses tenues, attentes.

import "./dom";
import type { TabApi, WidgetChange, WidgetStatus } from "../api";

// Voir ./dom.ts : React se charge après le DOM.
const { act } = await import("react");

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
