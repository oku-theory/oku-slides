// Lectures des packs épinglés par le deck : widgets et layouts.

import type { Snapshot } from "./api";
import type { PackInfo, PackPin } from "./packTypes";
import { t } from "./i18n";

/** Clé stable des épingles d'un deck : `espace@version` triés, joints par une virgule. */
export function pinsKey(pins: Record<string, PackPin>): string {
  return Object.entries(pins)
    .map(([ns, pin]) => `${ns}@${pin.version}`)
    .sort()
    .join(",");
}

/** Clé des épingles du deck, widgets puis layouts : change avec la version d'un pack épinglé. */
export function deckPinsKey(snap: Pick<Snapshot, "widgets" | "layouts"> | null): string {
  return `${pinsKey(snap?.widgets ?? {})}|${pinsKey(snap?.layouts ?? {})}`;
}

/** Nom d'un pack dans la liste des packs : intégré tel quel, sinon dit reçu (arrivé avec un deck) ou à moi. */
export function packOptionLabel(pack: PackInfo): string {
  if (pack.builtin) return pack.name;
  return pack.received ? t("ui-picker-set-received", { name: pack.name }) : t("ui-picker-set-mine", { name: pack.name });
}

/** Longueur maximale d'un espace de noms, en octets (comme le noyau). */
const MAX_NAMESPACE = 64;
/** Syntaxe d'un espace de noms : `[a-z][a-z0-9-]*`, éventuellement `éditeur.nom`. */
const NAMESPACE = /^[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)?$/;

/** Espace de noms et nom d'un genre qualifié, comme `split_kind` ; `null` pour le noyau. */
export function splitKind(kind: string): { ns: string; name: string } | null {
  const cut = kind.indexOf("/");
  if (cut < 0) return null;
  const ns = kind.slice(0, cut);
  const name = kind.slice(cut + 1);
  const valid = ns.length <= MAX_NAMESPACE && NAMESPACE.test(ns) && name !== "" && !name.includes("/");
  return valid ? { ns, name } : null;
}

/** Pack épinglé (id, widgets ou layouts) et nom de fichier d'un widget de pack ; `pack` nul hors des épingles. */
export function widgetToOpen(snap: Snapshot, kind: string): { pack: string | null; name: string } | null {
  const parts = splitKind(kind);
  if (!parts) return null;
  return { pack: (snap.widgets[parts.ns] ?? snap.layouts[parts.ns])?.id ?? null, name: parts.name };
}
