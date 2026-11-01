/**
 * The left column of a theme tab: its foldable specimens section. A deck shows its thumbnails
 * in a strip under the stage (`App`), a pack tab its items in a strip above the item (`WidgetPane`).
 */
import type { ReactNode } from "react";
import PileSection from "./PileSection";

export interface PileColumnProps {
  /** The thumbnails section's title (Layouts), the name of its fold button. */
  title: string;
  /** Thumbnails shown (kept by the host). */
  open: boolean;
  onToggle: () => void;
  /** The foldable thumbnails (`DeckPile`), which scroll alone. */
  slides: ReactNode;
}

/** The left column: a foldable thumbnails section. */
export default function PileColumn({ title, open, onToggle, slides }: PileColumnProps) {
  return (
    <div className="pile-column">
      <PileSection className="pile-thumbs" title={title} open={open} onToggle={onToggle}>
        {slides}
      </PileSection>
    </div>
  );
}
