// Rendu de la scène dans un DOM happy-dom : couche des champs, placeholders,
// éditeur posé sur un champ, Tab entre les champs.

import { afterEach, afterAll, beforeAll, expect, test } from "bun:test";
import "./test/dom";
import type { FieldView, SlideView } from "./api";
import { setMessages } from "./i18n";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: Stage } = await import("./Stage");
const { createRef } = await import("react");
const { EditorView } = await import("@codemirror/view");

const ftl = (lang: string) =>
  Bun.file(new URL(`../../crates/oku-slides-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();

beforeAll(async () => {
  setMessages("fr", [await ftl("en"), await ftl("fr")]);
});
// Les autres fichiers de la passe attendent les identifiants bruts.
afterAll(() => setMessages("en", []));

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const style = { size: 20, leading: 6, tracking: 0, font: ["Inter"], weight: 400, fill: null, align: "left", case: null, bg: null } as const;
const fv = (owner: string, field: string, text: string, y: number, extra: Partial<FieldView> = {}): FieldView => ({
  owner, owner_kind: "element", widget: "card", field, kind: "plain", label: null, text, empty: false, side: null,
  x: 10, y, w: 100, h: 20, ax: 10, aw: 300, style: { ...style }, ...extra,
});
const view = {
  slide: { id: "s", kind: "content", params: { blocks: [{ id: "c1", kind: "card", params: { title: "Titre", kicker: "Sur" } }] } },
  page: 1, hash: "h", width: 1920, height: 1080, zone: null,
  targets: [{ id: "c1", kind: "card", free: false, x: 0, y: 0, w: 400, h: 200 }],
  fields: [fv("c1", "kicker", "Sur", 20), fv("c1", "title", "Titre", 60)],
} as unknown as SlideView;

type StageProps = Parameters<typeof Stage>[0];

async function mount(props: Partial<StageProps> = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const calls: { edit: (string | null)[]; apply: unknown[][]; select: (string | null)[]; notice: string[] } = {
    edit: [], apply: [], select: [], notice: [],
  };
  const root = createRoot(host);
  mounted = { root, host };
  const render = async (more: Partial<StageProps> = {}) => {
    await act(async () => {
      root.render(
        createElement(Stage, {
          view, svgUrl: null, selected: null, editing: null, palette: [],
          onSelect: (id: string | null) => calls.select.push(id),
          onEdit: (k: string | null) => calls.edit.push(k),
          onApply: (...a: unknown[]) => calls.apply.push(a),
          onNotice: (m: string) => calls.notice.push(m),
          ...props,
          ...more,
        }),
      );
    });
  };
  await render();
  return { host, calls, render };
}

/** Remplace le texte de l'éditeur ouvert, comme une saisie. */
const type = (content: Element, text: string) =>
  act(async () => {
    const cm = EditorView.findFromDOM(content as HTMLElement);
    cm?.dispatch({ changes: { from: 0, to: cm.state.doc.length, insert: text }, userEvent: "input.type" });
  });

const key = (el: Element, k: string, shiftKey = false) =>
  act(async () => {
    el.dispatchEvent(new KeyboardEvent("keydown", { key: k, shiftKey, bubbles: true, cancelable: true }));
  });

test("Stage double click on a card title edits that field", async () => {
  const { host, calls } = await mount();
  const title = host.querySelector('.field[aria-label="Titre"]')!;
  await act(async () => {
    title.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  });
  expect(calls.edit).toEqual(["c1\u001ftitle"]);
});

test("Stage double click on an element outside its fields edits its first filled field", async () => {
  const { host, calls } = await mount();
  await act(async () => {
    host.querySelector(".target")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  });
  expect(calls.edit).toEqual(["c1\u001fkicker"]);
});

test("Stage double click on a card without kicker edits its title, not the empty kicker", async () => {
  const empty = { ...fv("c1", "kicker", "", 60), empty: true, side: "before" as const, h: 0, w: 0 };
  const v = { ...view, fields: [empty, fv("c1", "title", "Titre", 60)] } as SlideView;
  const { host, calls } = await mount({ view: v });
  await act(async () => {
    host.querySelector(".target")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  });
  expect(calls.edit).toEqual(["c1\u001ftitle"]);
});

test("Stage editing a field opens the editor on its text", async () => {
  const { host } = await mount({ editing: "c1\u001ftitle" });
  expect(host.querySelector(".editor .cm-content")?.textContent).toBe("Titre");
  expect(host.querySelector(".editor-hint")?.textContent).toContain("Tab");
});

test("Stage empty field of the selected element shows its placeholder", async () => {
  const empty = { ...fv("c1", "kicker", "", 60), empty: true, side: "before" as const, h: 0, w: 0 };
  const v = { ...view, fields: [empty, fv("c1", "title", "Titre", 60)] } as SlideView;
  const { host } = await mount({ view: v, selected: "c1" });
  expect(host.querySelector(".field-placeholder")?.textContent).toBe("Surtitre");
  expect(host.querySelector(".field.empty")?.classList.contains("hot")).toBe(true);
});

test("Stage empty field of an unselected element shows no placeholder", async () => {
  const empty = { ...fv("c1", "kicker", "", 60), empty: true, side: "before" as const, h: 0, w: 0 };
  const v = { ...view, fields: [empty, fv("c1", "title", "Titre", 60)] } as SlideView;
  const { host } = await mount({ view: v });
  expect(host.querySelector(".field-placeholder")).toBeNull();
});

test("Stage empty field in template mode shows its placeholder", async () => {
  const empty = { ...fv("c1", "kicker", "", 60), empty: true, side: "before" as const, h: 0, w: 0 };
  const v = { ...view, fields: [empty, fv("c1", "title", "Titre", 60)] } as SlideView;
  const { host } = await mount({ view: v, templates: true });
  expect(host.querySelector(".field-placeholder")?.textContent).toBe("Surtitre");
});

test("Stage empty field that keeps a text shows that text ghosted", async () => {
  const hidden = { ...fv("s", "kicker", "Chapitre 2", 60), owner_kind: "slide" as const, empty: true, side: "before" as const, h: 0, w: 0 };
  const v = { ...view, fields: [hidden] } as SlideView;
  const { host } = await mount({ view: v, templates: true });
  expect(host.querySelector(".field-placeholder")?.textContent).toBe("Chapitre 2");
});

test("Stage without ghosts draws no ghost layer", async () => {
  const { host } = await mount();
  expect(host.querySelector(".stage-ghost-layer")).toBeNull();
});

test("Stage with ghosts outlines each element with its kind", async () => {
  const { host } = await mount({ showGhosts: true });
  const tags = [...host.querySelectorAll(".stage-ghost-target .stage-ghost-tag")].map((e) => e.textContent);
  expect(tags).toEqual(["Carte"]);
});

test("Stage with ghosts outlines each field with its parameter name", async () => {
  const { host } = await mount({ showGhosts: true });
  const tags = [...host.querySelectorAll(".stage-ghost-field .stage-ghost-tag")].map((e) => e.textContent);
  expect(tags).toEqual(["Surtitre", "Titre"]);
});

test("Stage with ghosts shows placeholders of empty fields without selection", async () => {
  const empty = { ...fv("c1", "kicker", "", 60), empty: true, side: "before" as const, h: 0, w: 0 };
  const v = { ...view, fields: [empty, fv("c1", "title", "Titre", 60)] } as SlideView;
  const { host } = await mount({ view: v, showGhosts: true });
  expect(host.querySelector(".field-placeholder")?.textContent).toBe("Surtitre");
});

test("Stage with ghosts outlines the body zone and tags gridded elements with their cell", async () => {
  const v = {
    ...view,
    slide: { id: "s", kind: "content", params: { elements: [{ id: "t1", kind: "text", cell: { col: 1, row: 0, cols: 2 } }] } },
    zone: { page: 1, x: 100, y: 100, w: 1000, h: 600, grid: { columns: 4, rows: 3, gap: 0 } },
    targets: [{ id: "t1", kind: "text", free: true, x: 350, y: 100, w: 500, h: 200 }],
    fields: [],
  } as unknown as SlideView;
  const { host } = await mount({ view: v, showGhosts: true });
  expect(host.querySelector(".stage-ghost-zone .stage-ghost-tag")?.textContent).toBe("Zone");
  expect(host.querySelector(".stage-ghost-target .stage-ghost-tag")?.textContent).toBe("Texte · col. 2–3 · lig. 1");
});

test("Stage read only with ghosts outlines elements and fields but shows no field layer", async () => {
  const { host } = await mount({ readOnly: true, showGhosts: true });
  expect(host.querySelectorAll(".stage-ghost-target").length).toBe(1);
  expect(host.querySelectorAll(".stage-ghost-field").length).toBe(2);
  expect(host.querySelector(".fields")).toBeNull();
});

/** Slide 1000 × 600 pt, grille 4 × 3 : un texte libre accroché en col. 2–3, lig. 1, au bord haut. */
const gridded = {
  ...view,
  width: 1000, height: 600,
  slide: { id: "s", kind: "content", params: { elements: [{ id: "t1", kind: "text", cell: { col: 1, row: 0, cols: 2 } }] } },
  zone: { page: 1, x: 0, y: 0, w: 1000, h: 600, grid: { columns: 4, rows: 3, gap: 0 } },
  targets: [{ id: "t1", kind: "text", free: true, x: 250, y: 0, w: 500, h: 200 }],
  fields: [],
} as unknown as SlideView;

test("Stage selected element on the grid is tagged with its kind and cell", async () => {
  const { host } = await mount({ view: gridded, selected: "t1" });
  expect(host.querySelector(".target.selected .tag")?.textContent).toBe("Texte · col. 2–3 · lig. 1");
});

test("Stage with ghosts puts the tags of boxes at the slide edges inside them", async () => {
  const { host } = await mount({ view: gridded, showGhosts: true });
  expect(host.querySelector(".stage-ghost-target")?.classList.contains("stage-ghost-in")).toBe(true);
  expect(host.querySelector(".stage-ghost-zone")?.classList.contains("stage-ghost-in")).toBe(true);
});

test("Stage with ghosts keeps the tags of boxes away from the edges outside them", async () => {
  // Scène non mesurée (happy-dom) : échelle 0,1 ; 500 pt font 50 px du bord.
  const v = { ...view, fields: [fv("c1", "title", "Titre", 500)] } as SlideView;
  const { host } = await mount({ view: v, showGhosts: true });
  expect(host.querySelector(".stage-ghost-field")?.classList.contains("stage-ghost-in")).toBe(false);
});

test("Stage with ghosts hides the ghost of the element being dragged", async () => {
  const { host } = await mount({ view: gridded, showGhosts: true, selected: "t1" });
  await act(async () => {
    host.querySelector(".target")!.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, clientX: 10, clientY: 10 }));
  });
  await act(async () => {
    host.querySelector(".stage")!.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientX: 200, clientY: 10 }));
  });
  expect(host.querySelector(".stage-ghost-target")).toBeNull();
});

test("Stage read only shows no field layer", async () => {
  const { host } = await mount({ readOnly: true });
  expect(host.querySelector(".fields")).toBeNull();
});

test("Stage Tab in a field saves it and edits the next one, in one undo step", async () => {
  const { host, calls, render } = await mount({ editing: "c1\u001fkicker" });
  const content = host.querySelector(".editor .cm-content")!;
  await type(content, "Neuf");
  await key(content, "Tab");
  expect(calls.apply.length).toBe(1);
  const [label, ops, merge] = calls.apply[0] as [string, unknown, string];
  expect(label).toBe("Modifier la slide");
  expect(ops).toEqual([{ op: "set_param", target: { slide: "s", element: "c1" }, param: "kicker", value: "Neuf" }]);
  expect(typeof merge).toBe("string");
  expect(calls.edit).toEqual(["c1\u001ftitle"]);

  // Le champ suivant garde la même clé de fusion : une seule étape d'annulation.
  await render({ editing: "c1\u001ftitle" });
  const next = host.querySelector(".editor .cm-content")!;
  await type(next, "Autre");
  await key(next, "Tab", true);
  expect(calls.apply.length).toBe(2);
  expect((calls.apply[1] as unknown[])[2]).toBe(merge);
  expect(calls.edit.at(-1)).toBe("c1\u001fkicker");
});

test("Stage Tab on an unchanged field applies nothing", async () => {
  const { host, calls } = await mount({ editing: "c1\u001fkicker" });
  await key(host.querySelector(".editor .cm-content")!, "Tab");
  expect(calls.apply).toEqual([]);
  expect(calls.edit).toEqual(["c1\u001ftitle"]);
});

test("Stage field gone after a render closes the editor with a notice", async () => {
  const { calls, render } = await mount({ editing: "c1\u001ftitle" });
  await render({ view: { ...view, fields: [] } as unknown as SlideView });
  expect(calls.edit).toEqual([null]);
  expect(calls.notice).toEqual(["Le champ modifié n'existe plus : la saisie est perdue."]);
  expect(calls.apply).toEqual([]);
});

test("Stage click on another field while editing saves and switches", async () => {
  const { host, calls } = await mount({ editing: "c1\u001fkicker" });
  const content = host.querySelector(".editor .cm-content")!;
  await type(content, "Neuf");
  await act(async () => {
    host.querySelector('.field[aria-label="Titre"]')!.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
  });
  expect(calls.apply.length).toBe(1);
  expect(calls.edit).toEqual(["c1\u001ftitle"]);
});

test("Stage a new editing session gets a new merge key", async () => {
  const { host, calls, render } = await mount({ editing: "c1\u001fkicker" });
  await type(host.querySelector(".editor .cm-content")!, "Un");
  await key(host.querySelector(".editor .cm-content")!, "Tab");
  await render({ editing: null });
  await render({ editing: "c1\u001fkicker" });
  await type(host.querySelector(".editor .cm-content")!, "Deux");
  await key(host.querySelector(".editor .cm-content")!, "Tab");
  expect(calls.apply.length).toBe(2);
  const [first, second] = calls.apply.map((a) => a[2]);
  expect(typeof first).toBe("string");
  expect(typeof second).toBe("string");
  expect(second).not.toBe(first);
});

test("Stage blur of the editor saves the field and closes the editor", async () => {
  const { host, calls } = await mount({ editing: "c1\u001ftitle" });
  const content = host.querySelector(".editor .cm-content")!;
  await type(content, "Neuf");
  await act(async () => {
    content.dispatchEvent(new FocusEvent("blur"));
  });
  expect(calls.apply.length).toBe(1);
  expect((calls.apply[0] as unknown[])[1]).toEqual([{ op: "set_param", target: { slide: "s", element: "c1" }, param: "title", value: "Neuf" }]);
  expect(typeof (calls.apply[0] as unknown[])[2]).toBe("string");
  expect(calls.edit).toEqual([null]);
});

test("Stage slide change while editing saves the draft instead of losing it", async () => {
  const stage = createRef<import("./Stage").StageHandle>();
  const { host, calls, render } = await mount({ editing: "c1\u001ftitle", ref: stage });
  await type(host.querySelector(".editor .cm-content")!, "Neuf");
  // L'application quitte la slide (focus de l'agent, vignette) : la saisie s'enregistre d'abord.
  await act(async () => stage.current?.commitEditing());
  await render({ editing: null, view: { ...view, slide: { ...view.slide, id: "s2" }, fields: [] } as unknown as SlideView });
  expect(calls.apply.length).toBe(1);
  expect((calls.apply[0] as unknown[])[1]).toEqual([{ op: "set_param", target: { slide: "s", element: "c1" }, param: "title", value: "Neuf" }]);
  expect(typeof (calls.apply[0] as unknown[])[2]).toBe("string");
  expect(calls.edit).toEqual([null]);
  expect(calls.notice).toEqual([]);
});

test("Stage Tab into a field of another element selects that element", async () => {
  const v = { ...view, fields: [...view.fields, fv("c2", "title", "Autre", 300)] } as SlideView;
  const { host, calls } = await mount({ view: v, selected: "c1", editing: "c1\u001ftitle" });
  await key(host.querySelector(".editor .cm-content")!, "Tab");
  expect(calls.edit).toEqual(["c2\u001ftitle"]);
  expect(calls.select).toEqual(["c2"]);
});

test("Stage click on a field of another element while editing selects that element", async () => {
  const v = { ...view, fields: [...view.fields, fv("c2", "title", "Autre", 300)] } as SlideView;
  const { host, calls } = await mount({ view: v, selected: "c1", editing: "c1\u001ftitle" });
  await act(async () => {
    host.querySelector('.field[aria-label="Titre"]:last-child')!.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
  });
  expect(calls.edit).toEqual(["c2\u001ftitle"]);
  expect(calls.select).toEqual(["c2"]);
});

test("Stage selected element of a pack widget: the tag is a button that opens the widget without moving or reselecting", async () => {
  const opened: string[] = [];
  const { host, calls } = await mount({ selected: "c1", packWidgets: new Set(["card"]), onOpenWidget: (n: string) => opened.push(n) });
  const tag = host.querySelector("button.tag-open")!;
  expect(tag).not.toBeNull();
  await act(async () => {
    tag.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
    tag.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
  expect(opened).toEqual(["card"]);
  expect(calls.apply).toEqual([]);
  expect(calls.select).toEqual([]);
});

test("Stage double click on the tag button does not edit the field", async () => {
  const { host, calls } = await mount({ selected: "c1", packWidgets: new Set(["card"]), onOpenWidget: () => {} });
  await act(async () => {
    host.querySelector("button.tag-open")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  });
  expect(calls.edit).toEqual([]);
});

test("Stage selected built-in element keeps a plain tag, no button", async () => {
  const { host } = await mount({ view: gridded, selected: "t1", packWidgets: new Set(["card"]), onOpenWidget: () => {} });
  expect(host.querySelector("button.tag-open")).toBeNull();
  expect(host.querySelector(".target.selected span.tag")).not.toBeNull();
});

test("Stage double click on a pack widget element, outside the tag, still edits its main field", async () => {
  const { host, calls } = await mount({ selected: "c1", packWidgets: new Set(["card"]), onOpenWidget: () => {} });
  await act(async () => {
    host.querySelector(".target")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  });
  expect(calls.edit).toEqual(["c1\u001fkicker"]);
});

test("Stage Enter or Space on the focused tag button stays on the button: the window shortcuts never see it", async () => {
  const { host, calls } = await mount({ selected: "c1", packWidgets: new Set(["card"]), onOpenWidget: () => {} });
  const seen: string[] = [];
  const l = (e: KeyboardEvent) => seen.push(e.key);
  window.addEventListener("keydown", l);
  try {
    const tag = host.querySelector<HTMLElement>("button.tag-open")!;
    tag.focus();
    for (const key of ["Enter", " "]) {
      await act(async () => {
        tag.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
      });
    }
  } finally {
    window.removeEventListener("keydown", l);
  }
  expect(seen).toEqual([]);
  expect(calls.edit).toEqual([]);
});
