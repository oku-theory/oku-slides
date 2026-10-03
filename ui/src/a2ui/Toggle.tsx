// Une ligne qui ouvre et ferme ce qui la suit (timeline, détail, réglages du dock).

import type { ReactNode } from "react";

/** Un bouton à flèche ▸/▾ ; `className` le distingue selon l'endroit. */
export function Toggle({ open, onToggle, className, children }: {
  open: boolean;
  onToggle: () => void;
  className: string;
  children: ReactNode;
}) {
  return (
    <button type="button" className={className} aria-expanded={open} onClick={onToggle}>
      <span aria-hidden="true">{open ? "▾" : "▸"}</span> {children}
    </button>
  );
}
