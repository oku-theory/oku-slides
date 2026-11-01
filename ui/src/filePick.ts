// Choix d'un fichier : dialogue natif dans Tauri, saisie du chemin en développement.

import { inTauri } from "./hostCall";
import { cleanArg, t } from "./i18n";

/** Un chemin saisi (mode web) sans ses blancs autour ; vide ou abandonné : `null`. */
export function typedPath(typed: string | null): string | null {
  const path = typed?.trim() ?? "";
  return path === "" ? null : path;
}

/**
 * Le fichier à ouvrir, deck ou présentation PowerPoint ; `null` : abandonné.
 *
 * Un chemin saisi est rendu sans les blancs qui l'entourent.
 */
export async function pickOpenPath(): Promise<string | null> {
  if (inTauri) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const deck = { name: t("ui-brand"), extensions: ["pdf", "json"] };
    // Le premier filtre est celui du dialogue : il prend les deux.
    const filters = [
      { name: t("ui-file-filter-presentations"), extensions: [...deck.extensions, "pptx"] },
      deck,
      { name: t("ui-file-filter-pptx"), extensions: ["pptx"] },
    ];
    const p = await open({ multiple: false, filters });
    return typeof p === "string" ? p : null;
  }
  return typedPath(window.prompt(t("ui-file-open-prompt")));
}

/** Longest proposed file name, in code points: macOS caps a name at 255 bytes. */
const SAVE_NAME_MAX = 200;

/**
 * The file name proposed for a deck named `name` (t-1383).
 *
 * The name comes from the deck (an imported title): cleaned like a message argument, so a U+202E
 * cannot reverse the proposed name; path separators become `-`, the ends are trimmed and the
 * name clipped. Nothing left: the app's name.
 */
export function saveName(name: string): string {
  const cleaned = cleanArg(name).replace(/[/\\]/g, "-").trim();
  const clipped = Array.from(cleaned).slice(0, SAVE_NAME_MAX).join("").trim();
  return clipped === "" ? t("ui-brand") : clipped;
}

/** The path to save the deck to, proposed as `saveName(name)`; `null`: given up. */
export async function pickSavePath(name: string): Promise<string | null> {
  const defaultName = saveName(name);
  if (inTauri) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({ defaultPath: `${defaultName}.oku-slides.pdf`, filters: [{ name: t("ui-brand"), extensions: ["pdf"] }] });
  }
  return typedPath(window.prompt(t("ui-file-save-prompt"), `/tmp/${defaultName}.oku-slides.pdf`));
}
