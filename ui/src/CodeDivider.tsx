// Séparateur vertical entre les réglages d'un widget et son code, à leur droite :
// on le glisse pour régler la largeur du code, on le double-clique pour
// revenir à la proportion par défaut.

import { codeBounds } from "./codeSplit";
import Divider from "./Divider";

interface Props {
  label: string;
  /** Largeur du sous-onglet et largeur du code, lues au début d'un geste. */
  measure: () => { total: number; size: number };
  /** Largeur bornée : en direct pendant le glisser, puis `done` au relâcher ou au clavier. */
  onResize: (size: number, done: boolean) => void;
  /** Double-clic : proportion par défaut. */
  onReset: () => void;
}

/** Séparateur de la largeur du code ; le code est en fin de ligne, avancer vers lui le réduit. */
export default function CodeDivider({ label, measure, onResize, onReset }: Props) {
  return (
    <Divider
      className="code-divider"
      label={label}
      axis="x"
      grow={-1}
      measure={() => {
        const m = measure();
        // Pas de code mesuré : rien à régler.
        return m.total > 0 ? { size: m.size, bounds: codeBounds(m.total) } : null;
      }}
      onResize={onResize}
      onReset={onReset}
    />
  );
}
