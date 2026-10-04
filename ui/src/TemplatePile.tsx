/** Colonne de gauche du mode modèles : sections Modèles (qui défile) et Widgets (ancrée). */
import type { ReactNode } from "react";
import PileSection from "./PileSection";

export interface TemplatePileProps {
  /** Titre de la section des miniatures. */
  title: string;
  /** Section Modèles ouverte (gardé par l'hôte). */
  open: boolean;
  onToggle: () => void;
  /** La pile des miniatures (`SlidePile`), qui défile seule. */
  slides: ReactNode;
  /** La section Widgets (`PileWidgets`), repliable de son côté. */
  widgets: ReactNode;
}

export default function TemplatePile({ title, open, onToggle, slides, widgets }: TemplatePileProps) {
  return (
    <div className="pile-column">
      <PileSection className="pile-templates" title={title} open={open} onToggle={onToggle}>
        {slides}
      </PileSection>
      {widgets}
    </div>
  );
}
