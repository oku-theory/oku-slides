import { describe, expect, test } from "bun:test";
import type { SlideView } from "../api";
import { t } from "../i18n";
import { panelActionOps } from "./panelActions";

type Slide = SlideView["slide"];

const slide: Slide = {
  id: "t-1",
  kind: "content",
  params: {
    grid: { columns: 12, rows: 6 },
    elements: [
      { id: "a", kind: "text", cell: { col: 6, row: 3, cols: 6, rows: 3 } },
      { id: "b", kind: "text", frame: { x: 0, y: 0, w: 10, h: 10 } },
    ],
  },
};
const action = (name: string, context: Record<string, unknown>) => ({ name, context });

describe("panelActionOps", () => {
  test("rename_template renomme la slide du contexte, valeur transmise telle quelle", () => {
    expect(panelActionOps(action("rename_template", { slide: "t-1", value: "  Deux colonnes " }), slide)).toEqual({
      label: t("ui-undo-rename-template"),
      ops: [{ op: "set_slide_field", slide: "t-1", field: "name", value: "  Deux colonnes " }],
    });
  });

  test("describe_template blanche transmise telle quelle", () => {
    expect(panelActionOps(action("describe_template", { slide: "t-1", value: "  " }), slide)?.ops).toEqual([
      { op: "set_slide_field", slide: "t-1", field: "description", value: "  " },
    ]);
  });

  test("set_grid envoie l'op set_grid : la session recadre les éléments", () => {
    expect(panelActionOps(action("set_grid", { slide: "t-1", columns: 6, rows: 6 }), slide)).toEqual({
      label: t("ui-undo-grid", { columns: 6, rows: 6 }),
      ops: [{ op: "set_grid", slide: "t-1", columns: 6, rows: 6 }],
    });
  });

  test("set_grid sans nombres valides ne fait rien", () => {
    expect(panelActionOps(action("set_grid", { slide: "t-1", columns: null, rows: 6 }), slide)).toBeNull();
  });

  test("une action pour une autre slide ne fait rien", () => {
    expect(panelActionOps(action("rename_template", { slide: "autre", value: "x" }), slide)).toBeNull();
  });

  test("une action inconnue ne fait rien", () => {
    expect(panelActionOps(action("launch_rocket", { slide: "t-1" }), slide)).toBeNull();
  });
});
