import { StagePreviewContext, createStagePreviewStore } from "./a2ui/stagePreview";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { ImportTarget, InterfaceChange, SlideView, Snapshot, Surface } from "./api";
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
import ImportDialog from "./ImportDialog";
import type { ImportJob } from "./useImportJob";
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
import { canAdd } from "./slideTools";
import { panelOpen } from "./panelOpen";
import { NumberLangContext } from "./a2ui/numberLang";
import { AgentPanel } from "./agent/AgentPanel";
import { SlotView } from "./agent/slot";
import type { QuickTarget } from "./agent/useQuickKeys";
import { Latest } from "./latest";
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
import Presenter from "./Presenter";
import type { PresentMode } from "./present";
import SlideNav from "./SlideNav";
import TitleField from "./TitleField";
import { AppMenu } from "./AppMenu";
import { applyInterface } from "./interfaceChange";
import { useAgentLink, useAgentWiring } from "./useAgentWiring";
import { useAppKeys } from "./useAppKeys";
import { type Clip, type ClipRef, useClipboard, useClipboardEvents } from "./useClipboard";
import { useSlideOps } from "./useSlideOps";
import { useLayoutInsert } from "./useLayoutInsert";
import { useCloseGuard } from "./closeGuards";
import { usePackWidgets } from "./usePackWidgets";
import { ToolsTray } from "./ToolsTray";

/** Agent détaché : le dock n'est pas à l'écran, ses touches rapides ne le visent pas. */
const MAIN_PLACES: QuickTarget["kind"][] = ["sheet", "drawer", "modal"];

/** Un onglet : le deck de la session `tab`. */
export default function App({
  tab, clip: windowClip, clipGen, onCopy, agentDetached, nonce, notice, onOpen, importJob, onImportPick, onImport, onImportClose,
  onHelp, onAbout, aboutOpen = false, updates, onClose, onOpenPack, onOpenTheme, onNewPack, onNewTheme, onChange,
}: {
  tab: string;
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
  /** Montrer l'aide de l'app : l'espace de travail ouvre (ou retrouve) son onglet. */
  onHelp: () => void;
  /** Opens the window's About dialog (version and changelog). */
  onAbout: () => void;
  /** The window's About dialog is open: nothing behind it reacts. */
  aboutOpen?: boolean;
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
  // Dernier `onChange` reçu, sans faire changer `run` d'identité.
  const changed = useRef(onChange);
  changed.current = onChange;
  const [snap, setSnap] = useState<Snapshot | null>(null);
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
  const checkUpdates = useUpdateCheck(updates, say);
  const svgs = useSvgCache(api);
  // Miniatures des reçus de l'agent : même cache que la pile, empreintes actuelles du deck.
  const slideThumbs = useThumbsOver(svgs, snap?.slides);

  // Une relecture (`nonce`) croise parfois une modification : la réponse la
  // plus ancienne, arrivée en dernier, ne ramène pas le deck en arrière.
  const latest = useRef(new Latest());
  // The last accepted snapshot, current as soon as it arrives: read after an await,
  // before the render that follows.
  const snapRef = useRef(snap);
  snapRef.current = snap;
  // `null` : rien n'a été fait (dialogue abandonné), l'état ne change pas.
  const run = useCallback(async (p: Promise<Snapshot | null>) => {
    const n = latest.current.begin();
    try {
      const s = await p;
      if (!s) return null;
      if (latest.current.accept(n)) {
        snapRef.current = s;
        setSnap(s);
        changed.current?.();
      }
      // L'hôte n'envoie un avis qu'une fois : même dépassé, il s'affiche.
      if (s.notice) say(s.notice, true);
      return s;
    } catch (e) {
      say(errorText(e), true);
      return null;
    }
  }, []);

  useEffect(() => {
    if (notice) say(notice, true);
  }, [notice]);

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
  // `run` numérote la réponse d'un cran à son arrivée, pas à l'envoi : sans effet, car sortir (Revenir, Restaurer) annule d'abord le Seeker.
  const cursor = useHistoryCursor(api, snap?.history ?? null, (s) => run(Promise.resolve(s)), (e) => say(errorText(e), true));

  // Au montage, puis à chaque vue diffusée : un avis posé hors de cet onglet
  // (fichier reçu qui n'a pas pu s'ouvrir) n'attend pas la prochaine édition.
  useEffect(() => {
    run(api.snapshot());
  }, [run, nonce]);

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
  // An app dialog (layouts, add, import, about, a confirmation) is open: nothing behind it reacts.
  const dialogOpen = picking || adding || importJob !== null || aboutOpen || confirming;
  /** Ajoute la carte choisie dans la grille d'ajout, qui se ferme. */
  const addChoice = (c: WidgetChoice) => {
    setAdding(false);
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
  // The selected chart's or table's data (resolved in the view), at the agent's dock; not in a
  // pack tab, where no stage shows, nor in a theme tab (useDataSettings).
  const selectedElement = view && selected ? findElement(view.slide, selected) : undefined;
  const data = useDataSettings(api, snap, view?.slide.id ?? null, selectedElement, packTab, run, say);
  const dataSurfaces = useMemo(() => (data.surface ? [data.surface] : []), [data.surface]);

  const {
    agent, agentSlots, agentStatus, setAgentStatus, agentAvailability, route,
    agentUiNow, agentMode, agentHandle, promptAgent, cancelAgent, onSurfaceAction, onAgentFocus,
  } = useAgentWiring({
    tab, snap, view, current: packTab ? item.slide : current, selected, editing, editField, packItem: item.selected, packTab, importJob, showSlide,
    setSelected,
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

  // --- Présentation : la slide seule, sur tout l'écran ou toute la fenêtre ----
  const [presenting, setPresenting] = useState<PresentMode | null>(null);
  /** Présente le deck depuis la slide montrée ; la saisie ouverte s'enregistre d'abord. Lit le
   *  dernier instantané, pas celui du rendu : l'agent peut présenter juste après une modification. */
  const present = (mode: PresentMode) => {
    const s = snapRef.current;
    if (!s?.slides.length || s.editing || s.theming || s.history || s.read_only) return;
    stage.current?.commitEditing();
    setSelected(null);
    setEditing(null);
    setPresenting(mode);
  };

  /** Ouvre l'historique demandé par l'agent (outil `history`), une ouverture à la fois.
   *  La saisie ouverte s'enregistre d'abord, puis ce qui attend part au fichier : l'historique
   *  refuse d'écrire, le deck mis de côté ne repart au fichier qu'à sa fermeture. */
  const openingHistory = useRef(false);
  const openHistory = async () => {
    if (openingHistory.current) return;
    openingHistory.current = true;
    try {
      stage.current?.commitEditing();
      await autosave.flush();
      if (await run(api.historyOpen())) {
        setSelected(null);
        setEditing(null);
      }
    } finally {
      openingHistory.current = false;
    }
  };

  /** Ce que l'agent change dans l'interface (`set_interface`), par le chemin des commandes de l'app. */
  const onInterface = (c: InterfaceChange) =>
    applyInterface(c, {
      changeScheme,
      changeLocale,
      setGrid: keptUi.setGrid,
      setGhosts: keptUi.setGhosts,
      fail: (err) => say(errorText(err), true),
    });

  // Raccourcis clavier.
  useAppKeys({
    api, snap, view, current, selected, editing, presenting, agentStatus, dialogOpen, packTab, themeTab,
    cutMark, keptUi, cursor, autosave, run, apply, present, goToSlide, moveSlide, dropCut, open,
    saveCopy, cancelAgent, setEditing, setSelected,
  });

  // --- Dictée : où va le texte ----------------------------------------------
  const dictation = useDictation({ snap, editing, prompt: promptAgent, setDraft: agent.setDraft });
  const pasted = usePastedImages(dictation.draft.state.images, agent.pasteImage, (m) => say(m, true));

  // --- Fenêtre de l'agent : ce qu'elle apprend d'ici, ce qu'elle y écrit ------
  useAgentLink({ tab, agentDetached, editing, dictation, agentUiNow, agentStatus, dialogOpen, packTab, deckLook });

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
    {/* Agent détaché : la colonne de droite se replie, sauf pour montrer les données sélectionnées. */}
    <div className={`table${agentDetached && !themeTab && !data.surface ? " agent-detached" : ""}`}>
      {history ? (
        <HistoryBar
          history={history}
          cursor={cursor}
          onRestore={() => { cursor.cancel(); run(api.historyRestore()); }}
          onBack={() => { cursor.cancel(); run(api.historyClose()); }}
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
        api={api} snap={snap} view={view} item={null}
        keptUi={keptUi} mainMenu={mainMenu} themeTab={false}
        run={run} onAdd={() => setAdding(true)}
        onPacks={() => void agent.showPacks().catch((e) => say(errorText(e), true))}
        thumbs={{ open: thumbsOpen, onToggle: toggleThumbs }}
      />
      </>)}

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
            // L'aide se lit comme une présentation : sans grille ni fantômes.
            showGridAlways={keptUi.grid && !snap.read_only}
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
      </main>
      )}

      {/* Cartes à droite de la feuille. */}
      {/* A theme tab: its panel comes in (ThemePanels); the agent thread stays mounted, hidden (pack.css). */}
      <aside ref={setNotes} className={`notes${themeTab ? " theme-active" : ""}`} aria-label={t("ui-notes")}>
        {/* Agent détaché : pas de dock ici, les données sélectionnées se montrent seules. */}
        {agentDetached && data.surface && (
          <HostSettings surface={data.surface} onAction={onSurfaceAction} outcome={data.outcome} onError={(m) => say(m, true)} />
        )}
        <AgentPanel
          tab={tab} agent={agent} mode={agentMode} route={route} ui={agentUiNow}
          keysOff={!!editing || dialogOpen || packTab} thread={!snap.read_only && !agentDetached} keyPlaces={agentDetached ? MAIN_PLACES : undefined}
          onDetach={inTauri && !agentDetached ? () => void workspace.detachAgent().catch((e) => say(errorText(e), true)) : undefined} thumbs={slideThumbs} slots={agentSlots} handle={agentHandle}
          hostSurfaces={dataSurfaces} onHostAction={(s, a) => openRow(s, a) || host.onAction(s, a) || data.onAction(s, a)} onStatus={setAgentStatus}
          onChanged={() => void run(api.snapshot())} onFocus={onAgentFocus}
          onPresent={present} onInterface={onInterface} onOpenHistory={() => void openHistory()}
          onWidgetTouched={item.touched} say={say}
          onDraft={dictation.draft.onHost}
          onVoice={bumpVoiceSeq} onAvailability={agentAvailability.onAvailability}
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
          themeKey={deckLook}
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
        />
      )}

      <SlotView slot={agentSlots.modal} />

      {presenting && (
        <Presenter mode={presenting} slides={snap.slides} current={current} svg={svgs.svg} onShow={showSlide} onExit={() => setPresenting(null)} />
      )}

      {toastNode}
    </div>
    </StagePreviewContext.Provider>
    </NumberLangContext.Provider>
  );
}
