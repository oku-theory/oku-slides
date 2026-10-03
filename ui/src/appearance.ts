// Apparence de l'interface : elle suit les couleurs, les polices et les formes
// du thème des slides.

import type { Appearance } from "./api";
import { inTauri } from "./api";
import { type Scheme, cssRgb, slideEnds, uiPalette } from "./scheme";

/**
 * Pile CSS pour des familles Typst : la version variable embarquée d'abord.
 * Typst donne parfois les noms en minuscules (« newsreader ») : on les remet
 * en capitales, comme les déclarent les polices.
 */
export function fontStack(families: string[], fallback: string): string {
  const names = families.flatMap((f) => {
    const name = f.replace(/\b\w/g, (c) => c.toUpperCase());
    return [`"${name} Variable"`, `"${name}"`];
  });
  return [...names, fallback].join(", ");
}

const APP_COLORS = [
  "desk", "sheet", "piece", "ink", "ink-muted", "rule", "edge",
  "accent", "accent-text", "on-accent", "accent-soft", "danger", "shadow",
];

/**
 * Accorde toute l'interface au thème des slides, dans le schéma `scheme` :
 * couleurs (celles du thème pour ce schéma, sinon dérivées), fonds, polices, formes.
 */
export async function applyAppearance(a: Appearance, scheme: Scheme) {
  const root = document.documentElement;
  const style = root.style;
  const slide = (name: string) => a.slide[name];
  for (const name of ["bg", "ink", "ink-muted", "accent", "on-accent", "rule"]) {
    if (slide(name)) style.setProperty(`--t-${name}`, slide(name));
  }
  // Les deux bouts des slides : de quoi dériver une palette dans l'autre schéma.
  const ends = slideEnds(a.slide);
  if (ends) {
    style.setProperty("--t-light", ends.light);
    style.setProperty("--t-dark", ends.dark);
  }
  const palette = uiPalette(a, scheme);
  for (const name of APP_COLORS) {
    if (palette?.[name]) style.setProperty(`--app-${name}`, palette[name]);
    else style.removeProperty(`--app-${name}`);
  }
  if (palette) delete root.dataset.derived;
  else root.dataset.derived = scheme;
  root.dataset.scheme = scheme;
  for (const [name, value] of Object.entries(a.metrics)) {
    style.setProperty(`--app-${name}`, name.endsWith("elevation") ? String(value) : `${value}px`);
  }
  style.setProperty("--app-scheme", scheme);
  style.colorScheme = scheme;
  if (a.font_display.length) style.setProperty("--t-font-display", fontStack(a.font_display, "Georgia, serif"));
  if (a.font_body.length) style.setProperty("--t-font-body", fontStack(a.font_body, "system-ui, sans-serif"));

  // Le fond natif de la fenêtre suit aussi (visible au redimensionnement).
  if (inTauri) {
    const desk = cssRgb(palette?.desk ?? getComputedStyle(root).backgroundColor);
    if (desk) {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().setBackgroundColor([...desk, 255]);
      } catch {
        /* fond natif : facultatif */
      }
    }
  }
}

/**
 * Avant le premier thème reçu : les défauts de la feuille de style (Papier,
 * clair), dérivés si le schéma montré est sombre. Évite un éclair clair.
 */
export function applyDefaultScheme(scheme: Scheme) {
  const root = document.documentElement;
  root.dataset.scheme = scheme;
  if (scheme === "dark") root.dataset.derived = "dark";
  else delete root.dataset.derived;
  root.style.colorScheme = scheme;
}
