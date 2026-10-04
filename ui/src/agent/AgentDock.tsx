// Le dock : les surfaces vivantes du fil (question en attente, réglages), au-dessus du micro.

import type { ReactNode } from "react";
import { useState } from "react";
import { QuickHintsContext } from "../a2ui/quickHints";
import { Toggle } from "../a2ui/Toggle";
import type { Surface } from "../api";
import { t } from "../i18n";
import { isQuestion } from "./surfaces";

/**
 * Les surfaces vivantes ; rien de visible (0 px) sans elles, mais la région annoncée reste
 * montée : une question qui arrive est lue. Quand une question attend,
 * les réglages se replient sur une ligne qui les rouvre : la question reste à l'écran.
 * Ce choix vaut pour une question : la suivante les replie à nouveau.
 */
export function AgentDock({
  surfaces,
  view,
  keysTo,
}: {
  surfaces: Surface[];
  view: (s: Surface) => ReactNode;
  /** La surface qui reçoit les touches rapides (`quickTarget`) ; par défaut la dernière question, au plus près du micro. */
  keysTo?: string | null;
}) {
  const question = surfaces.find(isQuestion)?.id ?? null;
  const keyed = keysTo === undefined ? (surfaces.filter(isQuestion).at(-1)?.id ?? null) : keysTo;
  // L'ouverture des réglages est mémorisée pour une question donnée.
  const [openFor, setOpenFor] = useState<string | null>(null);
  const open = openFor !== null && openFor === question;
  return (
    <section
      className={`agent-dock agent-keys${surfaces.length === 0 ? " vacant" : ""}`}
      aria-label={t("ui-agent-dock")}
      aria-live="polite"
      aria-relevant="additions"
    >
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
            {/* Les chiffres 1–4 seulement sur la question qui reçoit les touches. */}
            {!folded && <QuickHintsContext.Provider value={s.id === keyed}>{view(s)}</QuickHintsContext.Provider>}
          </div>
        );
      })}
    </section>
  );
}
