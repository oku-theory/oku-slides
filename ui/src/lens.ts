// Lentille du verre (t-391) : le fond se déforme aux bords des objets, comme sur macOS.
// Seul Chromium accepte un filtre SVG en `backdrop-filter` ; WebKit (Tauri sur macOS
// et Linux) l'ignore : le verre y garde ses arêtes réfractées, dessinées en CSS.

/** Dit si le moteur de `userAgent` déforme le fond par un filtre SVG. */
export function backdropLens(userAgent: string): boolean {
  return /\b(Chrome|Chromium)\//.test(userAgent);
}

/** Active la lentille (filtre `#glass-lens` d'index.html) si le moteur la rend. */
export function applyLens(root: HTMLElement, userAgent: string) {
  if (backdropLens(userAgent)) root.dataset.lens = "";
}
