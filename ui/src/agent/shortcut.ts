// Raccourcis de l'agent reconnus par l'interface : le brouillon envoie « thème » même sans
// agent prêt, pendant un tour ou un tour ailleurs. Même règle et mêmes mots (`ui-shortcut-theme`)
// que l'hôte (oku-slides-agent, shortcut.rs), qui seul décide.

import { t } from "../i18n";

/** Au-delà, une consigne n'est jamais un raccourci. */
const SHORTCUT_BYTES = 64;
const encoder = new TextEncoder();
/** Diacritiques latins, grecs et cyrilliques ; les voyelles indiennes et thaïes restent. */
const ACCENTS = /[̀-ͯ]/g;
/** Bords qui ne sont ni lettre, ni chiffre, ni signe combinant. */
const EDGES = /^[^\p{L}\p{N}\p{M}]+|[^\p{L}\p{N}\p{M}]+$/gu;

/** `text` en minuscules, sans accents, espaces resserrés, bords nettoyés. */
export function normalizeShortcut(text: string): string {
  const folded = text.toLowerCase().normalize("NFD").replace(ACCENTS, "").replaceAll("’", "'");
  return folded.split(/\s+/).filter(Boolean).join(" ").replace(EDGES, "").normalize("NFC");
}

/** Vrai si `text` entier est un mot du raccourci « thème » de la langue de l'interface. */
export function isShortcut(text: string): boolean {
  if (encoder.encode(text).length > SHORTCUT_BYTES) return false;
  const said = normalizeShortcut(text);
  return said !== "" && t("ui-shortcut-theme").split(",").some((word) => normalizeShortcut(word) === said);
}
