/** Section repliable de la pile : un titre-bouton, des actions, un corps. */
import { useId, type ReactNode, type Ref } from "react";
import { Icon } from "./icons";

export interface PileSectionProps {
  title: string;
  /** Nom de la section pour les lecteurs d'écran ; défaut : le titre. */
  label?: string;
  open: boolean;
  onToggle: () => void;
  /** À droite du titre, visibles même repliée (« + » des widgets). */
  actions?: ReactNode;
  /** Bouton du titre, pour y porter le focus. */
  toggleRef?: Ref<HTMLButtonElement>;
  className?: string;
  children: ReactNode;
}

export default function PileSection({ title, label = title, open, onToggle, actions, toggleRef, className, children }: PileSectionProps) {
  const body = useId();
  return (
    <section className={`pile-section${className ? ` ${className}` : ""}`} data-open={open} aria-label={label}>
      <div className="pile-section-head">
        <h3>
          <button ref={toggleRef} className="pile-section-toggle" aria-expanded={open} aria-controls={body} onClick={onToggle}>
            <span className="pile-section-chevron">{Icon.chevron}</span>
            {title}
          </button>
        </h3>
        {actions}
      </div>
      {open && <div id={body} className="pile-section-body">{children}</div>}
    </section>
  );
}
