// Rendu de la scène dans un DOM happy-dom : couche des champs, placeholders,
// éditeur posé sur un champ, Tab entre les champs.

import { afterEach, afterAll, beforeAll, expect, test } from "bun:test";
import "./test/dom";
import type { FieldView, SlideView } from "./api";
import { setMessages } from "./i18n";
import { fieldKey } from "./stageFields";
import { resetMessages } from "./test/messages";

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
afterAll(resetMessages);

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
  owner, owner_kind: "element", widget: "card", field, list: null, item: null, column: field, kind: "plain", label: null, text, empty: false, side: null,
  x: 10, y, w: 100, h: 20, ax: 10, aw: 300, style: { ...style }, ...extra,
});
const view = {
  slide: { id: "s", kind: "content", params: { blocks: [{ id: "c1", kind: "card", params: { title: "Titre", kicker: "Sur" } }] } },
  page: 1, hash: "h", width: 1920, height: 1080, zone: null,
  targets: [{ id: "c1", kind: "card", free: false, x: 0, y: 0, w: 400, h: 200 }],
  fields: [fv("c1", "kicker", "Sur", 20), fv("c1", "title", "Titre", 60)],
  items: [], lists: [],
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

test("Stage marks the cut element", async () => {
  const { host, render } = await mount({ cut: "c1" });
  expect(host.querySelector(".target")?.classList.contains("cut")).toBe(true);
  await render({ cut: null });
  expect(host.querySelector(".target")?.classList.contains("cut")).toBe(false);
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
  // Hors contexte sûr, `crypto.randomUUID` lève : l'éditeur doit s'ouvrir quand même (t-695).
  const original = Object.getOwnPropertyDescriptor(globalThis.crypto, "randomUUID");
  Object.defineProperty(globalThis.crypto, "randomUUID", {
    value: () => { throw new TypeError("crypto.randomUUID is not a function"); },
    configurable: true,
  });
  try {
    await newMergeKeyScenario();
  } finally {
    if (original) Object.defineProperty(globalThis.crypto, "randomUUID", original);
    else delete (globalThis.crypto as { randomUUID?: unknown }).randomUUID;
  }
});

async function newMergeKeyScenario() {
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
}

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

/** La vue où le titre de `c1` vaut `text` (son surtitre `kicker`) : une écriture de l'agent pendant l'édition. */
const titled = (text: string, kicker = "Sur") =>
  ({ ...view, fields: [fv("c1", "kicker", kicker, 20), fv("c1", "title", text, 60)] }) as unknown as SlideView;

test("Stage agent write to the open field without local change shows the new value", async () => {
  const { host, calls, render } = await mount({ editing: "c1\u001ftitle" });
  await render({ view: titled("Agent") });
  const content = host.querySelector(".editor .cm-content")!;
  expect(content.textContent).toBe("Agent");
  await act(async () => {
    content.dispatchEvent(new FocusEvent("blur"));
  });
  expect(calls.apply).toEqual([]);
  expect(calls.notice).toEqual([]);
});

test("Stage agent write to the open field after local typing keeps the draft", async () => {
  const { host, calls, render } = await mount({ editing: "c1\u001ftitle" });
  await type(host.querySelector(".editor .cm-content")!, "Neuf");
  await render({ view: titled("Agent") });
  expect(host.querySelector(".editor .cm-content")?.textContent).toBe("Neuf");
  expect(calls.apply).toEqual([]);
  expect(calls.notice).toEqual([]);
});

test("Stage commit over an agent write keeps the user text in its own undo step, with a notice", async () => {
  const { host, calls, render } = await mount({ editing: "c1\u001fkicker" });
  await type(host.querySelector(".editor .cm-content")!, "Un");
  await key(host.querySelector(".editor .cm-content")!, "Tab");
  await render({ editing: "c1\u001ftitle", view: titled("Titre", "Un") });
  const content = host.querySelector(".editor .cm-content")!;
  await type(content, "Neuf");
  await render({ editing: "c1\u001ftitle", view: titled("Agent", "Un") });
  await act(async () => {
    content.dispatchEvent(new FocusEvent("blur"));
  });
  expect(calls.apply.length).toBe(2);
  const [, ops, merge] = calls.apply[1] as [string, unknown, string];
  expect(ops).toEqual([{ op: "set_param", target: { slide: "s", element: "c1" }, param: "title", value: "Neuf" }]);
  // Une étape d'annulation à part : Annuler rend le texte de l'agent, sans défaire le reste de la saisie.
  expect(merge).not.toBe((calls.apply[0] as unknown[])[2]);
  expect(calls.notice).toEqual(["Ce texte a changé pendant votre saisie : votre version l'a remplacé. Annuler rétablit l'autre."]);
});

test("Stage commit of a text equal to the agent write shows no notice", async () => {
  const { host, calls, render } = await mount({ editing: "c1\u001ftitle" });
  const content = host.querySelector(".editor .cm-content")!;
  await type(content, "Agent");
  await render({ view: titled("Agent") });
  await key(content, "Tab");
  expect(calls.apply).toEqual([]);
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

// --- Items de liste ---------------------------------------------------------

const itemOf = (id: string, index: number, x: number) => ({
  owner: "f", owner_kind: "element", list: "etapes", item: id, index, len: 2, min: 0, max: 8,
  item_label: "étape", x, y: 100, w: 100, h: 50,
});
/** Une frise de deux étapes côte à côte, chacune avec son champ `date`. */
const frise = {
  ...view,
  slide: { id: "s", kind: "content", params: { blocks: [{ id: "f", kind: "frise", params: { etapes: [{ id: "a", date: "2019" }, { id: "b", date: "2020" }] } }] } },
  targets: [{ id: "f", kind: "frise", free: false, x: 0, y: 100, w: 400, h: 200 }],
  fields: [
    fv("f", "etapes/a/date", "2019", 110, { list: "etapes", item: "a", column: "date", x: 0 }),
    fv("f", "etapes/b/date", "2020", 110, { list: "etapes", item: "b", column: "date", x: 120 }),
  ],
  items: [itemOf("a", 0, 0), itemOf("b", 1, 120)],
  lists: [],
} as unknown as SlideView;

/** Touche avec Alt, et Cmd (Mac) ou Ctrl (ailleurs) si `mod` : le `Mod` de CodeMirror. */
const altKey = (el: Element, k: string, mod = true) =>
  act(async () => {
    const mac = /Mac/.test(navigator.platform);
    el.dispatchEvent(new KeyboardEvent("keydown", {
      key: k, altKey: true, metaKey: mod && mac, ctrlKey: mod && !mac, bubbles: true, cancelable: true,
    }));
  });

test("Stage a selected element with a list shows no item handles", async () => {
  const { host } = await mount({ view: frise, selected: "f" });
  expect(host.querySelector(".items, [data-item-remove], [data-item-move]")).toBeNull();
});

test("Stage Cmd/Ctrl+Alt+arrow along the list in an item field saves the text and moves the item", async () => {
  const { host, calls } = await mount({ view: frise, selected: "f", editing: "f\u001fetapes/a/date" });
  const content = host.querySelector(".editor .cm-content")!;
  await type(content, "2018");
  await altKey(content, "ArrowRight");
  expect(calls.apply.length).toBe(2);
  expect((calls.apply[0] as unknown[])[1]).toEqual([
    { op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes", change: { op: "set_field", item: "a", field: "date", value: "2018" } },
  ]);
  expect((calls.apply[1] as unknown[]).slice(0, 2)).toEqual([
    "Déplacer étape",
    [{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes", change: { op: "move", item: "a", to: 1 } }],
  ]);
  // L'éditeur reste ouvert sur le champ de l'item déplacé.
  expect(calls.edit).toEqual([]);
});

test("Stage Cmd/Ctrl+Alt+arrow past the start of the list moves nothing", async () => {
  const { host, calls } = await mount({ view: frise, selected: "f", editing: "f\u001fetapes/a/date" });
  await altKey(host.querySelector(".editor .cm-content")!, "ArrowLeft");
  expect(calls.apply).toEqual([]);
});

test("Stage Cmd/Ctrl+Alt+arrow across the list axis does not move the item", async () => {
  const { host, calls } = await mount({ view: frise, selected: "f", editing: "f\u001fetapes/b/date" });
  await altKey(host.querySelector(".editor .cm-content")!, "ArrowUp");
  expect(calls.apply).toEqual([]);
});

test("Stage plain Alt+arrow in an item field does not move the item", async () => {
  const { host, calls } = await mount({ view: frise, selected: "f", editing: "f\u001fetapes/a/date" });
  await altKey(host.querySelector(".editor .cm-content")!, "ArrowRight", false);
  expect(calls.apply).toEqual([]);
});

test("Stage Cmd/Ctrl+Alt+arrow in the only item of a list moves nothing", async () => {
  const one = { ...frise, items: [{ ...itemOf("a", 0, 0), len: 1 }], fields: [frise.fields[0]] } as SlideView;
  const { host, calls } = await mount({ view: one, selected: "f", editing: "f\u001fetapes/a/date" });
  await altKey(host.querySelector(".editor .cm-content")!, "ArrowRight");
  expect(calls.apply).toEqual([]);
});

// --- Mode aperçu (sous-onglet d'un widget) -----------------------------------

/** Slide d'aperçu d'un widget « kpi », recadrée : titre, corps riche, source liée au deck. */
const pv = (field: string, kind: "plain" | "rich", text: string, y: number, extra: Partial<FieldView> = {}) =>
  fv("apercu-el", field, text, y, { widget: "kpi", kind, ...extra });
const previewView = {
  slide: { id: "apercu", kind: "blank", params: { elements: [{ id: "apercu-el", kind: "kpi", params: { body: [{ runs: [{ t: "Corps" }] }] } }] } },
  page: 1, hash: "h", width: 600, height: 300, zone: null, targets: [],
  fields: [
    pv("title", "plain", "Titre", 20),
    pv("source", "plain", "Deck", 60, { bound: true }),
    pv("body", "rich", "Corps", 100),
  ],
} as unknown as SlideView;

async function mountPreview(props: Record<string, unknown> = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const calls: { edit: (string | null)[]; field: [string, string][] } = { edit: [], field: [] };
  const root = createRoot(host);
  mounted = { root, host };
  const render = async (more: Record<string, unknown> = {}) => {
    await act(async () => {
      root.render(
        createElement(Stage, {
          mode: "preview", view: previewView, svgUrl: "blob:apercu", editing: null, palette: [],
          onEdit: (k: string | null) => calls.edit.push(k),
          onField: (f: FieldView, text: string) => calls.field.push([f.field, text]),
          ...props,
          ...more,
        } as StageProps),
      );
    });
  };
  await render();
  return { host, calls, render };
}

const dblclick = (el: Element) =>
  act(async () => {
    el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  });

test("Stage preview shows the image and the fields, without objects, grid or ghosts", async () => {
  const withTarget = { ...previewView, targets: [{ id: "apercu-el", kind: "kpi", free: true, x: 0, y: 0, w: 600, h: 300 }] } as SlideView;
  const { host } = await mountPreview({ view: withTarget });
  expect(host.querySelector(".stage-wrap.stage-preview .stage > img")?.getAttribute("src")).toBe("blob:apercu");
  expect(host.querySelector(".stage > img")?.getAttribute("alt")).toBe("Aperçu du widget");
  expect(host.querySelector(".target")).toBeNull();
  expect(host.querySelector(".grid-layer")).toBeNull();
  expect(host.querySelectorAll(".field").length).toBe(3);
});

test("Stage preview fields are all hot, their empty placeholders shown without selection", async () => {
  const empty = pv("title", "plain", "", 20, { empty: true, side: "before", h: 0, w: 0 });
  const { host } = await mountPreview({ view: { ...previewView, fields: [empty, previewView.fields[2]] } as SlideView });
  expect([...host.querySelectorAll(".field")].every((f) => f.classList.contains("hot"))).toBe(true);
  expect(host.querySelector(".field-placeholder")?.textContent).toBe("Titre");
});

test("Stage preview double click on a plain field edits it on one line", async () => {
  const { host, calls, render } = await mountPreview();
  await dblclick(host.querySelector('.field[aria-label="Titre"]')!);
  expect(calls.edit).toEqual(["apercu-el\u001ftitle"]);
  await render({ editing: "apercu-el\u001ftitle" });
  expect(host.querySelector(".editor .cm-content")?.getAttribute("aria-multiline")).toBe("false");
});

test("Stage preview editing a rich field opens a markdown editor", async () => {
  const { host } = await mountPreview({ editing: "apercu-el\u001fbody" });
  const content = host.querySelector(".editor .cm-content");
  expect(content?.getAttribute("aria-multiline")).toBe("true");
  expect(content?.textContent).toBe("Corps");
});

test("Stage preview blur sends the field and its text, never deck ops", async () => {
  const { host, calls } = await mountPreview({ editing: "apercu-el\u001ftitle" });
  const content = host.querySelector(".editor .cm-content")!;
  await type(content, "Météo");
  await act(async () => {
    content.dispatchEvent(new FocusEvent("blur"));
  });
  expect(calls.field).toEqual([["title", "Météo"]]);
  expect(calls.edit).toEqual([null]);
});

test("Stage preview Tab sends the field and skips the bound one", async () => {
  const { host, calls } = await mountPreview({ editing: "apercu-el\u001ftitle" });
  const content = host.querySelector(".editor .cm-content")!;
  await type(content, "Neuf");
  await key(content, "Tab");
  expect(calls.field).toEqual([["title", "Neuf"]]);
  expect(calls.edit).toEqual(["apercu-el\u001fbody"]);
});

test("Stage preview Escape closes the editor without sending anything", async () => {
  const { host, calls } = await mountPreview({ editing: "apercu-el\u001ftitle" });
  const content = host.querySelector(".editor .cm-content")!;
  await type(content, "Abandon");
  await key(content, "Escape");
  expect(calls.field).toEqual([]);
  expect(calls.edit).toEqual([null]);
});

test("Stage preview field bound to the deck is shown read only, with its reason", async () => {
  const { host, calls } = await mountPreview();
  const bound = host.querySelector('.field[aria-label="source"]')!;
  expect(bound.classList.contains("bound")).toBe(true);
  expect(bound.classList.contains("hot")).toBe(false);
  expect(bound.getAttribute("title")).toBe("Lié à la présentation : en lecture ici");
  await dblclick(bound);
  expect(calls.edit).toEqual([]);
});

test("Stage preview never edits a bound field, even asked to", async () => {
  const { host } = await mountPreview({ editing: "apercu-el\u001fsource" });
  expect(host.querySelector(".editor")).toBeNull();
});

test("Stage preview keeps the draft when the preview is rendered again", async () => {
  const { host, calls, render } = await mountPreview({ editing: "apercu-el\u001ftitle" });
  await type(host.querySelector(".editor .cm-content")!, "En cours");
  await render({ view: { ...previewView, hash: "h2" } as SlideView, svgUrl: "blob:apercu-2" });
  expect(host.querySelector(".editor .cm-content")?.textContent).toBe("En cours");
  expect(calls.field).toEqual([]);
});

test("Stage preview is never drawn larger than the image itself (1 pt = 4/3 px)", async () => {
  const proto = HTMLElement.prototype;
  const w = Object.getOwnPropertyDescriptor(proto, "clientWidth");
  const h = Object.getOwnPropertyDescriptor(proto, "clientHeight");
  Object.defineProperty(proto, "clientWidth", { configurable: true, get: () => 4000 });
  Object.defineProperty(proto, "clientHeight", { configurable: true, get: () => 4000 });
  try {
    const { host } = await mountPreview();
    const stage = host.querySelector<HTMLElement>(".stage")!;
    expect([stage.style.width, stage.style.height]).toEqual(["800px", "400px"]);
  } finally {
    if (w) Object.defineProperty(proto, "clientWidth", w);
    if (h) Object.defineProperty(proto, "clientHeight", h);
  }
});

/** Rectangles imposés par classe (happy-dom ne fait pas de mise en page). */
function stubRects(rects: Record<string, { left: number; top: number; width: number; height: number }>) {
  const original = HTMLElement.prototype.getBoundingClientRect;
  HTMLElement.prototype.getBoundingClientRect = function (this: HTMLElement) {
    const r = Object.entries(rects).find(([cls]) => this.classList.contains(cls))?.[1];
    if (!r) return original.call(this);
    return { ...r, x: r.left, y: r.top, right: r.left + r.width, bottom: r.top + r.height, toJSON: () => r } as DOMRect;
  };
  return () => {
    HTMLElement.prototype.getBoundingClientRect = original;
  };
}

test("Stage agent sheet near the bottom flips above its element and stays inside the sheet zone", async () => {
  // Zone de la feuille (stage-wrap) plus grande que la scène : cadre en (50, 20) dans la zone.
  const restore = stubRects({
    "stage-wrap": { left: 0, top: 0, width: 300, height: 200 },
    "stage-frame": { left: 50, top: 20, width: 192, height: 108 },
    "agent-sheet": { left: 0, top: 0, width: 120, height: 100 },
  });
  try {
    // Échelle 0,1 (happy-dom : la zone mesure 0) : l'élément est à (180, 100) px, 10 × 6.
    const v = { ...view, targets: [{ id: "c1", kind: "card", free: false, x: 1800, y: 1000, w: 100, h: 60 }] } as SlideView;
    const { host } = await mount({ view: v, overlay: { anchor: { owner: "c1" }, node: createElement("p", null, "fiche") } });
    const sheet = host.querySelector<HTMLElement>(".agent-sheet")!;
    // Dessous : 100 + 6 + 8 + 100 > 180 (bas de la zone) ; dessus : 100 − 8 − 100 = −8 ≥ −20.
    expect(sheet.style.top).toBe("-8px");
    // 180 + 120 > 250 (bord droit de la zone) : recadrée à 130.
    expect(sheet.style.left).toBe("130px");
    expect(sheet.classList.contains("above")).toBe(true);
    expect(sheet.style.visibility).toBe("");
  } finally {
    restore();
  }
});

test("Stage agent sheet keeps one size observer across renders", async () => {
  const original = globalThis.ResizeObserver;
  let created = 0;
  globalThis.ResizeObserver = class {
    constructor() {
      created++;
    }
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
  try {
    const overlay = { anchor: { owner: "c1" }, node: createElement("p", null, "fiche") };
    const { render } = await mount({ overlay });
    const before = created;
    await render({ selected: "c1" });
    await render({ selected: null });
    expect(created).toBe(before);
  } finally {
    globalThis.ResizeObserver = original;
  }
});
