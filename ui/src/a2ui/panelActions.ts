// Actions du panneau de droite (surface A2UI de la session) → ops du deck.
//
// La session décrit le panneau (requête `panel`) ; ses composants renvoient
// `rename_template`, `describe_template` et `set_grid`, avec la slide visée
// dans le contexte. La session recadre les éléments accrochés (`set_grid`).

import type { Op, SlideView } from "../api";
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
      const { columns, rows } = ctx;
      if (typeof columns !== "number" || typeof rows !== "number") return null;
      return { label: t("ui-undo-grid", { columns, rows }), ops: [{ op: "set_grid", slide: slide.id, columns, rows }] };
    }
    default:
      return null;
  }
}
