// Accès à l'hôte : requêtes par onglet, agent et dictée. Les types décrivent le JSON de
// l'hôte. L'espace de travail (`workspaceApi.ts`), la requête elle-même (`hostCall.ts`) et
// les choix de fichier (`filePick.ts`) et les mises à jour (`updateApi.ts`) vivent à côté, réexportés ici par leur nom.

import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import type { DataOp } from "./chart/chartTypes";
import type { DataEdit } from "./chart/dataEdit";
import type { Cell, ZoneView } from "./grid";
import { locale } from "./i18n";
import type { AddPackView, LayoutsView, PackInfo, PackKind, PackMode, PackPin, ThemeInfo, ThemePin } from "./packTypes";
import type { PresentMode } from "./present";
import type { View } from "./table/ShowAsButton";
import { host } from "./hostCall";

export { errorText, inTauri, isMac, listenTauri } from "./hostCall";
export { tabDrag, workspace } from "./workspaceApi";
export type { ImportPick, ImportTarget, TabAt, TabInfo, WindowView, WorkspaceView } from "./workspaceApi";
export { pickOpenPath, pickSavePath } from "./filePick";
export { subscribeUpdates, tauriUpdates } from "./updateApi";
export type { InstallMode, UpdateApi, UpdateEvents, UpdateOffer, UpdateProgress, UpdateStatus } from "./updateApi";

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
  /** Historique ouvert : la version montrée (le reste du snapshot la décrit). */
  history: HistoryView | null;
  /** L'aide de l'app : le deck se lit, il ne se modifie pas. */
  read_only: boolean;
  ui: UiState;
  /** Paramètres d'hôte des widgets d'élément du deck, par kind et type d'hôte. */
  host_params: HostParams;
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

/** Slide courante, grille, fantômes, colonnes, sous-onglets, code des widgets et sections repliées, gardés par l'hôte d'un lancement à l'autre. */
export interface UiState {
  slide: string | null;
  grid: boolean;
  /** Fantômes visibles : zone, objets et champs annotés. */
  ghosts: boolean;
  /** Largeur de la pile en px, choisie au séparateur ; `null` : largeur par défaut. */
  pile: number | null;
  /** Largeur du fil de l'agent en px, choisie au séparateur ; `null` : largeur par défaut. */
  notes: number | null;
  /** Sous-onglets de widgets ouverts en mode pack, dans l'ordre d'ouverture. */
  widgets: string[];
  /** Sous-onglet actif ; `null` : la scène, ou « Thème ». */
  widget: string | null;
  /** Code source des widgets déplié à droite de l'aperçu ; replié par défaut. */
  code: boolean;
  /** Largeur du code en px, choisie au séparateur ; `null` : proportion par défaut. */
  code_width: number | null;
  /** Sections repliées (miniatures, widgets) ; toutes ouvertes par défaut. */
  folded: PileSectionName[];
}

/** Ordre d'une sauvegarde de l'état d'interface : l'hôte ignore une sauvegarde plus ancienne (même `page`,
 *  `seq` inférieur ou égal) que la dernière appliquée ; une autre `page` (rechargée) est toujours acceptée. */
export interface UiOrder {
  page: string;
  seq: number;
}

/** Section repliable : slides hors mode pack, layouts et widgets en mode pack. */
export type PileSectionName = "slides" | "layouts" | "widgets";

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

export interface Frame {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Run {
  t: string;
  bold?: boolean;
  italic?: boolean;
  color?: string;
  /** Cible du lien (`http`, `https` ou `mailto`). */
  link?: string;
  /** Pastille : numéro (dès 1) de la couleur de fond dans la palette du thème. */
  highlight?: number;
}

export interface Paragraph {
  runs: Run[];
  list?: "bullet" | "number";
}

/** Paramètres d'une slide ou d'un élément : texte, slots d'éléments, valeurs, selon le widget. */
export type Params = Record<string, unknown>;

export interface Element {
  id: string;
  kind: string;
  frame?: Frame;
  /** Accroche à la grille de la slide (prioritaire sur `frame`). */
  cell?: Cell;
  style?: Record<string, unknown>;
  params?: Params;
}

export interface Slide {
  id: string;
  kind: SlideKind;
  params?: Params;
  notes?: string;
}

/** Typographie du rendu Typst (pt), pour caler l'éditeur sur le texte. */
export interface TextStyle {
  size: number;
  leading: number;
  tracking: number;
  font: string[];
  weight: number | string;
  fill: string | null;
  align: "left" | "center" | "right" | null;
  case: "upper" | "lower" | null;
  bg: string | null;
}

/** Objet sélectionnable de la vue (élément à l'écran) ; nom distinct de `Target` (adressage des ops). */
export interface ObjectTarget {
  id: string;
  kind: string;
  free: boolean;
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Champ de texte éditable en place : paramètre d'une slide ou d'un élément, avec sa boîte rendue. */
export interface FieldView {
  /** Id de la slide ou de l'élément qui porte le champ. */
  owner: string;
  owner_kind: "slide" | "element";
  /** Widget de l'objet. */
  widget: string;
  /** Nom du paramètre, ou `liste/id/colonne` pour un item de liste (`liste/id/sous-liste/sous-id/colonne` imbriqué). */
  field: string;
  /** Pour un item : le chemin de sa liste, le paramètre `list` ou `liste/id/colonne` (liste imbriquée). */
  list: string | null;
  /** Pour un item : son id. */
  item: string | null;
  /** Paramètre, ou colonne de la liste pour un item. */
  column: string;
  kind: "plain" | "rich";
  /** Libellé du paramètre dans le schéma du widget. */
  label: string | null;
  /** Texte tel que saisi ; markdown, un paragraphe par ligne, pour `rich`. */
  text: string;
  /** Sans texte rendu : placeholder à dessiner. */
  empty: boolean;
  /** Lié à un paramètre du deck (`bind`) : en lecture dans l'atelier. */
  bound: boolean;
  /** Champ vide sans place au rendu : dessiné avant ou après son ancre (`y`). */
  side: "before" | "after" | null;
  x: number;
  y: number;
  w: number;
  h: number;
  /** Zone disponible pour le texte : début et largeur (contient la boîte). */
  ax: number;
  aw: number;
  style: TextStyle | null;
}

/** Un item de liste sur la page (boîte en pt), avec le rang et les bornes de sa liste. */
export interface ItemView {
  /** Id de la slide ou de l'élément qui porte la liste. */
  owner: string;
  owner_kind: "slide" | "element";
  /** Chemin de la liste : le paramètre `list`, ou `liste/id/colonne` pour une liste imbriquée. */
  list: string;
  /** Id de l'item. */
  item: string;
  index: number;
  /** Nombre d'items de la liste. */
  len: number;
  min: number;
  /** Items au plus : le `max` du schéma, sinon la borne du cœur. */
  max: number;
  /** Nom d'un item, au singulier (« étape »). */
  item_label: string | null;
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Une liste de la page, même vide (sans item, aucune boîte n'offre de poignée). */
export type ListView = Omit<ItemView, "item" | "index" | "x" | "y" | "w" | "h">;

export interface SlideView {
  slide: Slide;
  page: number;
  hash: string;
  width: number;
  height: number;
  targets: ObjectTarget[];
  /** Champs éditables, dans l'ordre de Tab. */
  fields: FieldView[];
  /** Items des listes, dans l'ordre des boîtes du rendu. */
  items: ItemView[];
  /** Listes des objets de la page, une par paramètre `list`. */
  lists: ListView[];
  /** Zone des éléments libres (pt, page) et sa grille résolue. */
  zone: ZoneView | null;
}

/** Adressage d'une slide ou d'un élément par les ops. */
export interface Target {
  slide: string;
  element?: string;
}

/** Adressage d'un slot (paramètre qui contient des éléments) par les ops. */
export interface SlotRef {
  owner: Target;
  param: string;
}

// --- Dictée ---------------------------------------------------------------

export type SttModel = "apple" | "turbo" | "large" | "base" | "api";
export type SttState = "idle" | "downloading" | "recording" | "transcribing";

/** Service de dictée compatible OpenAI ; la clé n'est jamais ici, seulement le nom de sa variable. */
export interface SttApiConfig {
  url: string;
  model: string;
  key_env: string;
}

export interface SttConfig {
  model: SttModel;
  language: string | null;
  /** Arrêter l'écoute seule après un silence de `silence` secondes. */
  auto_stop: boolean;
  silence: number;
  /** Version des réglages (tenue par l'hôte). */
  version: number;
  api: SttApiConfig;
}

export interface SttStatus {
  state: SttState;
  config: SttConfig;
  /** `available` faux : le modèle ne gère pas la langue choisie. */
  models: { id: SttModel; label: string; size: number; installed: boolean; available: boolean }[];
  download: { model: SttModel; received: number; total: number } | null;
  level: number;
  /** Voix détectée en ce moment. */
  speaking: boolean;
  /** Phrases déjà transcrites (définitives) et phrase en cours (aperçu). */
  committed: string;
  partial: string;
  detected: string | null;
  /** Secondes avant l'arrêt automatique, pendant un silence. */
  stop_in: number | null;
  text: string | null;
  seq: number;
  seconds: number;
  error: string | null;
}

type SttRequest =
  | { cmd: "status" }
  | ({ cmd: "configure" } & SttConfig)
  | { cmd: "start"; vocabulary: string[] }
  | { cmd: "stop" }
  | { cmd: "cancel" }
  | { cmd: "download"; model: SttModel };

const sttCall = (req: SttRequest) => host<SttStatus>("stt", { req }, "/api/stt", req);

export const stt = {
  status: () => sttCall({ cmd: "status" }),
  configure: (config: SttConfig) => sttCall({ cmd: "configure", ...config }),
  start: (vocabulary: string[]) => sttCall({ cmd: "start", vocabulary }),
  stop: () => sttCall({ cmd: "stop" }),
  cancel: () => sttCall({ cmd: "cancel" }),
  download: (model: SttModel) => sttCall({ cmd: "download", model }),
};

/** Style d'un élément (`Style` du cœur) : propriétés posées par `set_style`, remplacées par `replace_style`. */
export type ElementStyle = Record<string, unknown>;

/**
 * Une opération sur le deck : miroir de l'enum Rust `Op` (`oku-slides-core`, `ops/mod.rs`),
 * étiquetée par `op`. Une variante ajoutée côté Rust s'ajoute ici.
 */
export type Op =
  | { op: "set_param"; target: Target; param: string; value: unknown }
  | { op: "set_deck_param"; key: string; value: unknown }
  | { op: "set_kind"; target: Target; kind: string }
  | { op: "set_style"; slide: string; element: string; style: ElementStyle }
  | { op: "replace_style"; slide: string; element: string; style: ElementStyle }
  | { op: "set_frame"; slide: string; element: string; frame: Frame | null }
  | { op: "set_cell"; slide: string; element: string; cell: Cell | null }
  | { op: "set_grid"; slide: string; columns?: number; rows?: number }
  | { op: "replace_element"; slide: string; element: Element }
  | { op: "set_chart_option"; target: Target; param: string; patch: unknown }
  | { op: "set_table_option"; target: Target; param: string; patch: unknown }
  | { op: "set_data"; target: Target; param: string; change: DataOp }
  | { op: "edit_list"; target: Target; param: string; change: ListOp }
  | { op: "add_element"; slot: SlotRef; index?: number; defaults?: boolean; element: Element }
  | { op: "remove_element"; slide: string; element: string }
  | { op: "move_element"; slide: string; element: string; to: SlotRef; index?: number }
  | { op: "add_slide"; index?: number; defaults?: boolean; slide: Slide }
  | { op: "remove_slide"; slide: string }
  | { op: "move_slide"; slide: string; index: number }
  | { op: "set_notes"; slide: string; notes: string }
  | { op: "set_theme"; theme: string | null }
  | { op: "add_pack"; kind: PackKind; namespace: string; pin: PackPin }
  | { op: "remove_pack"; kind: PackKind; namespace: string }
  | { op: "set_token"; path: string[]; value: unknown }
  | { op: "unset_token"; path: string[] }
  | { op: "set_tokens"; tokens: unknown }
  | { op: "set_var"; key: string; value: unknown };

/** Changement d'une liste (`edit_list` du cœur) : insertion, retrait, déplacement, champ d'un item. */
export type ListOp =
  | { op: "insert"; index?: number; id: string; values?: Record<string, unknown> }
  | { op: "remove"; item: string }
  | { op: "move"; item: string; to: number }
  | { op: "set_field"; item: string; field: string; value: unknown };

/** Changement de la liste `extends` du brouillon : ajouter, retirer, monter ou descendre un thème. */
export type ExtendsEdit = "add" | "remove" | "up" | "down";

/** Action d'une ligne de la surface des jetons : poser, revenir à la base, retirer du thème, vider. */
export type TokenAction = "set" | "reset" | "remove" | "clear";

type Request =
  | { cmd: "snapshot" }
  | { cmd: "save"; path: string | null; overwrite: boolean }
  | { cmd: "merge"; path: string }
  | { cmd: "save_copy"; path: string }
  | { cmd: "slide"; id: string }
  | { cmd: "svg"; id: string }
  | { cmd: "apply"; label: string; ops: Op[]; merge: string | null }
  | { cmd: "undo" }
  | { cmd: "redo" }
  | { cmd: "undo_turn"; turn: number }
  | { cmd: "layouts" }
  | { cmd: "packs"; kind: PackKind }
  | { cmd: "edit_pack"; pack: string }
  | { cmd: "save_pack"; as_new: boolean }
  | { cmd: "close_pack" }
  | { cmd: "delete_pack"; pack: string }
  | { cmd: "set_edited_theme"; id: string }
  | { cmd: "rename_edited_theme"; name: string }
  | { cmd: "set_edited_theme_extends"; extends: string[] }
  | { cmd: "edit_theme_extends"; edit: ExtendsEdit; theme: string }
  | { cmd: "host_settings"; widget: string | null }
  | { cmd: "data_panel"; slide: string; element: string }
  | { cmd: "data_edit"; slide: string; element: string; edit: DataEdit }
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
  | { cmd: "remove_widget"; name: string; pack?: string }
  | { cmd: "rename_widget"; from: string; to: string; pack?: string }
  | { cmd: "preview_widget"; name: string; pack?: string }
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
}

// --- Agent -----------------------------------------------------------------

/** Ce que voit l'utilisateur au moment de la consigne (bloc de contexte). */
export interface AgentUi {
  slide: string | null;
  element: string | null;
  field: string | null;
  editing: boolean;
  widget: string | null;
}

/** `AgentUi` tel qu'il part : avec la langue affichée, lue à l'envoi pour suivre un changement à chaud. */
export type AgentUiSent = AgentUi & { locale: string };

/** L'agent au repos, en lancement, ou en plein tour. */
export type AgentState = "idle" | "starting" | "working";

/**
 * Évènements du canal `agent` : état de l'agent, brouillon de l'onglet, panneau des agents et
 * disponibilité de l'agent (le fil passe par les surfaces).
 */
export type AgentEvent =
  | { kind: "status"; state: AgentState; scope?: AgentScope }
  | { kind: "draft"; text: string | null; origin: string | null }
  | { kind: "picker"; surface: Surface }
  | { kind: "availability"; unusable: string | null };

/** La portée d'une conversation : les slides, l'atelier d'un pack, ou un de ses widgets. */
export type AgentScope = { kind: "slides" } | { kind: "atelier"; pack: string } | { kind: "widget"; pack: string; name: string };

/** Où s'affiche une surface : fiche, fil, tiroir ou fenêtre. */
export type Place = "sheet" | "thread" | "drawer" | "modal";

/**
 * Où l'agent pose une fiche (session, `agent::Anchor`) : l'élément `owner` de la slide, ou son
 * champ texte `field` ; `owner` est l'id de la slide pour un champ de la slide. Résolue par la
 * session contre le deck avant d'arriver ici.
 */
export interface Anchor {
  slide: string;
  owner: string;
  field?: string;
}

/** Une surface de l'agent : ses messages A2UI et sa place (session, `agent::Surface`). */
export interface Surface {
  id: string;
  place: Place;
  anchor?: Anchor;
  /** Le tour qui l'a créée (surfaces du fil). */
  turn?: number;
  /** Vivante : dans le dock ; figée : dans son tour. */
  live: boolean;
  /** Une question (`ask_*`, confirmation, permission) : elle attend la réponse de l'utilisateur. */
  question: boolean;
  messages: A2uiMessage[];
}

/** Évènements des outils (crate oku-slides-mcp, `McpEvent`). */
export type McpEvent =
  | { kind: "changed" }
  | { kind: "surface"; surface: Surface }
  | { kind: "close_surface"; id: string }
  | { kind: "focus"; slide: string; element: string | null }
  | { kind: "surfaces"; list: Surface[] }
  | { kind: "surface_patch"; id: string; messages: A2uiMessage[] }
  | { kind: "widget_touched"; name: string; removed: boolean }
  | { kind: "present"; mode: PresentMode }
  | { kind: "open_history" }
  | { kind: "interface"; change: InterfaceChange }
  | { kind: "viewing"; scope: AgentScope };

/** Ce que l'agent change dans l'interface (`set_interface`) ; `null` : inchangé. */
export interface InterfaceChange {
  scheme: "light" | "dark" | "system" | null;
  /** Code d'une langue livrée, ou `system`. */
  locale: string | null;
  grid: boolean | null;
  guides: boolean | null;
}

/** Commandes vers l'agent (crate oku-slides-agent, `AgentCommand`), un onglet par commande. */
type AgentCommand =
  | { cmd: "prompt"; tab: string; scope: AgentScope; text: string; ui: AgentUiSent; answer: string | null }
  | { cmd: "cancel"; tab: string }
  | { cmd: "new_conversation"; tab: string; scope: AgentScope }
  | { cmd: "surface_action"; tab: string; surface: string; name: string; context: Record<string, unknown> }
  | { cmd: "resync"; tab: string; scope: AgentScope }
  | { cmd: "follow"; tab: string }
  | { cmd: "set_draft"; tab: string; text: string | null; origin: string }
  | { cmd: "picker"; tab: string }
  | { cmd: "show_packs"; tab: string }
  | { cmd: "availability"; tab: string }
  | { cmd: "select_agent"; tab: string; id: string }
  | { cmd: "download_model"; tab: string }
  | { cmd: "cancel_download"; tab: string };

const agentCall = (req: AgentCommand) => host<void>("agent", { req }, "/api/agent", req);

/** Consigne, arrêt du tour, nouvelle conversation, réponse à une surface, liste des surfaces. */
export function agentApi(tab: string) {
  return {
    /**
     * `answer` : la question à laquelle la consigne répond (`answerTarget`) ; sans elle,
     * ou si elle n'attend plus, une consigne ordinaire.
     */
    prompt: (scope: AgentScope, text: string, ui: AgentUi, answer: string | null) =>
      agentCall({ cmd: "prompt", tab, scope, text, ui: { ...ui, locale: locale() }, answer }),
    cancel: () => agentCall({ cmd: "cancel", tab }),
    newConversation: (scope: AgentScope) => agentCall({ cmd: "new_conversation", tab, scope }),
    surfaceAction: (surface: string, name: string, context: Record<string, unknown>) =>
      agentCall({ cmd: "surface_action", tab, surface, name, context }),
    /** Demande la liste entière des surfaces ouvertes (évènement `surfaces`). */
    resync: (scope: AgentScope) => agentCall({ cmd: "resync", tab, scope }),
    /** Une autre vue de l'onglet s'abonne : état, brouillon, puis le fil de la portée montrée. */
    follow: () => agentCall({ cmd: "follow", tab }),
    /** Le brouillon de l'onglet, tenu par l'hôte ; `null` : fermé. `origin` : la vue qui l'écrit. */
    setDraft: (text: string | null, origin: string) => agentCall({ cmd: "set_draft", tab, text, origin }),
    /** Demande le panneau des agents (évènement `picker` pour cet onglet). */
    picker: () => agentCall({ cmd: "picker", tab }),
    /** Ouvre le panneau des packs du deck (surface `host-packs` du hub de l'onglet). */
    showPacks: () => agentCall({ cmd: "show_packs", tab }),
    /** Demande si l'agent est utilisable (évènement `availability` pour cet onglet). */
    availability: () => agentCall({ cmd: "availability", tab }),
    /** Choisit l'agent de l'app ; l'hôte réémet le panneau. */
    selectAgent: (id: string) => agentCall({ cmd: "select_agent", tab, id }),
    /** Télécharge le modèle de l'agent local ; l'hôte réémet le panneau à chaque pourcent. */
    downloadModel: () => agentCall({ cmd: "download_model", tab }),
    /** Arrête le téléchargement du modèle ; l'hôte réémet le panneau. */
    cancelDownload: () => agentCall({ cmd: "cancel_download", tab }),
  };
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
    /** Règle une valeur d'exemple d'un widget, dans le pack (mode pack). */
    setExampleValue: (name: string, param: string, value: unknown) =>
      call<Snapshot>({ cmd: "set_example_value", name, param, value }),
    /** Change la liste `param` des valeurs d'exemple ; sans valeur gardée, elle part des items par défaut du schéma. */
    editExampleList: (name: string, param: string, change: ListOp) =>
      call<Snapshot>({ cmd: "edit_example_list", name, param, change }),
    apply: (label: string, ops: Op[], merge?: string) =>
      call<Snapshot>({ cmd: "apply", label, ops, merge: merge ?? null }),
    undo: () => call<Snapshot>({ cmd: "undo" }),
    redo: () => call<Snapshot>({ cmd: "redo" }),
    /** Annule le tour `turn` de l'agent, refusé s'il n'est plus la dernière modification. */
    undoTurn: (turn: number) => call<Snapshot>({ cmd: "undo_turn", turn }),
    /** Layouts des packs de layouts épinglés, rendus dans le thème du deck. */
    layouts: () => call<LayoutsView>({ cmd: "layouts" }),
    /** Packs de la bibliothèque de sorte `kind`, dans leur version courante. */
    packs: (kind: PackKind) => call<PackInfo[]>({ cmd: "packs", kind }),
    /** Passe en édition de la version courante du pack `pack` (id). */
    editPack: (pack: string) => call<Snapshot>({ cmd: "edit_pack", pack }),
    /** Enregistre le pack édité (une copie : l'agent, outil `save_pack`). */
    savePack: () => call<Snapshot>({ cmd: "save_pack", as_new: false }),
    closePack: () => call<Snapshot>({ cmd: "close_pack" }),
    /** Supprime un pack de l'utilisateur, toutes versions ; répond les packs restants de sa sorte. */
    deletePack: (pack: string) => call<PackInfo[]>({ cmd: "delete_pack", pack }),
    /** Fichiers de widgets du pack en cours, `_commun` en tête. */
    widgetStatus: (pack?: string) => call<WidgetStatus[]>({ cmd: "widget_status", pack }),
    widgetSource: (name: string, pack?: string) => call<string>({ cmd: "widget_source", name, pack }),
    /** Crée ou remplace un fichier ; en mode deck, le deck passe à la nouvelle version de son pack. */
    setWidget: (name: string, source: string, pack?: string) => call<WidgetChange>({ cmd: "set_widget", name, source, pack }),
    removeWidget: (name: string, pack?: string) => call<WidgetStatus[]>({ cmd: "remove_widget", name, pack }),
    renameWidget: (from: string, to: string, pack?: string) => call<WidgetStatus[]>({ cmd: "rename_widget", from, to, pack }),
    /** Aperçu d'un widget à ses valeurs d'exemple, sans `params` (ceux de l'agent) : la session les prend vides. */
    previewWidget: (name: string, pack?: string) => call<WidgetPreview>({ cmd: "preview_widget", name, pack }),
    /** Mode pack : le brouillon du thème repart de la version courante du thème `id` (annulable). */
    setEditedTheme: (id: string) => call<Snapshot>({ cmd: "set_edited_theme", id }),
    /** Mode pack : renomme le thème édité (annulable). */
    renameEditedTheme: (name: string) => call<Snapshot>({ cmd: "rename_edited_theme", name }),
    /** Mode pack : change la liste `extends` du brouillon par un delta appliqué par la session (annulable). */
    editThemeExtends: (edit: ExtendsEdit, theme: string) => call<Snapshot>({ cmd: "edit_theme_extends", edit, theme }),
    /** Surface A2UI des jetons du thème édité, filtrée ; `null` hors mode pack. */
    hostTheme: (filter: string) => call<Surface | null>({ cmd: "host_theme", filter }),
    /** Change le jeton `path` du thème édité (mode pack, annulable). */
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
