// Le fil de l'agent : ses tours (surfaces A2UI de l'hôte) et le dock des surfaces vivantes.

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { QuickHintsContext } from "../a2ui/quickHints";
import type { AgentState, Surface } from "../api";
import { t } from "../i18n";
import { Icon } from "../icons";
import { AgentDock } from "./AgentDock";
import { useSpeech } from "./speech";
import type { Routed } from "./surfaces";

// @a2ui ne se charge qu'avec la première surface (comme les réglages du dock).
const SurfaceView = lazy(() => import("../a2ui/SurfaceView").then((m) => ({ default: m.SurfaceView })));

/** Clés écrites en entier : `i18n-keys.test.ts` cherche chaque clé en littéral. */
const STATE_KEY: Record<AgentState, string> = {
  idle: "ui-agent-state-idle",
  starting: "ui-agent-state-starting",
  working: "ui-agent-state-working",
};

/**
 * Le fil et son dock. Sans tour ni surface, rien de visible (classe `vacant`, 0 px) : les
 * régions annoncées (`aria-live`) restent montées, sans quoi le premier ajout ne serait pas lu.
 */
export function AgentThread({
  state,
  turns,
  loose,
  dock,
  keysTo,
  resyncs = 0,
  onAction,
  onNewConversation,
  onError,
}: {
  state: AgentState;
  /** Nombre de listes entières reçues de l'hôte (resynchronisations) : leur contenu n'est pas lu. */
  resyncs?: number;
  /** La surface qui reçoit les touches rapides (voir `AgentDock`). */
  keysTo?: string | null;
  onAction: (surface: Surface, action: A2uiClientAction) => void;
  onNewConversation: () => void;
  onError: (message: string) => void;
} & Pick<Routed, "turns" | "loose" | "dock">) {
  const scroller = useRef<HTMLDivElement>(null);
  const [pinned, setPinned] = useState(true);
  const pinnedRef = useRef(pinned);
  /** Dernière position lue : distingue un geste vers le haut d'un retour en bas programmé. */
  const lastTop = useRef(0);
  pinnedRef.current = pinned;
  const lastTurn = turns.at(-1)?.turn;
  const toBottom = () => {
    const el = scroller.current;
    if (el) el.scrollTop = el.scrollHeight;
  };
  // Un nouveau tour se suit à nouveau, même si l'utilisateur était remonté (la référence d'abord :
  // l'observateur ci-dessous la lit avant le prochain rendu).
  // eslint-disable-next-line react-hooks/exhaustive-deps -- déclenché par le numéro du dernier tour
  useEffect(() => {
    pinnedRef.current = true;
    setPinned(true);
    toBottom();
  }, [lastTurn]);
  // Le contenu A2UI grandit sans re-rendre ce composant (signaux internes) : on observe la taille
  // de la liste entière, et celle de la zone qui défile (elle rétrécit quand le dock grandit).
  // Les deux sont toujours montées : une fois suffit.
  const list = useRef<HTMLOListElement>(null);
  useEffect(() => {
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(() => {
      const el = scroller.current;
      if (el && pinnedRef.current) el.scrollTop = el.scrollHeight;
    });
    for (const el of [list.current, scroller.current]) if (el) watch.observe(el);
    return () => watch.disconnect();
  }, []);
  // Ce que lit la région annoncée : le texte nouveau des surfaces du fil (`speech.ts`).
  const threadSurfaces = useMemo(() => [...turns.flatMap((x) => x.surfaces), ...loose], [turns, loose]);
  const spoken = useSpeech(threadSurfaces, dock, resyncs);
  const empty = turns.length === 0 && loose.length === 0 && dock.length === 0;
  const view = (s: Surface) => (
    <Suspense fallback={null}>
      <SurfaceView surfaceId={s.id} messages={s.messages} onAction={(a) => onAction(s, a)} onError={onError} />
    </Suspense>
  );
  /** Une surface du fil (tour, fiche hors de la slide) : chiffres 1–4 seulement si elle reçoit les touches. */
  const threadView = (s: Surface) => <QuickHintsContext.Provider value={s.id === keysTo}>{view(s)}</QuickHintsContext.Provider>;
  return (
    <section className={`agent-thread${empty ? " vacant" : ""}`} aria-label={t("ui-agent-thread")}>
      {!empty && (
        <header className="agent-thread-head">
          <span className="caption">{t(STATE_KEY[state])}</span>
          <button type="button" className="token ghost agent-new" aria-label={t("ui-agent-new-conversation")} title={t("ui-agent-new-conversation")} onClick={onNewConversation}>{Icon.newChat}</button>
        </header>
      )}
      <div
        ref={scroller}
        className="agent-scroller"
        onScroll={(e) => {
          // En bas : suivi. Détaché seulement par un geste vers le haut : un évènement de notre
          // propre retour en bas, lu après que le dock a encore grandi, n'est pas un geste.
          const el = e.currentTarget;
          const up = el.scrollTop < lastTop.current;
          lastTop.current = el.scrollTop;
          if (el.scrollHeight - el.scrollTop - el.clientHeight < 24) setPinned(true);
          else if (up) setPinned(false);
        }}
      >
        <ol ref={list} className="agent-turns">
          {turns.map(({ turn, surfaces }) => (
            <li key={turn} className="agent-turn">
              {surfaces.map((s) => (
                <div key={s.id} data-surface-id={s.id}>
                  {threadView(s)}
                </div>
              ))}
            </li>
          ))}
          {loose.map((s) => (
            <li key={s.id} className="agent-turn" data-surface-id={s.id}>
              {threadView(s)}
            </li>
          ))}
        </ol>
      </div>
      {!empty && !pinned && state === "working" && (
        <button
          type="button"
          className="token agent-jump"
          onClick={() => {
            pinnedRef.current = true;
            setPinned(true);
            toBottom();
          }}
        >
          {t("ui-agent-jump")}
        </button>
      )}
      {/* Une seule région, persistante et masquée : le texte nouveau de chaque surface est lu une
          fois ; une surface reconstruite à l'identique (resynchronisation, reçu sans ses suites) ne l'est pas. */}
      <div className="agent-announce" aria-live="polite" aria-relevant="additions">
        {spoken.map((x) => (
          <p key={x.key}>{x.text}</p>
        ))}
      </div>
      <AgentDock surfaces={dock} view={view} keysTo={keysTo} />
    </section>
  );
}
