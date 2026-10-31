// A pack tab's pile: the shared file, one thumbnail per item, add, rename and delete.

import { afterEach, describe, expect, spyOn, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi, WidgetChange } from "./api";
import type { PackKind } from "./packTypes";
import type { Confirm, Question } from "./useConfirm";

const { act, createElement, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: PackPile } = await import("./PackPile");
const { useWidgetFiles } = await import("./useWidgetFiles");
const { itemSlide } = await import("./packItems");
const { COMMON, skeleton } = await import("./widgetList");
const { memory, stub, wait } = await import("./test/widgetApi");

type Files = ReturnType<typeof useWidgetFiles>;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

interface Over {
  kind?: PackKind;
  confirm?: Confirm;
  selected?: string | null;
}

/** The pile over a real file queue; the snapshot's slides follow the statuses read. */
async function mount(fake: TabApi, over: Over = {}) {
  const kind = over.kind ?? "widgets";
  const selections: string[] = [];
  const removed: string[] = [];
  const errors: string[] = [];
  const asked: string[] = [];
  const box: { files: Files | null } = { files: null };
  const confirm = over.confirm ?? (async (q: Question) => (asked.push(q.message), true));
  const Probe = () => {
    const files = useWidgetFiles(fake, "p", kind, "r", { changed: () => {}, error: (m) => errors.push(m), renamed: () => {} });
    box.files = files;
    const [selected, setSelected] = useState<string | null>(over.selected ?? null);
    const names = (files.statuses ?? []).map((s) => s.name).filter((n) => n !== COMMON);
    const snap = {
      editing: { pack: "p", base: "b", kind },
      slides: names.map((n, i) => ({ id: itemSlide(kind, n), kind: "essentiel/content", title: null, page: i + 1, hash: `h${i}` })),
    } as unknown as Snapshot;
    return createElement(PackPile, {
      snap, kind, files, selected,
      svg: (id: string) => `blob:${id}`,
      onSelect: (n: string) => { selections.push(n); setSelected(n); },
      onRemoved: (n: string) => removed.push(n),
      confirm,
    });
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Probe)));
  await wait(10);
  return { host, selections, removed, errors, asked, box };
}

const thumbs = (host: HTMLElement) => [...host.querySelectorAll<HTMLButtonElement>(".thumb")];
const thumb = (host: HTMLElement, name: string) => thumbs(host).find((b) => b.textContent?.includes(name)) as HTMLButtonElement;
const common = (host: HTMLElement) => host.querySelector<HTMLButtonElement>(".pack-pile-common .pack-pile-item");
const add = (host: HTMLElement) => host.querySelector<HTMLButtonElement>(".pack-pile-add") as HTMLButtonElement;
const nameInput = (host: HTMLElement) => host.querySelector<HTMLInputElement>(".pack-pile-name input");
const trashOf = (host: HTMLElement, name: string) => thumb(host, name).parentElement?.querySelector(".thumb-remove") as HTMLButtonElement;
const press = (target: HTMLElement, key: string) =>
  act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true })));
const dblclick = (target: HTMLElement) => act(async () => void target.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));

async function typeName(host: HTMLElement, value: string) {
  const input = nameInput(host);
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value);
    input?.dispatchEvent(new Event("input", { bubbles: true }));
  });
  return nameInput(host) as HTMLInputElement;
}
async function submitName(host: HTMLElement, value: string) {
  await typeName(host, value);
  await act(async () => void host.querySelector("form.pack-pile-name")?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
  await wait(10);
}
const issue = (host: HTMLElement) => {
  const id = nameInput(host)?.getAttribute("aria-describedby");
  return id ? document.getElementById(id)?.textContent ?? null : null;
};

describe("PackPile, items", () => {
  test("PackPile shows the shared row first when the file exists", async () => {
    const { fake } = memory({ _commun: "c", kpi: "k" });
    const { host } = await mount(fake);
    const items = [...host.querySelectorAll(".pack-pile-item, .thumb")];
    expect([items[0] === common(host), items.length]).toEqual([true, 2]);
  });

  test("PackPile without the shared file has no shared row", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    expect(common(host)).toBeNull();
  });

  test("PackPile shows one img thumbnail per item, named after it, and no inline svg", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const { host } = await mount(fake);
    const imgs = [...host.querySelectorAll<HTMLImageElement>(".thumb img")];
    expect(imgs.map((i) => i.alt)).toEqual(["chart", "kpi"]);
    expect(host.querySelectorAll(".thumb svg, object, iframe").length).toBe(0);
  });

  test("PackPile Enter on a thumbnail selects its item", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host, selections } = await mount(fake);
    await press(thumb(host, "kpi"), "Enter");
    expect(selections).toEqual(["kpi"]);
  });

  test.each([["widgets", "ui-widgets-new"], ["layouts", "ui-layouts-new"]] as [PackKind, string][])(
    "PackPile of a %s pack names its add button %s",
    async (kind, label) => {
      const { fake } = memory({});
      const { host } = await mount(fake, { kind });
      expect(add(host).getAttribute("aria-label")).toBe(label);
    },
  );

  test("PackPile disables the add button before the statuses arrive", async () => {
    const { fake } = memory({});
    stub(fake, "widgetStatus", () => new Promise(() => {}));
    const { host } = await mount(fake);
    expect(add(host).disabled).toBe(true);
  });

  test.each([["widgets", "ui-widgets-empty"], ["layouts", "ui-layouts-empty"]] as [PackKind, string][])(
    "PackPile of an empty %s pack says %s, hidden while naming",
    async (kind, text) => {
      const { fake } = memory({});
      const { host } = await mount(fake, { kind });
      expect(host.textContent).toContain(text);
      await act(async () => add(host).click());
      expect(host.textContent).not.toContain(text);
    },
  );
});

describe("PackPile, naming", () => {
  test("PackPile lowers capitals as typed", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => add(host).click());
    const input = await typeName(host, "Carte");
    expect([input.value, input.getAttribute("aria-invalid"), issue(host)]).toEqual(["carte", "false", null]);
  });

  test("PackPile slugs accents and spaces, then Enter creates and selects", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host, selections } = await mount(fake);
    await act(async () => add(host).click());
    await submitName(host, "Météo du jour");
    expect(log).toEqual([`set meteo-du-jour:${skeleton("meteo-du-jour", "widgets")}`]);
    expect(selections).toEqual(["meteo-du-jour"]);
  });

  test("PackPile a taken name is invalid and says so", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => add(host).click());
    const input = await typeName(host, "Kpi");
    expect([input.getAttribute("aria-invalid"), issue(host)]).toEqual(["true", "ui-widgets-name-taken"]);
  });

  test("PackPile a name not starting with a letter says so", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => add(host).click());
    await typeName(host, "2d");
    expect(issue(host)).toBe("ui-widgets-name-letter");
  });

  test("PackPile a refused creation selects nothing", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "setWidget", async (name: string): Promise<WidgetChange> => ({
      status: { name, schema: null, diagnostics: [{ file: name, line: 1, column: 1, error: true, message: "rendu cassé" }] },
      broken: [],
      saved: false,
    }));
    const { host, selections, errors } = await mount(fake);
    await act(async () => add(host).click());
    await submitName(host, "abc");
    expect([selections, errors]).toEqual([[], ["rendu cassé"]]);
  });
});

describe("PackPile, wording", () => {
  test.each([["widgets", "ui-widgets-name", "ui-widgets-confirm-delete"], ["layouts", "ui-layouts-name", "ui-layouts-confirm-delete"]] as [PackKind, string, string][])(
    "PackPile of a %s pack names its field %s and asks %s",
    async (kind, field, question) => {
      const { fake } = memory({ kpi: "k" });
      const { host, asked } = await mount(fake, { kind });
      await act(async () => trashOf(host, "kpi").click());
      await wait(10);
      await act(async () => add(host).click());
      expect([nameInput(host)?.getAttribute("aria-label"), asked]).toEqual([field, [question]]);
    },
  );
});

describe("PackPile, renaming", () => {
  test("PackPile F2 on a thumbnail opens the rename field", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await press(thumb(host, "kpi"), "F2");
    expect(nameInput(host)?.value).toBe("kpi");
  });

  test("PackPile double-click then Enter renames the item", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await dblclick(thumb(host, "kpi"));
    await submitName(host, "kpi2");
    expect(log).toEqual(["rename kpi>kpi2"]);
    expect(document.activeElement).toBe(thumb(host, "kpi2"));
  });

  test("PackPile Escape closes the rename field and refocuses the thumbnail", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await press(thumb(host, "kpi"), "F2");
    await press(nameInput(host)!, "Escape");
    expect([nameInput(host), document.activeElement === thumb(host, "kpi")]).toEqual([null, true]);
  });

  test("PackPile the shared row cannot be renamed", async () => {
    const { fake } = memory({ _commun: "c", kpi: "k" });
    const { host } = await mount(fake);
    await press(common(host)!, "F2");
    await dblclick(common(host)!);
    expect(nameInput(host)).toBeNull();
  });

  test("PackPile a refused rename keeps the old name and its focus", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "renameWidget", () => Promise.reject(new Error("refusé")));
    const { host } = await mount(fake);
    await dblclick(thumb(host, "kpi"));
    await submitName(host, "kpi2");
    expect([thumbs(host).map((b) => b.textContent?.includes("kpi")), document.activeElement === thumb(host, "kpi")]).toEqual([[true], true]);
  });
});

describe("PackPile, deleting", () => {
  const ways: [string, (host: HTMLElement) => Promise<unknown>][] = [
    ["the trash button", (host) => act(async () => trashOf(host, "kpi").click())],
    ["Delete", (host) => press(thumb(host, "kpi"), "Delete")],
    ["Backspace", (host) => press(thumb(host, "kpi"), "Backspace")],
  ];

  test.each(ways)("PackPile %s asks through confirm, never window.confirm", async (_, how) => {
    const spy = spyOn(window, "confirm").mockImplementation(() => {
      throw new Error("window.confirm");
    });
    const { fake, log } = memory({ kpi: "k" });
    const { host, asked, removed } = await mount(fake);
    await how(host);
    await wait(10);
    spy.mockRestore();
    expect([asked, log, removed]).toEqual([["ui-widgets-confirm-delete"], ["remove kpi"], ["kpi"]]);
  });

  test("PackPile a refused confirmation deletes nothing", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host, removed } = await mount(fake, { confirm: async () => false });
    await act(async () => trashOf(host, "kpi").click());
    await wait(10);
    expect([log, removed]).toEqual([[], []]);
  });

  test("PackPile a removal refused by the session is said", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "removeWidget", () => Promise.reject(new Error("utilisé par un modèle")));
    const { host, removed, errors } = await mount(fake);
    await act(async () => trashOf(host, "kpi").click());
    await wait(10);
    expect([errors.length, errors[0]?.includes("utilisé par un modèle"), removed]).toEqual([1, true, []]);
  });

  test("PackPile after a delete the focus goes to the neighbour thumbnail", async () => {
    const { fake } = memory({ a: "a", b: "b", c: "c" });
    const { host } = await mount(fake);
    await press(thumb(host, "b"), "Delete");
    await wait(10);
    expect(document.activeElement).toBe(thumb(host, "c"));
  });

  test("PackPile after deleting the last item the focus goes to the shared row", async () => {
    const { fake } = memory({ _commun: "c", kpi: "k" });
    const { host } = await mount(fake);
    await press(thumb(host, "kpi"), "Delete");
    await wait(10);
    expect(document.activeElement).toBe(common(host));
  });

  test("PackPile after deleting the only item the focus goes to the add button", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await press(thumb(host, "kpi"), "Delete");
    await wait(10);
    expect(document.activeElement).toBe(add(host));
  });
});

describe("PackPile, context menu", () => {
  /** Right-clicks `target` and answers the menu's entry labels. */
  const openMenu = async (target: HTMLElement) => {
    await act(async () => void target.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 30, clientY: 40 })));
    return [...document.querySelectorAll<HTMLButtonElement>('[role="menu"] [role="menuitem"]')];
  };

  test("PackPile a thumbnail's context menu offers rename and delete", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    expect((await openMenu(thumb(host, "kpi"))).map((b) => b.textContent)).toEqual(["ui-pile-menu-rename", "ui-pile-menu-delete"]);
  });

  test("PackPile the shared row's context menu offers only delete", async () => {
    const { fake } = memory({ _commun: "c", kpi: "k" });
    const { host } = await mount(fake);
    expect((await openMenu(common(host) as HTMLButtonElement)).map((b) => b.textContent)).toEqual(["ui-pile-menu-delete"]);
  });

  test("PackPile the menu's rename starts naming the item", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    const [rename] = await openMenu(thumb(host, "kpi"));
    await act(async () => rename.click());
    expect([nameInput(host)?.value, document.querySelector('[role="menu"]')]).toEqual(["kpi", null]);
  });

  test("PackPile the menu's delete asks through confirm, then deletes", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host, asked } = await mount(fake);
    const [, remove] = await openMenu(thumb(host, "kpi"));
    await act(async () => remove.click());
    await wait(10);
    expect([asked, log]).toEqual([["ui-widgets-confirm-delete"], ["remove kpi"]]);
  });

  test("PackPile the menu is named after its item", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await openMenu(thumb(host, "kpi"));
    expect(document.querySelector('[role="menu"]')?.getAttribute("aria-label")).toBe("ui-pile-menu");
  });
});
