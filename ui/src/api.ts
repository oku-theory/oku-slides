// Accès à l'hôte : requêtes par onglet, espace de travail (fenêtres et
// onglets), dictée et dialogues de fichier. `invoke` dans Tauri, `POST` au
// serveur de développement ailleurs. Les types décrivent le JSON de l'hôte.

import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import type { Cell, ZoneView } from "./grid";
import { locale, t } from "./i18n";
import type { View } from "./table/ShowAsButton";

export type SlideKind = string;

export interface SlideSummary {
  id: string;
  kind: SlideKind;
  title: string | null;
  /** Nom de la slide quand elle sert de modèle. */
  name?: string | null;
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

/** Un widget de pack qu'on ajoute à une slide : son nom (`kind`) et son label. */
export interface AddWidget {
  name: string;
  label: string | null;
}

export interface Snapshot {
  path: string | null;
  name: string;
  dirty: boolean;
  /** Thème du deck : id et version embarquée. */
  theme: ThemeRef | null;
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
  /** Pack de modèles du deck : id et version embarquée. */
  templates: PackRef | null;
  /** Édition d'un pack de modèles en cours. */
  editing: TemplateMode | null;
  /** Le deck embarque une autre version de son pack que ma version courante. */
  pack_notice: PackNotice | null;
  /** Widgets du pack en cours (fichiers, sans `_commun`), sans sonde. */
  pack_widgets: string[];
  /** Widgets d'élément du pack au registre : ceux de la barre d'ajout. */
  add_widgets: AddWidget[];
  /** Révision des valeurs d'exemple des widgets : change quand une valeur est modifiée (tirée de la version du pack édité). */
  widget_trials: number;
  /** Historique ouvert : la version montrée (le reste du snapshot la décrit). */
  history: HistoryView | null;
  /** L'aide de l'app : le deck se lit, il ne se modifie pas. */
  read_only: boolean;
  ui: UiState;
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

/** Slide courante, grille, fantômes, colonnes, sous-onglets, code des widgets, bande et sections repliées, gardés par l'hôte d'un lancement à l'autre. */
export interface UiState {
  slide: string | null;
  grid: boolean;
  /** Fantômes visibles : zone, objets et champs annotés. */
  ghosts: boolean;
  /** Largeur de la pile en px, choisie au séparateur ; `null` : largeur par défaut. */
  pile: number | null;
  /** Largeur du fil de l'agent en px, choisie au séparateur ; `null` : largeur par défaut. */
  notes: number | null;
  /** Sous-onglets de widgets ouverts en mode modèles, dans l'ordre d'ouverture. */
  widgets: string[];
  /** Sous-onglet actif ; `null` : « Modèles ». */
  widget: string | null;
  /** Code source des widgets déplié à droite de l'aperçu ; replié par défaut. */
  code: boolean;
  /** Largeur du code en px, choisie au séparateur ; `null` : proportion par défaut. */
  code_width: number | null;
  /** Hauteur de la bande sous la scène en px, choisie au séparateur ; `null` : hauteur par défaut. */
  band: number | null;
  /** Sections repliées (pile, carte des paramètres) ; toutes ouvertes par défaut. */
  folded: PileSectionName[];
}

/** Ordre d'une sauvegarde de l'état d'interface : l'hôte ignore une sauvegarde plus ancienne (même `page`,
 *  `seq` inférieur ou égal) que la dernière appliquée ; une autre `page` (rechargée) est toujours acceptée. */
export interface UiOrder {
  page: string;
  seq: number;
}

/** Section repliable : slides hors mode modèles, modèles et widgets en mode modèles, carte des paramètres. */
export type PileSectionName = "slides" | "templates" | "widgets" | "params";

/** Un pack de modèles à une version donnée. */
export interface PackRef {
  id: string;
  version: string;
}

/** Le thème d'un deck à une version donnée. */
export interface ThemeRef {
  id: string;
  version: string;
}

/** Un thème de la bibliothèque, dans sa version courante. */
export interface ThemeInfo {
  id: string;
  version: string;
  name: string;
  builtin: boolean;
  /** Le thème d'un deck neuf. */
  default: boolean;
}

/** Version embarquée par le deck, comparée à ma version courante. */
export type PackRelation = "ancestor" | "descendant" | "sibling";

/** Bandeau du pack : textes et libellés localisés par la session. */
export interface PackNotice {
  pack: string;
  name: string;
  relation: PackRelation;
  message: string;
  use_mine: string;
  /** `null` : la bibliothèque n'a pas la version du deck, l'action est masquée. */
  make_current: string | null;
}

export interface TemplateMode {
  set: string;
  name: string;
  /** Pack intégré : l'enregistrer crée ma version sous son id. */
  builtin: boolean;
  /** Version dont viennent widgets et description. */
  base: string;
  /** Parente des versions de la session. */
  parent: string | null;
  /** Version de cette session d'édition, une fois enregistrée. */
  session_version: string | null;
  /** Le thème édité en brouillon : les jetons du deck de travail. */
  theme: ThemeEditView;
}

export interface ThemeEditView {
  /** Version de base du brouillon. */
  base: ThemeRef;
  name: string;
  /** Thème intégré : l'enregistrer en crée une copie. */
  builtin: boolean;
  /** Le brouillon diffère de sa base (jetons ou nom). */
  dirty: boolean;
  /** Empreinte des jetons du brouillon : change avec eux. */
  revision: string;
}

export interface PackInfo {
  id: string;
  version: string;
  name: string;
  description: string;
  builtin: boolean;
  /** Le pack d'un deck qui n'en choisit pas. */
  default: boolean;
  /** Cette version est arrivée avec un deck reçu, l'utilisateur ne l'a pas créée. */
  received: boolean;
  count: number;
}

export interface TemplatesView {
  set: PackInfo;
  sets: PackInfo[];
  templates: TemplateView[];
}

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
  /** Schéma du widget s'il est valide ; `null` pour `_commun`. */
  schema: unknown | null;
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
  /** Nom et description d'une slide modèle. */
  name?: string;
  description?: string;
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
  /** Nom du paramètre, ou `liste/id/colonne` pour un item de liste. */
  field: string;
  /** Pour un item : le paramètre `list` qui le porte. */
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
  /** Nom du paramètre `list`. */
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

export interface TemplateView {
  id: string;
  name: string;
  description: string;
  slide: Slide;
  svg: string;
}

// --- Dictée ---------------------------------------------------------------

export type SttModel = "apple" | "turbo" | "large" | "base";
export type SttState = "idle" | "downloading" | "recording" | "transcribing";

export interface SttConfig {
  model: SttModel;
  language: string | null;
  /** Arrêter l'écoute seule après un silence de `silence` secondes. */
  auto_stop: boolean;
  silence: number;
  /** Version des réglages (tenue par l'hôte). */
  version: number;
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

export type Op = { op: string } & Record<string, unknown>;

/** Changement d'une liste (`edit_list` du cœur) : insertion, retrait, déplacement, champ d'un item. */
export type ListOp =
  | { op: "insert"; index?: number; id: string; values?: Record<string, unknown> }
  | { op: "remove"; item: string }
  | { op: "move"; item: string; to: number }
  | { op: "set_field"; item: string; field: string; value: unknown };

/** Action d'une ligne de la surface des jetons : poser, revenir à la base, retirer du thème, vider. */
export type TokenAction = "set" | "reset" | "remove" | "clear";

type Request =
  | { cmd: "snapshot" }
  | { cmd: "save"; path: string | null; overwrite: boolean }
  | { cmd: "merge"; path: string }
  | { cmd: "move"; path: string }
  | { cmd: "save_copy"; path: string }
  | { cmd: "slide"; id: string }
  | { cmd: "svg"; id: string }
  | { cmd: "apply"; label: string; ops: Op[]; merge: string | null }
  | { cmd: "undo" }
  | { cmd: "redo" }
  | { cmd: "undo_turn"; turn: number }
  | { cmd: "templates"; set?: string | null }
  | { cmd: "template_sets" }
  | { cmd: "edit_templates"; set: string }
  | { cmd: "save_templates"; as_new: boolean }
  | { cmd: "close_templates" }
  | { cmd: "delete_template_set"; set: string }
  | { cmd: "pack_use_mine" }
  | { cmd: "set_theme"; id: string }
  | { cmd: "set_edited_theme"; id: string }
  | { cmd: "rename_edited_theme"; name: string }
  | { cmd: "pack_make_current" }
  | { cmd: "pack_notice_close" }
  | { cmd: "chart_kind"; label: string; slide: string; element: string; kind: string }
  | { cmd: "show_as"; label: string; slide: string; element: string; view: View }
  | { cmd: "host_settings"; slide: string; widget: string | null }
  | { cmd: "host_param"; slide: string; key: string; value: unknown }
  | { cmd: "set_example_value"; name: string; param: string; value: unknown }
  | { cmd: "edit_example_list"; name: string; param: string; change: ListOp }
  | { cmd: "host_theme"; filter: string }
  | { cmd: "host_token"; path: string[]; action: TokenAction; value?: unknown }
  | { cmd: "target_settings"; slide: string; element: string | null }
  | { cmd: "target_param"; slide: string; element: string | null; key: string; value: unknown }
  | { cmd: "target_list"; slide: string; element: string | null; key: string; change: ListOp }
  | ({ cmd: "ui"; order?: UiOrder } & UiState)
  | { cmd: "history_open" }
  | { cmd: "history_seek"; index: number }
  | { cmd: "history_restore" }
  | { cmd: "history_close" }
  | { cmd: "widget_status" }
  | { cmd: "widget_source"; name: string }
  | { cmd: "set_widget"; name: string; source: string }
  | { cmd: "remove_widget"; name: string }
  | { cmd: "rename_widget"; from: string; to: string }
  | { cmd: "preview_widget"; name: string }
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
  /** Langues livrées, l'anglais d'abord. */
  available: string[];
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
 * Évènements du canal `agent` : état de l'agent, brouillon de l'onglet et panneau des agents
 * (le fil passe par les surfaces).
 */
export type AgentEvent =
  | { kind: "status"; state: AgentState; scope?: AgentScope }
  | { kind: "draft"; text: string | null; origin: string | null }
  | { kind: "picker"; surface: Surface };

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
  | { kind: "viewing"; scope: AgentScope };

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
  | { cmd: "select_agent"; tab: string; id: string };

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
    /** Choisit l'agent de l'app ; l'hôte réémet le panneau. */
    selectAgent: (id: string) => agentCall({ cmd: "select_agent", tab, id }),
  };
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Requête à l'hôte : commande Tauri, ou `POST` en développement. */
async function host<T>(command: string, args: Record<string, unknown>, endpoint: string, body: unknown): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, args);
  }
  const res = await fetch(endpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  // Un refus (403) ou une page d'erreur du proxy peuvent arriver sans corps
  // JSON : le statut reste lisible.
  const text = await res.text();
  let json: { error?: string } = {};
  try {
    json = text ? JSON.parse(text) : {};
  } catch {
    throw new Error(`${res.status} ${res.statusText}`);
  }
  if (!res.ok) throw new Error(json.error ?? `${res.status} ${res.statusText}`);
  return json as unknown as T;
}

/** Le texte d'une erreur. */
export function errorText(e: unknown): string {
  return String(e instanceof Error ? e.message : e);
}

/** Sur macOS : ⌘ et barre de titre intégrée. */
export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);

/** Les requêtes d'un onglet : chacune ne touche que sa session. */
export function tabApi(tab: string) {
  const call = <T>(req: Request) => host<T>("request", { tab, req }, "/api", { tab, req });
  return {
    snapshot: () => call<Snapshot>({ cmd: "snapshot" }),
    chartKind: (label: string, slide: string, element: string, kind: string) =>
      call<Snapshot>({ cmd: "chart_kind", label, slide, element, kind }),
    showAs: (label: string, slide: string, element: string, view: View) =>
      call<Snapshot>({ cmd: "show_as", label, slide, element, view }),
    save: (path: string | null, overwrite = false) => call<Snapshot>({ cmd: "save", path, overwrite }),
    /** Fusionne un autre PDF du même deck ; l'avis de fusion arrive par `notice`. */
    merge: (path: string) => call<Snapshot>({ cmd: "merge", path }),
    /** Déplace le fichier du deck (l'ancien est supprimé). */
    move: (path: string) => call<Snapshot>({ cmd: "move", path }),
    /** Écrit une copie ailleurs ; le deck garde son fichier. */
    saveCopy: (path: string) => call<Snapshot>({ cmd: "save_copy", path }),
    slide: (id: string) => call<SlideView>({ cmd: "slide", id }),
    svg: (id: string) => call<string>({ cmd: "svg", id }),
    /** Surface A2UI des réglages de l'hôte pour une slide, ou du widget à ses valeurs d'exemple ; vide hors des modèles. */
    hostSettings: (slide: string, widget: string | null) => call<Surface | null>({ cmd: "host_settings", slide, widget }),
    hostParam: (slide: string, key: string, value: unknown) => call<Snapshot>({ cmd: "host_param", slide, key, value }),
    /** Règle une valeur d'exemple d'un widget, dans le pack (mode modèles). */
    setExampleValue: (name: string, param: string, value: unknown) =>
      call<Snapshot>({ cmd: "set_example_value", name, param, value }),
    /** Change la liste `param` des valeurs d'exemple ; sans valeur gardée, elle part des items par défaut du schéma. */
    editExampleList: (name: string, param: string, change: ListOp) =>
      call<Snapshot>({ cmd: "edit_example_list", name, param, change }),
    /** Surface des paramètres de l'élément `element` (ou, sans élément, du widget de la slide) ; vide si rien n'est réglable. */
    targetSettings: (slide: string, element: string | null) =>
      call<Surface | null>({ cmd: "target_settings", slide, element }),
    /** Règle le paramètre `key` de la cible (pour une liste, `value` porte tous ses items) : un pas d'annulation. */
    targetParam: (slide: string, element: string | null, key: string, value: unknown) =>
      call<Snapshot>({ cmd: "target_param", slide, element, key, value }),
    /** Change la liste `key` de la cible : un pas d'annulation. */
    targetList: (slide: string, element: string | null, key: string, change: ListOp) =>
      call<Snapshot>({ cmd: "target_list", slide, element, key, change }),
    apply: (label: string, ops: Op[], merge?: string) =>
      call<Snapshot>({ cmd: "apply", label, ops, merge: merge ?? null }),
    undo: () => call<Snapshot>({ cmd: "undo" }),
    redo: () => call<Snapshot>({ cmd: "redo" }),
    /** Annule le tour `turn` de l'agent, refusé s'il n'est plus la dernière modification. */
    undoTurn: (turn: number) => call<Snapshot>({ cmd: "undo_turn", turn }),
    templates: (set?: string | null) => call<TemplatesView>({ cmd: "templates", set: set ?? null }),
    templateSets: () => call<PackInfo[]>({ cmd: "template_sets" }),
    editTemplates: (set: string) => call<Snapshot>({ cmd: "edit_templates", set }),
    saveTemplates: (asNew = false) => call<Snapshot>({ cmd: "save_templates", as_new: asNew }),
    closeTemplates: () => call<Snapshot>({ cmd: "close_templates" }),
    deleteTemplateSet: (set: string) => call<PackInfo[]>({ cmd: "delete_template_set", set }),
    /** Fichiers de widgets du pack en cours, `_commun` en tête. */
    widgetStatus: () => call<WidgetStatus[]>({ cmd: "widget_status" }),
    widgetSource: (name: string) => call<string>({ cmd: "widget_source", name }),
    /** Crée ou remplace un fichier ; en mode deck, le deck passe à la nouvelle version de son pack. */
    setWidget: (name: string, source: string) => call<WidgetChange>({ cmd: "set_widget", name, source }),
    removeWidget: (name: string) => call<WidgetStatus[]>({ cmd: "remove_widget", name }),
    renameWidget: (from: string, to: string) => call<WidgetStatus[]>({ cmd: "rename_widget", from, to }),
    /** Aperçu d'un widget à ses valeurs d'exemple, sans `params` (ceux de l'agent) : la session les prend vides. */
    previewWidget: (name: string) => call<WidgetPreview>({ cmd: "preview_widget", name }),
    /** Le deck prend le thème `id` dans sa version courante, embarquée (annulable). */
    setTheme: (id: string) => call<Snapshot>({ cmd: "set_theme", id }),
    /** Mode modèles : le brouillon du thème repart de la version courante du thème `id` (annulable). */
    setEditedTheme: (id: string) => call<Snapshot>({ cmd: "set_edited_theme", id }),
    /** Mode modèles : renomme le thème édité (annulable). */
    renameEditedTheme: (name: string) => call<Snapshot>({ cmd: "rename_edited_theme", name }),
    /** Surface A2UI des jetons du thème édité, filtrée ; `null` hors mode modèles. */
    hostTheme: (filter: string) => call<Surface | null>({ cmd: "host_theme", filter }),
    /** Change le jeton `path` du thème édité (mode modèles, annulable). */
    hostToken: (path: string[], action: TokenAction, value?: unknown) =>
      call<Snapshot>({ cmd: "host_token", path, action, ...(value === undefined ? {} : { value }) }),
    /** Bandeau du pack : le deck passe à ma version courante (annulable). */
    packUseMine: () => call<Snapshot>({ cmd: "pack_use_mine" }),
    /** Bandeau du pack : sa version devient ma version courante. */
    packMakeCurrent: () => call<Snapshot>({ cmd: "pack_make_current" }),
    packNoticeClose: () => call<Snapshot>({ cmd: "pack_notice_close" }),
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

/** Un onglet, tel que la barre le montre. */
export interface TabInfo {
  id: string;
  title: string;
  dirty: boolean;
  editing_templates: boolean;
  path: string | null;
  pristine: boolean;
  /** Identité du deck : une autre copie du même deck la partage. */
  deck_uid: string | null;
}

/** Une fenêtre : ses onglets et l'onglet actif. */
export interface WindowView {
  label: string;
  active: string;
  tabs: TabInfo[];
}

/** Les fenêtres et leurs onglets ; `focused` : la dernière fenêtre active. */
export interface WorkspaceView {
  /** Numéro croissant de la vue : une vue plus petite est plus ancienne. */
  generation: number;
  focused: string;
  /** L'agent est dans sa propre fenêtre. */
  agent_detached: boolean;
  windows: WindowView[];
}

type WorkspaceRequest =
  | { cmd: "view" }
  | { cmd: "messages" }
  | { cmd: "set_locale"; locale: string | null }
  | { cmd: "scheme" }
  | { cmd: "set_scheme"; scheme: "light" | "dark" | null }
  | { cmd: "new_tab"; window: string }
  | { cmd: "open_tab"; window: string; path: string }
  | { cmd: "open_help"; window: string }
  | { cmd: "same_deck"; path: string }
  | { cmd: "activate"; tab: string }
  | { cmd: "close"; tab: string; discard: boolean }
  | { cmd: "reorder"; tab: string; index: number }
  | { cmd: "move_tab"; tab: string; window: string | null; index: number | null }
  | { cmd: "new_window" }
  | { cmd: "close_window"; window: string; discard: string[] }
  | { cmd: "detach_agent" }
  | { cmd: "attach_agent" };

const ws = <T>(req: WorkspaceRequest) => host<T>("workspace", { req }, "/api/workspace", req);

/** L'espace de travail : fenêtres et onglets. */
export const workspace = {
  view: () => ws<WorkspaceView>({ cmd: "view" }),
  messages: () => ws<Messages>({ cmd: "messages" }),
  /** Choisit la langue de l'interface (`null` : celle du système) ; ses messages. */
  setLocale: (locale: string | null) => ws<Messages>({ cmd: "set_locale", locale }),
  /** Schéma de l'interface choisi (`null` : celui du système). */
  scheme: () => ws<SchemeSetting>({ cmd: "scheme" }),
  /** Choisit le schéma de l'interface (`null` : celui du système). */
  setScheme: (scheme: "light" | "dark" | null) => ws<SchemeSetting>({ cmd: "set_scheme", scheme }),
  newTab: (window: string) => ws<WorkspaceView>({ cmd: "new_tab", window }),
  openTab: (window: string, path: string) => ws<WorkspaceView>({ cmd: "open_tab", window, path }),
  /** L'aide de l'app, en lecture seule : son onglet s'il est ouvert, sinon un nouveau. */
  openHelp: (window: string) => ws<WorkspaceView>({ cmd: "open_help", window }),
  /** L'onglet qui tient déjà le deck de `path` sous un autre fichier ; ne change rien. */
  sameDeck: (path: string) => ws<{ tab: string | null; title: string | null }>({ cmd: "same_deck", path }),
  activate: (tab: string) => ws<WorkspaceView>({ cmd: "activate", tab }),
  /** `discard` : l'utilisateur a choisi de perdre le travail non enregistré. */
  close: (tab: string, discard = false) => ws<WorkspaceView>({ cmd: "close", tab, discard }),
  reorder: (tab: string, index: number) => ws<WorkspaceView>({ cmd: "reorder", tab, index }),
  moveTab: (tab: string, window: string | null, index: number | null = null) =>
    ws<WorkspaceView>({ cmd: "move_tab", tab, window, index }),
  newWindow: () => ws<WorkspaceView>({ cmd: "new_window" }),
  closeWindow: (window: string, discard: string[]) => ws<WorkspaceView>({ cmd: "close_window", window, discard }),
  /** L'agent passe dans sa propre fenêtre. */
  detachAgent: () => ws<WorkspaceView>({ cmd: "detach_agent" }),
  /** L'agent revient dans la colonne de droite. */
  attachAgent: () => ws<WorkspaceView>({ cmd: "attach_agent" }),
};

/**
 * Glisser un onglet hors de sa fenêtre (Tauri) : l'hôte lit le curseur lui-même.
 *
 * Jamais `screenX/screenY` : WKWebView ne les donne pas dans le repère des
 * fenêtres (écran au-dessus du principal). La fenêtre visée reçoit
 * `tab-drag` (fantôme, place de dépôt) puis `tab-drop`.
 */
export const tabDrag = {
  over: async (tab: string) => (await import("@tauri-apps/api/core")).invoke<void>("drag_tab_over", { tab }),
  leave: async () => (await import("@tauri-apps/api/core")).invoke<void>("drag_tab_leave"),
  drop: async (tab: string) => (await import("@tauri-apps/api/core")).invoke<WorkspaceView>("drop_tab", { tab }),
};

/** Onglet d'une autre fenêtre sous le pointeur, en coordonnées de cette page. */
export interface TabAt {
  tab: string;
  x: number;
  y: number;
}

/** Écoute un événement Tauri ; l'abonnement suit la vie de l'effet (StrictMode compris). */
export function listenTauri(subscribe: () => Promise<() => void>): () => void {
  let stop: (() => void) | undefined;
  let dead = false;
  subscribe()
    .then((un) => (dead ? un() : (stop = un)))
    .catch((e) => console.error("événement de l'hôte", e));
  return () => {
    dead = true;
    stop?.();
  };
}

/** Choix de fichier : dialogue natif dans Tauri, saisie du chemin en développement. */
export async function pickOpenPath(): Promise<string | null> {
  if (inTauri) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({ multiple: false, filters: [{ name: t("ui-brand"), extensions: ["pdf", "json"] }] });
    return typeof p === "string" ? p : null;
  }
  return window.prompt(t("ui-file-open-prompt"));
}

export async function pickSavePath(defaultName: string): Promise<string | null> {
  if (inTauri) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({ defaultPath: `${defaultName}.oku-slides.pdf`, filters: [{ name: t("ui-brand"), extensions: ["pdf"] }] });
  }
  return window.prompt(t("ui-file-save-prompt"), `/tmp/${defaultName}.oku-slides.pdf`);
}
