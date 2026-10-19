import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import PackNotice from "./PackNotice";
import type { ImportTarget, PileSectionName, SlideView, Snapshot } from "./api";
import { errorText, inTauri, workspace, tabApi } from "./api";
import { firstPackId, widgetToOpen } from "./packOps";
import { appearanceLook } from "./appearance";
import { useAppearance } from "./useAppearance";
import { AttachButton } from "./agent/AttachButton";
import { useSchemeChoice } from "./scheme";
import { findElement, pickSlide, previewLook, slideToFetch } from "./deck";
import TemplatePicker from "./TemplatePicker";
import WidgetPicker from "./WidgetPicker";
import type { WidgetChoice } from "./widgetChoices";
import ImportDialog from "./ImportDialog";
import type { ImportJob } from "./useImportJob";
import { useFileActions } from "./useFileActions";
import SlidePile from "./SlidePile";
import { ColumnDividers } from "./ColumnDivider";
import Stage from "./Stage";
import { useFieldEditing } from "./useFieldEditing";
import { useSvgCache } from "./useSvgCache";
import { useToast } from "./useToast";
import { useUpdateCheck, type AppUpdates } from "./useUpdateCheck";
import { useThumbsOver } from "./useSlideThumbs";
import { useKeptUi } from "./useKeptUi";
import { t } from "./i18n";
import Dictation from "./Dictation";
import { DictationDraft } from "./DictationDraft";
import { useDictation } from "./useDictation";
import { Icon } from "./icons";
import { canAdd, canRemoveSlide } from "./slideTools";
import { NumberLangContext } from "./a2ui/numberLang";
import { AgentPanel } from "./agent/AgentPanel";
import { SlotView } from "./agent/slot";
import type { QuickTarget } from "./agent/useQuickKeys";
import { Latest } from "./latest";
import HistoryBar, { useHistoryCursor } from "./HistoryBar";
import HelpBar from "./HelpBar";
import { pileMarks } from "./history";
import { useTemplateWidgets } from "./useTemplateWidgets";
import { THEME_TAB, activeWidget } from "./widgetTabs";
import ThemePanel from "./ThemePanel";
import HostSettings from "./HostSettings";
import { HOST_TEMPLATE, HOST_WIDGET, useHostSettings } from "./useHostSettings";
import { TargetPanel } from "./TargetPanel";
import { useTargetSettings } from "./useTargetSettings";
import WidgetPane from "./WidgetPane";
import TemplateTabs from "./TemplateTabs";
import { TemplatesBrand } from "./TemplatesBrand";
import Presenter from "./Presenter";
import type { PresentMode } from "./present";
import PileWidgets from "./PileWidgets";
import PileColumn from "./PileColumn";
import SlideNav from "./SlideNav";
import TitleField from "./TitleField";
import { AppMenu } from "./AppMenu";
import { useAgentLink, useAgentWiring } from "./useAgentWiring";
import { useAppKeys } from "./useAppKeys";
import { useClipboard, useClipboardEvents } from "./useClipboard";
import { useSlideOps } from "./useSlideOps";
import { useTemplatesMode } from "./useTemplatesMode";
import { ToolsTray } from "./ToolsTray";

/** Agent détaché : le dock n'est pas à l'écran, ses touches rapides ne le visent pas. */
const MAIN_PLACES: QuickTarget["kind"][] = ["sheet", "drawer", "modal"];

/** Un onglet : le deck de la session `tab`. */
export default function App({
  tab, agentDetached, nonce, notice, onOpen, importJob, onImportPick, onImport, onImportClose,
  onHelp, updates, onClose, onChange,
}: {
  tab: string;
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
  /** Recherche de mise à jour de la fenêtre ; absente hors de l'app (pas d'entrée). */
  updates?: AppUpdates;
  /** Fermer cet onglet (bandeau de l'aide) ; absent : le dernier onglet reste. */
  onClose?: () => void;
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
  // Le dernier instantané accepté, à jour dès sa réception : `finishTemplates` le lit
  // juste après l'enregistrement en attente, avant le rendu qui suit.
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

  // --- Sous-onglets de widgets du mode modèles ------------------------------
  const widgets = useTemplateWidgets(api, snap, {
    changed: () => void run(api.snapshot()),
    error: (m) => say(m, true),
    notice: (m) => say(m),
    blocked: () => agentStatus.modal || editing !== null || dialogOpen || !!snapRef.current?.history || !!snapRef.current?.read_only,
  });
  const widgetActive = !!snap?.editing && activeWidget(widgets.tabs) !== null;
  // Onglet « Thème » : la scène au centre, les jetons à droite (à la place du fil de l'agent).
  const themeActive = !!snap?.editing && !snap.history && widgets.tabs.active === THEME_TAB;

  // --- Historique (lecture seule) -------------------------------------------
  // `run` numérote la réponse d'un cran à son arrivée, pas à l'envoi : sans effet, car sortir (Revenir, Restaurer) annule d'abord le Seeker.
  const cursor = useHistoryCursor(api, snap?.history ?? null, (s) => run(Promise.resolve(s)), (e) => say(errorText(e), true));
  const openHistory = async () => {
    // Ce qui attend s'enregistre avant : l'historique refuse d'écrire, le deck
    // mis de côté ne repart au fichier qu'à sa fermeture.
    await autosave.flush();
    if (await run(api.historyOpen())) {
      setSelected(null);
      setEditing(null);
    }
  };

  // Au montage, puis à chaque vue diffusée : un avis posé hors de cet onglet
  // (fichier reçu qui n'a pas pu s'ouvrir) n'attend pas la prochaine édition.
  useEffect(() => {
    run(api.snapshot());
  }, [run, nonce]);

  // L'interface suit le thème des slides et le schéma (clair, sombre), à chaque changement.
  const schemeChoice = useSchemeChoice();
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

  const { clip, cutMark, setCutMark, dropCut } = useClipboard(snap, view);
  const { apply, addElement, addPackWidget, moveSlide, removeSlide, goToSlide } = useSlideOps({
    api, snap, view, current, run, dropCut, setCurrent, setSelected, setEditing, editWhenShown,
    showSlide, widgetActive, widgets,
  });

  const [picking, setPicking] = useState(false);
  const [adding, setAdding] = useState(false);
  // Une fenêtre de l'app (modèles, ajout, import) est ouverte : rien derrière elle ne réagit.
  const dialogOpen = picking || adding || importJob !== null;
  /** Ajoute la carte choisie dans la grille d'ajout, qui se ferme. */
  const addChoice = (c: WidgetChoice) => {
    setAdding(false);
    if ("native" in c.pick) addElement(c.pick.native);
    else addPackWidget(c.pick.widget);
  };
  const addSlide = () => setPicking(true);
  useClipboardEvents({
    api, snap, snapRef, view, selected, editing, dialogOpen, widgetActive, apply, addElement,
    setSelected, clip, setCutMark, dropCut,
  });

  const { open, merge, autosave, moveFile, saveCopy } = useFileActions({ api, snap, snapRef, run, say, onOpen, onImportPick });

  // Réglages du modèle affiché, ou du widget du sous-onglet actif à ses valeurs d'exemple.
  // Sur l'onglet « Thème », aucun réglage du modèle sous la scène.
  const host = useHostSettings(api, snap, themeActive ? null : current, widgets.activeName, run, say);
  const selectedElement = view && selected ? findElement(view.slide, selected) : undefined;
  // Paramètres de l'élément sélectionné (résolu dans la vue), ou du widget de la slide. Sur
  // l'onglet « Thème », la colonne de droite montre les jetons : la cible n'est pas lue.
  const target = useTargetSettings(api, snap, current, selectedElement ?? null, widgetActive || themeActive, run, say);

  const {
    agent, agentSlots, agentStatus, setAgentStatus, agentPicker, agentAvailability, route,
    agentUiNow, scopeSource, agentHandle, promptAgent, cancelAgent, onSurfaceAction, onAgentFocus,
  } = useAgentWiring({
    tab, snap, view, current, selected, editing, editField, widgets, importJob, say, showSlide,
    setSelected,
  });

  const {
    insertTemplate, editTemplates, packWidgets, addChoices, finishTemplates, saveTemplates,
    adoptSet,
  } = useTemplatesMode({
    api, snap, snapRef, current, run, say, apply, autosave, widgets, setCurrent, setSelected,
    setPicking,
  });

  // --- Présentation : la slide seule, sur tout l'écran ou toute la fenêtre ----
  const [presenting, setPresenting] = useState<PresentMode | null>(null);
  /** Présente le deck depuis la slide montrée ; la saisie ouverte s'enregistre d'abord. */
  const present = (mode: PresentMode) => {
    if (!snap?.slides.length || snap.editing || snap.history || snap.read_only) return;
    stage.current?.commitEditing();
    setSelected(null);
    setEditing(null);
    setPresenting(mode);
  };

  // Raccourcis clavier.
  useAppKeys({
    api, snap, view, current, selected, editing, presenting, agentStatus, dialogOpen, widgetActive,
    cutMark, keptUi, cursor, autosave, run, apply, present, goToSlide, moveSlide, dropCut, open,
    saveCopy, cancelAgent, setEditing, setSelected,
  });

  // --- Dictée : où va le texte ----------------------------------------------
  const dictation = useDictation({ snap, editing, prompt: promptAgent, setDraft: agent.setDraft });

  // --- Fenêtre de l'agent : ce qu'elle apprend d'ici, ce qu'elle y écrit ------
  useAgentLink({ tab, agentDetached, editing, dictation, agentUiNow, agentStatus, dialogOpen, widgetActive, deckLook });

  if (!snap) return <div className="table"><p className="empty" style={{ padding: 40 }}>{t("ui-loading")}</p></div>;

  const cur = snap.slides.find((s) => s.id === current);
  const mainMenu = (
    <AppMenu
      api={api} snap={snap} schemeChoice={schemeChoice} keptUi={keptUi} run={run} say={say}
      present={present} open={open} merge={merge} moveFile={moveFile} saveCopy={saveCopy}
      editTemplates={editTemplates} openHistory={openHistory} saveTemplates={saveTemplates}
      onHelp={onHelp} onCheckUpdates={checkUpdates}
    />
  );
  const templateNames: Record<string, string> = Object.fromEntries(snap.slides.map((s) => [s.id, s.name ?? s.title ?? t("ui-template-default-name")]));
  const status = t(snap.dirty ? "ui-status-unsaved" : snap.path ? "ui-status-saved" : "ui-status-never-saved");
  // Historique : ni sélection, ni édition, ni outils (la session les refuserait).
  const history = snap.history;
  // Historique ou aide : la scène et la pile se lisent, elles ne se modifient pas.
  const readOnly = !!history || snap.read_only;
  const noop = () => {};
  const paramsHidden = keptUi.folded.includes("params");

  // Section des miniatures, repliée à part en mode modèles.
  const thumbsSection: PileSectionName = snap.editing ? "templates" : "slides";
  const pileView = (
    <SlidePile
      slides={snap.slides}
      current={current}
      svg={svgs.svg}
      names={snap.editing ? templateNames : undefined}
      label={t(snap.editing ? "ui-pile-templates" : "ui-pile-slides")}
      addLabel={t(snap.editing ? "ui-pile-add-template" : "ui-pile-add-slide")}
      marks={history ? pileMarks(history.changes) : undefined}
      onSelect={goToSlide}
      onMove={readOnly ? undefined : moveSlide}
      onAdd={readOnly ? undefined : addSlide}
      removeLabel={t(snap.editing ? "ui-toolbar-remove-template" : "ui-toolbar-remove-slide")}
      canRemove={canRemoveSlide(snap.slides.length)}
      onRemove={readOnly ? undefined : removeSlide}
    />
  );

  return (
    <NumberLangContext.Provider value={snap.lang ?? null}>
    <div className={`table${agentDetached && !themeActive ? " agent-detached" : ""}`}>
      {history ? (
        <HistoryBar
          history={history}
          cursor={cursor}
          onRestore={() => { cursor.cancel(); run(api.historyRestore()); }}
          onBack={() => { cursor.cancel(); run(api.historyClose()); }}
        />
      ) : snap.read_only ? (
        <HelpBar title={snap.title ?? snap.name} slides={snap.slides} current={current} onShow={showSlide} search={api.search} onClose={onClose} />
      ) : (<>
      {snap.pack_notice && (
        <PackNotice surface={snap.pack_notice} onCommand={(cmd) => run(api.packNotice(cmd))} onError={(m) => say(m, true)} />
      )}
      {/* Titre du document, discret, centré en haut ; la marque ne signale que l'édition des modèles. */}
      <header className={`label${snap.editing ? " editing-templates" : ""}`}>
        {snap.editing && <TemplatesBrand deckOnly={snap.editing.deck_only} />}
        <TitleField
          value={snap.title_source ?? ""}
          display={snap.title ?? snap.name}
          onCommit={(v) => apply(t(snap.editing ? "ui-undo-rename-set" : "ui-undo-rename-deck"), [{ op: "set_deck_param", key: "title", value: v }])}
        />
        <span className="caption" role="status">
          {snap.editing
            ? t("ui-caption-templates", {
                count: snap.slides.length,
                status: t(snap.dirty ? "ui-status-unsaved" : "ui-status-saved"),
              })
            : <>{snap.path ? `${snap.name}.oku-slides.pdf · ` : ""}{t("ui-caption-slides", { count: snap.slides.length, status })}</>}
        </span>
      </header>
      {snap.editing && <TemplateTabs tabs={widgets.tabs.tabs} active={widgets.tabs.active} unsaved={widgets.files.unsaved} onActivate={widgets.activate} onClose={widgets.close} />}

      <ToolsTray
        api={api} snap={snap} view={view} widgetActive={widgetActive} widgets={widgets}
        keptUi={keptUi} paramsHidden={paramsHidden} mainMenu={mainMenu}
        run={run} onAdd={() => setAdding(true)}
      />

      {/* En haut à droite, en mode modèles : terminer. */}
      {snap.editing && (
        <div className="tray">
          <button className="object token" aria-label={t("ui-toolbar-finish")} title={t("ui-toolbar-finish")} onClick={finishTemplates}>{Icon.check}</button>
        </div>
      )}
      </>)}

      {/* Pile de slides (repliable), à gauche ; en mode modèles, sections Modèles et Widgets. */}
      <PileColumn
        title={t(snap.editing ? "ui-pile-templates" : "ui-pile-slides")}
        open={!keptUi.folded.includes(thumbsSection)}
        onToggle={() => keptUi.setFolded(thumbsSection, !keptUi.folded.includes(thumbsSection))}
        slides={pileView}
        widgets={
          snap.editing && !history ? (
            <PileWidgets
              files={widgets.files}
              open={!keptUi.folded.includes("widgets")}
              onOpenChange={(open) => keptUi.setFolded("widgets", !open)}
              current={widgets.activeName}
              onOpen={widgets.open}
              onRemoved={widgets.removed}
            />
          ) : undefined
        }
      />

      {/* La feuille, posée au centre. */}
      <main className={`sheet-area${readOnly ? " read-only" : ""}`}>
        {/* Sous-onglet widget actif : la scène reste montée (sélection, défilement), cachée. */}
        <div id="templates-panel" className="templates-panel" role={snap.editing && !themeActive ? "tabpanel" : undefined} aria-labelledby={snap.editing && !themeActive ? "widget-tab-0" : undefined} hidden={widgetActive}>
        <div className="stage-box">
        {view && view.slide.id === current ? (
          <Stage
            ref={stage}
            view={view}
            svgUrl={cur ? svgs.svg(cur.id, cur.hash) : null}
            selected={readOnly ? null : selected}
            cut={!readOnly && cutMark?.slide === view.slide.id ? cutMark.element : null}
            editing={readOnly ? null : editing}
            onSelect={readOnly ? noop : setSelected}
            onEdit={readOnly ? noop : setEditing}
            onApply={readOnly ? noop : apply}
            readOnly={readOnly}
            templates={!!snap.editing}
            packWidgets={packWidgets}
            onOpenWidget={(kind) => {
              // Le genre est qualifié (`espace/nom`) : l'atelier du pack épinglé sous cet espace, onglet `nom`.
              const target = widgetToOpen(snap, kind);
              if (target) void widgets.enterAndOpen(() => editTemplates(target.pack), target.name);
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
        {/* Paramètres de la cible (ou panneau du graphique sélectionné), sous la slide, précédés dans la
            même fiche des réglages du modèle affiché (pas au dock de l'agent) ; sur l'onglet « Thème », les jetons à droite. */}
        {!themeActive && (
          <TargetPanel
            view={view} readOnly={readOnly} selected={selectedElement} hostParams={snap.host_params} target={target} palette={snap.appearance.chart_palette}
            onApply={apply} onSurfaceAction={onSurfaceAction} onError={(m) => say(m, true)}
            folded={paramsHidden}
            band={keptUi.band} onBand={keptUi.setBand}
            lead={host.surface?.id === HOST_TEMPLATE ? { surface: host.surface, outcome: host.outcome } : undefined}
            onKind={(kind) => view && selectedElement && run(api.chartKind(t("ui-undo-chart-kind"), view.slide.id, selectedElement.id, kind))}
            onShowAs={(v) => view && selectedElement && run(api.showAs(t(v === "table" ? "ui-undo-show-as-table" : "ui-undo-show-as-chart"), view.slide.id, selectedElement.id, v))}
          />
        )}
        </div>
        {snap.editing && !history && <WidgetPane api={api} widgets={widgets} trials={snap.widget_trials} look={look} code={keptUi.code} codeWidth={keptUi.codeWidth} onCodeWidth={keptUi.setCodeWidth}
          run={run} palette={snap.appearance.chart_palette}
          settings={host.surface?.id === HOST_WIDGET ? <HostSettings surface={host.surface} onAction={onSurfaceAction} outcome={host.outcome} onError={(m) => say(m, true)} /> : null} />}
        <SlotView slot={agentSlots.drawer} />
      </main>

      {/* Cartes à droite de la feuille. */}
      <aside className={`notes${themeActive ? " theme-active" : ""}`} aria-label={t("ui-notes")}>
        {/* Onglet « Thème » : les jetons en tête ; le fil de l'agent reste monté, caché (styles.css). */}
        {themeActive && <ThemePanel api={api} snap={snap} run={run} say={say} />}
        <AgentPanel
          tab={tab} agent={agent} scope={scopeSource} route={route} ui={agentUiNow}
          keysOff={!!editing || dialogOpen || widgetActive} thread={!snap.read_only && !agentDetached} keyPlaces={agentDetached ? MAIN_PLACES : undefined}
          onDetach={inTauri && !agentDetached ? () => void workspace.detachAgent().catch((e) => say(errorText(e), true)) : undefined} thumbs={slideThumbs} slots={agentSlots} handle={agentHandle}
          onHostAction={(s, a) => host.onAction(s, a) || target.onAction(s, a)} onStatus={setAgentStatus}
          onChanged={() => void run(api.snapshot())} onFocus={onAgentFocus}
          onWidgetTouched={(name, removed) => widgets.touched(name, removed)} say={say}
          onDraft={dictation.draft.onHost}
          onPicker={agentPicker.onPicker} onAvailability={agentAvailability.onAvailability}
          undoTurn={snap.undo_turn} onUndoTurn={(turn) => void run(api.undoTurn(turn))}
        />
        {!snap.read_only && !agentDetached && <DictationDraft
          text={dictation.draft.state.text}
          dictations={dictation.draft.state.dictations}
          typing={dictation.draft.state.typing}
          busy={agentStatus.busy}
          notice={agentStatus.notice}
          ready={agentStatus.ready}
          onChange={dictation.draft.change}
          onSend={(text) => void dictation.actions.sendDraft(text)}
          onDiscard={dictation.draft.close}
        />}
      </aside>
      {/* Agent détaché : la colonne reste pour rattacher, même si sa fenêtre n'a pas pu s'ouvrir. */}
      {agentDetached && !themeActive && (
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
        agentPicker={agentPicker.surface}
        onAgentPickerOpen={agentPicker.open}
        onAgentSelect={agentPicker.select}
        onAgentDownload={agentPicker.download}
        onAgentCancelDownload={agentPicker.cancelDownload}
      />}

      {picking && (
        <TemplatePicker
          themeKey={deckLook}
          initialSet={snap.editing ? snap.editing.set : firstPackId(snap)}
          title={t(snap.editing ? "ui-picker-new-template" : "ui-picker-new-slide")}
          onPick={insertTemplate}
          onSetChange={adoptSet}
          onClose={() => setPicking(false)}
          api={api}
        />
      )}

      {adding && (
        <WidgetPicker
          choices={addChoices}
          canPick={(c) => canAdd("native" in c.pick ? c.pick.native : "widget", view?.slide.kind)}
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
    </NumberLangContext.Provider>
  );
}
