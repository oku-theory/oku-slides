/**
 * La carte de la cible, sous la scène : le panneau d'un graphique
 * sélectionné, sinon les paramètres de la cible (`useTargetSettings`), sinon rien.
 * Hors vue, en lecture seule ou masquée depuis la barre d'outils, rien. Les réglages du
 * modèle (`lead`, mode modèles) s'affichent toujours, en tête de la fiche de la cible.
 */
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Element, Op, SlideView, Surface } from "./api";
import { ChartPanel } from "./chart/ChartPanel";
import HostSettings, { type SettingsSection } from "./HostSettings";
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
  /** Réglages du modèle affiché (mode modèles), réunis en tête de la même fiche. */
  lead?: SettingsSection;
};

export function TargetPanel({ view, readOnly, selected, target, palette, onApply, onKind, onSurfaceAction, onError, folded, lead }: Props) {
  const alone = lead ? <HostSettings {...lead} onAction={onSurfaceAction} onError={onError} /> : null;
  if (!view || readOnly || folded) return alone;
  // Données du graphique sélectionné : hors de la feuille, pour ne rien masquer.
  if (selected?.kind === "chart")
    return (
      <>
        {alone}
        <ChartPanel slide={view.slide.id} element={selected} palette={palette} onApply={onApply} onKind={onKind} />
      </>
    );
  if (!target.surface) return alone;
  return (
    <HostSettings surface={target.surface} outcome={target.outcome} lead={lead} onAction={onSurfaceAction} onError={onError} />
  );
}
