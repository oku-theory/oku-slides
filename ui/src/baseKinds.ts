// Kinds du pack de widgets intégré `base` ; mêmes valeurs que `oku_slides_core::kinds::base`.

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
