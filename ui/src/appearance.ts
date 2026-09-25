// Apparence de l'interface : elle suit les couleurs, les polices et les formes
// du thème des slides.

import type { Appearance } from "./api";
import { inTauri } from "./api";

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
