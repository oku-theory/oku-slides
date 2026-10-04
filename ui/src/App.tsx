import { Suspense, lazy, useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import PackNoticeBar from "./PackNoticeBar";
import { setTemplatesOp } from "./packOps";
import type { AddWidget, AgentEvent, AgentState, Element, McpEvent, Op, PackInfo, Paragraph, SlideView, Snapshot, Surface, TemplateView } from "./api";
import { agentApi, errorText, pickOpenPath, pickSavePath, tabApi } from "./api";
import { saveGuarded } from "./merge";
import { createAutosave, wantsAutosave } from "./autosave";
import type { MenuEntry } from "./ActionMenu";
import { ActionMenu } from "./ActionMenu";
import { applyAppearance } from "./appearance";
import { changeScheme, useScheme, useSchemeChoice } from "./scheme";
import SchemePicker from "./SchemePicker";
import { fieldOps, findElement, freeCell, gridPlacement, instantiate, neighbourSlide, newId, pasteOps, pickSlide, placeOnGridOps, slideToFetch, slot, textToParagraphs } from "./deck";
import { newElement, newPackWidget } from "./newElement";
import TemplatePicker from "./TemplatePicker";
import SlidePile from "./SlidePile";
import { ColumnDividers } from "./ColumnDivider";
import Stage from "./Stage";
import { anchorIds, fieldKey, fieldLabel, primaryFieldOf } from "./stageFields";
import { useFieldEditing } from "./useFieldEditing";
import { useSvgCache } from "./useSvgCache";
import { useViewOptions } from "./useViewOptions";
import { availableLocales, changeLocale, chosenLocale, languageName, t } from "./i18n";
import Dictation from "./Dictation";
import { DictationDraft } from "./DictationDraft";
import { appendDraft, dictationRoute } from "./draft";
import { ChartPanel } from "./chart/ChartPanel";
import { Icon } from "./icons";
import { moveCell, trackSize } from "./grid";
import type { AddKind } from "./slideTools";
import { ADD_KINDS, canAdd, canRemoveSlide, menuWidgets } from "./slideTools";
import type { A2uiClientAction, A2uiMessage } from "@a2ui/web_core/v0_9";
import { panelActionOps } from "./a2ui/panelActions";
import { SlideThumbsContext, type SlideThumbs } from "./a2ui/slideThumbs";
import { AgentThread } from "./agent/AgentThread";
import { subscribeHost } from "./agent/events";
import type { DictationMode } from "./agent/keys";
import { escapeAction } from "./agent/keys";
import type { QuickAction } from "./agent/quick";
import { fromA2ui } from "./agent/quick";
import { agentBusy, routeSurfaces, sendThenPrompt, surfacesReducer } from "./agent/surfaces";
import { withRewriteParagraphs } from "./agent/rewrite";
import { answerTarget, quickTarget, useQuickKeys } from "./agent/useQuickKeys";
import { agentUi } from "./agent/ui";
import { Latest } from "./latest";
import HistoryBar, { useHistoryCursor } from "./HistoryBar";
import { pileMarks } from "./history";
import { useTemplateWidgets } from "./useTemplateWidgets";
import WidgetPane from "./WidgetPane";
import CodeSwitch from "./CodeSwitch";
import TemplateTabs from "./TemplateTabs";
import PileWidgets from "./PileWidgets";


// @a2ui/react (et zod, lit…) ne sert qu'au panneau des modèles : chargé à la demande.
const PanelSurface = lazy(() => import("./a2ui/PanelSurface").then((m) => ({ default: m.PanelSurface })));
// Surfaces de l'agent : chargées avec @a2ui, à la première question.
const AgentSheet = lazy(() => import("./agent/AgentSheet").then((m) => ({ default: m.AgentSheet })));
const VariantDrawer = lazy(() => import("./agent/VariantDrawer").then((m) => ({ default: m.VariantDrawer })));
const ConfirmModal = lazy(() => import("./agent/ConfirmModal").then((m) => ({ default: m.ConfirmModal })));

/** Panneau vide (hors des modèles) : même tableau, pas de nouveau rendu. */
const NO_PANEL: A2uiMessage[] = [];

/** Titre de la présentation, modifiable sur place (clic, Entrée pour valider, Échap pour annuler). */
function TitleField({ value, display, onCommit }: { value: string; display: string; onCommit: (v: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  if (!editing) {
    return (
      <button
        className="doc-name doc-name-button"
        title={t("ui-title-rename")}
        onClick={() => { setDraft(value); setEditing(true); }}
      >
        {display}
      </button>
    );
  }
  const commit = () => {
    setEditing(false);
    if (draft !== value) onCommit(draft);
  };
  return (
    <input
      className="doc-name doc-name-input"
      aria-label={t("ui-title-field")}
      autoFocus
      value={draft}
      size={Math.max(12, draft.length + 1)}
      onFocus={(e) => e.currentTarget.select()}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") { e.preventDefault(); commit(); }
        else if (e.key === "Escape") { e.preventDefault(); setEditing(false); }
        e.stopPropagation();
      }}
    />
  );
}

/** Un onglet : le deck de la session `tab`. */
export default function App({ tab, nonce, notice, onOpen, onChange }: {
  tab: string;
  /** Change à chaque vue diffusée : le snapshot est relu (un avis a pu arriver). */
  nonce: number;
  /** Un avis venu d'ailleurs (fusion faite à l'ouverture), montré une fois. */
  notice?: string | null;
  /** Ouvrir un fichier : l'espace de travail choisit l'onglet. */
  onOpen: (path: string) => void;
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
  const [toast, setToast] = useState<{ text: string; error: boolean } | null>(null);
  const svgs = useSvgCache(api);
  // Miniatures des reçus de l'agent : même cache que la pile, empreintes actuelles du deck.
  const slides = snap?.slides;
  const slideThumbs = useMemo<SlideThumbs>(
    () => ({ cached: svgs.cached, svg: svgs.svg, current: (id) => slides?.find((s) => s.id === id)?.hash }),
    [svgs, slides],
  );

  const say = (text: string, error = false) => {
    setToast({ text, error });
    window.setTimeout(() => setToast((cur) => (cur?.text === text ? null : cur)), error ? 6000 : 2500);
  };

  // Une relecture (`nonce`) croise parfois une modification : la réponse la
  // plus ancienne, arrivée en dernier, ne ramène pas le deck en arrière.
  const latest = useRef(new Latest());
  // `null` : rien n'a été fait (dialogue abandonné), l'état ne change pas.
  const run = useCallback(async (p: Promise<Snapshot | null>) => {
    const n = latest.current.begin();
    try {
      const s = await p;
      if (!s) return null;
      if (latest.current.accept(n)) {
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
  const snapRef = useRef(snap);
  snapRef.current = snap;
  const widgets = useTemplateWidgets(api, snap, {
    changed: () => void run(api.snapshot()),
    error: (m) => say(m, true),
    notice: (m) => say(m),
    blocked: () => !!modal || editing !== null || picking || !!snapRef.current?.history,
  });
  const widgetActive = !!snap?.editing && widgets.tabs.active !== null;

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
  const scheme = useScheme();
  const schemeChoice = useSchemeChoice();
  const appearanceKey = snap ? JSON.stringify(snap.appearance) : "";
  useEffect(() => {
    if (snap) applyAppearance(snap.appearance, scheme);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [appearanceKey, scheme]);

  // --- Grille, fantômes et colonnes : repris et gardés par l'hôte ----------
  const viewOptions = useViewOptions(snap?.ui ?? null, current, api.ui, widgets.kept);

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

  // Presse-papiers : ⌘C, ⌘X et ⌘V passent par les événements du navigateur.
  // L'élément copié reste ici ; le presse-papiers du système reçoit son texte,
  // qui dit au collage si l'élément est toujours la dernière chose copiée.
  const clip = useRef<{ element: Element; text: string } | null>(null);
  useEffect(() => {
    const skip = (e: ClipboardEvent) => editing || picking || widgetActive || !view || snap?.history || (e.target as HTMLElement).closest?.('input, textarea, select, [contenteditable="true"]');
    const copy = (e: ClipboardEvent) => {
      if (skip(e) || !view || !selected) return false;
      const element = findElement(view.slide, selected);
      if (!element) return false;
      const text = primaryFieldOf(view, selected)?.text || element.kind;
      e.preventDefault();
      e.clipboardData?.setData("text/plain", text);
      clip.current = { element, text };
      return true;
    };
    const cut = (e: ClipboardEvent) => {
      if (!view || !selected || !copy(e)) return;
      apply(t("ui-undo-cut"), [{ op: "remove_element", slide: view.slide.id, element: selected }]);
      setSelected(null);
    };
    const paste = (e: ClipboardEvent) => {
      if (skip(e) || !view) return;
      const text = e.clipboardData?.getData("text/plain") ?? "";
      if (clip.current && text === clip.current.text) {
        e.preventDefault();
        if (!canAdd(clip.current.element.kind, view.slide.kind)) return;
        const { id, ops } = pasteOps(view, clip.current.element);
        apply(t("ui-undo-paste"), ops).then(() => setSelected(id));
      } else if (text.trim() && canAdd("text", view.slide.kind)) {
        // Du texte venu d'ailleurs devient un élément texte.
        e.preventDefault();
        addElement("text", textToParagraphs(text), t("ui-undo-paste"));
      }
    };
    window.addEventListener("copy", copy);
    window.addEventListener("cut", cut);
    window.addEventListener("paste", paste);
    return () => {
      window.removeEventListener("copy", copy);
      window.removeEventListener("cut", cut);
      window.removeEventListener("paste", paste);
    };
  });

  const apply = useCallback(
    async (label: string, ops: Op[], merge?: string) => {
      await run(api.apply(label, ops, merge));
    },
    [run],
  );

  const open = async () => {
    const path = await pickOpenPath();
    if (path) onOpen(path);
  };

  /** Fusionne un autre PDF du même deck dans celui de l'onglet. */
  const merge = async () => {
    const path = await pickOpenPath();
    if (path) await run(api.merge(path));
  };

  // --- Enregistrement automatique -----------------------------------------
  // On n'enregistre jamais à la main : chaque modification part au fichier
  // (au dossier par défaut pour un deck neuf) peu après la dernière.
  // Un fichier remplacé par un autre deck, et l'écrasement refusé : plus
  // d'enregistrement automatique jusqu'à un déplacement.
  const [autosavePaused, setAutosavePaused] = useState(false);
  const pausedRef = useRef(autosavePaused);
  pausedRef.current = autosavePaused;
  const autosave = useMemo(
    () =>
      createAutosave(async () => {
        const s = snapRef.current;
        if (!s || !wantsAutosave(s, pausedRef.current)) return;
        if (s.editing) {
          await run(api.saveTemplates(false));
          return;
        }
        const saved = await run(saveGuarded(api, s));
        if (!saved) setAutosavePaused(true);
      }),
    [api, run],
  );
  useEffect(() => {
    if (snap && wantsAutosave(snap, autosavePaused)) autosave.schedule();
  }, [snap, autosavePaused, autosave]);
  // Onglet quitté (`App` remonté) : ce qui attend part tout de suite.
  useEffect(() => () => void autosave.flush(), [autosave]);

  /** Déplace le fichier du deck ; l'enregistrement automatique reprend. */
  const moveFile = async () => {
    if (!snap) return;
    const path = await pickSavePath(snap.name);
    if (!path) return;
    await autosave.flush();
    if (await run(api.move(path))) {
      setAutosavePaused(false);
      say(t("ui-toast-moved"));
    }
  };

  /** Écrit une copie du deck ailleurs ; il garde son fichier. */
  const saveCopy = async () => {
    if (!snap) return;
    const path = await pickSavePath(snap.name);
    if (!path) return;
    if (await run(api.saveCopy(path))) say(t("ui-toast-copied"));
  };


  /** Ajoute un texte, une carte, une forme ou un graphique ; sur une grille, accroché aux nœuds. */
  const addElement = (kind: AddKind, text?: Paragraph[], label?: string) => {
    const element = newElement(kind, newId(kind[0]));
    if (kind === "text" && text) element.params = { ...element.params, body: text };
    place(element, label ?? t("ui-undo-add", { kind }), kind === "text" && !text);
  };

  /** Ajoute un widget du pack, avec les défauts de ses paramètres. */
  const addPackWidget = (w: AddWidget) => {
    place(newPackWidget(w.name, newId("w")), t("ui-undo-add-widget", { name: w.label ?? w.name }), false);
  };

  /** Pose `element` sur la slide affichée, puis le sélectionne (et l'édite si `edit`). */
  const place = (element: Element, label: string, edit: boolean) => {
    if (!view) return;
    const slide = view.slide;
    const zone = view.zone;
    const g = zone?.grid;
    let ops: Op[];
    const hasBlocks = slot(slide, "blocks").length > 0;
    if (zone && g && !hasBlocks) {
      // Sur la grille : le nouvel élément prend la moitié gauche de la première
      // ligne libre sous les autres. (Une slide à blocs reçoit un bloc de plus.)
      ops = [{ op: "add_element", slot: { owner: { slide: slide.id }, param: "elements" }, element: { ...element, cell: freeCell(slide, g) } }];
    } else {
      ops = [{ op: "add_element", slot: { owner: { slide: slide.id }, param: "blocks" }, element }];
    }
    apply(label, ops).then(() => {
      setSelected(element.id);
      if (edit) editWhenShown(element.id);
    });
  };

  const [picking, setPicking] = useState(false);
  const addSlide = () => setPicking(true);
  const insertTemplate = (tpl: TemplateView) => {
    if (!snap) return;
    setPicking(false);
    const index = snap.slides.findIndex((s) => s.id === current) + 1;
    const slide = instantiate(tpl.slide);
    if (snap.editing) {
      // Nouveau modèle : copie du modèle choisi, sous un nom à lui.
      slide.name = t("ui-template-variant-name", { name: tpl.name });
      slide.description = tpl.description;
    }
    apply(t(snap.editing ? "ui-undo-new-template" : "ui-undo-new-slide", { name: tpl.name }), [{ op: "add_slide", index, slide }]).then(() => {
      setCurrent(slide.id);
      setSelected(null);
    });
  };

  // Panneau de droite : surface A2UI de la session, en édition des modèles.
  const [panel, setPanel] = useState<A2uiMessage[]>(NO_PANEL);
  useEffect(() => {
    if (!current || !snap?.editing) return setPanel(NO_PANEL);
    api.panel(current).then(setPanel).catch((e) => say(String(e), true));
  }, [current, snap]);
  const onPanelAction = (action: A2uiClientAction) => {
    if (!view) return;
    const change = panelActionOps(action, view.slide);
    if (change) apply(change.label, change.ops);
  };

  // --- Agent ------------------------------------------------------------------
  // L'agent de cet onglet : `App` est remonté à chaque changement d'onglet (`key`).
  const agent = useMemo(() => agentApi(tab), [tab]);
  const [agentState, onAgentEvent] = useReducer((_: AgentState, e: AgentEvent) => e.state, "idle" as AgentState);
  const [surfaces, onSurfaceEvent] = useReducer(surfacesReducer, []);
  /** Listes entières reçues (`surfaces`, après un abonnement ou une reconnexion) : le fil ne les relit pas. */
  const [resyncs, countResync] = useReducer((n: number) => n + 1, 0);
  // Une fiche s'affiche sous son élément si celui-ci est sur la slide affichée,
  // sinon dans le fil ; le tiroir sous la scène ; la fenêtre par-dessus tout.
  const routed = useMemo(
    () =>
      routeSurfaces(
        surfaces,
        view && view.slide.id === current ? { slide: view.slide.id, targets: anchorIds(view) } : null,
      ),
    [surfaces, view, current],
  );
  const { stage: sheet, drawer, modal } = routed;
  const cancelAgent = () => agent.cancel().catch((err) => say(String(err), true));
  const touchedRef = useRef(widgets.touched);
  touchedRef.current = widgets.touched;
  useEffect(
    () =>
      subscribeHost(tab, {
        onMcp: (e: McpEvent) => {
          onSurfaceEvent(e);
          if (e.kind === "surfaces") countResync();
          if (e.kind === "changed") run(api.snapshot());
          if (e.kind === "focus") {
            showSlide(e.slide);
            setSelected(e.element);
          }
          if (e.kind === "widget_touched") touchedRef.current(e.name, e.removed);
        },
        onAgent: onAgentEvent,
        onError: (m) => console.warn(m),
        // Abonné ou reconnecté : l'hôte redonne les surfaces, et le deck est relu
        // (les évènements manqués pendant la coupure sont perdus). Sans outils
        // (pile absente), `resync` échoue : il n'y a alors pas de surface.
        onReady: () => {
          agent.resync().catch((err) => console.warn("resync", err));
          run(api.snapshot());
        },
      }),
    // S'abonner une fois par onglet : `run`, `api`, `agent` et les setters sont stables.
    [run, tab, api, agent],
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
  const keysTarget = quickTarget({ modal, sheet, drawer, dock: routed.dock });
  /**
   * La surface qui reçoit les touches 1–4, Entrée, Échap et montre leur rappel ; aucune pendant
   * l'édition d'un texte, le pointage ou un widget actif. Une consigne répond toujours à `keysTarget`.
   */
  const keyed = !editing && !picking && !widgetActive ? keysTarget : null;
  /** Un tour en cours qui n'attend pas l'utilisateur (aucune question ne reçoit la consigne) : ⌘⇧D et Échap l'arrêtent. */
  const busy = agentBusy(agentState, answerTarget(keysTarget));
  const hintsOn = (s: Surface) => keyed?.surface.id === s.id;
  /** Une consigne pour l'agent, avec ce que voit l'utilisateur ; elle répond par défaut à la question qui reçoit les touches. */
  const promptAgent = (text: string, answer = answerTarget(keysTarget)) =>
    agent
      .prompt(text, agentUi(current, selected, editField, widgets.activeName), answer)
      .catch((err) => say(String(err), true));
  const onSurfaceAction = (s: Surface, a: A2uiClientAction) => {
    const action = fromA2ui(a);
    // Une suite cliquée part vers l'agent, une fois l'action acceptée : une nouvelle consigne, qui ne répond à aucune question.
    void sendThenPrompt(action, () => sendAction(s, action), promptAgent);
  };
  useQuickKeys(keyed, sendAction);

  // --- Édition des modèles ------------------------------------------------
  const editTemplates = async (): Promise<boolean> => {
    if (!snap?.templates) return false;
    if (!(await run(api.editTemplates(snap.templates.id)))) return false;
    setCurrent(null);
    setSelected(null);
    return true;
  };
  // Clé par contenu : chaque snapshot apporte un nouveau tableau.
  const packWidgetsKey = (snap?.pack_widgets ?? []).join("\n");
  const packWidgets = useMemo(() => new Set(packWidgetsKey ? packWidgetsKey.split("\n") : []), [packWidgetsKey]);
  const addWidgets = menuWidgets(snap?.add_widgets ?? []);
  const finishTemplates = async () => {
    if (!snap?.editing) return;
    await autosave.flush();
    // Un enregistrement refusé laisse des modèles non enregistrés.
    if (snapRef.current?.dirty && !window.confirm(t("ui-confirm-quit-templates"))) return;
    let closed = false;
    await widgets.finish(async () => { closed = !!(await run(api.closeTemplates())); });
    if (closed) {
      setCurrent(null);
      setSelected(null);
    }
  };
  const saveTemplates = async (asNew = false) => {
    const s = await run(api.saveTemplates(asNew));
    if (s?.editing) say(t("ui-toast-templates-saved", { name: s.editing.name }));
  };
  const adoptSet = (set: PackInfo) => {
    if (!snap || snap.editing || set.id === snap.templates?.id) return;
    apply(t("ui-undo-templates", { name: set.name }), [setTemplatesOp(set)]);
  };

  // --- Réarranger les slides par glisser-déposer -----------------------------
  const moveSlide = (id: string, slot: number) => {
    if (!snap) return;
    const from = snap.slides.findIndex((s) => s.id === id);
    // `slot` : place entre deux vignettes (0 = avant la première).
    const to = slot > from ? slot - 1 : slot;
    if (from < 0 || to === from) return;
    apply(t(snap.editing ? "ui-undo-move-template" : "ui-undo-move-slide"), [{ op: "move_slide", slide: id, index: to }]);
  };

  /** Supprime une slide ; la slide affichée supprimée, on passe à sa voisine. */
  const removeSlide = (id: string) => {
    if (!snap || !canRemoveSlide(snap.slides.length)) return;
    const next = id === current ? neighbourSlide(snap.slides, id) : current;
    run(api.apply(t("ui-undo-remove-slide"), [{ op: "remove_slide", slide: id }])).then((s) => {
      if (s) setCurrent(next);
    });
  };

  // Raccourcis clavier.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Déjà prise par un composant (barre des sous-onglets, pile…) : pas deux actions pour une touche.
      if (e.defaultPrevented) return;
      // Dans une surface A2UI (panneau, agent), une fiche de l'agent ou l'éditeur de texte, les touches sont les leurs.
      // Une fenêtre de l'agent ouverte : rien derrière elle ne réagit.
      if (modal || editing || picking || (e.target as HTMLElement).closest('input, textarea, select, [contenteditable="true"], .a2ui-surface, .agent-keys')) return;
      const mod = e.metaKey || e.ctrlKey;
      // Grille et fantômes se basculent partout, historique compris.
      if (viewOptions.handleKey(e)) return;
      if (snap?.history) {
        // Lecture seule : ← / → (⇧ : par session) parcourent les versions ;
        // les raccourcis d'édition, qui prennent aussi les flèches, sont inertes.
        if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && !mod && !e.altKey) {
          e.preventDefault();
          cursor.arrow(e.key, e.shiftKey);
        }
        return;
      }
      // Sous-onglet widget : ⌘Z va à son éditeur (`useTemplateWidgets`), jamais aux modèles.
      if (mod && e.key.toLowerCase() === "z") {
        if (widgetActive) return;
        e.preventDefault();
        run(e.shiftKey ? api.redo() : api.undo());
      } else if (mod && e.key.toLowerCase() === "s") {
        // Tout s'enregistre de soi-même : ⌘S n'avance que l'échéance, ⌘⇧S fait une copie.
        e.preventDefault();
        if (e.shiftKey && !snap?.editing) saveCopy();
        else void autosave.flush();
      } else if (mod && e.key.toLowerCase() === "o") {
        e.preventDefault();
        open();
      } else if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown") && snap && current && !widgetActive) {
        e.preventDefault();
        const i = snap.slides.findIndex((s) => s.id === current);
        const slot = e.key === "ArrowUp" ? i - 1 : i + 2;
        if (slot >= 0 && slot <= snap.slides.length) moveSlide(current, slot);
      } else if (e.key === "Escape") {
        const action = escapeAction({ selected: selected !== null, agentBusy: busy });
        if (action === "deselect") setSelected(null);
        else if (action === "agent-cancel") {
          e.preventDefault();
          cancelAgent();
        }
      } else if (!view || !selected || widgetActive) {
        // Sous-onglet widget actif : la scène est cachée, ses raccourcis (Entrée, Suppr, flèches) aussi.
        return;
      } else if (e.key === "Enter") {
        const f = primaryFieldOf(view, selected);
        if (f) {
          e.preventDefault();
          setEditing(fieldKey(f));
        }
      } else if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        apply(t("ui-undo-remove"), [{ op: "remove_element", slide: view.slide.id, element: selected }]);
        setSelected(null);
      } else if (e.key.startsWith("Arrow")) {
        const dx = e.key === "ArrowLeft" ? -1 : e.key === "ArrowRight" ? 1 : 0;
        const dy = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
        // Sur la grille : une case à la fois. Un bloc fait passer la slide sur
        // la grille ; un élément libre s'y accroche s'il ne l'était pas.
        const place = gridPlacement(view, selected);
        if (place) {
          e.preventDefault();
          const { zone, grid } = place;
          const { cw, rh } = trackSize(zone, grid);
          const cell = moveCell(zone, grid, place.cell, dx * (cw + grid.gap), dy * (rh + grid.gap));
          apply(t(place.block ? "ui-undo-place-on-grid" : "ui-undo-move-on-grid"), placeOnGridOps(view, selected, cell));
          return;
        }
        const el = findElement(view.slide, selected);
        if (!el?.frame) return;
        e.preventDefault();
        const step = e.shiftKey ? 10 : 1;
        const f = { ...el.frame };
        if (e.key === "ArrowLeft") f.x -= step;
        if (e.key === "ArrowRight") f.x += step;
        if (e.key === "ArrowUp") f.y -= step;
        if (e.key === "ArrowDown") f.y += step;
        apply(t("ui-undo-move"), [{ op: "set_frame", slide: view.slide.id, element: selected, frame: f }]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // --- Dictée : où va le texte ----------------------------------------------
  const [dictated, setDictated] = useState<{ seq: number; text: string } | null>(null);
  const selectedElement = view && selected ? findElement(view.slide, selected) : undefined;
  // Hors édition, la dictée écrit dans le premier champ de l'élément sélectionné.
  const writable = view && selected ? primaryFieldOf(view, selected) : null;
  // Où « Insérer » écrit le brouillon ; `null` : aucune zone où écrire.
  const insertTo = editing
    ? t("ui-dictation-to-cursor")
    : writable
      ? writable.kind === "plain"
        ? t("ui-dictation-to-field", { field: fieldLabel(writable).toLowerCase() })
        : t("ui-dictation-to-block")
      : view?.slide.kind === "content"
        ? t("ui-dictation-new-block")
        : null;
  // Où va ⌘D : au curseur pendant l'édition, au brouillon ailleurs.
  const destination = t(editing ? "ui-dictation-to-cursor" : "ui-dictation-to-draft");
  const dictationVocabulary = useMemo(
    () => (snap ? [snap.title, ...snap.slides.map((s) => s.title)].filter((s): s is string => !!s) : []),
    [snap],
  );

  // Hors édition, une dictée s'écrit dans le brouillon : « Insérer » l'écrit
  // tel quel dans le deck, « Envoyer » la donne à l'agent. ⌘K l'ouvre pour écrire.
  const [draft, setDraft] = useState<{ text: string | null; dictations: number; typing: number }>({ text: null, dictations: 0, typing: 0 });
  const closeDraft = () => setDraft((d) => ({ ...d, text: null }));
  const typeDraft = () => setDraft((d) => ({ ...d, text: d.text ?? "", typing: d.typing + 1 }));
  const sendDraft = (text: string) => {
    closeDraft();
    promptAgent(text);
  };
  const insertDraft = (text: string) => {
    closeDraft();
    insertText(text);
  };

  const onDictated = (text: string, mode: DictationMode) => {
    if (dictationRoute(mode, editing !== null) === "cursor") insertText(text);
    else setDraft((d) => ({ ...d, text: appendDraft(d.text, text), dictations: d.dictations + 1 }));
  };

  /** Écrit un texte tel quel : au curseur, dans la zone sélectionnée ou en nouveau bloc. */
  const insertText = (text: string) => {
    if (editing) {
      setDictated((d) => ({ seq: (d?.seq ?? 0) + 1, text }));
      return;
    }
    if (!view) return;
    if (writable) {
      const previous = writable.text;
      const keep = writable.kind === "rich" && previous.trim() !== "" && previous !== t("ui-new-text");
      apply(t("ui-undo-dictation"), fieldOps(view, writable, keep ? `${previous}\n${text}` : text));
      return;
    }
    if (view.slide.kind === "content") {
      addElement("text", textToParagraphs(text), t("ui-undo-dictation"));
      return;
    }
    say(t("ui-toast-select-zone", { text }), true);
  };

  if (!snap) return <div className="table"><p className="empty" style={{ padding: 40 }}>{t("ui-loading")}</p></div>;

  const cur = snap.slides.find((s) => s.id === current);
  const themePicker = (
    <label className="menu-theme">
      <span className="caption">{t("ui-toolbar-theme")}</span>
      <select className="select" value={snap.theme ?? ""} onChange={(e) => apply(t("ui-undo-theme"), [{ op: "set_theme", theme: e.target.value || null }])}>
        <option value="">{t("ui-toolbar-theme-default")}</option>
        {snap.themes.map((name) => <option key={name} value={name}>{name}</option>)}
      </select>
    </label>
  );
  const languagePicker = (
    <label className="menu-theme">
      <span className="caption">{t("ui-language")}</span>
      <select
        className="select"
        value={chosenLocale() ?? ""}
        onChange={(e) => changeLocale(e.target.value || null).catch((err) => say(errorText(err), true))}
      >
        <option value="">{t("ui-language-system")}</option>
        {availableLocales().map((code) => <option key={code} value={code}>{languageName(code)}</option>)}
      </select>
    </label>
  );
  const schemePicker = (
    <SchemePicker value={schemeChoice} onChange={(c) => changeScheme(c).catch((err) => say(errorText(err), true))} />
  );
  const menuEntries: MenuEntry[] = snap.editing
    ? [
        { kind: "item" as const, label: t("ui-toolbar-save-copy"), onSelect: () => saveTemplates(true) },
        { kind: "separator" },
        { kind: "custom", node: themePicker },
        ...viewOptions.entries,
        { kind: "separator" },
        { kind: "custom", node: schemePicker },
        { kind: "custom", node: languagePicker },
      ]
    : [
        { kind: "item", label: t("ui-toolbar-open"), shortcut: "⌘O", onSelect: open },
        { kind: "item", label: t("ui-merge"), onSelect: merge },
        { kind: "separator" },
        { kind: "item", label: t("ui-menu-move"), onSelect: moveFile },
        { kind: "item", label: t("ui-toolbar-save-copy"), shortcut: "⌘⇧S", onSelect: saveCopy },
        { kind: "separator" },
        { kind: "item", label: t("ui-toolbar-templates"), onSelect: () => void editTemplates() },
        { kind: "item", label: t("ui-widgets-edit"), onSelect: () => void widgets.enterAndOpen(editTemplates, null) },
        { kind: "item", label: t("ui-history"), onSelect: openHistory },
        { kind: "custom", node: themePicker },
        ...viewOptions.entries,
        { kind: "separator" },
        { kind: "custom", node: schemePicker },
        { kind: "custom", node: languagePicker },
      ];
  const templateNames: Record<string, string> = Object.fromEntries(snap.slides.map((s) => [s.id, s.name ?? s.title ?? t("ui-template-default-name")]));
  const status = t(snap.dirty ? "ui-status-unsaved" : snap.path ? "ui-status-saved" : "ui-status-never-saved");
  // Historique : ni sélection, ni édition, ni outils (la session les refuserait).
  const history = snap.history;
  const noop = () => {};

  return (
    <div className="table">
      {history ? (
        <HistoryBar
          history={history}
          cursor={cursor}
          onRestore={() => { cursor.cancel(); run(api.historyRestore()); }}
          onBack={() => { cursor.cancel(); run(api.historyClose()); }}
        />
      ) : (<>
      {snap.pack_notice && (
        <PackNoticeBar
          notice={snap.pack_notice}
          onUseMine={() => run(api.packUseMine())}
          onMakeCurrent={() => run(api.packMakeCurrent())}
          onClose={() => run(api.packNoticeClose())}
        />
      )}
      {/* Titre du document, discret, centré en haut ; la marque ne signale que l'édition des modèles. */}
      <header className={`label${snap.editing ? " editing-templates" : ""}`}>
        {snap.editing && <span className="brand">{t("ui-brand-templates")}</span>}
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

      {/* Ce qu'on ajoute à la slide, puis annuler / rétablir : posés sous la feuille. */}
      <div className="object theme-card tools-tray">
        <div className="add-group" role="group" aria-label={t("ui-toolbar-add-label")} hidden={widgetActive}>
          {ADD_KINDS.map((k) => {
            const name = t("ui-add-kind", { kind: k });
            return (
              <button key={k} className="token ghost" aria-label={name} title={name} disabled={!canAdd(k, view?.slide.kind)} onClick={() => addElement(k)}>
                {Icon[k]}
              </button>
            );
          })}
          {addWidgets.length > 0 && (
            <ActionMenu
              label={t("ui-add-widget")}
              icon={Icon.widget}
              triggerClass="token ghost"
              disabled={!canAdd("widget", view?.slide.kind)}
              entries={addWidgets.map((w) => ({ kind: "item", label: w.label ?? w.name, onSelect: () => addPackWidget(w) }))}
            />
          )}
        </div>
        <span className="tools-sep" aria-hidden hidden={widgetActive} />
        {widgetActive ? (<>
          {/* Sous-onglet widget : l'historique de son éditeur, puis son code. */}
          <button className="token ghost" aria-label={t("ui-toolbar-undo")} title={t("ui-toolbar-undo")} disabled={widgets.history.undo === 0} onClick={widgets.undo}>{Icon.undo}</button>
          <button className="token ghost" aria-label={t("ui-toolbar-redo")} title={t("ui-toolbar-redo")} disabled={widgets.history.redo === 0} onClick={widgets.redo}>{Icon.redo}</button>
          <span className="tools-sep" aria-hidden />
          <CodeSwitch checked={viewOptions.code} controls={`widget-code-${widgets.tabs.active}`} onChange={viewOptions.setCode} />
        </>) : (<>
          <button className="token ghost" aria-label={snap.undo_label ? t("ui-toolbar-undo-label", { label: snap.undo_label }) : t("ui-toolbar-undo")} title={snap.undo_label ?? t("ui-toolbar-undo")} disabled={!snap.can_undo} onClick={() => run(api.undo())}>{Icon.undo}</button>
          <button className="token ghost" aria-label={snap.redo_label ? t("ui-toolbar-redo-label", { label: snap.redo_label }) : t("ui-toolbar-redo")} title={snap.redo_label ?? t("ui-toolbar-redo")} disabled={!snap.can_redo} onClick={() => run(api.redo())}>{Icon.redo}</button>
        </>)}
      </div>

      {/* En haut à droite, tout ce qui sert rarement : un seul menu. */}
      <div className="tray">
        {snap.editing && <button className="object token" aria-label={t("ui-toolbar-finish")} title={t("ui-toolbar-finish")} onClick={finishTemplates}>{Icon.check}</button>}
        <ActionMenu label={t("ui-menu")} entries={menuEntries} />
      </div>
      </>)}

      {/* Pile de slides, à gauche. */}
      <SlidePile
        slides={snap.slides}
        current={current}
        svg={svgs.svg}
        names={snap.editing ? templateNames : undefined}
        label={t(snap.editing ? "ui-pile-templates" : "ui-pile-slides")}
        addLabel={t(snap.editing ? "ui-pile-add-template" : "ui-pile-add-slide")}
        marks={history ? pileMarks(history.changes) : undefined}
        // Un sous-onglet widget ouvert cède la place au modèle cliqué.
        onSelect={(id) => { showSlide(id); setSelected(null); setEditing(null); widgets.activate(null); }}
        onMove={history ? undefined : moveSlide}
        onAdd={history ? undefined : addSlide}
        removeLabel={t(snap.editing ? "ui-toolbar-remove-template" : "ui-toolbar-remove-slide")}
        canRemove={canRemoveSlide(snap.slides.length)}
        onRemove={history ? undefined : removeSlide}
      >
        {snap.editing && !history && <PileWidgets files={widgets.files} current={widgets.activeName} onOpen={widgets.open} onRemoved={widgets.removed} focus={widgets.pileFocus} />}
      </SlidePile>

      {/* La feuille, posée au centre. */}
      <main className={`sheet-area${history ? " read-only" : ""}`}>
        {/* Sous-onglet widget actif : la scène reste montée (sélection, défilement), cachée. */}
        <div id="templates-panel" className="templates-panel" role={snap.editing ? "tabpanel" : undefined} aria-labelledby={snap.editing ? "widget-tab-0" : undefined} hidden={widgetActive}>
        {view && view.slide.id === current ? (
          <Stage
            ref={stage}
            view={view}
            svgUrl={cur ? svgs.svg(cur.id, cur.hash) : null}
            selected={history ? null : selected}
            editing={history ? null : editing}
            onSelect={history ? noop : setSelected}
            onEdit={history ? noop : setEditing}
            onApply={history ? noop : apply}
            readOnly={!!history}
            templates={!!snap.editing}
            packWidgets={packWidgets}
            onOpenWidget={(name) => void widgets.enterAndOpen(editTemplates, name)}
            onNotice={(m) => say(m, true)}
            dictated={dictated}
            showGridAlways={viewOptions.grid}
            showGhosts={viewOptions.ghosts}
            overlay={
              sheet?.anchor
                ? {
                    target: sheet.anchor.element,
                    node: (
                      <Suspense fallback={null}>
                        <AgentSheet key={sheet.id} surface={sheet} hints={hintsOn(sheet)} onAction={sendAction} onError={(m) => say(m, true)} />
                      </Suspense>
                    ),
                  }
                : undefined
            }
            palette={snap.appearance.chart_palette}
          />
        ) : (
          <div className="stage-wrap empty">{t("ui-stage-rendering")}</div>
        )}
        </div>
        {snap.editing && !history && <WidgetPane api={api} widgets={widgets} trials={snap.widget_trials} code={viewOptions.code} />}
        {drawer && (
          <Suspense fallback={null}>
            <VariantDrawer key={drawer.id} surface={drawer} hints={hintsOn(drawer)} onAction={sendAction} onError={(m) => say(m, true)} />
          </Suspense>
        )}
      </main>

      {/* Cartes à droite de la feuille. */}
      <aside className="notes" aria-label={t("ui-notes")}>
        {/* Données du graphique sélectionné : hors de la feuille, pour ne rien masquer. */}
        {view && !history && selectedElement?.kind === "chart" && (
          <ChartPanel
            slide={view.slide.id}
            element={selectedElement}
            palette={snap.appearance.chart_palette}
            onApply={apply}
            onKind={(kind) => run(api.chartKind(t("ui-undo-chart-kind"), view.slide.id, selectedElement.id, kind))}
          />
        )}
        <SlideThumbsContext.Provider value={slideThumbs}>
          <AgentThread
            state={agentState}
            turns={routed.turns}
            loose={routed.loose}
            dock={routed.dock}
            keysTo={keyed?.surface.id ?? null}
            resyncs={resyncs}
            onAction={onSurfaceAction}
            onNewConversation={() => agent.newConversation().catch((err) => say(String(err), true))}
            onError={(m) => say(m, true)}
          />
        </SlideThumbsContext.Provider>
        <DictationDraft
          text={draft.text}
          dictations={draft.dictations}
          typing={draft.typing}
          busy={busy}
          insertTo={insertTo}
          onChange={(text) => setDraft((d) => ({ ...d, text }))}
          onInsert={insertDraft}
          onSend={sendDraft}
          onDiscard={closeDraft}
        />
        {panel.length > 0 && (
          <Suspense fallback={null}>
            <PanelSurface messages={panel} onAction={onPanelAction} onError={(m) => say(m, true)} />
          </Suspense>
        )}
      </aside>

      {/* Entre pile et feuille, entre feuille et fil : de quoi régler les largeurs. */}
      <ColumnDividers columns={viewOptions.columns} onChange={viewOptions.setColumns} />

      {/* Appareil de dictée, posé devant la feuille. */}
      {/* Montée aussi dans l'historique : l'écoute reste visible et s'arrête ; un texte dicté y reçoit le refus de la session. */}
      <Dictation
        destination={destination}
        vocabulary={dictationVocabulary}
        onText={onDictated}
        onError={(m) => say(m, true)}
        agentBusy={busy}
        onAgentCancel={cancelAgent}
        onType={typeDraft}
      />

      {picking && (
        <TemplatePicker
          themeKey={appearanceKey}
          initialSet={snap.editing ? snap.editing.set : snap.templates?.id ?? null}
          title={t(snap.editing ? "ui-picker-new-template" : "ui-picker-new-slide")}
          onPick={insertTemplate}
          onSetChange={adoptSet}
          onClose={() => setPicking(false)}
          api={api}
        />
      )}

      {modal && (
        <Suspense fallback={null}>
          <ConfirmModal key={modal.id} surface={modal} hints={hintsOn(modal)} onAction={sendAction} onError={(m) => say(m, true)} />
        </Suspense>
      )}

      {toast && <div className={`toast${toast.error ? " error" : ""}`} role={toast.error ? "alert" : "status"}>{toast.text}</div>}
    </div>
  );
}
