/** Colonne de gauche du mode modèles : sections Modèles (qui défile) et Widgets (ancrée). */
import { useState, type ReactNode } from "react";
import PileSection from "./PileSection";

export interface TemplatePileProps {
  /** Titre de la section des miniatures. */
  title: string;
  /** La pile des miniatures (`SlidePile`), qui défile seule. */
  slides: ReactNode;
  /** La section Widgets (`PileWidgets`), repliable de son côté. */
  widgets: ReactNode;
}

export default function TemplatePile({ title, slides, widgets }: TemplatePileProps) {
  const [open, setOpen] = useState(true);
  return (
    <div className="pile-column">
      <PileSection className="pile-templates" title={title} open={open} onToggle={() => setOpen((o) => !o)}>
        {slides}
      </PileSection>
      {widgets}
    </div>
  );
}
