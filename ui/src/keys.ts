// Raccourcis clavier dans la notation de la plateforme : écrits à la macOS
// (`⌘⇧S`), lus `Ctrl+Shift+S` ailleurs. Même règle que `pc_keys`
// (oku-slides-i18n), qui récrit les messages et l'aide côté hôte ; ici, les
// raccourcis écrits dans le code (menus).

import { isMac } from "./api";

const MODIFIERS = /([⌃⌥⇧⌘]+)( (?=[\p{L}\p{N}]))?/gu;

/** `text` dans la notation de la plateforme ; `mac` : celle de macOS, telle quelle. */
export function keyLabel(text: string, mac = isMac): string {
  if (mac) return text;
  return text.replace(MODIFIERS, (run: string, mods: string, _space: string | undefined, offset: number) => {
    const names = [
      /[⌘⌃]/.test(mods) ? "Ctrl" : null,
      mods.includes("⌥") ? "Alt" : null,
      mods.includes("⇧") ? "Shift" : null,
    ].filter(Boolean);
    // La touche suit : collée, ou après une espace si c'est un mot (`⌘ Entrée`).
    const next = text[offset + run.length];
    const joined = run.length > mods.length || (next !== undefined && !/\s/.test(next));
    return names.join("+") + (joined ? "+" : "");
  });
}
