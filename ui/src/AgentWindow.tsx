// La fenêtre de l'agent : le fil, le brouillon et la dictée de l'onglet au premier plan.
// Ce qui n'existe que dans la fenêtre principale (où écrire, contexte de l'interface) passe par
// le canal `agent-link` ; tout le reste (fil, brouillon, portée) passe par l'hôte.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "./icons";
import { agentApi, inTauri, isMac, listenTauri, tabApi, workspace, type AgentUi, type Snapshot, type TabApi, type WorkspaceView } from "./api";
import { newest } from "./latest";
import { t } from "./i18n";
import { debounced } from "./debounce";
import { deckVocabulary } from "./draft";
import { useSlideThumbs } from "./useSlideThumbs";
import { useToast } from "./useToast";
import Dictation from "./Dictation";
import { DictationDraft } from "./DictationDraft";
import { AgentPanel, IDLE_AGENT, type AgentApi, type AgentHandle, type ScopeSource } from "./agent/AgentPanel";
import { draftActions } from "./agent/draftActions";
import { subscribeHost } from "./agent/events";
import { insertWithAck, linkTransport, onlyFor, type LinkContext, type LinkShortcut, type LinkTransport } from "./agent/link";
import { createSlot } from "./agent/slot";
import { routeStaged } from "./agent/surfaces";
import { agentTarget } from "./agent/target";
import { useDraft } from "./agent/useDraft";
import type { DictationMode } from "./agent/keys";
import type { QuickTarget } from "./agent/useQuickKeys";

/** Numéro du prochain raccourci relayé : jamais remis à zéro, même quand l'onglet suivi change. */
let remoteSeq = 0;
const FOLLOW: ScopeSource = { kind: "follow" };
const DOCK_ONLY: QuickTarget["kind"][] = ["dock"];
const NO_UI: AgentUi = { slide: null, element: null, field: null, editing: false, widget: null };
const NOOP = () => {};
/** Délai avant de relire l'instantané quand le contexte change (vocabulaire, miniatures). */
const REREAD_MS = 500;

/** La barre de saisie de la fenêtre (macOS, barre de titre fondue) : sans elle, elle ne se déplace pas. */
const TitleBar = () => (inTauri && isMac ? <div className="titlebar" data-tauri-drag-region /> : null);

/** Ce dont la fenêtre de l'agent dépend hors de la page : injectable pour les tests. */
export interface AgentWindowHost {
  agent: AgentApi;
  api: TabApi;
  subscribe: typeof subscribeHost;
  /** Met la fenêtre de l'agent devant (⌘K relayé). */
  focusSelf: () => void;
  /** La fermeture système demandée (`close-requested`) ; renvoie de quoi arrêter. */
  onCloseRequested: (cb: () => void) => () => void;
  attach: () => Promise<unknown>;
}

/** L'hôte réel d'un onglet. */
function tauriHost(tab: string): AgentWindowHost {
  return {
    agent: agentApi(tab),
    api: tabApi(tab),
    subscribe: subscribeHost,
    attach: workspace.attachAgent,
    focusSelf: () => {
      if (!inTauri) return;
      void import("@tauri-apps/api/window")
        .then(({ getCurrentWindow }) => getCurrentWindow().setFocus())
        .catch((e) => console.warn("fenêtre de l'agent", String(e)));
    },
    onCloseRequested,
  };
}

/** La fermeture système de cette fenêtre (`close-requested`), comme `Workspace.tsx` ; sans effet hors Tauri. */
function onCloseRequested(cb: () => void): () => void {
  if (!inTauri) return NOOP;
  return listenTauri(async () => {
    const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    return getCurrentWebviewWindow().listen("close-requested", () => cb());
  });
}

/** Ce que la page entière demande à l'hôte : la fermeture système et le rattachement. */
export type AgentWindowClose = Pick<AgentWindowHost, "onCloseRequested" | "attach">;
const TAURI_CLOSE: AgentWindowClose = { onCloseRequested, attach: workspace.attachAgent };

/** Le bouton qui ramène l'agent dans la fenêtre principale, dans tous les états de la fenêtre. */
const AttachButton = ({ onClick }: { onClick: () => void }) => (
  <button type="button" className="token ghost agent-attach" aria-label={t("ui-agent-attach")} title={t("ui-agent-attach")} onClick={onClick}>
    {Icon.attach}
  </button>
);

/** Vide la frappe en attente d'un onglet. */
type Flush = () => Promise<void>;

function TauriAgentWindow({ tab, onAttach, registerFlush }: { tab: string; onAttach: () => void; registerFlush: (flush: Flush | null) => void }) {
  // Une fois par onglet : un évènement `workspace` ne réabonne pas le panneau.
  const [host] = useState(() => tauriHost(tab));
  return <AgentWindowFor tab={tab} transport={linkTransport()} host={host} onAttach={onAttach} registerFlush={registerFlush} />;
}

/**
 * La vue de l'espace de travail reçue : chargement, aucun onglet à suivre, ou l'agent de l'onglet suivi.
 * Elle tient la fermeture système, valable dans tous les états : la frappe de l'onglet suivi est
 * vidée chez l'hôte, puis l'agent est rattaché.
 */
export function AgentWindowView({ view, close = TAURI_CLOSE }: { view: WorkspaceView | null; close?: AgentWindowClose }) {
  const flush = useRef<Flush | null>(null);
  const registerFlush = useCallback((f: Flush | null) => void (flush.current = f), []);
  const attachNow = useCallback(
    () =>
      void (async () => {
        await (flush.current?.() ?? Promise.resolve());
        await close.attach();
      })().catch((e) => console.warn("rattacher l'agent", String(e))),
    [close],
  );
  useEffect(() => close.onCloseRequested(attachNow), [close, attachNow]);
  if (view === null)
    return (
      <div className="agent-window agent-window-loading">
        <TitleBar />
        <p>{t("ui-loading")}</p>
        <AttachButton onClick={attachNow} />
      </div>
    );
  const tab = agentTarget(view);
  if (tab === null)
    return (
      <div className="agent-window agent-window-empty">
        <TitleBar />
        <p>{t("ui-agent-window-empty")}</p>
        <AttachButton onClick={attachNow} />
      </div>
    );
  return <TauriAgentWindow key={tab} tab={tab} onAttach={attachNow} registerFlush={registerFlush} />;
}

/** La fenêtre de l'agent pour un onglet. */
export function AgentWindowFor({
  tab,
  transport,
  host,
  onAttach,
  registerFlush,
}: {
  tab: string;
  transport: LinkTransport;
  host: AgentWindowHost;
  /** Le rattachement de la page (vide la frappe, rattache) ; défaut : celui de cet hôte. */
  onAttach?: () => void;
  /** Donne à la page de quoi vider la frappe de cet onglet avant de rattacher. */
  registerFlush?: (flush: Flush | null) => void;
}) {
  const [context, setContext] = useState<LinkContext | null>(null);
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [remote, setRemote] = useState<{ seq: number; key: LinkShortcut } | undefined>();
  const [status, setStatus] = useState(IDLE_AGENT);
  const handle = useRef<AgentHandle>(null);
  // Jamais affichés : fiches, tiroir et fenêtre restent dans la fenêtre principale.
  const [slots] = useState(() => ({ sheet: createSlot(), drawer: createSlot(), modal: createSlot() }));
  const { say, node: toastNode } = useToast();

  const draft = useDraft(host.agent.setDraft);
  const insertOrKeep = async (text: string) => {
    const ok = await insertWithAck(transport, tab, text);
    if (!ok) say(t("ui-agent-kept-in-draft"), true);
    return ok;
  };
  const actions = draftActions({
    draft,
    editing: context?.editing ?? false,
    insert: insertOrKeep,
    prompt: (text) => handle.current?.prompt(text) ?? Promise.resolve(false),
  });
  // La dernière frappe est chez l'hôte avant que la fenêtre se ferme.
  const flush = draft.flush;
  const ownAttach = useCallback(() => void flush().then(() => host.attach()), [flush, host]);
  const attach = onAttach ?? ownAttach;
  useEffect(() => {
    registerFlush?.(flush);
    return () => registerFlush?.(null);
  }, [registerFlush, flush]);

  // Le canal : le contexte de l'onglet, les raccourcis relayés ; `hello` dit à la fenêtre principale qu'on écoute.
  useEffect(() => {
    let live = true;
    const stop = transport.listen(
      onlyFor("agent", tab, (m) => {
        if (m.kind === "context") setContext(m.context);
        if (m.kind === "shortcut") {
          if (m.key === "type") host.focusSelf();
          setRemote({ seq: ++remoteSeq, key: m.key });
        }
      }),
    );
    void transport.ready.then(() => live && transport.send({ to: "main", kind: "hello", tab }));
    return () => {
      live = false;
      stop();
    };
  }, [tab, transport, host]);

  const reread = useCallback(() => void host.api.snapshot().then(setSnap).catch((e) => say(String(e), true)), [host, say]);
  // Une modification faite dans la fenêtre principale n'émet pas `changed` : relire aussi quand le contexte change.
  const rereadSoon = useMemo(() => debounced(reread, REREAD_MS), [reread]);
  useEffect(() => () => rereadSoon.cancel(), [rereadSoon]);
  const contextKey = JSON.stringify(context);
  useEffect(() => {
    if (context) rereadSoon();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- déclenché par le contenu du contexte
  }, [contextKey, rereadSoon]);

  const thumbs = useSlideThumbs(host.api, snap?.slides);
  const vocabulary = useMemo(() => deckVocabulary(snap), [snap]);
  const staged = context?.staged ?? null;
  const route = useCallback((list: Parameters<typeof routeStaged>[0]) => routeStaged(list, staged), [staged]);

  const dictation = (onText: (text: string, mode: DictationMode) => void) => (
    <Dictation
      destination={t(context?.editing ? "ui-dictation-to-cursor" : "ui-dictation-to-draft")}
      vocabulary={vocabulary}
      onText={onText}
      onError={(m) => say(m, true)}
      agentBusy={status.busy}
      onAgentCancel={() => handle.current?.cancel()}
      onType={draft.type}
      remote={remote}
    />
  );

  if (snap?.read_only)
    return (
      <div className="agent-window agent-window-empty">
        <TitleBar />
        <p>{t("ui-agent-window-empty")}</p>
        <AttachButton onClick={attach} />
        {/* Lecture seule : pas de brouillon (rien ne l'afficherait) ; le texte part à la fenêtre principale, qui le refuse. */}
        {dictation((text) => void insertWithAck(transport, tab, text))}
        {toastNode}
      </div>
    );
  return (
    <div className="agent-window">
      <TitleBar />
      <AgentPanel
        tab={tab}
        agent={host.agent}
        scope={FOLLOW}
        route={route}
        ui={context?.ui ?? NO_UI}
        keysOff={context?.keysOff ?? false}
        keyPlaces={DOCK_ONLY}
        thread
        thumbs={thumbs}
        slots={slots}
        handle={handle}
        // Les réglages de l'hôte (modèle, cible) restent dans la fenêtre principale.
        onHostAction={() => false}
        onStatus={setStatus}
        onChanged={reread}
        onFocus={NOOP}
        onWidgetTouched={NOOP}
        onDraft={draft.onHost}
        say={say}
        subscribe={host.subscribe}
        onAttach={attach}
      />
      <DictationDraft
        text={draft.state.text}
        dictations={draft.state.dictations}
        typing={draft.state.typing}
        busy={status.busy}
        notice={status.notice}
        ready={status.ready}
        insertTo={context?.insertTo ?? null}
        onChange={draft.change}
        onInsert={(text) => void actions.insertDraft(text)}
        onSend={(text) => void actions.sendDraft(text)}
        onDiscard={draft.close}
      />
      {dictation(actions.onDictated)}
      {toastNode}
    </div>
  );
}

/** La page de la fenêtre de l'agent : suit la vue de l'espace de travail. */
export default function AgentWindow() {
  const [view, setView] = useState<WorkspaceView | null>(null);
  // L'écoute d'abord, la vue de départ ensuite : aucune diffusion perdue entre les deux.
  useEffect(() => {
    let alive = true;
    const accept = (v: WorkspaceView) => alive && setView((cur) => newest(cur, v));
    const first = () => workspace.view().then(accept).catch((e) => console.warn("espace de travail", String(e)));
    if (!inTauri) {
      void first();
      return () => void (alive = false);
    }
    const stop = listenTauri(async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const un = await listen<WorkspaceView>("workspace", (e) => accept(e.payload));
      void first();
      return un;
    });
    return () => {
      alive = false;
      stop();
    };
  }, []);
  return <AgentWindowView view={view} />;
}
