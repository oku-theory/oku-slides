/**
 * La carte de la cible, sous la scène : le panneau propre de l'élément sélectionné
 * (graphique ou tableau, choisi par `host_params`), sinon les paramètres de la cible
 * (`useTargetSettings`), sinon rien.
 * Hors vue, en lecture seule ou masquée depuis la barre d'outils, rien. En mode pack, la
 * cible d'une slide sans sélection est l'exemple de son layout.
 * Le tout dans la bande sous la scène (`.settings-band`), présente même sans carte quand
 * une carte peut venir.
 */
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { useRef } from "react";
import type { Element, HostParams, Op, SlideView, Surface } from "./api";
import BandDivider from "./BandDivider";
import { bandHeight } from "./bandSplit";
import { ChartPanel } from "./chart/ChartPanel";
import HostSettings from "./HostSettings";
import { ownPanel } from "./hostParams";
import { t } from "./i18n";
import type { View } from "./table/ShowAsButton";
import { TablePanel } from "./table/TablePanel";
import type { HostSettings as Target } from "./useHostSettings";

type Props = {
  view: SlideView | null;
  readOnly: boolean;
  /** L'élément sélectionné résolu dans la vue. */
  selected: Element | undefined;
  /** Paramètres d'hôte des widgets du deck (`Snapshot.host_params`) : choisissent le panneau propre. */
  hostParams: HostParams;
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
  /** Hauteur de la bande choisie au séparateur, en px ; `null` : celle de la feuille de style. */
  band: number | null;
  /** Retient une hauteur de bande (au relâcher, au clavier, ou `null` au double-clic). */
  onBand: (band: number | null) => void;
};

export function TargetPanel(props: Props) {
  const { view, readOnly, folded, band, onBand } = props;
  const box = useRef<HTMLDivElement>(null);
  const card = cardOf(props);
  // La bande sous la scène, de hauteur fixe (styles.css, ou choisie au séparateur) : posée dès que la
  // carte peut s'afficher, même vide le temps de la lire, pour que la slide ne change pas de taille à la
  // sélection (t-647).
  if (!card && (!view || readOnly || folded)) return null;
  // Pendant un glisser, la hauteur suit le pointeur sur la bande seule ; elle n'est retenue qu'au relâcher,
  // et reposée alors : retenue égale à celle gardée, React ne rendrait rien et la dernière montrée resterait.
  const show = (size: number) => box.current?.style.setProperty("--band-h", bandHeight(size));
  // Hauteur de la colonne (scène et bande) et de la bande.
  const measure = () => {
    const el = box.current;
    return { total: el?.parentElement?.clientHeight ?? 0, size: el?.offsetHeight ?? 0 };
  };
  const height = bandHeight(band);
  return (
    <div ref={box} className="settings-band" style={height === null ? undefined : ({ "--band-h": height } as React.CSSProperties)}>
      <BandDivider
        label={t("ui-divider-band")}
        measure={measure}
        onResize={(size, done) => {
          show(size);
          if (done) onBand(size);
        }}
        onReset={() => onBand(null)}
      />
      {card}
    </div>
  );
}

function cardOf({ view, readOnly, selected, hostParams, target, palette, onApply, onKind, onShowAs, onSurfaceAction, onError, folded }: Props) {
  if (!view || readOnly || folded) return null;
  // Données du graphique ou du tableau sélectionné : hors de la feuille, pour ne rien masquer.
  const own = ownPanel(hostParams, selected?.kind);
  if (selected && own?.panel === "chart")
    return <ChartPanel slide={view.slide.id} element={selected} param={own.param} palette={palette} onApply={onApply} onKind={onKind} onShowAs={onShowAs} />;
  if (selected && own?.panel === "table")
    return <TablePanel slide={view.slide.id} element={selected} param={own.param} onApply={onApply} onShowAs={onShowAs} />;
  if (!target.surface) return null;
  return <HostSettings surface={target.surface} outcome={target.outcome} onAction={onSurfaceAction} onError={onError} />;
}
