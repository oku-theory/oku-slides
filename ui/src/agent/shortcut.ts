// Raccourcis de l'agent reconnus par l'interface : le brouillon envoie « thème » même sans
// agent prêt, pendant un tour ou un tour ailleurs. Même règle et mêmes mots (`ui-shortcut-theme`,
// dans la langue de l'interface ou dans celle de la dictée) que l'hôte (oku-slides-agent,
// shortcut.rs), qui seul décide.

import { locale, shortcutWords, t } from "../i18n";

/** Au-delà, une consigne n'est jamais un raccourci. */
const SHORTCUT_BYTES = 64;
const encoder = new TextEncoder();
/** Diacritiques latins, grecs et cyrilliques ; les voyelles indiennes et thaïes restent. */
const ACCENTS = /[̀-ͯ]/g;
/** Letters spelled out by the fold, as `oku_slides_i18n::fold`. */
const SPELLED: Record<string, string> = { "œ": "oe", "æ": "ae", "ø": "o", "ß": "ss", "’": "'" };
const SPELLED_RE = /[œæøß’]/g;
/** Bords qui ne sont ni lettre, ni chiffre, ni signe combinant. */
const EDGES = /^[^\p{L}\p{N}\p{M}]+|[^\p{L}\p{N}\p{M}]+$/gu;

/** `text` en minuscules, sans accents, ligatures épelées, espaces resserrés, bords nettoyés. */
export function normalizeShortcut(text: string): string {
  const folded = text.toLowerCase().normalize("NFD").replace(ACCENTS, "").replace(SPELLED_RE, (c) => SPELLED[c]);
  return folded.split(/\s+/).filter(Boolean).join(" ").replace(EDGES, "").normalize("NFC");
}

/** Shipped locale of a language tag (`fr-CA` → `fr`, `zh-TW` → `zh-Hant`), as `oku_slides_i18n::shipped`. */
export function shippedLocale(tag: string): string | null {
  const [language = "", ...rest] = tag.replaceAll("_", "-").toLowerCase().split("-");
  const traditional = rest.some((p) => ["hant", "tw", "hk", "mo"].includes(p));
  const code = language !== "zh" ? language : traditional ? "zh-Hant" : "zh-Hans";
  return shortcutWords(code) === undefined ? null : code;
}

let dictation: string | null = null;

/** Remembers the dictation's locale: its chosen language, else the detected one, as the host. */
export function setDictationLanguage(chosen: string | null, detected: string | null): void {
  const tag = chosen || detected;
  dictation = tag ? shippedLocale(tag) : null;
}

/** Vrai si `text` entier est un mot du raccourci « thème » de l'interface ou de la dictée. */
export function isShortcut(text: string): boolean {
  if (encoder.encode(text).length > SHORTCUT_BYTES) return false;
  const said = normalizeShortcut(text);
  if (said === "") return false;
  const words = [t("ui-shortcut-theme")];
  if (dictation && dictation !== locale()) words.push(shortcutWords(dictation) ?? "");
  return words.flatMap((w) => w.split(",")).some((word) => normalizeShortcut(word) === said);
}
