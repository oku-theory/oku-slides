import { StagePreviewContext, createStagePreviewStore } from "./a2ui/stagePreview";
import { useEffect, useMemo, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { ImportTarget, InterfaceChange, SlideView, Surface } from "./api";
import { errorText, inTauri, workspace, tabApi } from "./api";
import { bumpVoiceSeq } from "./voiceSeq";
import { packGestures, widgetGesture } from "./packGestures";
import { appearanceLook } from "./appearance";
import { useAppearance } from "./useAppearance";
import { AttachButton } from "./agent/AttachButton";
import { changeScheme } from "./scheme";
import { findElement, pickSlide, previewLook, slideToFetch } from "./deck";
import LayoutPicker from "./LayoutPicker";
import WidgetPicker from "./WidgetPicker";
import type { WidgetChoice } from "./widgetChoices";
import ReceivedDialog from "./ReceivedDialog";
import ImportDialog from "./ImportDialog";
import type { ImportJob } from "./useImportJob";
import type { WaitingPicks } from "./useImportPick";
import { useFileActions } from "./useFileActions";
import DeckPile, { pileSection } from "./DeckPile";
import { ColumnDividers } from "./ColumnDivider";
import Stage from "./Stage";
import { useFieldEditing } from "./useFieldEditing";
import { useSvgCache } from "./useSvgCache";
import { useToast } from "./useToast";
import { useUpdateCheck, type AppUpdates } from "./useUpdateCheck";
import { useThumbsOver } from "./useSlideThumbs";
import { useKeptUi } from "./useKeptUi";
import { changeLocale, t } from "./i18n";
import Dictation from "./Dictation";
import { DictationDraft } from "./DictationDraft";
import { usePastedImages } from "./agent/usePastedImages";
import { useDictation } from "./useDictation";
import { useContextWriter } from "./useVoiceContext";
import { acceptsElements, canAdd } from "./slideTools";
import { panelOpen } from "./panelOpen";
import { NumberLangContext } from "./a2ui/numberLang";
import { AgentPanel } from "./agent/AgentPanel";
import { SlotView } from "./agent/slot";
import { DETACHED_MAIN_PLACES } from "./agent/useQuickKeys";
import { useTabSnapshot } from "./useTabSnapshot";
import HistoryBar, { useHistoryCursor } from "./HistoryBar";
import HelpBar from "./HelpBar";
import { usePackItem } from "./usePackItem";
import { tabKind } from "./tabKind";
import type { PackKind } from "./packTypes";
import PackPanels from "./PackPanels";
import ThemePanels from "./ThemePanels";
import HostSettings from "./HostSettings";
import { HOST_WIDGET, useHostSettings } from "./useHostSettings";
import { useDataSettings } from "./useDataSettings";
import { useElementSettings } from "./useElementSettings";
import { useInspector } from "./useInspector";
import { SidePanel, useSidePanel } from "./SidePanel";
import { Inspector } from "./Inspector";
import { DataPane } from "./DataPane";
import Presenter from "./Presenter";
import { usePageEvents } from "./usePageEvents";
import { usePresentReports } from "./usePresentReports";
import SlideNav from "./SlideNav";
import TitleField from "./TitleField";
import { AppMenu } from "./AppMenu";
import { applyInterface, layoutCommands } from "./interfaceChange";
import { useAgentLink, useAgentWiring } from "./useAgentWiring";
import { useTrace } from "./useTrace";
import { fromA2ui } from "./agent/quick";
import { useAppKeys } from "./useAppKeys";
import { type Clip, type ClipRef, useClipboard, useClipboardEvents } from "./useClipboard";
import { useSlideOps } from "./useSlideOps";
import { useLayoutInsert } from "./useLayoutInsert";
import { useCloseGuard } from "./closeGuards";
import { usePackWidgets } from "./usePackWidgets";
import { ToolsTray } from "./ToolsTray";
import { createPreviewCache } from "./widgetPreviewCache";

/** Un onglet : le deck de la session `tab`. */
export default function App({
  tab, clip: windowClip, clipGen, onCopy, agentDetached, nonce, notice, onOpen, importJob, onImportPick, onImport, onImportClose, importWaiting,
  onHelp, onAbout, windowDialog = false, updates, onClose, onOpenPack, onOpenTheme, onNewPack, onNewTheme, onChange, windowLabel = null,
}: {
  tab: string;
  /** This window's label: its page reports with it (`usePresentReports`); `null`: unknown yet. */
  windowLabel?: string | null;
  /** L'élément copié, ici ou dans une autre fenêtre : il se colle dans un autre onglet. */
  clip: ClipRef;
  /** Croît à chaque copie venue d'une autre fenêtre. */
  clipGen: number;
  /** Envoie l'élément copié ici aux autres fenêtres. */
  onCopy: (clip: Clip) => void;
  /** L'agent est dans sa propre fenêtre : ni colonne, ni fil, ni brouillon ici. */
  agentDetached?: boolean;
  /** Change à chaque vue diffusée : le snapshot est relu (un avis a pu arriver). */
  nonce: number;
  /** Un avis venu d'ailleurs (fusion faite à l'ouverture), montré une fois. */
  notice?: string | null;
  /** Ouvrir un fichier : l'espace de travail choisit l'onglet. */
  onOpen: (path: string) => void;
  /** L'import d'un `.pptx` dont le dialogue est à l'écran (tenu par l'espace de travail). */
  importJob: ImportJob | null;
  /** Un `.pptx` choisi par « Ouvrir » : l'espace de travail ouvre son dialogue d'import. */
  onImportPick: (path: string) => void;
  /** Importe le `.pptx` du dialogue dans un nouvel onglet ; l'espace de travail suit l'issue. */
  onImport: (target: ImportTarget, tune: boolean) => void;
  /** Ferme le dialogue d'import ; un import lancé continue. */
  onImportClose: () => void;
  /** Other `.pptx` files given by the OS, waiting behind the import dialog; absent: none. */
  importWaiting?: WaitingPicks | null;
  /** Montrer l'aide de l'app : l'espace de travail ouvre (ou retrouve) son onglet. */
  onHelp: () => void;
  /** Opens the window's About dialog (version and changelog). */
  onAbout: () => void;
  /** A window dialog (About, a close question) is open: nothing behind it reacts. */
  windowDialog?: boolean;
  /** Recherche de mise à jour de la fenêtre ; absente hors de l'app (pas d'entrée). */
  updates?: AppUpdates;
  /** Closes this tab (help bar, a pack tab's finish button); absent: the last tab stays. */
  onClose?: () => void;
  /** Opens pack `pack` on widget `widget`; true when this tab is still the active one after. */
  onOpenPack: (pack: string, widget: string | null) => Promise<boolean>;
  /** Opens theme `theme` in its own tab (the theme row's "Open" in the deck's packs panel). */
  onOpenTheme: (theme: string) => void;
  /** Creates an empty pack of `kind` in its own tab (the app menu). */
  onNewPack: (kind: PackKind) => void;
  /** Creates an empty theme in its own tab (the app menu). */
  onNewTheme: () => void;
  /** Un snapshot est arrivé (la barre d'onglets se relit, en développement). */
  onChange?: () => void;
}) {
  // Monté par onglet (`key`) : l'API ne change pas pendant la vie du composant.
  const [api] = useState(() => tabApi(tab));
  const [current, setCurrent] = useState<string | null>(null);
  const [view, setView] = useState<SlideView | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const { editing, editField, setEditing, editWhenShown, stage, leaveSlide } = useFieldEditing(view);
  /** Change de slide ; la saisie ouverte s'enregistre d'abord. */
  const showSlide = (id: string) => {
    leaveSlide(id);
    setCurrent(id);
  };
  const { say, node: toastNode } = useToast();
  const { snap, snapRef, run } = useTabSnapshot(api, { nonce, notice, onChange, say });
  const checkUpdates = useUpdateCheck(updates, say);
  const svgs = useSvgCache(api);
  // Miniatures des reçus de l'agent : même cache que la pile, empreintes actuelles du deck.
  const slideThumbs = useThumbsOver(svgs, snap?.slides);

  // --- A pack tab's selected item --------------------------------------------
  const item = usePackItem(api, snap, {
    changed: () => void run(api.snapshot()),
    error: (m) => say(m, true),
    notice: (m) => say(m),
    blocked: () => agentStatus.modal || editing !== null || dialogOpen || !!snapRef.current?.history || !!snapRef.current?.read_only,
  });
  // A pack tab lists its items and shows one; no stage, no slide keys, no clipboard.
  const packTab = !!snap && tabKind(snap) === "pack";
  // A theme tab, known by its live snapshot: the specimens at the centre, the tokens on the right.
  const themeTab = !!snap?.theming;

  // --- Historique (lecture seule) -------------------------------------------
  // `run` numbers a step's reply when it arrives, not when sent: harmless, since leaving (Close, Restore) cancels the Seeker first.
  const cursor = useHistoryCursor(api, snap?.history ?? null, (s) => run(Promise.resolve(s)), (e) => say(errorText(e), true));

  // L'interface suit le thème des slides et le schéma (clair, sombre), à chaque changement.
  useAppearance(snap?.appearance);
  const deckLook = useMemo(() => (snap ? appearanceLook(snap.appearance) : ""), [snap?.appearance]);

  // --- Grille, fantômes et colonnes : repris et gardés par l'hôte ----------
  // A pack tab's current slide is its selected item's (none for `_commun`), for the host and the agent.
  const keptUi = useKeptUi(snap?.ui ?? null, packTab ? item.kept.slide : current, api.ui, item.kept);
  // Style du deck vu par les aperçus de widgets : un changement de thème ou du brouillon les relance.
  const look = snap ? previewLook(snap) : "";

  // L'interface reprend la slide gardée par l'hôte ; ensuite, la slide
  // courante, ou la première si elle a disparu.
  useEffect(() => {
    if (!snap) return;
    const next = pickSlide(snap.slides, current, snap.ui.slide);
    if (next !== current) setCurrent(next);
  }, [snap, current]);

  // Jamais la vue d'une slide que le snapshot n'a plus (elle vient d'être supprimée).
  // A pack tab shows no stage: no slide view is fetched.
  const fetchId = snap && !packTab ? slideToFetch(snap.slides, current) : null;
  const currentHash = snap?.slides.find((s) => s.id === fetchId)?.hash;
  useEffect(() => {
    if (!fetchId) return;
    api.slide(fetchId).then(setView).catch((e) => say(String(e), true));
  }, [fetchId, currentHash]);

  const { clip, cutMark, setCutMark, dropCut } = useClipboard(windowClip, clipGen, snap, view);
  const { apply, addElement, moveSlide, removeSlide, goToSlide } = useSlideOps({
    api, snap, view, current, run, dropCut, setCurrent, setSelected, setEditing, editWhenShown,
    showSlide,
  });

  const [picking, setPicking] = useState(false);
  const [adding, setAdding] = useState(false);
  // A pack tab's delete confirmation (`PackPanels`).
  const [confirming, setConfirming] = useState(false);
  // An app dialog (layouts, add, import, a confirmation) or a window one is open: nothing behind it reacts.
  // Packs and themes the deck brings that the library lacks, or whose guides await approval:
  // asked at once, nothing behind reacts.
  const receivedToReview = snap?.received_to_review ?? [];
  const dialogOpen = picking || adding || importJob !== null || windowDialog || confirming || receivedToReview.length > 0;
  // The last widget added from the grid: it reopens on it (A then Enter adds it again).
  const [lastAdded, setLastAdded] = useState<string | null>(null);
  // The grid's previews, kept across openings (key: pack versions, look and language).
  const [previewCache] = useState(createPreviewCache);
  /** Ajoute la carte choisie dans la grille d'ajout, qui se ferme. */
  const addChoice = (c: WidgetChoice) => {
    setAdding(false);
    setLastAdded(c.key);
    addElement(c.widget);
  };
  const addSlide = () => setPicking(true);
  useClipboardEvents({
    tab, api, snap, snapRef, view, selected, editing, dialogOpen, packTab, themeTab, apply, addElement,
    setSelected, say, share: onCopy, clip, setCutMark, dropCut,
  });

  const { open, autosave, saveCopy } = useFileActions({ api, snap, snapRef, run, say, onOpen, onImportPick });

  // The settings of a pack tab's selected item, at its example values.
  const host = useHostSettings(api, snap, item.selected, run, say);
  // The selected element's settings (inspector) and its chart's or table's data (data pane), resolved
  // in the view; not in a pack tab, where no stage shows, nor in a theme tab (useSelectionSurface).
  const selectedElement = view && selected ? findElement(view.slide, selected) : undefined;
  // Each read only while its place is open.
  const element = useElementSettings(api, snap, view?.slide.id ?? null, selectedElement, packTab || !keptUi.inspector, run, say);
  const data = useDataSettings(api, snap, view?.slide.id ?? null, selectedElement, packTab || !keptUi.dataPane, run, say);
  /** The inspector's and data pane's surface actions: host surfaces, never sent to the agent. */
  const paneAction = (s: Surface, a: A2uiClientAction) => element.onAction(s, a) || data.onAction(s, a);

  // The slide shown: a pack tab's item slide, else the deck's current one.
  const shownSlide = packTab ? item.slide : current;
  const {
    agent, agentSlots, agentStatus, setAgentStatus, agentAvailability, route,
    agentUiNow, agentMode, agentHandle, promptAgent, cancelAgent, onSurfaceAction, onAgentFocus,
  } = useAgentWiring({
    tab, snap, view, current: shownSlide, selected, editing, editField, packItem: item.selected, packTab, importJob, showSlide,
    setSelected,
  });

  // The side panel's tabs, one open at a time: the deck's packs and themes (the agent's boxes) and the inspector.
  const side = useSidePanel({
    agent, packs: agentSlots.side, themes: agentSlots.sideThemes, inspector: keptUi.inspector, setInspector: keptUi.setInspector, say,
  });
  const inspector = useInspector({
    snap, keptUi, selected: selectedElement, closeBoxes: side.closeBoxes,
    // The agent offer opened from its chip: App re-renders only when it comes or goes.
    offer: agentStatus.offer ? <SlotView slot={agentSlots.inspector} /> : undefined,
  });
  // A receipt's trace: its slide shows, its element is selected, then the inspector or the data pane
  // opens (also from the agent window).
  const trace = useTrace({
    snap, view, current: shownSlide, editing, show: showSlide, select: setSelected,
    onPane: inspector.onPane, onInspector: inspector.onInspector, say,
  });

  /** The packs panel's "Open" on a theme or pack row: this tab asks the workspace for its tab. */
  const openRow = (s: Surface, a: A2uiClientAction): boolean => {
    const open = panelOpen(s, a);
    if (open?.kind === "theme") onOpenTheme(open.id);
    else if (open) void onOpenPack(open.id, null);
    return open !== null;
  };

  const { insertLayout } = useLayoutInsert({ snap, current, apply, setCurrent, setSelected, setPicking });
  const { packWidgets, addSections } = usePackWidgets(snap);
  const { editPack } = packGestures({ onOpenPack, onClose });
  useCloseGuard(tab, async () => { await item.files.settle(); await autosave.flush(); });

  const { presenting, setPresenting, present, openHistory } = usePageEvents({
    api, snapRef, stage, autosave, run, setSelected, setEditing,
  });
  const presentWiring = usePresentReports(tab, windowLabel, presenting, present);

  /** Ce que l'agent change dans l'interface (`set_interface`), par le chemin des commandes de l'app. */
  const onInterface = (c: InterfaceChange) =>
    applyInterface(c, {
      changeScheme,
      changeLocale,
      ...layoutCommands(keptUi),
      fail: (err) => say(errorText(err), true),
    });

  // The add grid's opener, shared by the A key and the add button: a deck tab's slide that
  // accepts elements (t-1390).
  const openAdd = !packTab && !themeTab && snap && acceptsElements(view?.slide.kind, snap.free_slots) ? () => setAdding(true) : null;

  // Raccourcis clavier.
  useAppKeys({
    api, snap, view, current, selected, editing, presenting, agentStatus, dialogOpen, packTab, themeTab,
    cutMark, keptUi, cursor, autosave, run, apply, present, goToSlide, moveSlide, dropCut, open,
    saveCopy, cancelAgent, setEditing, setSelected, openAdd,
  });

  // --- Dictée : où va le texte ----------------------------------------------
  const dictation = useDictation({ snap, editing, prompt: promptAgent, setDraft: agent.setDraft });
  const pasted = usePastedImages(dictation.draft.state.images, agent.pasteImage, (m) => say(m, true));

  // --- Fenêtre de l'agent : ce qu'elle apprend d'ici, ce qu'elle y écrit ------
  useAgentLink({ tab, agentDetached, editing, dictation, agentUiNow, agentStatus, dialogOpen, packTab, deckLook, reveal: trace.reveal });
  // What hands-free routing needs of this tab, reported by the window (`useVoiceContext`).
  useContextWriter(tab, agentUiNow);

  const [previewStore] = useState(createStagePreviewStore);
  // The notes column, where a theme tab's panel goes (`ThemePanels`).
  const [notes, setNotes] = useState<HTMLElement | null>(null);

  if (!snap) return <div className="table"><p className="empty" style={{ padding: 40 }}>{t("ui-loading")}</p></div>;

  const cur = snap.slides.find((s) => s.id === current);
  const mainMenu = (
    <AppMenu kind={tabKind(snap)} onNewPack={onNewPack} onNewTheme={onNewTheme} present={present} open={open} onHelp={onHelp} onAbout={onAbout} onCheckUpdates={checkUpdates} />
  );
  const status = t(snap.dirty ? "ui-status-unsaved" : snap.path ? "ui-status-saved" : "ui-status-never-saved");
  // Historique : ni sélection, ni édition, ni outils (la session les refuserait).
  const history = snap.history;
  // Historique ou aide : la scène et la pile se lisent, elles ne se modifient pas.
  const readOnly = !!history || snap.read_only;
  // Deck mode: no pack, theme tab, history or read-only (there the host passes "theme" to the agent).
  const deckMode = !snap.editing && !themeTab && !readOnly;
  // A theme tab's stage shows specimens: no selection, no field editing.
  const locked = readOnly || themeTab;
  const noop = () => {};

  // The foldable thumbnails: the deck's strip under the stage (a theme tab has its own column).
  const thumbsSection = pileSection(themeTab);
  const thumbsOpen = !keptUi.folded.includes(thumbsSection);
  const toggleThumbs = () => keptUi.setFolded(thumbsSection, thumbsOpen);

  return (
    <NumberLangContext.Provider value={snap.lang ?? null}>
    <StagePreviewContext.Provider value={previewStore}>
    {/* Detached agent: the right column folds away. */}
    <div className={`table${agentDetached && !themeTab ? " agent-detached" : ""}`}>
      {history ? (
        <HistoryBar
          history={history}
          cursor={cursor}
          onRestore={() => { cursor.cancel(); run(api.historyRestore()); }}
          onClose={() => { cursor.cancel(); run(api.historyClose()); }}
        />
      ) : snap.read_only ? (
        <HelpBar title={snap.title ?? snap.name} slides={snap.slides} current={current} onShow={showSlide} search={api.search} onClose={onClose} />
      ) : themeTab ? (
        <ThemePanels
          api={api} snap={snap} keptUi={keptUi} current={current} run={run} say={say} svg={svgs.svg}
          mainMenu={mainMenu} notes={notes} onSelect={goToSlide} onClose={onClose}
        />
      ) : packTab ? (
        <PackPanels
          api={api} snap={snap} item={item} keptUi={keptUi} look={look} run={run} svg={svgs.svg} mainMenu={mainMenu}
          settings={host.surface?.id === HOST_WIDGET ? <HostSettings surface={host.surface} onAction={onSurfaceAction} outcome={host.outcome} onError={(m) => say(m, true)} /> : null}
          drawer={<SlotView slot={agentSlots.drawer} />} onClose={onClose} onDialog={setConfirming}
        />
      ) : (<>
      {/* The document's title, discreet, centred at the top. */}
      <header className="label">
        <TitleField
          value={snap.title_source ?? ""}
          display={snap.title ?? snap.name}
          onCommit={(v) => apply(t("ui-undo-rename-deck"), [{ op: "set_deck_param", key: "title", value: v }])}
        />
        <span className="caption" role="status">
          {snap.path ? `${snap.name}.oku-slides.pdf · ` : ""}{t("ui-caption-slides", { count: snap.slides.length, status })}
        </span>
      </header>

      <ToolsTray
        api={api} snap={snap} item={null}
        keptUi={keptUi} mainMenu={mainMenu} themeTab={false}
        run={run} onAdd={openAdd}
        onHistory={() => void openHistory()}
        thumbs={{ open: thumbsOpen, onToggle: toggleThumbs }}
      />
      </>)}

      {/* A deck tab's side panel, on the left: the deck's packs, its themes and the inspector, foldable to a rail. */}
      {inspector.available && (
        <SidePanel
          packs={agentSlots.side}
          themes={agentSlots.sideThemes}
          inspector={inspector.open ? <Inspector element={element} {...inspector.props} onAction={paneAction} onError={(e) => say(errorText(e), true)} /> : null}
          updates={snap.pack_updates > 0}
          onPacks={side.openPacks}
          onThemes={() => side.openThemes(view?.slide.id ?? null)}
          onInspector={inspector.onInspector}
          onFold={inspector.onFold}
          onCloseBox={side.closeBoxes}
        />
      )}

      {/* The sheet, at the centre; a pack tab has its own (`PackPanels`). */}
      {!packTab && (
      <main className={`sheet-area${readOnly ? " read-only" : ""}`}>
        <div className="stage-panel">
        <div className="stage-box">
        {view && view.slide.id === current ? (
          <Stage
            ref={stage}
            view={view}
            svgUrl={cur ? svgs.svg(cur.id, cur.hash) : null}
            selected={locked ? null : selected}
            cut={!locked && cutMark?.slide === view.slide.id ? cutMark.element : null}
            editing={locked ? null : editing}
            onSelect={locked ? noop : setSelected}
            onEdit={locked ? noop : setEditing}
            onApply={locked ? noop : apply}
            readOnly={locked}
            packWidgets={packWidgets}
            hoverPreview={previewStore}
            onOpenWidget={(kind) => {
              // A qualified kind (`namespace/name`): the pack pinned under that namespace opens.
              const gesture = widgetGesture(snap, kind);
              if (gesture.kind === "open") void editPack(gesture.pack, gesture.name);
            }}
            onNotice={(m) => say(m, true)}
            dictated={dictation.dictated}
            // L'aide se lit comme une présentation : sans fantômes.
            showGhosts={keptUi.ghosts && !snap.read_only}
            overlay={agentStatus.sheet ? { anchor: agentStatus.sheet, node: <SlotView slot={agentSlots.sheet} /> } : undefined}
            palette={snap.appearance.chart_palette}
          />
        ) : (
          <div className="stage-wrap empty">{t("ui-stage-rendering")}</div>
        )}
        {!readOnly && <SlideNav slides={snap.slides} current={current} onShow={goToSlide} />}
        </div>
        {/* The thumbnails in a strip under the stage; hidden from the toolbar, they give the slide all their
            room. Help and history, without a toolbar, always show them. A theme tab has its column. */}
        {!themeTab && (thumbsOpen || readOnly) && (
          <div className="slide-strip">
            <DeckPile snap={snap} themeTab={false} current={current} svg={svgs.svg} onSelect={goToSlide} onMove={moveSlide} onAdd={addSlide} onRemove={removeSlide} axis="x" />
          </div>
        )}
        </div>
        <SlotView slot={agentSlots.drawer} />
        {inspector.pane && (
          <DataPane data={data} onAction={paneAction} onError={(e) => say(errorText(e), true)} onClose={inspector.closePane}
            height={keptUi.dataHeight} onHeight={keptUi.setDataHeight} />
        )}
      </main>
      )}

      {/* Cartes à droite de la feuille. */}
      {/* A theme tab: its panel comes in (ThemePanels), then the agent thread, then the composer (pack.css). */}
      <aside ref={setNotes} className={`notes${themeTab ? " theme-active" : ""}`} aria-label={t("ui-notes")}>
        <AgentPanel
          tab={tab} agent={agent} mode={agentMode} route={route} ui={agentUiNow}
          selected={selected} onOpenInspector={inspector.available ? inspector.onInspector : undefined}
          keysOff={!!editing || dialogOpen || packTab} thread={!snap.read_only && !agentDetached} keyPlaces={agentDetached ? DETACHED_MAIN_PLACES : undefined}
          onDetach={inTauri && !agentDetached ? () => void workspace.detachAgent().catch((e) => say(errorText(e), true)) : undefined} thumbs={slideThumbs} slots={agentSlots} handle={agentHandle}
          onHostAction={(s, a) => trace(s, fromA2ui(a)) || openRow(s, a) || host.onAction(s, a)} onStatus={setAgentStatus}
          onChanged={() => void run(api.snapshot())} onFocus={onAgentFocus}
          onPresent={presentWiring.onCommand} onInterface={onInterface} onOpenHistory={() => void openHistory()}
          onWidgetTouched={item.touched} say={say}
          onDraft={dictation.draft.onHost}
          onVoice={bumpVoiceSeq} onAvailability={agentAvailability.onAvailability}
          fallback={agentAvailability.fallback} onFallback={agentAvailability.onFallback}
          undoTurn={snap.undo_turn} onUndoTurn={(turn) => void run(api.undoTurn(turn))}
        />
        {!snap.read_only && !agentDetached && <DictationDraft
          text={dictation.draft.state.text}
          dictations={dictation.draft.state.dictations}
          typing={dictation.draft.state.typing}
          busy={agentStatus.busy}
          ready={agentStatus.ready}
          shortcuts={deckMode}
          onChange={dictation.draft.change}
          images={dictation.draft.state.images}
          thumbs={pasted.thumbs}
          onPasteImages={pasted.onPaste}
          onRemoveImage={(id) => void agent.removeImage(id).catch((e) => say(errorText(e), true))}
          onSend={(text, images) => void dictation.actions.sendDraft(text, images)}
          onDiscard={dictation.draft.close}
        />}
      </aside>
      {/* Agent détaché : la colonne reste pour rattacher, même si sa fenêtre n'a pas pu s'ouvrir. */}
      {agentDetached && !themeTab && (
        <AttachButton className="agent-reattach" onClick={() => void workspace.attachAgent().catch((e) => say(errorText(e), true))} />
      )}

      {/* Entre pile et feuille, entre feuille et fil : de quoi régler les largeurs. */}
      <ColumnDividers columns={keptUi.columns} onChange={keptUi.setColumns} />

      {/* Appareil de dictée, posé devant la feuille. */}
      {/* Montée aussi dans l'historique : l'écoute reste visible et s'arrête ; un texte dicté y reçoit le refus de la session. */}
      {!snap.read_only && !agentDetached && <Dictation
        destination={dictation.destination}
        vocabulary={dictation.vocabulary}
        onText={dictation.actions.onDictated}
        onError={(m) => say(m, true)}
        agentBusy={agentStatus.busy}
        onAgentCancel={cancelAgent}
        onType={dictation.draft.type}
        onSettings={() => void agent.showSettings().catch((e) => say(errorText(e), true))}
      />}

      {picking && (
        <LayoutPicker themeKey={deckLook} onPick={insertLayout} onClose={() => setPicking(false)} api={api} />
      )}

      {adding && (
        <WidgetPicker
          sections={addSections}
          canPick={(c) => canAdd(view?.slide.kind, c.key, snap.free_slots)}
          renderKey={[deckLook, snap.preview_set, snap.widget_trials].join("|")}
          initialKey={lastAdded}
          cache={previewCache}
          preview={api.previewWidget}
          onPick={addChoice}
          onClose={() => setAdding(false)}
        />
      )}

      {importJob && (
        <ImportDialog
          path={importJob.path}
          api={api}
          themes={snap.themes}
          busy={importJob.busy}
          initial={importJob.choice}
          error={importJob.error}
          agentUnusable={agentAvailability.unusable}
          onImport={onImport}
          onClose={onImportClose}
          waiting={importWaiting}
        />
      )}

      {receivedToReview.length > 0 && <ReceivedDialog api={api} reviews={receivedToReview} run={run} />}

      {/* The received dialog stays on top: the agent's modal waits until it closes. */}
      {receivedToReview.length === 0 && <SlotView slot={agentSlots.modal} />}

      {presenting && (
        <Presenter mode={presenting} slides={snap.slides} current={current} svg={svgs.svg} onShow={showSlide} onExit={() => setPresenting(null)} commands={presentWiring.commands} />
      )}

      {/* The agent's popovers, each placed next to its opener. */}
      <SlotView slot={agentSlots.popover} />

      {toastNode}
    </div>
    </StagePreviewContext.Provider>
    </NumberLangContext.Provider>
  );
}
