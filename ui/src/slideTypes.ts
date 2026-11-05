// A slide as the host sends it (frames, runs, elements, field and item views) and its edit ops.

import type { SlideKind } from "./api";
import type { DataOp } from "./chart/chartTypes";
import type { Cell, ZoneView } from "./grid";
import type { PackKind, PackPin } from "./packTypes";

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

/** Heading level of a paragraph: 1 (largest) to 6. */
export type HeadingLevel = 1 | 2 | 3 | 4 | 5 | 6;

export interface Paragraph {
  runs: Run[];
  list?: "bullet" | "number";
  /** Heading; wins over `list` when both are set. */
  heading?: HeadingLevel;
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
