/**
 * La carte de la cible, sous la scène : le panneau d'un graphique
 * sélectionné, sinon les paramètres de la cible (`useTargetSettings`), sinon rien.
 * Hors vue, en lecture seule ou masquée depuis la barre d'outils, rien.
 */
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Element, Op, SlideView, Surface } from "./api";
import { ChartPanel } from "./chart/ChartPanel";
import HostSettings from "./HostSettings";
import type { HostSettings as Target } from "./useHostSettings";

type Props = {
  view: SlideView | null;
  readOnly: boolean;
  /** L'élément sélectionné résolu dans la vue. */
  selected: Element | undefined;
  target: Target;
  /** Palette des graphiques du thème (apparence). */
  palette: string[];
  onApply: (label: string, ops: Op[]) => void;
  onKind: (kind: string) => void;
  onSurfaceAction: (s: Surface, a: A2uiClientAction) => void;
  onError: (m: string) => void;
  /** Carte masquée depuis la barre d'outils (gardé par l'hôte). */
  folded: boolean;
};

export function TargetPanel({ view, readOnly, selected, target, palette, onApply, onKind, onSurfaceAction, onError, folded }: Props) {
  if (!view || readOnly || folded) return null;
  // Données du graphique sélectionné : hors de la feuille, pour ne rien masquer.
  if (selected?.kind === "chart")
    return <ChartPanel slide={view.slide.id} element={selected} palette={palette} onApply={onApply} onKind={onKind} />;
  if (!target.surface) return null;
  return (
    <HostSettings surface={target.surface} onAction={onSurfaceAction} outcome={target.outcome} onError={onError} />
  );
}
