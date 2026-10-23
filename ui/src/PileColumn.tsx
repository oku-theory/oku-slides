/**
 * Colonne de gauche du mode pack : section repliable des miniatures (qui défile, layouts d'un
 * pack de layouts ; aucune pour un pack de widgets), puis la section Widgets (ancrée). Hors mode
 * pack, les miniatures sont en bande sous la scène (`App`), sans colonne.
 */
import type { ReactNode } from "react";
import PileSection from "./PileSection";

export interface PileColumnProps {
  /** Titre de la section des miniatures (Layouts), nom du bouton de repli. */
  title: string;
  /** Miniatures montrées (gardé par l'hôte). */
  open: boolean;
  onToggle: () => void;
  /** La pile des miniatures (`DeckPile`), qui défile seule ; nulle pour un pack de widgets. */
  slides: ReactNode | null;
  /** La section Widgets (`PileWidgets`), repliable de son côté. */
  widgets: ReactNode;
}

export default function PileColumn({ title, open, onToggle, slides, widgets }: PileColumnProps) {
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
}
