import { describe, expect, test } from "bun:test";
import type { Element, Paragraph, SlideView } from "./api";
import { gridPlacement, instantiate, neighbourSlide, pasteOps, placeOnGridOps, slideToFetch, textToParagraphs } from "./deck";

describe("textToParagraphs", () => {
  test("lignes, puces et listes numérotées donnent leurs paragraphes", () => {
    expect(textToParagraphs("Titre\n- Point\n12. Douze")).toEqual([
      { runs: [{ t: "Titre" }] },
      { runs: [{ t: "Point" }], list: "bullet" },
      { runs: [{ t: "Douze" }], list: "number" },
    ]);
  });

  test("un paragraphe inchangé garde sa mise en forme", () => {
    const bold: Paragraph = { runs: [{ t: "Gras", bold: true }] };
    expect(textToParagraphs("Gras\nNouveau", [bold])[0]).toBe(bold);
  });
});

test("instantiate renouvelle les identifiants de la slide et des éléments", () => {
  const template = { id: "t", kind: "content" as const, elements: [{ id: "a", kind: "group" as const, children: [{ id: "b", kind: "text" as const }] }] };
  const copy = instantiate(template);
  expect(copy.id).not.toBe("t");
  expect(copy.elements?.[0].id).not.toBe("a");
  expect(copy.elements?.[0].children?.[0].id).not.toBe("b");
  expect(template.elements[0].id).toBe("a");
});

/** Une slide 1000 × 600 pt, grille 4 × 3 sans gouttière : des cases de 250 × 200. */
function view(slide: Partial<SlideView["slide"]>, targets: SlideView["targets"] = []): SlideView {
  return {
    slide: { id: "s", kind: "content", ...slide },
    page: 1,
    hash: "",
    width: 1920,
    height: 1080,
    targets,
    zone: { page: 1, x: 100, y: 100, w: 1000, h: 600, grid: { columns: 4, rows: 3, gap: 0 } },
  };
}

const box = (id: string, x: number, y: number, w: number, h: number) => ({
  id, target: "element" as const, kind: "text", free: false, text: "", x, y, w, h, ax: x, aw: w, style: null,
});

describe("gridPlacement et placeOnGridOps", () => {
  test("un élément accroché garde sa case ; le poser ailleurs ne touche que sa case", () => {
    const v = view({ elements: [{ id: "e", kind: "text", cell: { col: 1, row: 1 } }] });
    const p = gridPlacement(v, "e")!;
    expect(p).toMatchObject({ cell: { col: 1, row: 1 }, frame: { x: 250, y: 200, w: 250, h: 200 }, block: false, onCell: true });
    expect(placeOnGridOps(v, "e", { col: 2, row: 0 })).toEqual([
      { op: "set_cell", slide: "s", element: "e", cell: { col: 2, row: 0 } },
    ]);
  });

  test("un élément à cadre s'accroche à la case la plus proche et perd son cadre", () => {
    const v = view({ elements: [{ id: "e", kind: "text", frame: { x: 240, y: 10, w: 260, h: 190 } }] });
    const p = gridPlacement(v, "e")!;
    expect(p.cell).toEqual({ col: 1, row: 0, cols: 1, rows: 1 });
    expect(p.onCell).toBe(false);
    expect(placeOnGridOps(v, "e", p.cell).map((o) => o.op)).toEqual(["set_cell", "set_frame"]);
  });

  test("un bloc part de sa place à l'écran, et le poser fait passer toute la slide sur la grille", () => {
    const v = view(
      { blocks: [{ id: "a", kind: "text" }, { id: "b", kind: "text" }] },
      [box("a", 100, 100, 500, 200), box("b", 100, 300, 1000, 400)],
    );
    const p = gridPlacement(v, "a")!;
    expect(p).toMatchObject({ cell: { col: 0, row: 0, cols: 2, rows: 1 }, block: true });
    const ops = placeOnGridOps(v, "a", { col: 2, row: 0, cols: 2, rows: 1 });
    expect(ops.map((o) => o.op)).toEqual(["remove_element", "remove_element", "add_element", "add_element"]);
    expect(ops[2]).toMatchObject({ zone: "elements", element: { id: "a", cell: { col: 2, row: 0 } } });
    expect(ops[3]).toMatchObject({ element: { id: "b", cell: { col: 0, row: 1, cols: 4, rows: 2 } } });
  });

  test("un élément d'un groupe ou sans place ne se pose pas sur la grille", () => {
    const v = view({ elements: [{ id: "g", kind: "group", children: [{ id: "c", kind: "text", cell: { col: 0, row: 0 } }] }] });
    expect(gridPlacement(v, "c")).toBeNull();
    expect(gridPlacement(v, "g")).toBeNull();
  });
});

describe("pasteOps", () => {
  test("une copie sur la grille prend de nouveaux identifiants et la première ligne libre, même étendue", () => {
    const source = { id: "e", kind: "group" as const, cell: { col: 2, row: 0, cols: 2, rows: 1 }, children: [{ id: "c", kind: "text" as const }] };
    const v = view({ elements: [source, { id: "f", kind: "text", cell: { col: 0, row: 1 } }] });
    const { id, ops } = pasteOps(v, source);
    expect(ops).toHaveLength(1);
    expect(ops[0]).toMatchObject({ op: "add_element", slide: "s", zone: "elements", element: { id, cell: { col: 0, row: 2, cols: 2, rows: 1 } } });
    const el = ops[0].element as Element;
    expect(id).not.toBe("e");
    expect(el.children?.[0].id).not.toBe("c");
    expect(source.id).toBe("e");
  });

  test("une grille pleine reçoit la copie sur sa première ligne", () => {
    const source = { id: "e", kind: "text" as const, cell: { col: 0, row: 0, cols: 4, rows: 3 } };
    const { ops } = pasteOps(view({ elements: [source] }), source);
    expect(ops[0]).toMatchObject({ element: { cell: { col: 0, row: 0, cols: 4, rows: 3 } } });
  });

  test("un bloc collé sur une grille prend la moitié gauche de la ligne libre", () => {
    const { ops } = pasteOps(view({ elements: [] }), { id: "b", kind: "text" });
    expect(ops[0]).toMatchObject({ zone: "elements", element: { cell: { col: 0, row: 0, cols: 2, rows: 1 } } });
  });

  test("sans grille, un élément à cadre se colle décalé", () => {
    const v = { ...view({ elements: [] }), zone: null };
    const { ops } = pasteOps(v, { id: "e", kind: "shape", frame: { x: 10, y: 20, w: 100, h: 50 } });
    expect(ops[0]).toMatchObject({ zone: "elements", element: { frame: { x: 22, y: 32, w: 100, h: 50 } } });
  });

  test("une slide à blocs reçoit un bloc de plus, sans case ni cadre", () => {
    const v = view({ blocks: [{ id: "a", kind: "text" }] });
    const { ops } = pasteOps(v, { id: "e", kind: "text", cell: { col: 1, row: 1 } });
    expect(ops[0]).toMatchObject({ zone: "blocks" });
    expect((ops[0].element as Element).cell).toBeUndefined();
  });
});

describe("neighbourSlide", () => {
  const slides = [{ id: "a" }, { id: "b" }, { id: "c" }];

  test("neighbourSlide au milieu renvoie la suivante", () => {
    expect(neighbourSlide(slides, "b")).toBe("c");
  });

  test("neighbourSlide en dernier renvoie la précédente", () => {
    expect(neighbourSlide(slides, "c")).toBe("b");
  });

  test("neighbourSlide seule slide renvoie null", () => {
    expect(neighbourSlide([{ id: "a" }], "a")).toBeNull();
  });
});

describe("slideToFetch", () => {
  test("slideToFetch id présent renvoie l'id", () => {
    expect(slideToFetch([{ id: "a" }, { id: "b" }], "b")).toBe("b");
  });

  test("slideToFetch id supprimé renvoie null", () => {
    expect(slideToFetch([{ id: "a" }], "s-constat")).toBeNull();
  });
});
