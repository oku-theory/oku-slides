// Le dock : les surfaces vivantes du fil (question en attente, réglages), au-dessus du micro.

import type { ReactNode } from "react";
import { useState } from "react";
import { Toggle } from "../a2ui/Toggle";
import type { Surface } from "../api";
import { t } from "../i18n";
import { isQuestion } from "./surfaces";

/**
 * Les surfaces vivantes ; rien (0 px) sans elles. Quand une question attend,
 * les réglages se replient sur une ligne qui les rouvre : la question reste à l'écran.
 * Ce choix vaut pour une question : la suivante les replie à nouveau.
 */
export function AgentDock({ surfaces, view }: { surfaces: Surface[]; view: (s: Surface) => ReactNode }) {
  const question = surfaces.find(isQuestion)?.id ?? null;
  // L'ouverture des réglages est mémorisée pour une question donnée.
  const [openFor, setOpenFor] = useState<string | null>(null);
  if (surfaces.length === 0) return null;
  const open = openFor !== null && openFor === question;
  return (
    <section className="agent-dock agent-keys" aria-label={t("ui-agent-dock")} aria-live="polite">
      {surfaces.map((s) => {
        const settings = question !== null && !isQuestion(s);
        const folded = settings && !open;
        return (
          <div key={s.id} className={`agent-dock-item${isQuestion(s) ? " ask" : ""}`} data-surface-id={s.id}>
            {settings && (
              <Toggle open={!folded} onToggle={() => setOpenFor(open ? null : question)} className="a2ui-hint">
                {t("ui-agent-dock-settings")}
              </Toggle>
            )}
            {!folded && view(s)}
          </div>
        );
      })}
    </section>
  );
}
