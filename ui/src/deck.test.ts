import { describe, expect, test } from "bun:test";
import type { Element, FieldView, Paragraph, SlideView } from "./api";
import {
  fieldChanged,
  fieldOps,
  fieldValue,
  gridPlacement,
  instantiate,
  neighbourSlide,
  newElementOps,
  pasteOps,
  pickSlide,
  previewLook,
  placeOnGridOps,
  rich,
  slideToFetch,
  slot,
  textToParagraphs,
} from "./deck";

describe("textToParagraphs", () => {
  test("lignes, puces et listes numérotées donnent leurs paragraphes", () => {
    expect(textToParagraphs("Titre\n- Point\n12. Douze")).toEqual([
      { runs: [{ t: "Titre" }] },
      { runs: [{ t: "Point" }], list: "bullet" },
      { runs: [{ t: "Douze" }], list: "number" },
    ]);
  });

  test("une ligne inchangée garde son paragraphe", () => {
    const bold: Paragraph = { runs: [{ t: "Gras", bold: true, color: "accent" }] };
    const previous = { paragraphs: [bold], text: "**Gras**" };
    expect(textToParagraphs("**Gras**\nNouveau", previous)[0]).toBe(bold);
  });

  test("une ligne aux bords perdus garde son paragraphe tant qu'elle n'est pas éditée", () => {
    const edges: Paragraph = { runs: [{ t: "a" }, { t: " x ", bold: true, color: "accent" }, { t: "b" }] };
    const previous = { paragraphs: [edges], text: "a **x** b" };
    expect(textToParagraphs("Avant\na **x** b", previous)[1]).toBe(edges);
  });

  test("une ligne éditée est relue", () => {
    const bold: Paragraph = { runs: [{ t: "Gras", bold: true, color: "accent" }] };
    const previous = { paragraphs: [bold], text: "**Gras**" };
    expect(textToParagraphs("**Gras** !", previous)).toEqual([{ runs: [{ t: "Gras", bold: true }, { t: " !" }] }]);
  });

  test("numéro suivi d'une fin de ligne Windows : reste une liste, comme côté Rust", () => {
    expect(textToParagraphs("1. a\r")).toEqual([{ runs: [{ t: "a\r" }], list: "number" }]);
  });

  test("un run contenant un saut de ligne désaligne les indices : pas de réutilisation ni de doublon", () => {
    const previous = {
      paragraphs: [{ runs: [{ t: "a\nb" }] }, { runs: [{ t: "c" }] }] as Paragraph[],
      text: "a\nb\nc",
    };
    const edited = "a\nb\nc d";
    const result = textToParagraphs(edited, previous);
    const texts = result.map((p) => p.runs.map((r) => r.t).join(""));
    expect(new Set(texts).size).toBe(texts.length);
    expect(texts.join("\n")).toBe(edited);
  });
});

test("instantiate renouvelle les identifiants de la slide et des éléments, dans tous leurs slots", () => {
  const template = {
    id: "t",
    kind: "content",
    params: { elements: [{ id: "a", kind: "canvas", params: { children: [{ id: "b", kind: "text" }] } }] },
  };
  const copy = instantiate(template);
  expect(copy.id).not.toBe("t");
  const els = slot(copy, "elements");
  expect(els[0].id).not.toBe("a");
  expect(slot(els[0], "children")[0].id).not.toBe("b");
  expect((template.params.elements[0] as { id: string }).id).toBe("a");
});

/** Une slide 1000 × 600 pt, grille 4 × 3 sans gouttière : des cases de 250 × 200. */
function view(params: Record<string, unknown>, targets: SlideView["targets"] = []): SlideView {
  return {
    slide: { id: "s", kind: "content", params },
    page: 1,
    hash: "",
    width: 1920,
    height: 1080,
    targets,
    zone: { page: 1, x: 100, y: 100, w: 1000, h: 600, grid: { columns: 4, rows: 3, gap: 0 } },
  };
}

const box = (id: string, x: number, y: number, w: number, h: number) => ({
  id, kind: "text", free: false, x, y, w, h,
});

const fv: FieldView = {
  owner: "s", owner_kind: "slide", widget: "content", field: "title", kind: "plain", label: null,
  text: "Titre", empty: false, side: null, x: 0, y: 0, w: 0, h: 0, ax: 0, aw: 0, style: null,
  list: null, item: null, column: "title",
};

describe("fieldOps", () => {
  test("fieldOps of a slide field sets the slide param as text", () => {
    const ops = fieldOps(view({}), { ...fv, owner: "s", owner_kind: "slide", field: "title", kind: "plain" }, "Nouveau");
    expect(ops).toEqual([{ op: "set_param", target: { slide: "s" }, param: "title", value: "Nouveau" }]);
  });

  test("fieldOps of a plain field passes the text as is, without normalisation", () => {
    expect(fieldOps(view({}), fv, "   ")).toEqual([{ op: "set_param", target: { slide: "s" }, param: "title", value: "   " }]);
  });

  test("fieldOps of an element rich field sets paragraphs", () => {
    const v = view({ blocks: [{ id: "c1", kind: "card", params: { body: [{ runs: [{ t: "Avant" }] }] } }] });
    const ops = fieldOps(v, { ...fv, owner: "c1", owner_kind: "element", field: "body", kind: "rich", text: "Avant" }, "Titre\n- Point");
    expect(ops).toEqual([
      {
        op: "set_param",
        target: { slide: "s", element: "c1" },
        param: "body",
        value: [{ runs: [{ t: "Titre" }] }, { runs: [{ t: "Point" }], list: "bullet" }],
      },
    ]);
  });

  test("fieldOps of an element rich field keeps the runs of an unchanged line", () => {
    const colored: Paragraph = { runs: [{ t: "Rouge", color: "accent" }] };
    const v = view({ blocks: [{ id: "c1", kind: "card", params: { body: [colored] } }] });
    const ops = fieldOps(v, { ...fv, owner: "c1", owner_kind: "element", field: "body", kind: "rich", text: "Rouge" }, "Rouge\nB");
    expect((ops[0] as { value: Paragraph[] }).value[0]).toEqual(colored);
  });

  test("fieldOps of a slide rich field keeps the runs from the slide params", () => {
    const colored: Paragraph = { runs: [{ t: "Rouge", color: "accent" }] };
    const v = view({ body: [colored] });
    const ops = fieldOps(v, { ...fv, field: "body", kind: "rich", text: "Rouge" }, "Rouge\nB");
    expect(ops[0]).toMatchObject({ op: "set_param", target: { slide: "s" }, param: "body" });
    expect((ops[0] as { value: Paragraph[] }).value).toEqual([colored, { runs: [{ t: "B" }] }]);
  });

  test("fieldOps of an element plain field sets its text", () => {
    const ops = fieldOps(view({}), { ...fv, owner: "c1", owner_kind: "element", field: "title", kind: "plain" }, "T");
    expect(ops).toEqual([{ op: "set_param", target: { slide: "s", element: "c1" }, param: "title", value: "T" }]);
  });
});

describe("fieldOps d'un item de liste", () => {
  const frise = (etapes: unknown[]) => view({ blocks: [{ id: "f", kind: "frise", params: { etapes } }] });
  const itemField = (kind: "plain" | "rich", column: string, text = ""): FieldView => ({
    ...fv, owner: "f", owner_kind: "element", field: `etapes/d0/${column}`, list: "etapes", item: "d0", column, kind, text,
  });

  test("fieldOps : un champ d'item plain → edit_list set_field", () => {
    const ops = fieldOps(frise([{ id: "d0", date: "" }]), itemField("plain", "date"), "2020");
    expect(ops).toEqual([{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes",
      change: { op: "set_field", item: "d0", field: "date", value: "2020" } }]);
  });

  test("fieldOps : un champ d'item rich → paragraphes, les lignes inchangées gardent leurs runs", () => {
    const colored: Paragraph = { runs: [{ t: "Rouge", color: "accent" }] };
    const ops = fieldOps(frise([{ id: "d0", texte: [colored] }]), itemField("rich", "texte", "Rouge"), "Rouge\nB");
    expect(ops).toEqual([{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes",
      change: { op: "set_field", item: "d0", field: "texte", value: [colored, { runs: [{ t: "B" }] }] } }]);
  });

  test("fieldOps : un champ d'item d'une slide vise la slide", () => {
    const ops = fieldOps(view({ etapes: [{ id: "d0" }] }), { ...itemField("plain", "date"), owner: "s", owner_kind: "slide" }, "x");
    expect(ops[0]).toMatchObject({ op: "edit_list", target: { slide: "s" }, param: "etapes" });
  });
});

describe("fieldValue", () => {
  test("fieldValue of a plain field is its text", () => {
    expect(fieldValue(view({}), fv, "Météo")).toBe("Météo");
  });

  test("fieldValue of a rich field is the paragraphs set_param would write, unchanged lines keeping their runs", () => {
    const colored: Paragraph = { runs: [{ t: "Rouge", color: "accent" }] };
    const v = view({ blocks: [{ id: "c1", kind: "card", params: { body: [colored] } }] });
    const f = { ...fv, owner: "c1", owner_kind: "element" as const, field: "body", kind: "rich" as const, text: "Rouge" };
    expect(fieldValue(v, f, "Rouge\n- **Point**")).toEqual([
      colored,
      { runs: [{ t: "Point", bold: true }], list: "bullet" },
    ]);
  });

  test("fieldValue of a rich list item field keeps the runs of its column", () => {
    const colored: Paragraph = { runs: [{ t: "Rouge", color: "accent" }] };
    const v = view({ blocks: [{ id: "f", kind: "frise", params: { etapes: [{ id: "d0", texte: [colored] }] } }] });
    const f: FieldView = {
      ...fv, owner: "f", owner_kind: "element", field: "etapes/d0/texte", list: "etapes", item: "d0", column: "texte", kind: "rich", text: "Rouge",
    };
    expect(fieldValue(v, f, "Rouge\nB")).toEqual([colored, { runs: [{ t: "B" }] }]);
  });
});

describe("rich", () => {
  test("un tableau de paragraphes est rendu tel quel", () => {
    const e = { id: "e", kind: "text", params: { body: [{ runs: [{ t: "a" }] }] } };
    expect(rich(e, "body")).toEqual([{ runs: [{ t: "a" }] }]);
  });

  test("un tableau qui a la forme d'un slot (items avec id et kind) est refusé, symétrique de slot()", () => {
    const e = { id: "e", kind: "card", params: { body: [{ id: "c", kind: "text" }] } };
    expect(rich(e, "body")).toBeUndefined();
  });

  test("un paramètre absent ou non tableau rend undefined", () => {
    const e = { id: "e", kind: "text", params: { title: "x" } };
    expect(rich(e, "body")).toBeUndefined();
    expect(rich(e, "title")).toBeUndefined();
  });
});

describe("fieldChanged", () => {
  test("fieldChanged of a plain field ignores newlines turned into spaces", () => {
    expect(fieldChanged({ kind: "plain", text: "a\nb" }, "a b")).toBe(false);
    expect(fieldChanged({ kind: "plain", text: "a\nb" }, "a c")).toBe(true);
    expect(fieldChanged({ kind: "plain", text: "" }, "")).toBe(false);
  });

  test("fieldChanged of a rich field counts newlines as content", () => {
    expect(fieldChanged({ kind: "rich", text: "a\nb" }, "a b")).toBe(true);
    expect(fieldChanged({ kind: "rich", text: "a\nb" }, "a\nb")).toBe(false);
  });
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
    expect(ops[2]).toMatchObject({ slot: { owner: { slide: "s" }, param: "elements" }, element: { id: "a", cell: { col: 2, row: 0 } } });
    expect(ops[3]).toMatchObject({ element: { id: "b", cell: { col: 0, row: 1, cols: 4, rows: 2 } } });
  });

  test("un élément d'un groupe ou sans place ne se pose pas sur la grille", () => {
    const v = view({ elements: [{ id: "g", kind: "canvas", params: { children: [{ id: "c", kind: "text", cell: { col: 0, row: 0 } }] } }] });
    expect(gridPlacement(v, "c")).toBeNull();
    expect(gridPlacement(v, "g")).toBeNull();
  });
});

describe("pasteOps", () => {
  test("une copie sur la grille prend de nouveaux identifiants et la première ligne libre, même étendue", () => {
    const source = { id: "e", kind: "canvas", cell: { col: 2, row: 0, cols: 2, rows: 1 }, params: { children: [{ id: "c", kind: "text" }] } };
    const v = view({ elements: [source, { id: "f", kind: "text", cell: { col: 0, row: 1 } }] });
    const { id, ops } = pasteOps(v, source);
    expect(ops).toHaveLength(1);
    expect(ops[0]).toMatchObject({ op: "add_element", slot: { owner: { slide: "s" }, param: "elements" }, element: { id, cell: { col: 0, row: 2, cols: 2, rows: 1 } } });
    const el = ops[0].element as Element;
    expect(id).not.toBe("e");
    expect((el.params?.children as Element[])[0].id).not.toBe("c");
    expect(source.id).toBe("e");
  });

  test("une grille pleine reçoit la copie sur sa première ligne", () => {
    const source = { id: "e", kind: "text", cell: { col: 0, row: 0, cols: 4, rows: 3 } };
    const { ops } = pasteOps(view({ elements: [source] }), source);
    expect(ops[0]).toMatchObject({ element: { cell: { col: 0, row: 0, cols: 4, rows: 3 } } });
  });

  test("un bloc collé sur une grille prend la moitié gauche de la ligne libre", () => {
    const { ops } = pasteOps(view({ elements: [] }), { id: "b", kind: "text" });
    expect(ops[0]).toMatchObject({ slot: { owner: { slide: "s" }, param: "elements" }, element: { cell: { col: 0, row: 0, cols: 2, rows: 1 } } });
  });

  test("sans grille, un élément à cadre se colle décalé", () => {
    const v = { ...view({ elements: [] }), zone: null };
    const { ops } = pasteOps(v, { id: "e", kind: "shape", frame: { x: 10, y: 20, w: 100, h: 50 } });
    expect(ops[0]).toMatchObject({ slot: { owner: { slide: "s" }, param: "elements" }, element: { frame: { x: 22, y: 32, w: 100, h: 50 } } });
  });

  test("une slide à blocs reçoit un bloc de plus, sans case ni cadre", () => {
    const v = view({ blocks: [{ id: "a", kind: "text" }] });
    const { ops } = pasteOps(v, { id: "e", kind: "text", cell: { col: 1, row: 1 } });
    expect(ops[0]).toMatchObject({ slot: { owner: { slide: "s" }, param: "blocks" } });
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

describe("pickSlide", () => {
  const slides = [{ id: "a" }, { id: "b" }, { id: "c" }];
  test("pickSlide courante présente la garde", () => {
    expect(pickSlide(slides, "b", "c")).toBe("b");
  });
  test("pickSlide sans courante prend la slide retenue", () => {
    expect(pickSlide(slides, null, "c")).toBe("c");
  });
  test("pickSlide retenue disparue prend la première", () => {
    expect(pickSlide(slides, null, "z")).toBe("a");
  });
  test("pickSlide deck vide renvoie null", () => {
    expect(pickSlide([], null, "a")).toBeNull();
  });
});


describe("amorce des listes (defaults)", () => {
  test("newElementOps : un élément neuf de la bibliothèque demande l'amorce des listes", () => {
    const ops = newElementOps(view({ blocks: [] }), { id: "w-1", kind: "frise" });
    expect(ops).toEqual([{ op: "add_element", slot: { owner: { slide: "s" }, param: "elements" }, defaults: true,
      element: { id: "w-1", kind: "frise", cell: expect.anything() } }]);
  });

  test("newElementOps : sur une slide à blocs, un bloc de plus, avec l'amorce", () => {
    const ops = newElementOps(view({ blocks: [{ id: "b", kind: "text" }] }), { id: "w-1", kind: "frise" });
    expect(ops).toEqual([{ op: "add_element", slot: { owner: { slide: "s" }, param: "blocks" }, defaults: true, element: { id: "w-1", kind: "frise" } }]);
  });

  test("pasteOps : une copie collée garde ses items, sans amorce", () => {
    const { ops } = pasteOps(view({ blocks: [] }), { id: "f", kind: "frise", params: { etapes: [] } });
    expect(ops[0]).not.toHaveProperty("defaults");
  });
});

describe("previewLook", () => {
  type Source = Parameters<typeof previewLook>[0];
  const draft = (dirty: boolean, version = "v1", revision = "r1") => ({
    theme: { base: { id: "t", version }, name: "Papier", builtin: true, dirty, revision },
  }) as Source["editing"];
  // Mode modèles : le deck de travail n'a pas de thème, le brouillon en tient lieu.
  const snap = (over: Partial<Source> = {}): Source => ({
    theme: null,
    appearance: { slide: { accent: "#123456" } } as Source["appearance"],
    editing: draft(false),
    undo_label: null,
    redo_label: null,
    ...over,
  });

  test("previewLook_draft_change_without_deck_theme_changes_the_key", () => {
    expect(previewLook(snap({ editing: draft(true) }))).not.toBe(previewLook(snap()));
  });

  test("previewLook_new_base_changes_the_key", () => {
    expect(previewLook(snap({ editing: draft(false, "v2") }))).not.toBe(previewLook(snap()));
  });

  test("previewLook_undo_or_redo_changes_the_key", () => {
    const before = previewLook(snap({ undo_label: "Jeton" }));
    expect(previewLook(snap({ undo_label: null, redo_label: "Jeton" }))).not.toBe(before);
  });

  test("previewLook_new_draft_revision_with_same_labels_changes_the_key", () => {
    // Deux réglages successifs du même jeton (taille 16 → 18 → 20) : même libellé, même `dirty`.
    const at = (revision: string) => previewLook(snap({ editing: draft(true, "v1", revision), undo_label: "Jeton" }));
    expect(at("r2")).not.toBe(at("r1"));
  });

  test("previewLook_same_snapshot_keeps_the_key", () => {
    expect(previewLook(snap())).toBe(previewLook(snap()));
  });
});
