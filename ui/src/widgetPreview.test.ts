import { describe, expect, test } from "bun:test";
import type { FieldView, ItemView, ListView, SlideView } from "./api";
import { croppedSvg, croppedView } from "./widgetPreview";
import { ESSENTIEL } from "./builtinKinds";

const SVG = '<svg viewBox="0 0 1920 1080" width="1920pt" height="1080pt" xmlns="http://www.w3.org/2000/svg"><path d="M0 0"/></svg>';

/** Un attribut de la balise `<svg>` racine. */
function attr(svg: string, name: string): string | undefined {
  const tag = /<svg\b[^>]*>/.exec(svg)?.[0] ?? "";
  return new RegExp(`\\s${name}="([^"]*)"`).exec(tag)?.[1];
}

describe("croppedSvg", () => {
  test("un widget d'élément : la vue est sa boîte, sans marge", () => {
    const svg = croppedSvg(SVG, { x: 124, y: 150, w: 736, h: 30 });
    expect(attr(svg, "viewBox")).toBe("124 150 736 30");
    expect(attr(svg, "width")).toBe("736pt");
    expect(attr(svg, "height")).toBe("30pt");
    // Le contenu ne change pas.
    expect(svg).toContain('<path d="M0 0"/>');
  });

  test("un widget de slide : toute la slide", () => {
    expect(croppedSvg(SVG, null)).toBe(SVG);
  });
});

const field = (owner: string, widget: string, x: number, y: number): FieldView => ({
  owner, owner_kind: "element", widget, field: "title", kind: "plain", label: null, text: "T", empty: false,
  bound: false, side: null, x, y, w: 50, h: 20, ax: x - 10, aw: 300, style: null,
});
const slideView = (fields: FieldView[], items: ItemView[] = [], lists: ListView[] = []) => ({
  slide: { id: "apercu", kind: ESSENTIEL.blank, params: {} }, page: 1, hash: "h", width: 1920, height: 1080,
  targets: [{ id: "x", kind: "kpi", free: false, x: 0, y: 0, w: 1, h: 1 }], fields, items, lists,
  zone: { page: 1, x: 0, y: 0, w: 1920, h: 1080, grid: null },
}) as unknown as SlideView;
const item = (owner: string, owner_kind: "element" | "slide", x: number, y: number): ItemView => ({
  owner, owner_kind, list: "etapes", item: "d0", index: 0, len: 1, min: 0, max: 8, item_label: null,
  x, y, w: 100, h: 50,
});
const list = (owner: string, owner_kind: "element" | "slide"): ListView => ({
  owner, owner_kind, list: "etapes", len: 1, min: 0, max: 8, item_label: null,
});

describe("croppedView", () => {
  test("un widget d'élément : ses champs dans le repère de sa boîte, à sa taille", () => {
    const v = croppedView(slideView([field("e", "kpi", 150, 230)]), { x: 100, y: 200, w: 600, h: 300 }, "kpi");
    expect([v.width, v.height]).toEqual([600, 300]);
    expect(v.fields.map((f) => [f.x, f.y, f.ax, f.w, f.h, f.aw])).toEqual([[50, 30, 40, 50, 20, 300]]);
  });

  test("un widget de slide : les champs tels quels, la slide entière", () => {
    const v = croppedView(slideView([field("apercu", "titre", 150, 230)]), null, "titre");
    expect([v.width, v.height]).toEqual([1920, 1080]);
    expect(v.fields.map((f) => [f.x, f.y, f.ax])).toEqual([[150, 230, 140]]);
  });

  test("seuls les champs du widget restent ; ni cibles ni zone", () => {
    const v = croppedView(slideView([field("apercu", "blank", 0, 0), field("e", "kpi", 10, 10)]), null, "kpi");
    expect(v.fields.map((f) => f.widget)).toEqual(["kpi"]);
    expect(v.targets).toEqual([]);
    expect(v.zone).toBeNull();
  });

  test("un widget d'élément : ses items dans le repère de sa boîte, l'élément pour seule cible, sur toute l'image", () => {
    const v = croppedView(slideView([], [item("e", "element", 150, 230)], [list("e", "element")]), { x: 100, y: 200, w: 600, h: 300 }, "kpi");
    expect(v.items.map((it) => [it.x, it.y, it.w, it.h])).toEqual([[50, 30, 100, 50]]);
    expect(v.targets).toEqual([{ id: "e", kind: "kpi", free: false, x: 0, y: 0, w: 600, h: 300 }]);
  });

  test("un widget de slide : ses items tels quels, sans cible", () => {
    const v = croppedView(slideView([], [item("apercu", "slide", 150, 230)], [list("apercu", "slide")]), null, "titre");
    expect(v.items.map((it) => [it.x, it.y])).toEqual([[150, 230]]);
    expect(v.targets).toEqual([]);
  });
});
