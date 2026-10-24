// Images collées dans le brouillon de l'agent : ce que l'UI en lit et en montre.

import { t } from "../i18n";

/** Le libellé d'une image : son type court et sa taille, « PNG 1200×800 ». */
export function imageLabel(mime: string, width: number, height: number): string {
  const type = mime.replace(/^image\//, "").toUpperCase();
  return t("ui-pasted-image", { type, width: String(width), height: String(height) });
}

/** Les fichiers image d'un collage ; aucun s'il porte du texte (une copie de Word y joint une image du rendu). */
export function clipboardImages(dt: DataTransfer | null): File[] {
  if (!dt || dt.getData("text/plain").trim() !== "") return [];
  const files = [...dt.files].filter((f) => f.type.startsWith("image/"));
  if (files.length > 0) return files;
  // Certains navigateurs ne remplissent que `items` pour une image copiée.
  const fromItems: File[] = [];
  for (const item of dt.items ?? []) {
    const file = item.kind === "file" && item.type.startsWith("image/") ? item.getAsFile() : null;
    if (file) fromItems.push(file);
  }
  return fromItems;
}

/** Les octets en base64, par tranches : `String.fromCharCode` refuse les très longues listes. */
export function toBase64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf);
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

/** L'identifiant d'asset de l'hôte (`sha256:<hex>`) ; `null` hors d'un contexte sûr (pas de `crypto.subtle`). */
export async function assetId(buf: ArrayBuffer): Promise<string | null> {
  if (!globalThis.crypto?.subtle) return null;
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", buf));
  return `sha256:${[...digest].map((b) => b.toString(16).padStart(2, "0")).join("")}`;
}
