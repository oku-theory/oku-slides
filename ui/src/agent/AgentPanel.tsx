// Le panneau de l'agent d'un onglet : ses surfaces (réducteur, abonnement à l'hôte, resynchronisations,
// routage), son fil, ses touches rapides et ses consignes.
//
// Conteneur à part : un morceau de réponse (`surface_patch`) ne re-rend que lui et les places où il
// pose ses surfaces (`SlotView`), jamais `App`, la scène ni la pile. `App` ne reçoit que ce qu'il
// route : un état réduit à des valeurs simples (`onStatus`, appelé quand elles changent) et une
// poignée (`handle`) pour envoyer une consigne, une action de surface ou arrêter le tour.

import { Suspense, lazy, useEffect, useImperativeHandle, useLayoutEffect, useMemo, useReducer, useRef } from "react";
import type { Ref } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { FieldView, McpEvent, SlideView, Surface } from "../api";
import { agentApi } from "../api";
import { SlideThumbsContext, type SlideThumbs } from "../a2ui/slideThumbs";
import { anchorBox } from "../stageFields";
import { AgentThread } from "./AgentThread";
import { subscribeHost } from "./events";
import type { QuickAction } from "./quick";
import { fromA2ui } from "./quick";
import { withRewriteParagraphs } from "./rewrite";
import type { ScopeView } from "./scope";
import type { Slot } from "./slot";
import { agentBusy, routeSurfaces, sendThenPrompt, suggestedPrompt, surfacesReducer } from "./surfaces";
import { agentUi } from "./ui";
import { useAgentScope } from "./useAgentScope";
import { answerTarget, quickTarget, useQuickKeys } from "./useQuickKeys";

// Surfaces de l'agent : chargées avec @a2ui, à la première question.
const AgentSheet = lazy(() => import("./AgentSheet").then((m) => ({ default: m.AgentSheet })));
const VariantDrawer = lazy(() => import("./VariantDrawer").then((m) => ({ default: m.VariantDrawer })));
const ConfirmModal = lazy(() => import("./ConfirmModal").then((m) => ({ default: m.ConfirmModal })));

/** L'agent d'un onglet, tel que l'appelle le panneau. */
export type AgentApi = ReturnType<typeof agentApi>;

/** Ce que `App` lit de l'agent : des valeurs simples, transmises seulement quand elles changent. */
export interface AgentStatusView {
  /** Une fenêtre de l'agent est ouverte : rien derrière elle ne réagit au clavier. */
  modal: boolean;
  /** Un tour en cours qui n'attend pas l'utilisateur (`agentBusy`) : ⌘⇧D l'arrête, l'envoi attend. */
  busy: boolean;
  /** Échap arrête le tour : occupé, et pas dans une autre portée. */
  cancellable: boolean;
  /** L'ancre (élément ou champ) sous laquelle poser la fiche de la scène, sinon `null`. */
  sheet: { owner: string; field?: string } | null;
  /** « L'agent répond ailleurs », quand aucune question ne recevrait la consigne ; sinon `null`. */
  notice: string | null;
  /** Un agent répond (`useAgentScope`) : une consigne peut partir. */
  ready: boolean;
}

export const IDLE_AGENT: AgentStatusView = { modal: false, busy: false, cancellable: false, sheet: null, notice: null, ready: false };

/** Ce que `App` demande à l'agent. */
export interface AgentHandle {
  /**
   * Une consigne, avec ce que voit l'utilisateur ; elle répond par défaut à la question qui reçoit
   * les touches. Vrai si elle est partie (un refus ou un échec est signalé).
   */
  prompt: (text: string) => Promise<boolean>;
  /** Une action d'une surface (réglages de l'hôte d'abord, sinon l'agent). */
  onSurfaceAction: (s: Surface, a: A2uiClientAction) => void;
  /** Arrête le tour en cours. */
  cancel: () => void;
}

/** Les places, hors du fil, où le panneau pose ses surfaces. */
export interface AgentSlots {
  /** Fiche sous son élément, sur la scène. */
  sheet: Slot;
  /** Tiroir des variantes, sous la scène. */
  drawer: Slot;
  /** Fenêtre de confirmation, par-dessus tout. */
  modal: Slot;
}

export interface AgentPanelProps {
  tab: string;
  agent: AgentApi;
  /** La vue qui fixe la portée de l'agent ; `null` avant le premier instantané. */
  scopeView: ScopeView | null;
  /** La slide affichée (fiches ancrées) et ce que voit l'agent avec une consigne. */
  view: SlideView | null;
  current: string | null;
  selected: string | null;
  editField: FieldView | null;
  widget: string | null;
  /** Touches rapides coupées : édition d'un texte, pointage, widget actif. */
  keysOff: boolean;
  /** Le fil se montre (pas dans l'aide). */
  thread: boolean;
  thumbs: SlideThumbs;
  slots: AgentSlots;
  handle: Ref<AgentHandle>;
  /** Une action de réglages de l'hôte (modèle, cible) ; `true` si elle a été prise. */
  onHostAction: (s: Surface, a: A2uiClientAction) => boolean;
  onStatus: (status: AgentStatusView) => void;
  /** Le deck a pu changer (outils de l'agent, abonnement prêt) : à relire. */
  onChanged: () => void;
  onFocus: (slide: string, element: string | null) => void;
  onWidgetTouched: (name: string, removed: boolean) => void;
  say: (text: string, error?: boolean) => void;
  /** Pour les tests : l'abonnement aux évènements de l'hôte. */
  subscribe?: typeof subscribeHost;
}

/** Le fil de l'agent, et ses surfaces posées dans les places de `App`. */
export function AgentPanel(props: AgentPanelProps) {
  const { tab, agent, view, current, selected, editField, widget, keysOff, slots, say } = props;
  // Les derniers rappels reçus : l'abonnement est posé une fois par onglet.
  const latest = useRef(props);
  latest.current = props;
  const [surfaces, onSurfaceEvent] = useReducer(surfacesReducer, []);
  const agentScopeState = useAgentScope(agent, props.scopeView, onSurfaceEvent);
  const { scope, status: agentStatus, elsewhere, notice: elsewhereNotice, ready } = agentScopeState;
  const agentState = agentStatus.state;
  /** Listes entières reçues (`surfaces`, après un abonnement ou une reconnexion) : le fil ne les relit pas. */
  const [resyncs, countResync] = useReducer((n: number) => n + 1, 0);
  // Une fiche s'affiche sous son élément si celui-ci est sur la slide affichée,
  // sinon dans le fil ; le tiroir sous la scène ; la fenêtre par-dessus tout.
  const routed = useMemo(
    () =>
      routeSurfaces(
        surfaces,
        view && view.slide.id === current ? { slide: view.slide.id, has: (a) => anchorBox(view, a) !== null } : null,
      ),
    [surfaces, view, current],
  );
  const { stage: sheet, drawer, modal, dock } = routed;
  const cancel = () => void agent.cancel().catch((err) => say(String(err), true));
  const subscribe = props.subscribe ?? subscribeHost;
  useEffect(
    () =>
      subscribe(tab, {
        onMcp: (e: McpEvent) => {
          onSurfaceEvent(e);
          if (e.kind === "surfaces") countResync();
          if (e.kind === "changed") latest.current.onChanged();
          if (e.kind === "focus") latest.current.onFocus(e.slide, e.element);
          if (e.kind === "widget_touched") latest.current.onWidgetTouched(e.name, e.removed);
        },
        onAgent: agentScopeState.onAgentEvent,
        onError: (m) => console.warn(m),
        // Abonné ou reconnecté : l'hôte redonne les surfaces, et le deck est relu
        // (les évènements manqués pendant la coupure sont perdus). Sans outils
        // (pile absente), `resync` échoue : il n'y a alors pas de surface.
        onReady: () => {
          agentScopeState.onReady();
          latest.current.onChanged();
        },
      }),
    // S'abonner une fois par onglet : les rappels sont lus dans `latest`, `onReady` et `onAgentEvent` sont stables.
    [tab, agent, subscribe],
  );
  const sendAction = (s: Surface, a: QuickAction) => {
    const action = withRewriteParagraphs(s, a);
    return agent.surfaceAction(s.id, action.name, action.context).then(
      () => true,
      (err) => {
        say(String(err), true);
        return false;
      },
    );
  };
  const keysTarget = quickTarget({ modal, sheet, drawer, dock });
  /**
   * La surface qui reçoit les touches 1–4, Entrée, Échap et montre leur rappel ; aucune pendant
   * l'édition d'un texte, le pointage ou un widget actif. Une consigne répond toujours à `keysTarget`.
   */
  const keyed = !keysOff ? keysTarget : null;
  /** Un tour en cours qui n'attend pas l'utilisateur (aucune question ne reçoit la consigne) : ⌘⇧D et Échap l'arrêtent. */
  const busy = agentBusy(agentState, answerTarget(keysTarget));
  /** Une consigne pour l'agent, avec ce que voit l'utilisateur ; elle répond par défaut à la question qui reçoit les touches. */
  const promptAgent = (text: string, answer = answerTarget(keysTarget)): Promise<boolean> => {
    // Pendant un tour ailleurs, seule une réponse à une question part.
    if (elsewhereNotice && answer === null) {
      say(elsewhereNotice, true);
      return Promise.resolve(false);
    }
    return agent.prompt(scope, text, agentUi(current, selected, editField, widget), answer).then(
      () => true,
      (err) => {
        say(String(err), true);
        return false;
      },
    );
  };
  const onSurfaceAction = (s: Surface, a: A2uiClientAction) => {
    if (props.onHostAction(s, a)) return;
    const action = fromA2ui(a);
    if (elsewhereNotice && suggestedPrompt(action) !== null) {
      say(elsewhereNotice, true);
      return;
    }
    // Une suite cliquée part vers l'agent, une fois l'action acceptée : une nouvelle consigne, qui ne répond à aucune question.
    void sendThenPrompt(action, () => sendAction(s, action), promptAgent);
  };
  useQuickKeys(keyed, sendAction);
  useImperativeHandle(props.handle, () => ({ prompt: (text) => promptAgent(text), onSurfaceAction, cancel }));

  // `App` ne se re-rend que si l'une de ces valeurs change.
  const sheetOwner = sheet?.anchor?.owner ?? null;
  const sheetField = sheet?.anchor?.field;
  const notice = answerTarget(keysTarget) === null ? elsewhereNotice : null;
  const { onStatus } = props;
  // Des valeurs simples seulement : une surface patchée est un nouvel objet à chaque morceau.
  const hasModal = !!modal;
  const cancellable = busy && !elsewhere;
  useLayoutEffect(
    () => {
      const sheetAt = sheetOwner === null ? null : sheetField === undefined ? { owner: sheetOwner } : { owner: sheetOwner, field: sheetField };
      onStatus({ modal: hasModal, busy, cancellable, sheet: sheetAt, notice, ready });
    },
    [onStatus, hasModal, busy, cancellable, sheetOwner, sheetField, notice, ready],
  );

  // Les surfaces hors du fil, posées dans les places de `App` (vues `SlotView`).
  // Les rappels des surfaces posées : stables, ils lisent la dernière version (`latestSurface`).
  const onError = (m: string) => say(m, true);
  const latestSurface = useRef({ sendAction, onError });
  latestSurface.current = { sendAction, onError };
  const stable = useMemo(
    () => ({
      onAction: (s: Surface, a: QuickAction) => void latestSurface.current.sendAction(s, a),
      onError: (m: string) => latestSurface.current.onError(m),
    }),
    [],
  );
  const keysTo = keyed?.surface.id ?? null;
  // Reconstruites seulement quand une surface ou la cible des touches change.
  const nodes = useMemo(() => {
    const surfaceProps = (s: Surface) => ({ surface: s, hints: keysTo === s.id, ...stable });
    return {
      sheet: sheet?.anchor ? <Suspense fallback={null}><AgentSheet key={sheet.id} {...surfaceProps(sheet)} /></Suspense> : null,
      drawer: drawer ? <Suspense fallback={null}><VariantDrawer key={drawer.id} {...surfaceProps(drawer)} /></Suspense> : null,
      modal: modal ? <Suspense fallback={null}><ConfirmModal key={modal.id} {...surfaceProps(modal)} /></Suspense> : null,
    };
  }, [sheet, drawer, modal, keysTo, stable]);
  useLayoutEffect(() => {
    slots.sheet.set(nodes.sheet);
    slots.drawer.set(nodes.drawer);
    slots.modal.set(nodes.modal);
  }, [slots, nodes]);
  useLayoutEffect(
    () => () => {
      slots.sheet.set(null);
      slots.drawer.set(null);
      slots.modal.set(null);
    },
    [slots],
  );

  if (!props.thread) return null;
  return (
    <SlideThumbsContext.Provider value={props.thumbs}>
      <AgentThread
        state={elsewhere ? "idle" : agentState}
        turns={routed.turns}
        loose={routed.loose}
        dock={dock}
        keysTo={keysTo}
        resyncs={resyncs}
        onAction={onSurfaceAction}
        onNewConversation={() => agent.newConversation(scope).catch((err) => say(String(err), true))}
        onError={onError}
      />
    </SlideThumbsContext.Provider>
  );
}
