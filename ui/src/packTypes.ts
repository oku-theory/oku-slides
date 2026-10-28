// Types des packs et des thèmes vus par l'interface : épinglages, bibliothèque, mode pack,
// grille d'ajout et layouts. Ils décrivent le JSON de l'hôte, comme `api.ts`.

import type { ElementStyle, Slide } from "./api";

/** Le contenu de départ d'un élément ajouté depuis la grille (session, `ElementSeed`). */
export interface ElementSeed {
  /** Les valeurs d'exemple du pack, traduites, que le schéma admet. */
  params: Record<string, unknown>;
  /** Le style de départ du manifeste (`styles`), s'il en a un. */
  style?: ElementStyle;
}

/** Un widget d'élément à ajouter : kind, nom montré, description et départ. */
export interface AddWidget {
  /** Le kind qualifié (`base/card`). */
  name: string;
  /** Le nom montré, dans la langue de l'utilisateur. */
  label: string;
  /** Ce que montre le widget, en une phrase ; cherché par le filtre de la grille d'ajout. */
  description: string | null;
  /** Le contenu de départ de l'élément. */
  seed: ElementSeed;
}

/** Les widgets à ajouter d'un pack de widgets épinglé (une section de la grille d'ajout). */
export interface AddPackView {
  namespace: string;
  name: string;
  /** Ses widgets d'élément, dans l'ordre du registre. */
  widgets: AddWidget[];
}

/** Sorte d'un pack : widgets d'éléments ou layouts de slide. */
export type PackKind = "widgets" | "layouts";

/** Un pack épinglé par le deck : id, version embarquée et espaces de noms requis. */
export interface PackPin {
  id: string;
  version: string;
  requires?: string[];
}

/** Un thème épinglé par le deck : id, version embarquée et espaces de noms des parents. */
export interface ThemePin {
  id: string;
  version: string;
  extends?: string[];
}

/** Un thème de la bibliothèque dans sa version courante. */
export interface ThemeInfo {
  id: string;
  version: string;
  name: string;
  /** Son espace de noms, unique dans la bibliothèque : le deck l'épingle sous ce nom. */
  namespace: string;
  /** Les thèmes qu'il étend, dans l'ordre (les suivants l'emportent). */
  extends: string[];
  builtin: boolean;
  /** Le thème d'un deck neuf. */
  default: boolean;
}

/** The pack edited in pack mode. */
export interface PackMode {
  /** Id du pack édité. */
  pack: string;
  name: string;
  kind: PackKind;
  /** Pack intégré : l'enregistrer crée ma version sous son id. */
  builtin: boolean;
  /** Version dont viennent widgets et description. */
  base: string;
  /** Parente des versions de la session. */
  parent: string | null;
  /** Version de cette session d'édition, une fois enregistrée. */
  session_version: string | null;
  /** Pack du deck seul (celui d'un import) : ses enregistrements restent dans le deck, jamais dans la bibliothèque. */
  deck_only: boolean;
  /** True when this tab cannot save its pack now: held elsewhere, or gone. */
  read_only: boolean;
  /** True when this tab may write without first taking the lease (autosave needs it). */
  leased: boolean;
  /** The edited pack's name, cleaned for titles and dialogs. */
  display_name: string;
  /** True while a deck-pack tab holds a save its owner deck has not acknowledged. */
  unsent: boolean;
}

/** A theme tab: the theme draft and its flags (Rust `ThemeTabView`). */
export interface ThemeTabView {
  /** The draft, as the theme panel shows it. */
  theme: ThemeEditView;
  /** The version this tab wrote last, if it wrote one. */
  session_version: string | null;
  /** True when this tab lacks its theme's lease: it cannot save until it retakes it. */
  read_only: boolean;
  /** True when this tab may write without first taking the lease (autosave needs it). */
  leased: boolean;
  /** The theme's name, cleaned for titles and dialogs. */
  display_name: string;
}

/** Un thème qu'étend le brouillon : son espace de noms et son nom (l'espace de noms s'il est introuvable). */
export interface ThemeParentView {
  namespace: string;
  name: string;
}

/** The theme draft of a theme tab (session, `ThemeEditView`). */
export interface ThemeEditView {
  /** Version de base du brouillon. */
  base: ThemePin;
  /** Espace de noms de la base : la valeur du menu des thèmes, même hors de la bibliothèque. */
  namespace: string;
  name: string;
  /** La base est intégrée : l'enregistrer en crée un enfant qui l'étend. */
  child: boolean;
  /** Les thèmes qu'étend le brouillon, dans l'ordre (édités par la surface `host-theme`). */
  extends: ThemeParentView[];
  /** Le brouillon diffère de son point de départ (jetons, nom ou parents). */
  dirty: boolean;
  /** Empreinte des jetons du brouillon : change avec eux. */
  revision: string;
}

/** Un pack de la bibliothèque dans sa version courante. */
export interface PackInfo {
  kind: PackKind;
  id: string;
  version: string;
  name: string;
  /** Espace de noms de ses widgets : leur `kind` est `<namespace>/<nom>`. */
  namespace: string;
  /** Espaces de noms des packs dont il a besoin. */
  requires: string[];
  description: string;
  builtin: boolean;
  /** Un pack qu'épingle un deck neuf (`base`, `essentiel`). */
  default: boolean;
  /** Cette version est arrivée avec un deck reçu, l'utilisateur ne l'a pas créée. */
  received: boolean;
  /** Le nombre de ses fichiers de widgets, `_commun` excepté. */
  widgets: number;
  /** Le nombre de ses profils d'import (pack de layouts). */
  profiles: number;
}

/** Un layout avec son aperçu (SVG) dans le thème du deck. */
export interface LayoutView {
  /** Son kind qualifié (`essentiel/content`). */
  kind: string;
  /** Son nom dans la langue de l'utilisateur. */
  label: string;
  /** Sa slide de départ, à sa valeur d'exemple. */
  slide: Slide;
  svg: string;
}

/** Les layouts d'un pack de layouts épinglé, avec leurs aperçus. */
export interface LayoutPackView {
  namespace: string;
  name: string;
  layouts: LayoutView[];
}

/** Les layouts de chaque pack de layouts épinglé, dans l'ordre des espaces de noms. */
export interface LayoutsView {
  packs: LayoutPackView[];
}
