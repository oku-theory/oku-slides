// Sortes d'élément qui ont leur propre panneau à la place des réglages génériques.

// À garder alignée sur les branches chart/table de TargetPanel.tsx.
const OWN_PANEL: readonly string[] = ["chart", "table"];

/** Vrai pour un graphique ou un tableau : leur panneau remplace les réglages de la cible. */
export const hasOwnPanel = (kind: string | undefined): boolean => kind !== undefined && OWN_PANEL.includes(kind);
