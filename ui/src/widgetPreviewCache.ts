// Cache des aperçus de la grille d'ajout : un SVG recadré par widget, version de pack et look.

/** Aperçus gardés au plus : au-delà, le moins récemment lu part. */
const MAX_PREVIEWS = 120;

/** Un cache borné d'aperçus SVG, par clé (voir `previewKey`). */
export interface PreviewCache {
  get(key: string): string | undefined;
  set(key: string, svg: string): void;
}

/** Un cache vide d'au plus `max` aperçus, le moins récemment lu évincé d'abord. */
export function createPreviewCache(max = MAX_PREVIEWS): PreviewCache {
  // L'ordre d'insertion d'une Map sert d'ordre d'usage : une lecture remet la clé à la fin.
  const entries = new Map<string, string>();
  return {
    get(key) {
      const svg = entries.get(key);
      if (svg !== undefined) {
        entries.delete(key);
        entries.set(key, svg);
      }
      return svg;
    },
    set(key, svg) {
      entries.delete(key);
      entries.set(key, svg);
      while (entries.size > max) entries.delete(entries.keys().next().value as string);
    },
  };
}

/**
 * La clé de l'aperçu du widget `name` : la version de son pack, le look du deck et la langue (les
 * exemples sont traduits). Un widget réécrit change la version de son pack, donc la clé.
 */
export function previewKey(version: string, look: string, lang: string, name: string): string {
  return JSON.stringify([version, look, lang, name]);
}
