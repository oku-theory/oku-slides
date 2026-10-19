// Le panneau de l'agent d'un onglet : ses surfaces (réducteur, abonnement à l'hôte, resynchronisations,
// routage), son fil, ses touches rapides et ses consignes.
//
// Conteneur à part : un morceau de réponse (`surface_patch`) ne re-rend que lui et les places où il
// pose ses surfaces (`SlotView`), jamais `App`, la scène ni la pile. `App` ne reçoit que ce qu'il
// route : un état réduit à des valeurs simples (`onStatus`, appelé quand elles changent) et une
// poignée (`handle`) pour envoyer une consigne, une action de surface ou arrêter le tour.

import { isShortcut } from "./shortcut";
import { Suspense, lazy, useEffect, useImperativeHandle, useLayoutEffect, useMemo, useReducer, useRef } from "react";
import type { Ref } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { AgentUi, InterfaceChange, McpEvent, Surface } from "../api";
import type { PresentMode } from "../present";
import { agentApi } from "../api";
import { SlideThumbsContext, type SlideThumbs } from "../a2ui/slideThumbs";
import { AgentThread } from "./AgentThread";
import { subscribeHost } from "./events";
import type { QuickAction } from "./quick";
import { fromA2ui } from "./quick";
import { withRewriteParagraphs } from "./rewrite";
import type { ScopeView } from "./scope";
import type { Slot } from "./slot";
import { agentBusy, sendThenPrompt, suggestedPrompt, surfacesReducer, type Routed } from "./surfaces";
import { useAgentScope, type AgentScopeState } from "./useAgentScope";
import { useFollowScope } from "./useFollowScope";
import { answerTarget, quickTarget, useQuickKeys, type QuickTarget } from "./useQuickKeys";

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
  /** L'identifiant de la fiche posée, sinon `null` (la fenêtre de l'agent le lit dans le contexte). */
  sheetId: string | null;
  /** « L'agent répond ailleurs », quand aucune question ne recevrait la consigne ; sinon `null`. */
  notice: string | null;
  /** Un agent répond (`useAgentScope`) : une consigne peut partir. */
  ready: boolean;
}

export const IDLE_AGENT: AgentStatusView = { modal: false, busy: false, cancellable: false, sheet: null, sheetId: null, notice: null, ready: false };

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

/** D'où vient la portée : cette fenêtre la fixe (`drive`), ou elle suit celle de l'hôte (`follow`). */
export type ScopeSource = { kind: "drive"; view: ScopeView | null } | { kind: "follow" };

export interface AgentPanelProps {
  tab: string;
  agent: AgentApi;
  /** D'où vient la portée de l'agent ; `view` vaut `null` avant le premier instantané. */
  scope: ScopeSource;
  /** Le routage des surfaces, stable tant que la scène ne change pas. */
  route: (list: Surface[]) => Routed;
  /** Ce que voit l'utilisateur, joint aux consignes. */
  ui: AgentUi;
  /** Où les touches rapides agissent ; défaut : toutes. */
  /** Mode deck de l'onglet principal : l'hôte traite un mot-raccourci sans agent ni tour (faux dans la fenêtre de l'agent). */
  shortcuts: boolean;
  keyPlaces?: QuickTarget["kind"][];
  /** Un brouillon reçu de l'hôte. */
  onDraft?: (e: { text: string | null; origin: string | null }) => void;
  /** Le panneau des agents reçu de l'hôte (menu des réglages de la dictée, `useAgentPicker`). */
  onPicker?: (surface: Surface) => void;
  /** La disponibilité de l'agent reçue de l'hôte (`useAgentAvailability`) ; `null` : utilisable. */
  onAvailability?: (unusable: string | null) => void;
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
  /** L'agent lance la présentation (fenêtre de l'agent : rien, la principale s'en charge). */
  onPresent: (mode: PresentMode) => void;
  /** L'agent change l'interface (fenêtre de l'agent : rien, la principale s'en charge). */
  onInterface: (change: InterfaceChange) => void;
  /** L'agent ouvre l'historique (fenêtre de l'agent : rien, la principale s'en charge). */
  onOpenHistory: () => void;
  say: (text: string, error?: boolean) => void;
  /** Ouvre l'agent dans sa propre fenêtre : le bouton n'existe que si la fenêtre principale le donne. */
  onDetach?: () => void;
  /** Ramène l'agent dans la fenêtre principale : le bouton n'existe que dans la fenêtre de l'agent. */
  onAttach?: () => void;
  /** Le tour au sommet de la pile d'annulation du deck (`Snapshot.undo_turn`). */
  undoTurn?: number | null;
  /** Annule ce tour depuis son reçu ; sans rappel, aucun reçu ne l'offre. */
  onUndoTurn?: (turn: number) => void;
  /** Pour les tests : l'abonnement aux évènements de l'hôte. */
  subscribe?: typeof subscribeHost;
}

const ALL_PLACES: QuickTarget["kind"][] = ["sheet", "drawer", "modal", "dock"];
const NOOP = () => {};

/** Ce que lit l'abonnement, posé une fois par onglet : toujours la dernière version. */
interface Latest {
  props: AgentPanelProps;
  scopeState: AgentScopeState;
  onMcpFollow: (e: McpEvent) => void;
}

/** Le fil de l'agent, et ses surfaces posées dans les places de `App`. */
export function AgentPanel(props: AgentPanelProps) {
  const { tab, agent, keysOff, slots, say } = props;
  const [surfaces, onSurfaceEvent] = useReducer(surfacesReducer, []);
  // Un panneau ne change jamais de mode (monté par onglet et par fenêtre) ; un `useAgentScope`
  // sans vue ne rappelle jamais le fil.
  const driven = useAgentScope(agent, props.scope.kind === "drive" ? props.scope.view : null, onSurfaceEvent);
  const followed = useFollowScope(agent);
  const agentScopeState: AgentScopeState = props.scope.kind === "follow" ? followed : driven;
  const latest = useRef<Latest | null>(null);
  latest.current = { props, scopeState: agentScopeState, onMcpFollow: props.scope.kind === "follow" ? followed.onMcp : NOOP };
  const { scope, status: agentStatus, elsewhere, notice: elsewhereNotice, ready } = agentScopeState;
  const agentState = agentStatus.state;
  /** Listes entières reçues (`surfaces`, après un abonnement ou une reconnexion) : le fil ne les relit pas. */
  const [resyncs, countResync] = useReducer((n: number) => n + 1, 0);
  // Une fiche s'affiche sous son élément si celui-ci est sur la slide affichée,
  // sinon dans le fil ; le tiroir sous la scène ; la fenêtre par-dessus tout.
  const routed = useMemo(() => props.route(surfaces), [surfaces, props.route]);
  const { stage: sheet, drawer, modal, dock } = routed;
  const cancel = () => void agent.cancel().catch((err) => say(String(err), true));
  const subscribe = props.subscribe ?? subscribeHost;
  useEffect(
    () =>
      subscribe(tab, {
        onMcp: (e: McpEvent) => {
          onSurfaceEvent(e);
          if (e.kind === "surfaces") countResync();
          latest.current!.onMcpFollow(e);
          const p = latest.current!.props;
          if (e.kind === "changed") p.onChanged();
          if (e.kind === "focus") p.onFocus(e.slide, e.element);
          if (e.kind === "widget_touched") p.onWidgetTouched(e.name, e.removed);
          if (e.kind === "present") p.onPresent(e.mode);
          if (e.kind === "interface") p.onInterface(e.change);
          if (e.kind === "open_history") p.onOpenHistory();
        },
        onAgent: (e) => {
          latest.current!.scopeState.onAgentEvent(e);
          if (e.kind === "draft") latest.current!.props.onDraft?.({ text: e.text, origin: e.origin });
          if (e.kind === "picker") latest.current!.props.onPicker?.(e.surface);
          if (e.kind === "availability") latest.current!.props.onAvailability?.(e.unusable);
        },
        onError: (m) => console.warn(m),
        // Abonné ou reconnecté : l'hôte redonne les surfaces, et le deck est relu
        // (les évènements manqués pendant la coupure sont perdus). Sans outils
        // (pile absente), `resync` échoue : il n'y a alors pas de surface.
        onReady: () => {
          latest.current!.scopeState.onReady();
          latest.current!.props.onChanged();
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
  const keyed = !keysOff && keysTarget && (props.keyPlaces ?? ALL_PLACES).includes(keysTarget.kind) ? keysTarget : null;
  /** Un tour en cours qui n'attend pas l'utilisateur (aucune question ne reçoit la consigne) : ⌘⇧D et Échap l'arrêtent. */
  const busy = agentBusy(agentState, answerTarget(keysTarget));
  /** Une consigne pour l'agent, avec ce que voit l'utilisateur ; elle répond par défaut à la question qui reçoit les touches. */
  const promptAgent = (text: string, answer = answerTarget(keysTarget)): Promise<boolean> => {
    // Pendant un tour ailleurs, seule une réponse à une question part ; un mot-raccourci
    // n'ouvre jamais de tour en mode deck (l'hôte l'ouvre seul) : il part alors toujours.
    if (elsewhereNotice && answer === null && !(props.shortcuts && isShortcut(text))) {
      say(elsewhereNotice, true);
      return Promise.resolve(false);
    }
    return agent.prompt(scope, text, props.ui, answer).then(
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
  const sheetId = sheet?.id ?? null;
  const notice = answerTarget(keysTarget) === null ? elsewhereNotice : null;
  const { onStatus } = props;
  // Des valeurs simples seulement : une surface patchée est un nouvel objet à chaque morceau.
  const hasModal = !!modal;
  const cancellable = busy && !elsewhere;
  useLayoutEffect(
    () => {
      const sheetAt = sheetOwner === null ? null : sheetField === undefined ? { owner: sheetOwner } : { owner: sheetOwner, field: sheetField };
      onStatus({ modal: hasModal, busy, cancellable, sheet: sheetAt, sheetId, notice, ready });
    },
    [onStatus, hasModal, busy, cancellable, sheetOwner, sheetField, sheetId, notice, ready],
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
        onDetach={props.onDetach}
        onAttach={props.onAttach}
        onError={onError}
        undoTurn={props.undoTurn}
        onUndoTurn={props.onUndoTurn}
      />
    </SlideThumbsContext.Provider>
  );
}
