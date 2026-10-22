/**
 * Colonne de gauche. En mode pack : section repliable des miniatures (qui défile, layouts d'un
 * pack de layouts ; aucune pour un pack de widgets), puis la section Widgets (ancrée). Sinon,
 * les miniatures se replient vers la gauche : la colonne devient un rail (`data-folded`) et la
 * feuille reprend la place.
 */
import type { ReactNode } from "react";
import { Icon } from "./icons";
import PileSection from "./PileSection";

export interface PileColumnProps {
  /** Titre de la section des miniatures (Slides ou Layouts), nom du bouton de repli. */
  title: string;
  /** Miniatures montrées (gardé par l'hôte). */
  open: boolean;
  onToggle: () => void;
  /** La pile des miniatures (`DeckPile`), qui défile seule ; nulle pour un pack de widgets. */
  slides: ReactNode | null;
  /** La section Widgets (`PileWidgets`) du mode pack, repliable de son côté. */
  widgets?: ReactNode;
}

export default function PileColumn({ title, open, onToggle, slides, widgets }: PileColumnProps) {
  if (widgets)
    return (
      <div className="pile-column">
        {slides !== null && (
          <PileSection className="pile-thumbs" title={title} open={open} onToggle={onToggle}>
            {slides}
          </PileSection>
        )}
        {widgets}
      </div>
    );
  return (
    <div className="pile-column pile-rail" data-folded={!open}>
      <button className="token ghost pile-fold" aria-expanded={open} aria-label={title} title={title} onClick={onToggle}>
        {Icon.sidebar}
      </button>
      {open && slides}
    </div>
  );
}
