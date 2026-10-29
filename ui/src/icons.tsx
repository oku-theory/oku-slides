// Pictogrammes de l'interface : un seul trait (24 × 24, 1,5 px, bouts ronds).
// Un bouton à pictogramme porte son nom en `aria-label` et en infobulle.

import type { ReactNode } from "react";

/** Dessine un pictogramme au trait, décoratif pour les lecteurs d'écran. */
export const stroke = (children: ReactNode) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
    {children}
  </svg>
);

/** Pictogrammes partagés, par usage. */
export const Icon = {
  undo: stroke(<><path d="M9 14L4 9l5-5" /><path d="M4 9h11a5 5 0 0 1 0 10h-3" /></>),
  redo: stroke(<><path d="M15 14l5-5-5-5" /><path d="M20 9H9a5 5 0 0 0 0 10h3" /></>),
  // Graphique et tableau : les deux vues d'un tableau de données (bouton « Afficher en »).
  chart: stroke(<><path d="M4 20h16" /><path d="M7 20v-6M12 20V6M17 20v-9" /></>),
  table: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M4 10h16M4 14.5h16M10 5v14" /></>),
  // La grille d'ajout : trois tuiles et une pièce posée de biais.
  widget: stroke(<><rect x="4" y="4" width="7" height="7" rx="1.5" /><rect x="4" y="13" width="7" height="7" rx="1.5" /><rect x="13" y="13" width="7" height="7" rx="1.5" /><path d="M16.5 3.5l3.5 3.5-3.5 3.5-3.5-3.5z" /></>),
  // Les packs du deck : une boîte, son couvercle et son ruban.
  // A thème : une palette de peintre et ses taches de couleur.
  palette: stroke(<><path d="M12 4a8 8 0 1 0 0 16c1.1 0 1.6-.8 1.3-1.7-.3-.9.2-1.8 1.2-1.8H16a4 4 0 0 0 4-4c0-4-3.6-8.5-8-8.5z" /><circle cx="8" cy="11" r="1" /><circle cx="11" cy="7.5" r="1" /><circle cx="15.5" cy="8.5" r="1" /></>),
  packs: stroke(<><path d="M4 8l8-4 8 4v8l-8 4-8-4z" /><path d="M4 8l8 4 8-4" /><path d="M12 12v8" /></>),
  // Actions.
  add: stroke(<path d="M12 5v14M5 12h14" />),
  // Titre d'une section repliable : pointe à droite, tourné vers le bas une fois ouvert.
  chevron: stroke(<path d="M9 6l6 6-6 6" />),
  check: stroke(<path d="M5 12.5l4.5 4.5L19 7.5" />),
  close: stroke(<path d="M6 6l12 12M18 6L6 18" />),
  // Monter, descendre, retirer une entrée d'une liste ordonnée (parents d'un thème).
  up: stroke(<><path d="M12 19V5" /><path d="M6 11l6-6 6 6" /></>),
  down: stroke(<><path d="M12 5v14" /><path d="M6 13l6 6 6-6" /></>),
  remove: stroke(<path d="M5 12h14" />),
  trash: stroke(<><path d="M4.5 7h15" /><path d="M9.5 7V4.5h5V7" /><path d="M6.5 7l1 12.5h9l1-12.5" /></>),
  send: stroke(<><path d="M4 12l16-8-6 16-2.5-6.5z" /><path d="M11.5 13.5L20 4" /></>),
  keyboard: stroke(<><rect x="3" y="6" width="18" height="12" rx="2" /><path d="M7 10h.01M10.5 10h.01M14 10h.01M17 10h.01M8 14h8" /></>),
  restore: stroke(<><path d="M4 12a8 8 0 1 0 2.4-5.7" /><path d="M4 4v4.5h4.5" /><path d="M12 8v4l3 2" /></>),
  code: stroke(<><path d="M8.5 7.5L4 12l4.5 4.5" /><path d="M15.5 7.5L20 12l-4.5 4.5" /><path d="M13.5 5l-3 14" /></>),
  back: stroke(<><path d="M10 6l-6 6 6 6" /><path d="M4 12h16" /></>),
  // Slide précédente, suivante (aide en lecture seule).
  previous: stroke(<path d="M14.5 6l-6 6 6 6" />),
  next: stroke(<path d="M9.5 6l6 6-6 6" />),
  /** La bande des miniatures sous la slide. */
  strip: stroke(<><rect x="3.5" y="4.5" width="17" height="15" rx="2" /><path d="M3.5 14.5h17" /></>),
  detach: stroke(<><path d="M14 4h6v6" /><path d="M20 4l-8 8" /><path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" /></>),
  attach: stroke(<><path d="M10 20H4v-6" /><path d="M4 20l8-8" /><path d="M6 10V5a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1h-5" /></>),
  newChat: stroke(<><path d="M20 12.5a7.5 7.5 0 0 1-11 6.6L4 20l1-4.4A7.5 7.5 0 1 1 20 12.5z" /><path d="M12.5 9.5v6M9.5 12.5h6" /></>),
  addRow: stroke(<><rect x="4" y="4" width="16" height="10" rx="1.5" /><path d="M4 9h16" /><path d="M12 16.5v4M10 18.5h4" /></>),
  addColumn: stroke(<><rect x="4" y="4" width="10" height="16" rx="1.5" /><path d="M9 4v16" /><path d="M16.5 12h4M18.5 10v4" /></>),
  /** Barre latérale : replier / rouvrir la pile des miniatures. */
  /** Panneau du bas : montrer / masquer les paramètres sous la slide. */
  stack: stroke(<><rect x="5.5" y="12" width="5" height="8" rx="0.5" /><rect x="5.5" y="6" width="5" height="6" rx="0.5" /><rect x="13.5" y="14" width="5" height="6" rx="0.5" /><rect x="13.5" y="4" width="5" height="10" rx="0.5" /></>),
};
