import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import PackNoticeBar from "./PackNoticeBar";
import { setTemplatesOp } from "./packOps";
import type { Anchor, AddWidget, Element, ImportTarget, Op, PackInfo, Paragraph, PileSectionName, SlideView, Snapshot, Surface, TemplateView } from "./api";
import { agentApi, errorText, inTauri, workspace, tabApi } from "./api";
import type { MenuEntry } from "./ActionMenu";
import { ActionMenu } from "./ActionMenu";
import { appearanceLook } from "./appearance";
import { useAppearance } from "./useAppearance";
import { AttachButton } from "./agent/AttachButton";
import { changeScheme, useSchemeChoice } from "./scheme";
import SchemePicker from "./SchemePicker";
import { copyAfterRefusedMove, findElement, gridPlacement, instantiate, literalToParagraphs, moveOps, neighbourSlide, newElementOps, newId, pasteOps, pickSlide, placeOnGridOps, previewLook, slideToFetch } from "./deck";
import { newElement, newPackWidget } from "./newElement";
import TemplatePicker from "./TemplatePicker";
import ImportDialog from "./ImportDialog";
import type { ImportJob } from "./useImportJob";
import { useFileActions } from "./useFileActions";
import SlidePile from "./SlidePile";
import { ColumnDividers } from "./ColumnDivider";
import Stage from "./Stage";
import { anchorBox, fieldKey, primaryFieldOf } from "./stageFields";
import { useFieldEditing } from "./useFieldEditing";
import { useSvgCache } from "./useSvgCache";
import { useToast } from "./useToast";
import { useThumbsOver } from "./useSlideThumbs";
import { useKeptUi } from "./useKeptUi";
import ThemeMenu from "./ThemeMenu";
import { availableLocales, changeLocale, chosenLocale, languageName, t } from "./i18n";
import Dictation from "./Dictation";
import { DictationDraft } from "./DictationDraft";
import { useDictation } from "./useDictation";
import { Icon } from "./icons";
import { moveCell, trackSize } from "./grid";
import type { AddKind } from "./slideTools";
import { ADD_KINDS, canAdd, canRemoveSlide, menuWidgets } from "./slideTools";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { NumberLangContext } from "./a2ui/numberLang";
import { AgentPanel, IDLE_AGENT, type AgentHandle, type ScopeSource } from "./agent/AgentPanel";
import { routeSurfaces } from "./agent/surfaces";
import { useAgentPicker } from "./agent/useAgentPicker";
import { agentUi } from "./agent/ui";
import { escapeAction } from "./agent/keys";
import { appKeyScope } from "./appKeyScope";
import { createSlot, SlotView } from "./agent/slot";
import { useAgentFocus } from "./agent/useAgentFocus";
import type { QuickTarget } from "./agent/useQuickKeys";
import { linkTransport, type LinkContext } from "./agent/link";
import { useMainLink } from "./agent/useMainLink";
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
import WidgetTools from "./WidgetTools";
import TemplateTabs from "./TemplateTabs";
import Presenter from "./Presenter";
import type { PresentMode } from "./present";
import { presentShortcut } from "./present";
import PileWidgets from "./PileWidgets";
import PileColumn from "./PileColumn";
import SlideNav from "./SlideNav";
import { slideTarget } from "./slideKeys";
import TitleField from "./TitleField";

/** Agent détaché : le dock n'est pas à l'écran, ses touches rapides ne le visent pas. */
const MAIN_PLACES: QuickTarget["kind"][] = ["sheet", "drawer", "modal"];

/** Un onglet : le deck de la session `tab`. */
export default function App({ tab, agentDetached, nonce, notice, onOpen, importJob, onImportPick, onImport, onImportClose, onHelp, onClose, onChange }: {
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

  // Presse-papiers : ⌘C, ⌘X et ⌘V passent par les événements du navigateur.
  // L'élément copié reste ici ; le presse-papiers du système reçoit son texte,
  // qui dit au collage si l'élément est toujours la dernière chose copiée.
  // Couper, comme dans un tableur, ne retire rien : l'élément est marqué, et
  // le collage le déplace (`move_element`), ids et éditions des pairs gardés.
  const clip = useRef<{ element: Element; text: string; cut?: { slide: string } } | null>(null);
  const [cutMark, setCutMark] = useState<{ slide: string; element: string } | null>(null);
  /** Abandonne la coupe : le presse-papiers redevient une simple copie. */
  const dropCut = useCallback(() => {
    if (clip.current) clip.current.cut = undefined;
    setCutMark(null);
  }, []);
  // L'élément coupé (ou sa slide) a disparu : plus rien à déplacer.
  useEffect(() => {
    if (!cutMark) return;
    const gone = (snap && !snap.slides.some((s) => s.id === cutMark.slide))
      || (view?.slide.id === cutMark.slide && !findElement(view.slide, cutMark.element));
    if (gone) dropCut();
  }, [snap, view, cutMark, dropCut]);
  useEffect(() => {
    const skip = (e: ClipboardEvent) => editing || dialogOpen || widgetActive || !view || snap?.history || snap?.read_only || (e.target as HTMLElement).closest?.('input, textarea, select, [contenteditable="true"]');
    const copy = (e: ClipboardEvent) => {
      if (skip(e) || !view || !selected) return false;
      const element = findElement(view.slide, selected);
      if (!element) return false;
      const text = primaryFieldOf(view, selected)?.text || element.kind;
      e.preventDefault();
      e.clipboardData?.setData("text/plain", text);
      clip.current = { element, text };
      setCutMark(null);
      return true;
    };
    const cut = (e: ClipboardEvent) => {
      if (!view || !selected || !copy(e) || !clip.current) return;
      clip.current.cut = { slide: view.slide.id };
      setCutMark({ slide: view.slide.id, element: selected });
    };
    const paste = (e: ClipboardEvent) => {
      if (skip(e) || !view) return;
      const text = e.clipboardData?.getData("text/plain") ?? "";
      if (clip.current && text === clip.current.text) {
        e.preventDefault();
        const { element, cut: from } = clip.current;
        if (!canAdd(element.kind, view.slide.kind)) return;
        const here = view;
        const copyHere = () => {
          const { id, ops } = pasteOps(here, element);
          return apply(t("ui-undo-paste"), ops).then((ok) => ok && setSelected(id));
        };
        if (!from) {
          copyHere();
          return;
        }
        // Déplacer, une seule fois : ensuite, le presse-papiers colle des copies.
        dropCut();
        const { id, ops } = moveOps(here, element, from.slide);
        // Refusé : une copie seulement si l'élément n'est plus sur sa slide
        // (retiré entre-temps). Sinon l'erreur de l'hôte, déjà affichée, suffit.
        apply(t("ui-undo-move"), ops).then(async (ok) => {
          if (ok) setSelected(id);
          else if (await copyAfterRefusedMove(snapRef.current?.slides ?? [], from.slide, element.id, (s) => api.slide(s))) copyHere();
        });
      } else if (text.trim() && canAdd("text", view.slide.kind)) {
        // Du texte venu d'ailleurs devient un élément texte ; son markdown reste littéral.
        e.preventDefault();
        dropCut();
        addElement("text", literalToParagraphs(text), t("ui-undo-paste"));
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
    /** Vrai si l'hôte a appliqué les ops. */
    async (label: string, ops: Op[], merge?: string) => {
      // Toute modification abandonne la coupe, comme dans un tableur.
      dropCut();
      return (await run(api.apply(label, ops, merge))) !== null;
    },
    [run, dropCut],
  );

  const { open, merge, autosave, moveFile, saveCopy } = useFileActions({ api, snap, snapRef, run, say, onOpen, onImportPick });

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
    const ops = newElementOps(view, element);
    apply(label, ops).then(() => {
      setSelected(element.id);
      if (edit) editWhenShown(element.id);
    });
  };

  const [picking, setPicking] = useState(false);
  // Une fenêtre de l'app (modèles, import) est ouverte : rien derrière elle ne réagit.
  const dialogOpen = picking || importJob !== null;
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

  // Réglages du modèle affiché, ou du widget du sous-onglet actif à ses valeurs d'exemple.
  // Sur l'onglet « Thème », aucun réglage du modèle sous la scène.
  const host = useHostSettings(api, snap, themeActive ? null : current, widgets.activeName, run, say);
  const selectedElement = view && selected ? findElement(view.slide, selected) : undefined;
  // Paramètres de l'élément sélectionné (résolu dans la vue), ou du widget de la slide. Sur
  // l'onglet « Thème », la colonne de droite montre les jetons : la cible n'est pas lue.
  const target = useTargetSettings(api, snap, current, selectedElement ?? null, widgetActive || themeActive, run, say);

  // --- Agent ------------------------------------------------------------------
  // Son état vit dans `AgentPanel` : un morceau de réponse ne re-rend pas `App`. `App` n'en reçoit
  // que des valeurs simples (`agentStatus`) et une poignée (consigne, action, arrêt du tour).
  const [agent] = useState(() => agentApi(tab));
  const [agentSlots] = useState(() => ({ sheet: createSlot(), drawer: createSlot(), modal: createSlot() }));
  const [agentStatus, setAgentStatus] = useState(IDLE_AGENT);
  // Le panneau des agents, dans le menu des réglages de la dictée.
  const agentPicker = useAgentPicker(agent, (m) => say(m, true));
  // Les fiches ancrées se posent sous leur élément, sur la slide affichée.
  const shown = useMemo(
    () => (view && view.slide.id === current ? { slide: view.slide.id, has: (a: Anchor) => anchorBox(view, a) !== null } : null),
    [view, current],
  );
  const route = useCallback((list: Surface[]) => routeSurfaces(list, shown), [shown]);
  const agentUiNow = useMemo(() => agentUi(current, selected, editField, widgets.activeName), [current, selected, editField, widgets.activeName]);
  const templates = !!snap?.editing;
  const inHistory = !!snap?.history;
  const pack = snap?.editing?.set ?? null;
  const scopeSource = useMemo<ScopeSource>(
    () => ({ kind: "drive", view: { templates, history: inHistory, pack, widget: widgets.activeName } }),
    [templates, inHistory, pack, widgets.activeName],
  );
  const agentHandle = useRef<AgentHandle>(null);
  const promptAgent = (text: string) => agentHandle.current?.prompt(text) ?? Promise.resolve(false);
  const cancelAgent = () => agentHandle.current?.cancel();
  const onSurfaceAction = (s: Surface, a: A2uiClientAction) => agentHandle.current?.onSurfaceAction(s, a);
  // Un focus de l'agent pendant l'édition d'un texte attend la fin de l'édition.
  const onAgentFocus = useAgentFocus({ slides: snap?.slides, view, current, editing, show: showSlide, select: setSelected });

  // --- Édition des modèles ------------------------------------------------
  // Un pack du deck seul (import) n'est pas dans la bibliothèque : l'atelier ne l'ouvre pas.
  const editTemplates = async (): Promise<boolean> => {
    if (!snap?.templates || snap.deck_pack) return false;
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

  /** Montre une slide, sans sélection ; un sous-onglet widget ouvert cède la place au modèle, « Thème » reste. */
  const goToSlide = (id: string) => {
    showSlide(id);
    setSelected(null);
    setEditing(null);
    if (widgetActive) widgets.activate(null);
  };

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
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Déjà prise par un composant (barre des sous-onglets, pile…) : pas deux actions pour une touche.
      if (e.defaultPrevented) return;
      // La présentation prend ses touches (`Presenter`) ; derrière elle, rien ne réagit.
      if (presenting) return;
      // Dans une saisie de texte, ou (touches seules) une surface A2UI ou le dock, les touches sont les leurs ; ⌘Z passe depuis un bouton.
      // Seuls ⌘' et ⌘⇧G, qu'aucun éditeur ne lie, y passent (t-364). Une fenêtre de l'agent ou de l'app ouverte : rien derrière elle ne réagit.
      const scope = appKeyScope({ modal: agentStatus.modal, dialog: dialogOpen, editing: !!editing }, e);
      if (scope === "none") return;
      // L'aide : `HelpBar` a ses touches ; rien ne s'édite ni ne se bascule.
      if (snap?.read_only) return;
      if (scope === "display") {
        keptUi.handleKey(e);
        return;
      }
      const mod = e.metaKey || e.ctrlKey;
      const presentMode = presentShortcut(e);
      if (presentMode && !snap?.editing && !snap?.history) {
        e.preventDefault();
        present(presentMode);
        return;
      }
      // Grille et fantômes se basculent partout, historique compris.
      if (keptUi.handleKey(e)) return;
      if (snap?.history) {
        // Lecture seule : ← / → (⇧ : par session) parcourent les versions ;
        // les raccourcis d'édition, qui prennent aussi les flèches, sont inertes.
        if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && !mod && !e.altKey) {
          e.preventDefault();
          cursor.arrow(e.key, e.shiftKey);
        }
        return;
      }
      // Slide précédente / suivante : ← → ↑ ↓ sans sélection (sinon elles déplacent l'élément), Page↑ Page↓, Début, Fin.
      if (snap && !widgetActive) {
        const to = slideTarget(e, snap.slides.findIndex((s) => s.id === current), snap.slides.length, { space: false, arrows: selected === null });
        if (to !== null) {
          e.preventDefault();
          goToSlide(snap.slides[to].id);
          return;
        }
      }
      // Sous-onglet widget : ⌘Z va à son éditeur (`useTemplateWidgets`), jamais aux modèles.
      if (mod && e.key.toLowerCase() === "z") {
        if (widgetActive) return;
        e.preventDefault();
        dropCut();
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
      } else if (e.key === "Escape" && cutMark) {
        // Échap abandonne d'abord la coupe ; l'appui suivant reprend son rôle.
        dropCut();
      } else if (e.key === "Escape") {
        const action = escapeAction({ selected: selected !== null, agentBusy: agentStatus.cancellable });
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
  const dictation = useDictation({ snap, view, selected, editing, apply, addElement, say, prompt: promptAgent, setDraft: agent.setDraft });

  // --- Fenêtre de l'agent : ce qu'elle apprend d'ici, ce qu'elle y écrit ------
  const [transport] = useState(() => linkTransport());
  const linkContext = useMemo<LinkContext>(
    () => ({ editing: editing !== null, insertTo: dictation.insertTo, ui: agentUiNow, staged: agentStatus.sheet ? agentStatus.sheetId : null, keysOff: !!editing || dialogOpen || widgetActive, look: deckLook }),
    [editing, dictation.insertTo, agentUiNow, agentStatus.sheet, agentStatus.sheetId, dialogOpen, widgetActive, deckLook],
  );
  useMainLink({ tab, detached: !!agentDetached, context: linkContext, insert: dictation.insertText, transport });

  if (!snap) return <div className="table"><p className="empty" style={{ padding: 40 }}>{t("ui-loading")}</p></div>;

  const cur = snap.slides.find((s) => s.id === current);
  const themePicker = <ThemeMenu theme={snap.theme} themeName={snap.theme_name} themes={snap.themes} onPick={(id) => run(api.setTheme(id))} />;
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
        ...keptUi.entries,
        { kind: "separator" },
        { kind: "custom", node: schemePicker },
        { kind: "custom", node: languagePicker },
        { kind: "separator" },
        { kind: "item", label: t("ui-help"), shortcut: "⌘?", onSelect: onHelp },
      ]
    : [
        { kind: "item", label: t("ui-present-screen"), shortcut: "⌘↩", onSelect: () => present("screen") },
        { kind: "item", label: t("ui-present-window"), shortcut: "⌘⇧↩", onSelect: () => present("window") },
        { kind: "separator" },
        { kind: "item", label: t("ui-toolbar-open"), shortcut: "⌘O", onSelect: open },
        { kind: "item", label: t("ui-merge"), onSelect: merge },
        { kind: "separator" },
        { kind: "item", label: t("ui-menu-move"), onSelect: moveFile },
        { kind: "item", label: t("ui-toolbar-save-copy"), shortcut: "⌘⇧S", onSelect: saveCopy },
        { kind: "separator" },
        {
          kind: "item",
          label: t("ui-menu-workshop"),
          disabled: snap.deck_pack,
          hint: snap.deck_pack ? t("ui-menu-workshop-deck-only") : undefined,
          onSelect: () => void editTemplates(),
        },
        { kind: "item", label: t("ui-history"), onSelect: openHistory },
        { kind: "custom", node: themePicker },
        ...keptUi.entries,
        { kind: "separator" },
        { kind: "custom", node: schemePicker },
        { kind: "custom", node: languagePicker },
        { kind: "separator" },
        { kind: "item", label: t("ui-help"), shortcut: "⌘?", onSelect: onHelp },
      ];
  const mainMenu = <ActionMenu label={t("ui-menu")} entries={menuEntries} triggerClass="token ghost" />;
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

      {/* Ce qu'on ajoute à la slide, les paramètres et le menu de ce qui sert rarement, puis
          annuler / rétablir : posés sous la feuille. */}
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
          {mainMenu}
          <span className="tools-sep" aria-hidden />
          <WidgetTools tab={activeWidget(widgets.tabs)} history={widgets.history} onUndo={widgets.undo} onRedo={widgets.redo} options={keptUi} />
        </>) : (<>
          {/* Les paramètres de contenu sous la slide : affichés ou masqués (gardé par l'hôte). */}
          <button className="token ghost" aria-pressed={!paramsHidden} aria-label={t(paramsHidden ? "ui-toolbar-params-show" : "ui-toolbar-params-hide")} title={t(paramsHidden ? "ui-toolbar-params-show" : "ui-toolbar-params-hide")} onClick={() => keptUi.setFolded("params", !paramsHidden)}>{Icon.panelBottom}</button>
          {mainMenu}
          <span className="tools-sep" aria-hidden />
          <button className="token ghost" aria-label={snap.undo_label ? t("ui-toolbar-undo-label", { label: snap.undo_label }) : t("ui-toolbar-undo")} title={snap.undo_label ?? t("ui-toolbar-undo")} disabled={!snap.can_undo} onClick={() => run(api.undo())}>{Icon.undo}</button>
          <button className="token ghost" aria-label={snap.redo_label ? t("ui-toolbar-redo-label", { label: snap.redo_label }) : t("ui-toolbar-redo")} title={snap.redo_label ?? t("ui-toolbar-redo")} disabled={!snap.can_redo} onClick={() => run(api.redo())}>{Icon.redo}</button>
        </>)}
      </div>

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
            onOpenWidget={snap.deck_pack ? undefined : (name) => void widgets.enterAndOpen(editTemplates, name)}
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
            view={view} readOnly={readOnly} selected={selectedElement} target={target} palette={snap.appearance.chart_palette}
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
          onPicker={agentPicker.onPicker}
          undoTurn={snap.undo_turn} onUndoTurn={(turn) => void run(api.undoTurn(turn))}
        />
        {!snap.read_only && !agentDetached && <DictationDraft
          text={dictation.draft.state.text}
          dictations={dictation.draft.state.dictations}
          typing={dictation.draft.state.typing}
          busy={agentStatus.busy}
          notice={agentStatus.notice}
          ready={agentStatus.ready}
          insertTo={dictation.insertTo}
          onChange={dictation.draft.change}
          onInsert={(text) => void dictation.actions.insertDraft(text)}
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
      />}

      {picking && (
        <TemplatePicker
          themeKey={deckLook}
          initialSet={snap.editing ? snap.editing.set : snap.templates?.id ?? null}
          title={t(snap.editing ? "ui-picker-new-template" : "ui-picker-new-slide")}
          onPick={insertTemplate}
          onSetChange={adoptSet}
          onClose={() => setPicking(false)}
          api={api}
        />
      )}

      {importJob && (
        <ImportDialog
          path={importJob.path}
          api={api}
          themes={snap.themes}
          busy={importJob.busy}
          error={importJob.error}
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
