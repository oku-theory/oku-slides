// Le fil de l'agent : ses tours (surfaces A2UI de l'hôte) et le dock des surfaces vivantes.

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { lazy, Suspense, useCallback, useEffect, useRef, useState } from "react";
import type { AgentState, Surface } from "../api";
import { t } from "../i18n";
import { Icon } from "../icons";
import { AgentDock } from "./AgentDock";
import type { Routed } from "./surfaces";

// @a2ui ne se charge qu'avec la première surface (comme le panneau des modèles).
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
  onAction,
  onNewConversation,
  onError,
}: {
  state: AgentState;
  /** La surface qui reçoit les touches rapides (voir `AgentDock`). */
  keysTo?: string | null;
  onAction: (surface: Surface, action: A2uiClientAction) => void;
  onNewConversation: () => void;
  onError: (message: string) => void;
} & Pick<Routed, "turns" | "loose" | "dock">) {
  const scroller = useRef<HTMLDivElement>(null);
  const observer = useRef<ResizeObserver | null>(null);
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
  // Ref de rappel : elle suit la liste si React la recrée.
  const list = useCallback((el: HTMLOListElement | null) => {
    observer.current?.disconnect();
    observer.current = null;
    if (!el || typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(() => {
      if (pinnedRef.current) toBottom();
    });
    watch.observe(el);
    if (el.parentElement) watch.observe(el.parentElement);
    observer.current = watch;
  }, []);
  const empty = turns.length === 0 && loose.length === 0 && dock.length === 0;
  const view = (s: Surface) => (
    <Suspense fallback={null}>
      <SurfaceView surfaceId={s.id} messages={s.messages} onAction={(a) => onAction(s, a)} onError={onError} />
    </Suspense>
  );
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
        {/* Une seule région, persistante : chaque ajout (tour, surface, étape) est lu une fois ; rien n'est réannoncé. */}
        <ol ref={list} className="agent-turns" aria-live="polite" aria-relevant="additions">
          {turns.map(({ turn, surfaces }) => (
            <li key={turn} className="agent-turn">
              {surfaces.map((s) => (
                <div key={s.id} data-surface-id={s.id}>
                  {view(s)}
                </div>
              ))}
            </li>
          ))}
          {loose.map((s) => (
            <li key={s.id} className="agent-turn" data-surface-id={s.id}>
              {view(s)}
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
      <AgentDock surfaces={dock} view={view} keysTo={keysTo} />
    </section>
  );
}
