// Le fil de l'agent dans le panneau de droite : consignes, outils, plan,
// réponses, et les surfaces qui y vivent (questions sans ancre, paramètres).

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { lazy, Suspense } from "react";
import type { AgentState, Place, StopReason, Surface } from "../api";
import { t } from "../i18n";
import type { Thread } from "./thread";
import { Markdown } from "./Markdown";
import { threadItems } from "./thread";

// @a2ui ne se charge qu'avec la première surface (comme le panneau des modèles).
const SurfaceView = lazy(() => import("../a2ui/SurfaceView").then((m) => ({ default: m.SurfaceView })));

/** Marque d'un outil ou d'une étape du plan, selon son statut ACP. */
const TOOL_MARK: Record<string, string> = { pending: "☐", in_progress: "◌", completed: "☑", failed: "✕" };

/** Classe CSS d'un statut venu de l'hôte : seules les valeurs connues passent. */
const STATUS_CLASS = new Set(["pending", "in_progress", "completed", "failed"]);
const statusClass = (status: string) => (STATUS_CLASS.has(status) ? status : "other");

/** Places rendues dans le fil ; une autre place ne donne pas sa valeur en classe. */
const PLACE_CLASS = new Set<Place>(["thread", "postit"]);

/** Une fin de tour dite dans la langue de l'utilisateur ; une raison inconnue est citée telle quelle. */
function stopText(stop: StopReason): string {
  switch (stop) {
    case "cancelled":
      return t("ui-agent-stopped");
    case "max_tokens":
      return t("ui-agent-stop-max-tokens");
    case "max_turn_requests":
      return t("ui-agent-stop-max-turns");
    case "refusal":
      return t("ui-agent-stop-refusal");
    default:
      return t("ui-agent-ended", { stop });
  }
}

/** Clés écrites en entier : `i18n-keys.test.ts` cherche chaque clé en littéral. */
const STATE_KEY: Record<AgentState, string> = {
  idle: "ui-agent-state-idle",
  starting: "ui-agent-state-starting",
  working: "ui-agent-state-working",
};

/** Le fil de l'agent et ses surfaces ; rien tant qu'il est vide. */
export function AgentThread({
  thread,
  surfaces,
  onAction,
  onNewConversation,
  onError,
}: {
  thread: Thread;
  surfaces: Surface[];
  onAction: (surface: Surface, action: A2uiClientAction) => void;
  onNewConversation: () => void;
  onError: (message: string) => void;
}) {
  if (thread.entries.length === 0 && surfaces.length === 0) return null;
  return (
    <section className="agent-thread" aria-label={t("ui-agent-thread")}>
      <header className="agent-thread-head">
        <span className="caption">{t(STATE_KEY[thread.state])}</span>
        <button type="button" className="linkish" onClick={onNewConversation}>{t("ui-agent-new-conversation")}</button>
      </header>
      <ol className="agent-entries" aria-live="polite">
        {threadItems(thread.entries).map((e) => (
          <li key={e.at} className={`agent-entry ${e.kind}`}>
            {e.kind === "user" && <span className="you postit">{e.text}</span>}
            {e.kind === "agent" && <div className="msg md"><Markdown text={e.text} /></div>}
            {e.kind === "tools" && (
              // Fermé par défaut ; l'état ouvert tient tant que la clé (`at`) ne change pas.
              <details className="tools">
                <summary className="caption">
                  {TOOL_MARK[e.tools.at(-1)!.status] ?? "·"} {t("ui-agent-tools", { count: e.tools.length })}
                </summary>
                <ul className="tool-list">
                  {e.tools.map((x) => <li key={x.id} className="tool">{TOOL_MARK[x.status] ?? "·"} {x.title}</li>)}
                </ul>
              </details>
            )}
            {e.kind === "plan" && (
              <ul className="plan">
                {e.entries.map((p, j) => <li key={j} className={statusClass(p.status)}>{TOOL_MARK[p.status] ?? "☐"} {p.content}</li>)}
              </ul>
            )}
            {e.kind === "error" && <span className="error" role="alert">{e.text}</span>}
            {e.kind === "stop" && <span className="caption">{stopText(e.stop)}</span>}
          </li>
        ))}
      </ol>
      <Suspense fallback={null}>
        {surfaces.map((s) => (
          <div key={s.id} className={`agent-surface${PLACE_CLASS.has(s.place) ? ` ${s.place}` : ""}`}>
            <SurfaceView surfaceId={s.id} messages={s.messages} onAction={(a) => onAction(s, a)} onError={onError} />
          </div>
        ))}
      </Suspense>
    </section>
  );
}
