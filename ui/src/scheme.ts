// Schéma de l'interface : clair, sombre ou celui du système, choisi dans le
// menu et gardé par l'hôte (requêtes « scheme » et « set_scheme »).

import { useSyncExternalStore } from "react";
import { type Appearance, type SchemeSetting, inTauri, listenTauri, workspace } from "./api";
import { luminance } from "./color";

/** Schéma montré par l'interface. */
export type Scheme = "light" | "dark";

/** Schéma choisi ; `null` : celui du système. */
export type SchemeChoice = Scheme | null;

/** Schéma montré : le choix, sinon celui du système. */
export function resolveScheme(choice: SchemeChoice, systemDark: boolean): Scheme {
  return choice ?? (systemDark ? "dark" : "light");
}

/**
 * Couleurs d'interface du thème pour `scheme` : les siennes, ou sa variante
 * pour l'autre schéma ; `null` : à dériver des couleurs des slides.
 */
export function uiPalette(a: Appearance, scheme: Scheme): Record<string, string> | null {
  const own: Scheme = a.scheme === "dark" ? "dark" : "light";
  return scheme === own ? a.app : (a.variants?.[scheme] ?? null);
}

/** Fond et encre des slides, du plus clair au plus foncé ; `null` s'ils sont illisibles. */
export function slideEnds(slide: Record<string, string>): { light: string; dark: string } | null {
  const lit = [slide.bg, slide.ink].filter((c) => c && luminance(c) !== null) as string[];
  if (lit.length < 2) return null;
  const [a, b] = lit;
  return luminance(a)! >= luminance(b)! ? { light: a, dark: b } : { light: b, dark: a };
}

/** Composantes 0–255 d'une couleur CSS `#hex`, `rgb()` ou `color(srgb …)` ; `null` sinon. */
export function cssRgb(value: string): [number, number, number] | null {
  const v = value.trim();
  const hex = /^#([0-9a-f]{6})$/i.exec(v);
  if (hex) {
    const n = parseInt(hex[1], 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }
  const rgb = /^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)/i.exec(v);
  if (rgb) return [rgb[1], rgb[2], rgb[3]].map((c) => Math.round(Number(c))) as [number, number, number];
  const srgb = /^color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)/i.exec(v);
  if (srgb) return [srgb[1], srgb[2], srgb[3]].map((c) => Math.round(Number(c) * 255)) as [number, number, number];
  return null;
}

let chosen: SchemeChoice = null;
const listeners = new Set<() => void>();
const notify = () => {
  for (const listener of listeners) listener();
};
const systemDark = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: dark)") : null;
systemDark?.addEventListener("change", notify);

/** Schéma montré maintenant (le choix, sinon celui du système). */
export function currentScheme(): Scheme {
  return resolveScheme(chosen, systemDark?.matches ?? false);
}

/** Prévient quand le schéma montré peut changer ; rend de quoi se désabonner. */
export function onSchemeChange(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/**
 * Retient le choix reçu de l'hôte, puis prévient. La fenêtre native prend
 * le même choix : sans choix, elle (et donc la webview) suit le système.
 */
export function applyScheme(choice: SchemeChoice): void {
  chosen = choice === "light" || choice === "dark" ? choice : null;
  if (inTauri) {
    import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) => getCurrentWindow().setTheme(chosen))
      .catch(() => {
        /* thème natif : facultatif */
      });
  }
  notify();
}

/** Charge le choix gardé par l'hôte. */
export async function loadScheme(): Promise<void> {
  applyScheme((await workspace.scheme()).scheme);
}

/** Suit le choix fait dans n'importe quelle fenêtre (`scheme`, diffusé par l'hôte) ; rend de quoi arrêter. */
export function followScheme(): () => void {
  if (!inTauri) return () => {};
  return listenTauri(async () => {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<SchemeSetting>("scheme", (e) => applyScheme(e.payload.scheme));
  });
}

/** Change le schéma de l'interface (`null` : celui du système). */
export async function changeScheme(choice: SchemeChoice): Promise<void> {
  applyScheme((await workspace.setScheme(choice)).scheme);
}

/** Schéma montré, relu à chaque changement de choix ou du système. */
export function useScheme(): Scheme {
  return useSyncExternalStore(onSchemeChange, currentScheme);
}
