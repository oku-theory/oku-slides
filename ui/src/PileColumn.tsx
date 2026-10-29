/**
 * The left column of a pack or theme tab: a foldable thumbnails section (a layout pack's pile,
 * a theme tab's specimens), or a widget pack's pile alone. A deck shows its thumbnails in a
 * strip under the stage (`App`), with no column.
 */
import type { ReactNode } from "react";
import PileSection from "./PileSection";

export interface PileColumnProps {
  /** The thumbnails section's title (Layouts), the name of its fold button. */
  title: string;
  /** Thumbnails shown (kept by the host). */
  open: boolean;
  onToggle: () => void;
  /** The foldable thumbnails (`DeckPile`, `PackPile`), which scroll alone; `null`: no section. */
  slides: ReactNode | null;
  /** A widget pack's pile (`PackPile`), the whole column; `null` elsewhere. */
  widgets: ReactNode;
}

/** The left column: a foldable thumbnails section, then a widget pack's pile. */
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
