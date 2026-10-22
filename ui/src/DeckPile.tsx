/**
 * La pile des miniatures d'un onglet : les slides du deck, ou, en mode pack, les slides du deck
 * de travail d'un pack de layouts (une par layout, nommée d'après lui). En mode pack, ni ajout,
 * ni déplacement, ni suppression : l'hôte les refuse, la pile ne les offre pas.
 */
import type { PileSectionName, SlideSummary, Snapshot } from "./api";
import { pileMarks } from "./history";
import { t } from "./i18n";
import { splitKind } from "./packOps";
import SlidePile from "./SlidePile";
import { canRemoveSlide } from "./slideTools";

/** L'onglet dont la pile montre les slides, et les gestes qu'elle offre hors mode pack. */
interface Props {
  /** Instantané de l'onglet : slides, mode pack, historique, aide. */
  snap: Snapshot;
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
}

/** Section repliable des miniatures : slides, layouts d'un pack de layouts, aucune pour un pack de widgets. */
export function pileSection(snap: Snapshot): PileSectionName | null {
  if (!snap.editing) return "slides";
  return snap.editing.kind === "layouts" ? "layouts" : null;
}

/** Le nom du layout de chaque slide du deck de travail (genre `espace/nom`). */
function layoutNames(slides: SlideSummary[]): Record<string, string> {
  return Object.fromEntries(slides.map((s) => [s.id, splitKind(s.kind)?.name ?? s.id]));
}

export default function DeckPile({ snap, current, svg, onSelect, onMove, onAdd, onRemove }: Props) {
  // Historique ou aide : la pile se lit, elle ne se modifie pas ; le mode pack ne change pas les slides.
  const fixed = !!snap.history || snap.read_only || !!snap.editing;
  return (
    <SlidePile
      slides={snap.slides}
      current={current}
      svg={svg}
      names={snap.editing ? layoutNames(snap.slides) : undefined}
      label={t(snap.editing ? "ui-pile-layouts" : "ui-pile-slides")}
      addLabel={t("ui-pile-add-slide")}
      marks={snap.history ? pileMarks(snap.history.changes) : undefined}
      onSelect={onSelect}
      onMove={fixed ? undefined : onMove}
      onAdd={fixed ? undefined : onAdd}
      removeLabel={t("ui-toolbar-remove-slide")}
      canRemove={canRemoveSlide(snap.slides.length)}
      onRemove={fixed ? undefined : onRemove}
    />
  );
}
