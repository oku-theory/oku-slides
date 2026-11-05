// Host access: per-tab requests (`tabApi`) and the snapshot they return. The types describe the
// host's JSON. The workspace (`workspaceApi.ts`), the agent (`agentApi.ts`), the slide model and
// its ops (`slideTypes.ts`), received packs and guides (`receivedTypes.ts`), the request itself
// (`hostCall.ts`), file picks (`filePick.ts`) and updates (`updateApi.ts`) live beside it,
// re-exported here by name; dictation lives in `voiceApi.ts`, imported from there.

import type { Surface } from "./agentApi";
import type { DataEdit } from "./chart/dataEdit";
import { host } from "./hostCall";
import type { AddPackView, LayoutsView, PackInfo, PackKind, PackMode, PackPin, ThemeInfo, ThemePin, ThemeTabView } from "./packTypes";
import type { GuideKind, GuidesView, ReceivedReview, VersionRef } from "./receivedTypes";
import type { ExtendsEdit, Frame, ListOp, Op, SlideView, TokenAction } from "./slideTypes";
import type { View } from "./table/ShowAsButton";

export { errorText, inTauri, isMac, listenTauri } from "./hostCall";
export { closeStays, linkedTabs, tabDrag, tabPath, workspace } from "./workspaceApi";
export type { ClipReply, ImportPick, ImportTarget, OpenerOffer, SharedClip, TabAt, TabDocument, TabInfo, WindowView, WorkspaceView } from "./workspaceApi";
export { pickOpenPath, pickSavePath } from "./filePick";
export { subscribeUpdates, tauriUpdates } from "./updateApi";
export type { InstallMode, UpdateApi, UpdateEvents, UpdateOffer, UpdateProgress, UpdateStatus } from "./updateApi";
export type { Capabilities, CapabilityTotals, GuideKind, GuideOrigin, Guides, GuidesView, ReceivedRelation, ReceivedReview, VersionRef } from "./receivedTypes";
export type { Element, ElementStyle, ExtendsEdit, FieldView, Frame, HeadingLevel, ItemView, ListOp, ListView, ObjectTarget, Op, Paragraph, Params, Run, Slide, SlideView, SlotRef, Target, TextStyle, TokenAction } from "./slideTypes";
export { agentApi, takeCloseDecision } from "./agentApi";
export type { AgentEvent, AgentState, AgentUi, AgentUiSent, Anchor, CloseDecision, CloseTarget, InterfaceChange, McpEvent, PastedImage, Place, Surface } from "./agentApi";

export type SlideKind = string;

export interface SlideSummary {
  id: string;
  kind: SlideKind;
  title: string | null;
  page: number;
  hash: string;
}

export interface Appearance {
  scheme: "light" | "dark" | string;
  /** Couleurs des slides (bg, ink, accent…), en hex. */
  slide: Record<string, string>;
  /** Couleurs de l'interface (desk, sheet, piece, ink…) ; absentes = dérivées. */
  app: Record<string, string>;
  /** Couleurs de l'interface pour l'autre schéma (`dark`, `light`) ; absent = dérivé. */
  variants: Record<string, Record<string, string>>;
  /** radius-sm, radius-md, radius-lg (px), elevation. */
  metrics: Record<string, number>;
  font_display: string[];
  font_body: string[];
  /** Palette des graphiques du thème (jetons chart.palette), en hex. */
  chart_palette: string[];
}

/** Type d'hôte d'un paramètre de widget : le panneau ou l'éditeur qui le lit. */
export type HostType = "chart" | "table" | "path";

/** Paramètres d'hôte des widgets d'élément : kind, puis type d'hôte, puis nom du paramètre. */
export type HostParams = Record<string, Partial<Record<HostType, string>>>;

export interface Snapshot {
  path: string | null;
  name: string;
  dirty: boolean;
  /** Espace de noms du thème du deck. */
  theme: string | null;
  /** Thème du deck et ses parents, épinglés par espace de noms. */
  theme_pins: Record<string, ThemePin>;
  /** Nom du thème tel qu'embarqué par le deck. */
  theme_name: string | null;
  /** Thèmes de la bibliothèque dans leur version courante : intégrés, puis ceux de l'utilisateur. */
  themes: ThemeInfo[];
  title: string | null;
  title_source: string | null;
  /** Langue du deck (`params.lang`) : les grilles écrivent les nombres comme les slides. */
  lang: string | null;
  slides: SlideSummary[];
  can_undo: boolean;
  undo_label: string | null;
  can_redo: boolean;
  redo_label: string | null;
  /** Le tour de l'agent dont l'étape est au sommet de la pile d'annulation : son reçu propose de l'annuler. */
  undo_turn: number | null;
  warnings: string[];
  notice: string | null;
  /** Le dernier enregistrement n'a rien écrit : le fichier est devenu un autre deck (vrai une fois). */
  disk_replaced: boolean;
  appearance: Appearance;
  /** Packs de widgets épinglés par le deck, par espace de noms. */
  widgets: Record<string, PackPin>;
  /** Packs de layouts épinglés par le deck, par espace de noms. */
  layouts: Record<string, PackPin>;
  /** Édition d'un pack en cours (mode pack). */
  editing: PackMode | null;
  /** The theme draft of a theme tab; null in any other tab. */
  theming: ThemeTabView | null;
  /** Le pack du deck n'appartient qu'à lui (import) : jamais dans la bibliothèque. */
  deck_pack: boolean;
  /** Épinglages (packs et thème) plus anciens que ma version courante : la pastille du bouton des packs. */
  pack_updates: number;
  /** Widgets des packs en cours, qualifiés `<namespace>/<nom>` (sans `_commun`), sans sonde. */
  pack_widgets: string[];
  /** La grille d'ajout : les widgets d'élément de chaque pack de widgets épinglé, avec leur départ. */
  add_packs: AddPackView[];
  /**
   * Le premier slot libre de chaque layout épinglé et les widgets qu'il accepte (`null` : tous).
   * Un layout sans slot libre est absent : ses slides n'acceptent pas d'élément ajouté.
   */
  free_slots: Record<string, string[] | null>;
  /** Révision des valeurs d'exemple des widgets : change quand une valeur est modifiée (tirée de la version du pack édité). */
  widget_trials: number;
  /** Fingerprint of the packs the deck renders with: add grid previews depend on all of them. */
  preview_set: string;
  /** Historique ouvert : la version montrée (le reste du snapshot la décrit). */
  history: HistoryView | null;
  /** L'aide de l'app : le deck se lit, il ne se modifie pas. */
  read_only: boolean;
  /** The tab can present its deck (`Session::can_present`): Present follows it. */
  can_present: boolean;
  ui: UiState;
  /** Paramètres d'hôte des widgets d'élément du deck, par kind et type d'hôte. */
  host_params: HostParams;
  /** The edited pack's or theme's agent guides; `null` outside a pack or theme tab. */
  guides: GuidesView | null;
  /** Received versions the library lacks or whose guides await approval, to validate (t-1414). */
  received_to_review: ReceivedReview[];
}


/** Une slide où le texte cherché apparaît. */
export interface SearchHit {
  slide: string;
  title: string | null;
  /** La ligne du corps trouvée, raccourcie autour de la correspondance ; `null` : seul le titre correspond. */
  excerpt: string | null;
}

/** Une version du deck (un commit), de la plus ancienne à la plus récente. */
export interface Version {
  /** Nom de la session qui l'a faite ; `null` : anonyme. */
  author: string | null;
  /** Secondes Unix. */
  timestamp: number;
  label: string | null;
  /** Première version d'une session. */
  session_start: boolean;
  merge: boolean;
}

/** Slides du deck courant comparées à la version montrée (identifiants). */
export interface SlideChanges {
  added_since: string[];
  removed_since: string[];
  changed_since: string[];
}

/** Le cran montré : `index` de 0 à `count - 1` (la plus récente). */
export interface HistoryView {
  index: number;
  count: number;
  version: Version;
  changes: SlideChanges;
}

/** Current slide, ghosts, columns, selected item, code and folded sections, kept across launches. */
export interface UiState {
  slide: string | null;
  /** Fantômes visibles : zone, objets et champs annotés. */
  ghosts: boolean;
  /** Largeur de la pile en px, choisie au séparateur ; `null` : largeur par défaut. */
  pile: number | null;
  /** Largeur du fil de l'agent en px, choisie au séparateur ; `null` : largeur par défaut. */
  notes: number | null;
  /** A pack tab's selected item (widget or layout file); `null`: none. */
  widget: string | null;
  /** A pack item's Code tab is open, rather than its Settings tab (the default). */
  code: boolean;
  /** Folded sections (thumbnails); all open by default. */
  folded: PileSectionName[];
  /** A pack tab's preview slot for element widgets; full by default. */
  preview_span: PreviewSpan;
  /** A deck tab's inspector column is open; closed by default. */
  inspector: boolean;
  /** A deck tab's data pane under the stage is open; closed by default. */
  data_pane: boolean;
}

/** The share of the grid an element widget's preview spans. */
export type PreviewSpan = "full" | "half" | "third";

/** Ordre d'une sauvegarde de l'état d'interface : l'hôte ignore une sauvegarde plus ancienne (même `page`,
 *  `seq` inférieur ou égal) que la dernière appliquée ; une autre `page` (rechargée) est toujours acceptée. */
export interface UiOrder {
  page: string;
  seq: number;
}

/** A foldable section: a deck's slides, or the layouts of a theme tab. */
export type PileSectionName = "slides" | "layouts";

/** Un message de Typst (ou du contrôle) sur un fichier de widget ; ligne et colonne à partir de 1. */
export interface WidgetDiagnostic {
  file: string | null;
  line: number | null;
  column: number | null;
  error: boolean;
  message: string;
}

/** Un fichier de widget du pack en cours et son statut. */
export interface WidgetStatus {
  name: string;
  /** Schéma du widget s'il est valide (`name` qualifié `espace/nom`) ; `null` pour `_commun`. */
  schema: { name: string } | null;
  diagnostics: WidgetDiagnostic[];
}

/** Answer to a duplicate: the copy's name and the statuses after it. */
export interface WidgetDuplicate {
  name: string;
  statuses: WidgetStatus[];
}

/** Réponse d'une écriture : le fichier, et les widgets qu'elle a cassés. */
export interface WidgetChange {
  status: WidgetStatus;
  broken: WidgetStatus[];
  /** L'écriture est enregistrée ; faux en mode deck quand le rendu échoue (rien n'est enregistré). */
  saved: boolean;
}

/** Aperçu d'un widget seul : SVG, ou les diagnostics qui l'en empêchent. */
export interface WidgetPreview {
  svg: string | null;
  /** Ce que dessine un widget d'élément sur la slide (pt) ; `null` pour un widget de slide. */
  bbox?: Frame | null;
  /**
   * La slide d'aperçu entière, sans cibles ; champs et items (ceux du seul
   * widget montré, comme `lists`) en pt de la slide. L'image couvre `bbox`
   * (un champ s'y place en `x - bbox.x`, `y - bbox.y`), ou toute la slide
   * sans `bbox`. `null` sans rendu.
   */
  view?: SlideView | null;
  diagnostics: WidgetDiagnostic[];
}


type Request =
  | { cmd: "snapshot" }
  | { cmd: "save"; path: string | null; overwrite: boolean }
  | { cmd: "merge"; path: string }
  | { cmd: "save_copy"; path: string }
  | { cmd: "slide"; id: string }
  | { cmd: "svg"; id: string }
  | { cmd: "apply"; label: string; ops: Op[]; merge: string | null }
  | { cmd: "set_preview_theme"; theme: string }
  | { cmd: "undo" }
  | { cmd: "redo" }
  | { cmd: "undo_turn"; turn: number }
  | { cmd: "layouts" }
  | { cmd: "packs"; kind: PackKind }
  | { cmd: "save_pack"; as_new: boolean }
  | { cmd: "delete_pack"; pack: string }
  | { cmd: "use_pack_version"; pack: string; version: string; deck_uid: string }
  | { cmd: "use_pack_copy"; from: string; pack: string; version: string; deck_uid: string }
  | { cmd: "use_theme_version"; theme: string; version: string; deck_uid: string }
  | { cmd: "rename_edited_theme"; name: string }
  | { cmd: "set_edited_theme_extends"; extends: string[] }
  | { cmd: "edit_theme_extends"; edit: ExtendsEdit; theme: string }
  // Answers the saved ThemePin, or null when unchanged.
  | { cmd: "save_theme"; as_new: boolean }
  | { cmd: "host_settings"; widget: string | null }
  | { cmd: "data_panel"; slide: string; element: string }
  | { cmd: "data_edit"; slide: string; element: string; edit: DataEdit }
  | { cmd: "element_panel"; slide: string; element: string }
  | { cmd: "element_param"; slide: string; element: string; key: string; value: unknown }
  | { cmd: "element_list"; slide: string; element: string; key: string; change: ListOp }
  // Demandées par l'agent (outils MCP) ; l'interface passe par `data_edit`.
  | { cmd: "chart_kind"; label: string; slide: string; element: string; kind: string }
  | { cmd: "show_as"; label: string; slide: string; element: string; view: View }
  | { cmd: "set_example_value"; name: string; param: string; value: unknown }
  | { cmd: "edit_example_list"; name: string; param: string; change: ListOp }
  | { cmd: "host_theme"; filter: string }
  | { cmd: "host_token"; path: string[]; action: TokenAction; value?: unknown }
  | ({ cmd: "ui"; order?: UiOrder } & UiState)
  | { cmd: "history_open" }
  | { cmd: "history_seek"; index: number }
  | { cmd: "history_restore" }
  | { cmd: "history_close" }
  | { cmd: "widget_status"; pack?: string }
  | { cmd: "widget_source"; name: string; pack?: string }
  | { cmd: "set_widget"; name: string; source: string; pack?: string }
  | { cmd: "set_guide"; which: GuideKind; text: string }
  | { cmd: "approve_guides"; versions: VersionRef[] }
  | { cmd: "accept_received"; versions: VersionRef[] }
  | { cmd: "later_received"; versions: VersionRef[] }
  | { cmd: "remove_widget"; name: string; pack?: string }
  | { cmd: "rename_widget"; from: string; to: string; pack?: string }
  | { cmd: "duplicate_widget"; from: string; to?: string; pack?: string }
  | { cmd: "reorder_widgets"; order: string[]; pack?: string }
  | { cmd: "preview_widget"; name: string; pack?: string; span?: PreviewSpan }
  | { cmd: "history_versions" }
  | { cmd: "search"; query: string };

/** Schéma de l'interface gardé par l'hôte ; `null` : celui du système. */
export interface SchemeSetting {
  scheme: "light" | "dark" | null;
}

/** Langue de l'utilisateur et ressources Fluent de l'interface (anglais d'abord). */
export interface Messages {
  locale: string;
  resources: string[];
  /** Langue choisie ; `null` : celle du système. */
  chosen: string | null;
  /** The theme shortcut's comma-separated words, by shipped locale (`ui-shortcut-theme`). */
  shortcuts: Record<string, string>;
}

/** Les requêtes d'un onglet : chacune ne touche que sa session. */
export function tabApi(tab: string) {
  const call = <T>(req: Request) => host<T>("request", { tab, req }, "/api", { tab, req });
  return {
    snapshot: () => call<Snapshot>({ cmd: "snapshot" }),
    save: (path: string | null, overwrite = false) => call<Snapshot>({ cmd: "save", path, overwrite }),
    /** Fusionne un autre PDF du même deck ; l'avis de fusion arrive par `notice`. */
    merge: (path: string) => call<Snapshot>({ cmd: "merge", path }),
    /** Écrit une copie ailleurs ; le deck garde son fichier. */
    saveCopy: (path: string) => call<Snapshot>({ cmd: "save_copy", path }),
    slide: (id: string) => call<SlideView>({ cmd: "slide", id }),
    svg: (id: string) => call<string>({ cmd: "svg", id }),
    /** Surface A2UI des réglages du widget à ses valeurs d'exemple ; vide hors du mode pack ou sans widget. */
    hostSettings: (widget: string | null) => call<Surface | null>({ cmd: "host_settings", widget }),
    /** Surface `host-data` du graphique ou du tableau `element` ; `null` pour un autre élément. */
    dataPanel: (slide: string, element: string) => call<Surface | null>({ cmd: "data_panel", slide, element }),
    /** Applique une modification de la surface `host-data` : une étape d'annulation. */
    dataEdit: (slide: string, element: string, edit: DataEdit) => call<Snapshot>({ cmd: "data_edit", slide, element, edit }),
    /** `host-element` surface: the settings of `element`; `null` for an unknown element or outside an editable deck. */
    elementPanel: (slide: string, element: string) => call<Surface | null>({ cmd: "element_panel", slide, element }),
    /** Sets the param `key` of the element (`null` clears it): one undo step. */
    elementParam: (slide: string, element: string, key: string, value: unknown) =>
      call<Snapshot>({ cmd: "element_param", slide, element, key, value }),
    /** Changes the list `key` of the element (`param/item/column` for a nested list): one undo step. */
    elementList: (slide: string, element: string, key: string, change: ListOp) =>
      call<Snapshot>({ cmd: "element_list", slide, element, key, change }),
    /** Règle une valeur d'exemple d'un widget, dans le pack (mode pack). */
    setExampleValue: (name: string, param: string, value: unknown) =>
      call<Snapshot>({ cmd: "set_example_value", name, param, value }),
    /** Change la liste `param` des valeurs d'exemple ; sans valeur gardée, elle part des items par défaut du schéma. */
    editExampleList: (name: string, param: string, change: ListOp) =>
      call<Snapshot>({ cmd: "edit_example_list", name, param, change }),
    apply: (label: string, ops: Op[], merge?: string) =>
      call<Snapshot>({ cmd: "apply", label, ops, merge: merge ?? null }),
    /** Previews a pack tab's working deck in theme `theme` (a namespace), silently: no undo step. */
    setPreviewTheme: (theme: string) => call<Snapshot>({ cmd: "set_preview_theme", theme }),
    undo: () => call<Snapshot>({ cmd: "undo" }),
    redo: () => call<Snapshot>({ cmd: "redo" }),
    /** Annule le tour `turn` de l'agent, refusé s'il n'est plus la dernière modification. */
    undoTurn: (turn: number) => call<Snapshot>({ cmd: "undo_turn", turn }),
    /** Layouts des packs de layouts épinglés, rendus dans le thème du deck. */
    layouts: () => call<LayoutsView>({ cmd: "layouts" }),
    /** Packs de la bibliothèque de sorte `kind`, dans leur version courante. */
    packs: (kind: PackKind) => call<PackInfo[]>({ cmd: "packs", kind }),
    /** Enregistre le pack édité (une copie : l'agent, outil `save_pack`). */
    savePack: () => call<Snapshot>({ cmd: "save_pack", as_new: false }),
    /**
     * Pins an offered pack version if it still holds (sends `use_pack_version`).
     *
     * Not named `usePackVersion`: hook lint rules would read a `use…` name as a hook.
     */
    takePackVersion: (pack: string, version: string, deckUid: string) =>
      call<Snapshot>({ cmd: "use_pack_version", pack, version, deck_uid: deckUid }),
    /** Switches from pack `from` to an offered copy if the offer still holds (sends `use_pack_copy`). */
    takePackCopy: (from: string, pack: string, version: string, deckUid: string) =>
      call<Snapshot>({ cmd: "use_pack_copy", from, pack, version, deck_uid: deckUid }),
    /** Pins an offered version of the deck's theme if it still holds (sends `use_theme_version`). */
    takeThemeVersion: (theme: string, version: string, deckUid: string) =>
      call<Snapshot>({ cmd: "use_theme_version", theme, version, deck_uid: deckUid }),
    /** Supprime un pack de l'utilisateur, toutes versions ; répond les packs restants de sa sorte. */
    deletePack: (pack: string) => call<PackInfo[]>({ cmd: "delete_pack", pack }),
    /** Fichiers de widgets du pack en cours, `_commun` en tête. */
    widgetStatus: (pack?: string) => call<WidgetStatus[]>({ cmd: "widget_status", pack }),
    widgetSource: (name: string, pack?: string) => call<string>({ cmd: "widget_source", name, pack }),
    /** Crée ou remplace un fichier ; en mode deck, le deck passe à la nouvelle version de son pack. */
    setWidget: (name: string, source: string, pack?: string) => call<WidgetChange>({ cmd: "set_widget", name, source, pack }),
    /** Writes the edit or use guide of the edited pack or theme; empty text clears it. */
    setGuide: (which: GuideKind, text: string) => call<Snapshot>({ cmd: "set_guide", which, text }),
    /** Approves the guides of these received versions for good. */
    approveGuides: (versions: VersionRef[]) => call<Snapshot>({ cmd: "approve_guides", versions }),
    /** Adds these received versions to the library and approves their guides, as validated. */
    acceptReceived: (versions: VersionRef[]) => call<Snapshot>({ cmd: "accept_received", versions }),
    /** Puts the review of these versions off for this tab; the deck asks again when reopened. */
    laterReceived: (versions: VersionRef[]) => call<Snapshot>({ cmd: "later_received", versions }),
    removeWidget: (name: string, pack?: string) => call<WidgetStatus[]>({ cmd: "remove_widget", name, pack }),
    renameWidget: (from: string, to: string, pack?: string) => call<WidgetStatus[]>({ cmd: "rename_widget", from, to, pack }),
    /** Copies an item under a fresh name the session picks; answers that name and the statuses. */
    duplicateWidget: (from: string, pack?: string) => call<WidgetDuplicate>({ cmd: "duplicate_widget", from, pack }),
    /** Stores the pile's order (every item once, `_commun` excluded); answers the statuses. */
    reorderWidgets: (order: string[], pack?: string) => call<WidgetStatus[]>({ cmd: "reorder_widgets", order, pack }),
    /** Aperçu d'un widget à ses valeurs d'exemple, sans `params` (ceux de l'agent) : la session les prend vides. */
    previewWidget: (name: string, pack?: string, span: PreviewSpan = "full") =>
      call<WidgetPreview>({ cmd: "preview_widget", name, pack, span }),
    /** Theme tab: saves the draft; the saved version, or null when unchanged. */
    saveTheme: (asNew = false) => call<ThemePin | null>({ cmd: "save_theme", as_new: asNew }),
    /** Theme tab: renames the draft (undoable). */
    renameEditedTheme: (name: string) => call<Snapshot>({ cmd: "rename_edited_theme", name }),
    /** Theme tab: changes the draft's `extends` list by a delta the session applies (undoable). */
    editThemeExtends: (edit: ExtendsEdit, theme: string) => call<Snapshot>({ cmd: "edit_theme_extends", edit, theme }),
    /** The A2UI surface of the draft's tokens, filtered; `null` outside a theme tab. */
    hostTheme: (filter: string) => call<Surface | null>({ cmd: "host_theme", filter }),
    /** Changes token `path` of the draft (theme tab, undoable). */
    hostToken: (path: string[], action: TokenAction, value?: unknown) =>
      call<Snapshot>({ cmd: "host_token", path, action, ...(value === undefined ? {} : { value }) }),
    ui: (ui: UiState, order?: UiOrder) => call<Snapshot>({ cmd: "ui", ...ui, order }),
    /** Ouvre l'historique (lecture seule) sur la version la plus récente. */
    historyOpen: () => call<Snapshot>({ cmd: "history_open" }),
    historySeek: (index: number) => call<Snapshot>({ cmd: "history_seek", index }),
    /** Restaure la version montrée : nouveau commit, annulable, retour au deck. */
    historyRestore: () => call<Snapshot>({ cmd: "history_restore" }),
    /** Referme l'historique : le deck courant, intact. */
    historyClose: () => call<Snapshot>({ cmd: "history_close" }),
    historyVersions: () => call<Version[]>({ cmd: "history_versions" }),
    /** Les slides où le texte apparaît (sans casse ni accents), titres d'abord. */
    search: (query: string) => call<SearchHit[]>({ cmd: "search", query }),
  };
}

/** Les requêtes d'un onglet (voir `tabApi`). */
export type TabApi = ReturnType<typeof tabApi>;
