// Lectures des packs épinglés par le deck : widgets et layouts.

import type { PackInfo, PackPin, Snapshot } from "./api";
import { BASE_NAMESPACE, ESSENTIEL_NAMESPACE } from "./builtinKinds";
import { t } from "./i18n";

/** Espaces de noms réservés aux packs du deck seul : ses widgets et ses layouts. */
const DECK_OWN = ["deck", "deck-layouts"];

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

/**
 * Id du pack édité par défaut (l'atelier du menu) : le pack de layouts propre du deck, sinon son
 * pack de widgets propre, sinon le premier pack non intégré (layouts d'abord), sinon Essentiel,
 * enfin `base`.
 */
export function firstPackId(snap: Snapshot | null): string | null {
  const layouts = Object.entries(snap?.layouts ?? {});
  const widgets = Object.entries(snap?.widgets ?? {});
  const named = (ns: string) => ([n]: [string, PackPin]) => n === ns;
  const own = ([ns]: [string, PackPin]) => !DECK_OWN.includes(ns) && ns !== BASE_NAMESPACE && ns !== ESSENTIEL_NAMESPACE;
  const pin =
    layouts.find(named("deck-layouts")) ??
    widgets.find(named("deck")) ??
    layouts.find(own) ??
    widgets.find(own) ??
    layouts.find(named(ESSENTIEL_NAMESPACE)) ??
    widgets.find(named(BASE_NAMESPACE));
  return pin?.[1].id ?? null;
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
