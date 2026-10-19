// URL d'image d'un SVG rendu par la session : dans un <img>, il n'exécute ni script ni lien.

/** Le SVG en URL `data:` pour un `<img>`. */
export const svgSrc = (svg: string): string => `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
