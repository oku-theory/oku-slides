// Séparateur horizontal entre la scène et la bande de réglages dessous : on le
// glisse pour régler la hauteur de la bande, on le double-clique pour revenir
// à la hauteur par défaut.

import { bandBounds } from "./bandSplit";
import Divider from "./Divider";

interface Props {
  label: string;
  /** Hauteur de la colonne (scène et bande) et hauteur de la bande, lues au début d'un geste. */
  measure: () => { total: number; size: number };
  /** Hauteur bornée : en direct pendant le glisser, puis `done` au relâcher ou au clavier. */
  onResize: (size: number, done: boolean) => void;
  /** Double-clic : hauteur par défaut. */
  onReset: () => void;
}

/** Séparateur de la hauteur de la bande ; elle est dessous, monter l'agrandit. */
export default function BandDivider({ label, measure, onResize, onReset }: Props) {
  return (
    <Divider
      className="band-divider"
      label={label}
      axis="y"
      grow={-1}
      measure={() => {
        const m = measure();
        // Rien de mesuré : rien à régler.
        return m.total > 0 ? { size: m.size, bounds: bandBounds(m.total) } : null;
      }}
      onResize={onResize}
      onReset={onReset}
    />
  );
}
