/**
 * Champs de la scène : clé stable, navigation au clavier, libellé du
 * placeholder et boîte cliquable (un champ vide a une ligne de haut) ;
 * étiquette d'un objet (sorte et case de grille).
 */
import type { FieldView, SlideView, TextStyle } from "./api";
import { capHeight, cssFont } from "./fontMetrics";
import type { Cell } from "./grid";
import { cellLabel } from "./grid";
import { t } from "./i18n";

export type FieldKey = string;

/** Clé d'un champ : objet et paramètre (séparateur d'unité ASCII, absent des ids). */
export const fieldKey = (f: Pick<FieldView, "owner" | "field">): FieldKey => `${f.owner}\u001f${f.field}`;

/** Champ de la vue par sa clé ; `null` sans clé ou si le champ n'y est plus. */
export function findField(view: SlideView, key: FieldKey | null): FieldView | null {
  if (key === null) return null;
  return view.fields.find((f) => fieldKey(f) === key) ?? null;
}

/** Champ suivant (`1`) ou précédent (`-1`) dans l'ordre de Tab ; `null` au bout. */
export function nextField(view: SlideView, key: FieldKey, dir: 1 | -1): FieldView | null {
  const i = view.fields.findIndex((f) => fieldKey(f) === key);
  if (i < 0) return null;
  return view.fields[i + dir] ?? null;
}

/**
 * Champ principal d'un objet, où l'on écrit sans viser (Entrée, double-clic,
 * dictée, copie) : le premier non vide, sinon le premier `rich`, sinon le
 * premier. Le surtitre d'une carte, souvent vide, ne passe pas avant le titre.
 */
export function primaryFieldOf(view: SlideView, owner: string): FieldView | null {
  const own = view.fields.filter((f) => f.owner === owner);
  return own.find((f) => !f.empty) ?? own.find((f) => f.kind === "rich") ?? own[0] ?? null;
}

/** Nom d'une sorte d'élément (texte, carte…) ; une sorte inconnue garde son identifiant. */
export const kindLabel = (kind: string) => t("ui-element-kind", { kind });

/** Étiquette d'un objet : sa sorte, et sa case s'il est sur la grille. */
export const elementTag = (kind: string, cell: Cell | null | undefined) =>
  cell ? `${kindLabel(kind)} · ${cellLabel(cell)}` : kindLabel(kind);

/** Libellé du placeholder : traduction du nom, sinon libellé du schéma, sinon le nom. */
export function fieldLabel(f: Pick<FieldView, "field" | "label">): string {
  const translated = t("ui-field", { kind: f.field });
  return translated !== f.field ? translated : f.label ?? f.field;
}

/**
 * Pas des lignes (px écran), celui de Typst : hauteur de capitale de la police
 * (mesurée ; 0,7 em sans mise en page) + interligne.
 */
export function lineHeight(style: TextStyle, scale: number): number {
  const size = style.size * scale;
  const cap = capHeight({ ...cssFont(style), size }) ?? size * 0.7;
  return cap + style.leading * scale;
}

/** Hauteur d'une ligne sans typographie connue (px écran). */
const FALLBACK_LINE = 16;

/** Champ vide sans place au rendu : sa ligne est dessinée à côté de son ancre. */
export const unplaced = (f: FieldView) => f.empty && f.h <= 0;

/** Hauteur de la ligne d'un champ vide (px écran). */
const lineOf = (f: FieldView, scale: number) => (f.style ? lineHeight(f.style, scale) : FALLBACK_LINE);

/**
 * Boîte cliquable (px écran). Un champ vide sans place a une ligne de haut,
 * avant ou après son ancre. Plusieurs champs vides du même objet sur la même
 * ancre (carte sans surtitre ni titre) s'empilent, de haut en bas dans l'ordre
 * de Tab : avant l'ancre, les premiers montent au-dessus des suivants ; après,
 * les suivants descendent sous les premiers. `fields` : les champs de la vue.
 */
export function fieldHitBox(f: FieldView, scale: number, fields: readonly FieldView[]) {
  if (!unplaced(f)) {
    return { left: f.x * scale, top: f.y * scale, width: Math.max(f.w * scale, 8), height: Math.max(f.h * scale, 8) };
  }
  const key = fieldKey(f);
  const shared = fields.filter((g) => unplaced(g) && g.owner === f.owner && g.side === f.side && Math.abs(g.y - f.y) < 0.01);
  const i = shared.findIndex((g) => fieldKey(g) === key);
  const lines = (gs: readonly FieldView[]) => gs.reduce((sum, g) => sum + lineOf(g, scale), 0);
  const line = lineOf(f, scale);
  const top = f.side === "after"
    ? f.y * scale + lines(shared.slice(0, Math.max(i, 0)))
    : f.y * scale - (i < 0 ? 0 : lines(shared.slice(i + 1))) - line;
  return { left: f.ax * scale, top, width: f.aw * scale, height: line };
}

/**
 * Ancres possibles d'une fiche de l'agent sur la scène : les éléments, et les
 * champs de la slide sous leur nom `@param` (protocole de l'agent).
 */
export function anchorIds(view: SlideView): string[] {
  const slideFields = view.fields.filter((f) => f.owner_kind === "slide").map((f) => `@${f.field}`);
  return [...view.targets.map((x) => x.id), ...slideFields];
}
