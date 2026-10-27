/**
 * La pile des miniatures d'un onglet : les slides du deck, ou, en mode pack, les slides du deck
 * de travail d'un pack de layouts (une par layout, nommée d'après lui), or a theme tab's
 * specimens (likewise). In pack mode and in a theme tab, no add, move or delete: the host
 * refuses them, the pile does not offer them.
 */
import type { PileSectionName, SlideSummary, Snapshot } from "./api";
import { pileMarks } from "./history";
import type { PileAxis } from "./pileSlot";
import { t } from "./i18n";
import { splitKind } from "./packOps";
import SlidePile from "./SlidePile";
import { canRemoveSlide } from "./slideTools";

/** L'onglet dont la pile montre les slides, et les gestes qu'elle offre hors mode pack. */
interface Props {
  /** Instantané de l'onglet : slides, mode pack, historique, aide. */
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
  /** Colonne du mode pack (`y`) ou bande sous la scène (`x`). */
  axis: PileAxis;
}

/** The thumbnails' foldable section: slides, layouts (layout pack, theme tab), none for a widget pack. */
export function pileSection(snap: Snapshot, themeTab: boolean): PileSectionName | null {
  if (themeTab) return "layouts";
  if (!snap.editing) return "slides";
  return snap.editing.kind === "layouts" ? "layouts" : null;
}

/** Le nom du layout de chaque slide du deck de travail (genre `espace/nom`). */
function layoutNames(slides: SlideSummary[]): Record<string, string> {
  return Object.fromEntries(slides.map((s) => [s.id, splitKind(s.kind)?.name ?? s.id]));
}

export default function DeckPile({ snap, themeTab, current, svg, onSelect, onMove, onAdd, onRemove, axis }: Props) {
  // History or help: the pile reads, it does not change; neither pack mode nor a theme tab changes the slides.
  const fixed = !!snap.history || snap.read_only || !!snap.editing || themeTab;
  const layouts = !!snap.editing || themeTab;
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
