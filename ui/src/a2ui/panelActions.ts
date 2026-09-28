// Actions du panneau de droite (surface A2UI de la session) → ops du deck.
//
// La session décrit le panneau (requête `panel`) ; ses composants renvoient
// `rename_template`, `describe_template` et `set_grid`, avec la slide visée
// dans le contexte. Changer la grille déplace aussi les éléments accrochés.

import type { Op, SlideView } from "../api";
import { slot } from "../deck";
import { rescaleCell } from "../grid";
import { t } from "../i18n";

/** Action reçue d'une surface : son nom et son contexte, chemins déjà lus. */
export interface PanelAction {
  name: string;
  context?: Record<string, unknown>;
}

/** Ops d'une action, avec leur libellé d'annulation ; `null` si rien à faire. */
export function panelActionOps(
  action: PanelAction,
  slide: SlideView["slide"],
  grid: { columns: number; rows: number } | undefined,
): { label: string; ops: Op[] } | null {
  const ctx = action.context ?? {};
  if (ctx.slide !== slide.id) return null;
  const text = () => String(ctx.value ?? "");
  switch (action.name) {
    case "rename_template":
      return { label: t("ui-undo-rename-template"), ops: [{ op: "set_slide_field", slide: slide.id, field: "name", value: text() }] };
    case "describe_template":
      return {
        label: t("ui-undo-describe-template"),
        ops: [{ op: "set_slide_field", slide: slide.id, field: "description", value: text() }],
      };
    case "set_grid": {
      if (!grid) return null;
      const columns = Number(ctx.columns);
      const rows = Number(ctx.rows);
      const next = { ...(slide.params?.grid as object | undefined ?? {}), columns, rows };
      const ops: Op[] = [{ op: "set_param", target: { slide: slide.id }, param: "grid", value: next }];
      // Les éléments gardent leurs proportions sur la nouvelle grille.
      for (const e of slot(slide, "elements")) {
        if (e.cell) ops.push({ op: "set_cell", slide: slide.id, element: e.id, cell: rescaleCell(e.cell, grid, next) });
      }
      return { label: t("ui-undo-grid", { columns, rows }), ops };
    }
    default:
      return null;
  }
}
