/**
 * A deck tab's thumbnail pile, or a theme tab's specimens (one per layout, named after it).
 *
 * A pack tab has its own pile (`PackPile`). In a theme tab, no add, move or delete: the host
 * refuses them, the pile does not offer them.
 */
import type { PileSectionName, SlideSummary, Snapshot } from "./api";
import { pileMarks } from "./history";
import type { PileAxis } from "./pileSlot";
import { t } from "./i18n";
import { splitKind } from "./packOps";
import SlidePile from "./SlidePile";
import { canRemoveSlide } from "./slideTools";

/** The tab whose pile shows the slides, and the gestures it offers on a deck. */
interface Props {
  /** The tab's snapshot: slides, history, help. */
  snap: Snapshot;
  /** A theme tab: the specimen deck, named after its layouts and fixed. */
  themeTab: boolean;
  /** Slide montrée sur la scène. */
  current: string | null;
  /** URL de la miniature d'une slide à son empreinte ; `null` tant qu'elle se rend. */
  svg: (id: string, hash: string) => string | null;
  /** Montre une slide. */
  onSelect: (id: string) => void;
  /** Déplace une slide à la place `slot` (0 = avant la première). */
  onMove: (id: string, slot: number) => void;
  /** Ouvre le choix d'un layout à ajouter. */
  onAdd: () => void;
  /** Supprime une slide. */
  onRemove: (id: string) => void;
  /** The theme tab's column (`y`) or the strip under the stage (`x`). */
  axis: PileAxis;
}

/** The thumbnails' foldable section: layouts in a theme tab, else slides. */
export function pileSection(themeTab: boolean): PileSectionName {
  return themeTab ? "layouts" : "slides";
}

/** The layout name of each specimen slide (kind `namespace/name`). */
function layoutNames(slides: SlideSummary[]): Record<string, string> {
  return Object.fromEntries(slides.map((s) => [s.id, splitKind(s.kind)?.name ?? s.id]));
}

export default function DeckPile({ snap, themeTab, current, svg, onSelect, onMove, onAdd, onRemove, axis }: Props) {
  // History or help: the pile reads, it does not change; nor does a theme tab change the slides.
  const fixed = !!snap.history || snap.read_only || themeTab;
  const layouts = themeTab;
  return (
    <SlidePile
      slides={snap.slides}
      current={current}
      svg={svg}
      names={layouts ? layoutNames(snap.slides) : undefined}
      label={t(layouts ? "ui-pile-layouts" : "ui-pile-slides")}
      addLabel={t("ui-pile-add-slide")}
      marks={snap.history ? pileMarks(snap.history.changes) : undefined}
      onSelect={onSelect}
      onMove={fixed ? undefined : onMove}
      onAdd={fixed ? undefined : onAdd}
      removeLabel={t("ui-toolbar-remove-slide")}
      canRemove={canRemoveSlide(snap.slides.length)}
      onRemove={fixed ? undefined : onRemove}
      axis={axis}
    />
  );
}
