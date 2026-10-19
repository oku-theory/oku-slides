// Choix d'un fichier : dialogue natif dans Tauri, saisie du chemin en développement.

import { inTauri } from "./hostCall";
import { t } from "./i18n";

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

/** Le chemin où enregistrer le deck, proposé sous `defaultName` ; `null` : abandonné. */
export async function pickSavePath(defaultName: string): Promise<string | null> {
  if (inTauri) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({ defaultPath: `${defaultName}.oku-slides.pdf`, filters: [{ name: t("ui-brand"), extensions: ["pdf"] }] });
  }
  return typedPath(window.prompt(t("ui-file-save-prompt"), `/tmp/${defaultName}.oku-slides.pdf`));
}
