/**
 * La carte de la cible, sous la scène : le panneau d'un graphique ou d'un tableau
 * sélectionné, sinon les paramètres de la cible (`useTargetSettings`), sinon rien.
 * Hors vue, en lecture seule ou masquée depuis la barre d'outils, rien. Les réglages du
 * modèle (`lead`, mode modèles) s'affichent toujours, en tête de la fiche de la cible.
 * Le tout dans la bande sous la scène (`.settings-band`), présente même sans carte quand
 * une carte peut venir.
 */
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Element, Op, SlideView, Surface } from "./api";
import { ChartPanel } from "./chart/ChartPanel";
import HostSettings, { type SettingsSection } from "./HostSettings";
import type { View } from "./table/ShowAsButton";
import { TablePanel } from "./table/TablePanel";
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
  onShowAs: (view: View) => void;
  onSurfaceAction: (s: Surface, a: A2uiClientAction) => void;
  onError: (m: string) => void;
  /** Carte masquée depuis la barre d'outils (gardé par l'hôte). */
  folded: boolean;
  /** Réglages du modèle affiché (mode modèles), réunis en tête de la même fiche. */
  lead?: SettingsSection;
};

export function TargetPanel(props: Props) {
  const { view, readOnly, folded } = props;
  const card = cardOf(props);
  // La bande sous la scène, de hauteur fixe (styles.css) : posée dès que la carte peut s'afficher,
  // même vide le temps de la lire, pour que la slide ne change pas de taille à la sélection (t-647).
  if (!card && (!view || readOnly || folded)) return null;
  return <div className="settings-band">{card}</div>;
}

function cardOf({ view, readOnly, selected, target, palette, onApply, onKind, onShowAs, onSurfaceAction, onError, folded, lead }: Props) {
  const alone = lead ? <HostSettings {...lead} onAction={onSurfaceAction} onError={onError} /> : null;
  if (!view || readOnly || folded) return alone;
  // Données du graphique sélectionné : hors de la feuille, pour ne rien masquer.
  // Les sortes ci-dessous doivent rester celles de OWN_PANEL (ownPanel.ts).
  if (selected?.kind === "chart")
    return (
      <>
        {alone}
        <ChartPanel slide={view.slide.id} element={selected} palette={palette} onApply={onApply} onKind={onKind} onShowAs={onShowAs} />
      </>
    );
  if (selected?.kind === "table")
    return (
      <>
        {alone}
        <TablePanel slide={view.slide.id} element={selected} onApply={onApply} onShowAs={onShowAs} />
      </>
    );
  if (!target.surface) return alone;
  return (
    <HostSettings surface={target.surface} outcome={target.outcome} lead={lead} onAction={onSurfaceAction} onError={onError} />
  );
}
