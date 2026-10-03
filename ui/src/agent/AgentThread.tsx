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

/** Le fil et son dock ; rien tant qu'il n'y a ni tour ni surface. */
export function AgentThread({
  state,
  turns,
  loose,
  dock,
  onAction,
  onNewConversation,
  onError,
}: {
  state: AgentState;
  onAction: (surface: Surface, action: A2uiClientAction) => void;
  onNewConversation: () => void;
  onError: (message: string) => void;
} & Pick<Routed, "turns" | "loose" | "dock">) {
  const scroller = useRef<HTMLDivElement>(null);
  const observer = useRef<ResizeObserver | null>(null);
  const [pinned, setPinned] = useState(true);
  const pinnedRef = useRef(pinned);
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
  // de la liste entière. Ref de rappel : la liste n'existe pas tant que le fil est vide (le composant
  // rend `null`), un effet sans dépendances ne la verrait jamais.
  const list = useCallback((el: HTMLOListElement | null) => {
    observer.current?.disconnect();
    observer.current = null;
    if (!el || typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(() => {
      if (pinnedRef.current) toBottom();
    });
    watch.observe(el);
    observer.current = watch;
  }, []);
  if (turns.length === 0 && loose.length === 0 && dock.length === 0) return null;
  const view = (s: Surface) => (
    <Suspense fallback={null}>
      <SurfaceView surfaceId={s.id} messages={s.messages} onAction={(a) => onAction(s, a)} onError={onError} />
    </Suspense>
  );
  return (
    <section className="agent-thread" aria-label={t("ui-agent-thread")}>
      <header className="agent-thread-head">
        <span className="caption">{t(STATE_KEY[state])}</span>
        <button type="button" className="token ghost agent-new" aria-label={t("ui-agent-new-conversation")} title={t("ui-agent-new-conversation")} onClick={onNewConversation}>{Icon.newChat}</button>
      </header>
      <div
        ref={scroller}
        className="agent-scroller"
        onScroll={(e) => {
          const el = e.currentTarget;
          setPinned(el.scrollHeight - el.scrollTop - el.clientHeight < 24);
        }}
      >
        <ol ref={list} className="agent-turns">
          {turns.map(({ turn, surfaces }) => (
            // Seul le dernier tour est annoncé : les anciens ne changent plus.
            <li key={turn} className="agent-turn" aria-live={turn === lastTurn ? "polite" : undefined}>
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
      {!pinned && state === "working" && (
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
      <AgentDock surfaces={dock} view={view} />
    </section>
  );
}
