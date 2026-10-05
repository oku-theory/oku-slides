/** Section repliable de la pile : un titre-bouton, des actions, un corps. */
import { useId, type ReactNode } from "react";
import { Icon } from "./icons";

export interface PileSectionProps {
  title: string;
  /** Nom de la section pour les lecteurs d'écran ; défaut : le titre. */
  label?: string;
  open: boolean;
  onToggle: () => void;
  /** À droite du titre, visibles même repliée (« + » des widgets). */
  actions?: ReactNode;
  className?: string;
  children: ReactNode;
}

export default function PileSection({ title, label = title, open, onToggle, actions, className, children }: PileSectionProps) {
  const body = useId();
  return (
    <section className={`pile-section${className ? ` ${className}` : ""}`} data-open={open} aria-label={label}>
      <div className="pile-section-head">
        <h3>
          <button className="pile-section-toggle" aria-expanded={open} aria-controls={body} onClick={onToggle}>
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
