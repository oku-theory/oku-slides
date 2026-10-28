import { StagePreviewContext, createStagePreviewStore } from "./a2ui/stagePreview";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { ImportTarget, InterfaceChange, SlideView, Snapshot, Surface } from "./api";
import { errorText, inTauri, workspace, tabApi } from "./api";
import { bumpVoiceSeq } from "./voiceSeq";
import { packGestures, showFinish, widgetGesture } from "./packGestures";
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
import { Icon } from "./icons";
import { canAdd } from "./slideTools";
import { themeToOpen } from "./themeToOpen";
import { NumberLangContext } from "./a2ui/numberLang";
import { AgentPanel } from "./agent/AgentPanel";
import { SlotView } from "./agent/slot";
import type { QuickTarget } from "./agent/useQuickKeys";
import { Latest } from "./latest";
import HistoryBar, { useHistoryCursor } from "./HistoryBar";
import HelpBar from "./HelpBar";
import { usePackWidgetEditor } from "./usePackWidgetEditor";
import { COMMON } from "./widgetList";
import { activeWidget } from "./widgetTabs";
import ThemePanel from "./ThemePanel";
import ThemeTabHeader from "./ThemeTabHeader";
import HostSettings from "./HostSettings";
import { HOST_WIDGET, useHostSettings } from "./useHostSettings";
import { useDataSettings } from "./useDataSettings";
import WidgetPane from "./WidgetPane";
import PackTabs from "./PackTabs";
import { PackBrand } from "./PackBrand";
import Presenter from "./Presenter";
import type { PresentMode } from "./present";
import PileWidgets from "./PileWidgets";
import PileColumn from "./PileColumn";
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
  onHelp, onAbout, aboutOpen = false, updates, onClose, onOpenPack, onOpenTheme, onChange,
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
  onOpenPack: (pack: string, widget: string) => Promise<boolean>;
  /** Opens theme `theme` in its own tab (the theme row's "Open" in the deck's packs panel). */
  onOpenTheme: (theme: string) => void;
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

  // --- Sous-onglets de widgets du mode pack --------------------------------
  const widgets = usePackWidgetEditor(api, snap, {
    changed: () => void run(api.snapshot()),
    error: (m) => say(m, true),
    notice: (m) => say(m),
    blocked: () => agentStatus.modal || editing !== null || dialogOpen || !!snapRef.current?.history || !!snapRef.current?.read_only,
  });
  const widgetActive = !!snap?.editing && activeWidget(widgets.tabs) !== null;
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
  const keptUi = useKeptUi(snap?.ui ?? null, current, api.ui, widgets.kept);
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
  const fetchId = snap ? slideToFetch(snap.slides, current) : null;
  const currentHash = snap?.slides.find((s) => s.id === fetchId)?.hash;
  useEffect(() => {
    if (!fetchId) return;
    api.slide(fetchId).then(setView).catch((e) => say(String(e), true));
  }, [fetchId, currentHash]);

  const { clip, cutMark, setCutMark, dropCut } = useClipboard(windowClip, clipGen, snap, view);
  const { apply, addElement, moveSlide, removeSlide, goToSlide } = useSlideOps({
    api, snap, view, current, run, dropCut, setCurrent, setSelected, setEditing, editWhenShown,
    showSlide, widgetActive, widgets,
  });

  const [picking, setPicking] = useState(false);
  const [adding, setAdding] = useState(false);
  // Une fenêtre de l'app (layouts, ajout, import, à propos) est ouverte : rien derrière elle ne réagit.
  const dialogOpen = picking || adding || importJob !== null || aboutOpen;
  /** Ajoute la carte choisie dans la grille d'ajout, qui se ferme. */
  const addChoice = (c: WidgetChoice) => {
    setAdding(false);
    addElement(c.widget);
  };
  const addSlide = () => setPicking(true);
  useClipboardEvents({
    tab, api, snap, snapRef, view, selected, editing, dialogOpen, widgetActive, themeTab, apply, addElement,
    setSelected, say, share: onCopy, clip, setCutMark, dropCut,
  });

  const { open, autosave, saveCopy } = useFileActions({ api, snap, snapRef, run, say, onOpen, onImportPick });

  // Réglages du widget du sous-onglet actif, à ses valeurs d'exemple.
  const host = useHostSettings(api, snap, widgets.activeName, run, say);
  // The selected chart's or table's data (resolved in the view), at the agent's dock; not on a
  // widget sub-tab, where the stage is not shown, nor in a theme tab (useDataSettings).
  const selectedElement = view && selected ? findElement(view.slide, selected) : undefined;
  const data = useDataSettings(api, snap, view?.slide.id ?? null, selectedElement, widgetActive, run, say);
  const dataSurfaces = useMemo(() => (data.surface ? [data.surface] : []), [data.surface]);

  const {
    agent, agentSlots, agentStatus, setAgentStatus, agentAvailability, route,
    agentUiNow, agentMode, agentHandle, promptAgent, cancelAgent, onSurfaceAction, onAgentFocus,
  } = useAgentWiring({
    tab, snap, view, current, selected, editing, editField, widgets, widgetActive, importJob, showSlide,
    setSelected,
  });

  /** The packs panel's "Open" on the theme row: this tab asks the workspace for the theme's tab. */
  const openTheme = (s: Surface, a: A2uiClientAction): boolean => {
    const theme = themeToOpen(s, a);
    if (theme !== null) onOpenTheme(theme);
    return theme !== null;
  };

  const { insertLayout } = useLayoutInsert({ snap, current, apply, setCurrent, setSelected, setPicking });
  const { packWidgets, addSections } = usePackWidgets(snap);
  const { editPack, finishPack } = packGestures({ onOpenPack, onClose });
  useCloseGuard(tab, async () => { await widgets.files.settle(); await autosave.flush(); });

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
    api, snap, view, current, selected, editing, presenting, agentStatus, dialogOpen, widgetActive, themeTab,
    cutMark, keptUi, cursor, autosave, run, apply, present, goToSlide, moveSlide, dropCut, open,
    saveCopy, cancelAgent, setEditing, setSelected,
  });

  // --- Dictée : où va le texte ----------------------------------------------
  const dictation = useDictation({ snap, editing, prompt: promptAgent, setDraft: agent.setDraft });
  const pasted = usePastedImages(dictation.draft.state.images, agent.pasteImage, (m) => say(m, true));

  // --- Fenêtre de l'agent : ce qu'elle apprend d'ici, ce qu'elle y écrit ------
  useAgentLink({ tab, agentDetached, editing, dictation, agentUiNow, agentStatus, dialogOpen, widgetActive, deckLook });

  const [previewStore] = useState(createStagePreviewStore);

  if (!snap) return <div className="table"><p className="empty" style={{ padding: 40 }}>{t("ui-loading")}</p></div>;

  const cur = snap.slides.find((s) => s.id === current);
  const mainMenu = (
    <AppMenu packMode={!!snap.editing} themeTab={themeTab} present={present} open={open} onHelp={onHelp} onAbout={onAbout} onCheckUpdates={checkUpdates} />
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

  // Section des miniatures (repliée à part en mode pack) ; aucune pour un pack de widgets.
  const thumbsSection = pileSection(snap, themeTab);
  const thumbsOpen = thumbsSection === null || !keptUi.folded.includes(thumbsSection);
  const thumbsTitle = t(thumbsSection === "layouts" ? "ui-pile-layouts" : "ui-pile-slides");
  const toggleThumbs = () => thumbsSection && keptUi.setFolded(thumbsSection, thumbsOpen);
  const deckPile = (axis: "x" | "y") => (
    <DeckPile snap={snap} themeTab={themeTab} current={current} svg={svgs.svg} onSelect={goToSlide} onMove={moveSlide} onAdd={addSlide} onRemove={removeSlide} axis={axis} />
  );
  // The left column in pack mode and theme tabs only; elsewhere, the thumbnails in a strip under the stage.
  const packColumn = (!!snap.editing || themeTab) && !history;
  // A widget pack has no slides to show: its empty stage invites to open a widget.
  const widgetPack = snap.editing?.kind === "widgets";
  const layoutPackStage = snap.editing?.kind === "layouts";
  // Légende du pack édité : ses layouts, ou ses widgets (sans le fichier commun).
  const packCount = widgetPack ? (widgets.files.statuses ?? []).filter((w) => w.name !== COMMON).length : snap.slides.length;

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
      ) : snap.theming ? (<>
      <ThemeTabHeader theming={snap.theming} dirty={snap.dirty} onFinish={onClose} />
      <ToolsTray
        api={api} snap={snap} view={view} widgetActive={false} widgets={widgets}
        keptUi={keptUi} mainMenu={mainMenu} themeTab
        run={run} onAdd={noop} onPacks={noop} thumbs={null}
      />
      </>) : (<>
      {/* Titre du document, discret, centré en haut ; la marque ne signale que l'édition d'un pack. */}
      <header className={`label${snap.editing ? " editing-pack" : ""}`}>
        {snap.editing && <PackBrand name={snap.editing.name} kind={snap.editing.kind} deckOnly={snap.editing.deck_only} />}
        <TitleField
          value={snap.title_source ?? ""}
          display={snap.title ?? snap.name}
          onCommit={(v) => apply(t(snap.editing ? "ui-undo-rename-pack" : "ui-undo-rename-deck"), [{ op: "set_deck_param", key: "title", value: v }])}
        />
        <span className="caption" role="status">
          {snap.editing
            ? t("ui-caption-pack", {
                kind: snap.editing.kind,
                count: packCount,
                status: t(snap.dirty ? "ui-status-unsaved" : "ui-status-saved"),
              })
            : <>{snap.path ? `${snap.name}.oku-slides.pdf · ` : ""}{t("ui-caption-slides", { count: snap.slides.length, status })}</>}
        </span>
      </header>
      {snap.editing && <PackTabs kind={snap.editing.kind} tabs={widgets.tabs.tabs} active={widgets.tabs.active} unsaved={widgets.files.unsaved} onActivate={widgets.activate} onClose={widgets.close} />}

      <ToolsTray
        api={api} snap={snap} view={view} widgetActive={widgetActive} widgets={widgets}
        keptUi={keptUi} mainMenu={mainMenu} themeTab={false}
        run={run} onAdd={() => setAdding(true)}
        onPacks={() => void agent.showPacks().catch((e) => say(errorText(e), true))}
        thumbs={!packColumn && thumbsSection ? { open: thumbsOpen, onToggle: toggleThumbs } : null}
      />

      {/* Top right, in pack mode: finish (a lone pack tab that cannot close has none). */}
      {showFinish(snap, onClose !== undefined) && (
        <div className="tray">
          <button className="object token" aria-label={t("ui-toolbar-finish")} title={t("ui-toolbar-finish")} onClick={finishPack}>{Icon.check}</button>
        </div>
      )}
      </>)}

      {/* En mode pack, à gauche : sections Layouts (pack de layouts) et Widgets. */}
      {packColumn && (
        <PileColumn
          title={thumbsTitle}
          open={thumbsOpen}
          onToggle={toggleThumbs}
          slides={thumbsSection && deckPile("y")}
          widgets={themeTab ? null : (
            <PileWidgets
              files={widgets.files}
              open={!keptUi.folded.includes("widgets")}
              onOpenChange={(open) => keptUi.setFolded("widgets", !open)}
              current={widgets.activeName}
              onOpen={widgets.open}
              onRemoved={widgets.removed}
            />
          )}
        />
      )}

      {/* La feuille, posée au centre. */}
      <main className={`sheet-area${readOnly ? " read-only" : ""}`}>
        {/* Sous-onglet widget actif : la scène reste montée (sélection, défilement), cachée. */}
        {/* The stage: the panel of a layout pack's "Layouts" sub-tab. */}
        <div id="stage-panel" className="stage-panel" role={layoutPackStage ? "tabpanel" : undefined} aria-labelledby={layoutPackStage ? "widget-tab-0" : undefined} hidden={widgetActive}>
        <div className="stage-box">
        {widgetPack ? (
          <div className="stage-wrap empty">{t("ui-stage-widget-pack")}</div>
        ) : view && view.slide.id === current ? (
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
            packMode={!!snap.editing}
            packWidgets={packWidgets}
            hoverPreview={previewStore}
            onOpenWidget={(kind) => {
              // A qualified kind (`namespace/name`): its sub-tab here, or the pack pinned under that namespace.
              const gesture = widgetGesture(snap, kind);
              if (gesture.kind === "local") widgets.open(gesture.name);
              else if (gesture.kind === "open") void editPack(gesture.pack, gesture.name);
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
        {!readOnly && !widgetPack && <SlideNav slides={snap.slides} current={current} onShow={goToSlide} />}
        </div>
        {/* Hors mode pack, les miniatures en bande sous la scène ; cachées depuis la barre d'outils, elles rendent
            toute leur place à la slide. L'aide et l'historique, sans barre d'outils, les montrent toujours. */}
        {!packColumn && thumbsSection && (thumbsOpen || readOnly) && <div className="slide-strip">{deckPile("x")}</div>}
        </div>
        {snap.editing && !history && <WidgetPane api={api} widgets={widgets} trials={snap.widget_trials} look={look} code={keptUi.code} codeWidth={keptUi.codeWidth} onCodeWidth={keptUi.setCodeWidth}
          run={run} palette={snap.appearance.chart_palette}
          settings={host.surface?.id === HOST_WIDGET ? <HostSettings surface={host.surface} onAction={onSurfaceAction} outcome={host.outcome} onError={(m) => say(m, true)} /> : null} />}
        <SlotView slot={agentSlots.drawer} />
      </main>

      {/* Cartes à droite de la feuille. */}
      <aside className={`notes${themeTab ? " theme-active" : ""}`} aria-label={t("ui-notes")}>
        {/* A theme tab: the tokens first; the agent thread stays mounted, hidden (workshop.css). */}
        {themeTab && <ThemePanel api={api} snap={snap} run={run} say={say} />}
        {/* Agent détaché : pas de dock ici, les données sélectionnées se montrent seules. */}
        {agentDetached && data.surface && (
          <HostSettings surface={data.surface} onAction={onSurfaceAction} outcome={data.outcome} onError={(m) => say(m, true)} />
        )}
        <AgentPanel
          tab={tab} agent={agent} mode={agentMode} route={route} ui={agentUiNow}
          keysOff={!!editing || dialogOpen || widgetActive} thread={!snap.read_only && !agentDetached} keyPlaces={agentDetached ? MAIN_PLACES : undefined}
          onDetach={inTauri && !agentDetached ? () => void workspace.detachAgent().catch((e) => say(errorText(e), true)) : undefined} thumbs={slideThumbs} slots={agentSlots} handle={agentHandle}
          hostSurfaces={dataSurfaces} onHostAction={(s, a) => openTheme(s, a) || host.onAction(s, a) || data.onAction(s, a)} onStatus={setAgentStatus}
          onChanged={() => void run(api.snapshot())} onFocus={onAgentFocus}
          onPresent={present} onInterface={onInterface} onOpenHistory={() => void openHistory()}
          onWidgetTouched={(name, removed) => widgets.touched(name, removed)} say={say}
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
