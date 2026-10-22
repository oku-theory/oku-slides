// Kinds des packs intégrés `base` (widgets) et `essentiel` (layouts) ; mêmes valeurs que
// `oku_slides_core::kinds::base` et `oku_slides_core::kinds::essentiel`.

/** L'espace de noms du pack `base`. */
export const BASE_NAMESPACE = "base";
const PREFIX = `${BASE_NAMESPACE}/`;

/** Les widgets du pack `base`, par nom. */
export const BASE = {
  text: "base/text",
  card: "base/card",
  image: "base/image",
  shape: "base/shape",
  chart: "base/chart",
  table: "base/table",
  stack: "base/stack",
} as const;

/** Le nom d'un widget de `base`. */
export type BaseName = keyof typeof BASE;

/** Le nom de base d'un kind `base/<nom>`, ou null. */
export function baseName(kind: string): BaseName | null {
  const name = kind.startsWith(PREFIX) ? kind.slice(PREFIX.length) : "";
  return Object.hasOwn(BASE, name) ? (name as BaseName) : null;
}

/** L'espace de noms du pack de layouts `essentiel`. */
export const ESSENTIEL_NAMESPACE = "essentiel";

/** Les layouts du pack `essentiel`, par nom. */
export const ESSENTIEL = {
  title: "essentiel/title",
  toc: "essentiel/toc",
  chapter: "essentiel/chapter",
  content: "essentiel/content",
  end: "essentiel/end",
  blank: "essentiel/blank",
} as const;
