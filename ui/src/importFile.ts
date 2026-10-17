// Un fichier choisi par « Ouvrir » : un .pptx passe par le dialogue d'import.

/** Vrai pour une présentation PowerPoint (`.pptx`, extension sans casse). */
export function isPptx(path: string): boolean {
  return /\.pptx$/i.test(path);
}

/** Le nom du fichier, sans ses dossiers (séparateurs `/` et `\`). */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
