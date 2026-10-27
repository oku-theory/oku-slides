// Rendu de la barre des sous-onglets du mode pack dans un DOM happy-dom.

import { afterEach, describe, expect, mock, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const PackTabs = (await import("./PackTabs")).default;
const { widgetLabel } = await import("./widgetList");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const TABS = [
  { id: 1, name: "_commun" },
  { id: 2, name: "titre" },
];

async function mount(over: Record<string, unknown> = {}) {
  const props = {
    kind: "widgets",
    tabs: TABS,
    active: null as number | null,
    unsaved: new Set<string>(),
    onActivate: mock((_id: number | null) => {}),
    onClose: mock((_id: number) => {}),
    ...over,
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(PackTabs, props as never));
  });
  return { host, props };
}

const tabs = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>('[role="tab"]')];
const closeBtn = (host: HTMLElement, i: number) =>
  host.querySelectorAll<HTMLElement>(".pack-tab-slot button[aria-hidden]")[i];
const press = (el: HTMLElement, key: string) =>
  act(async () => {
    el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  });

describe("PackTabs", () => {
  test("PackTabs has no templates sub-tab", async () => {
    const { host } = await mount({ kind: "layouts" });
    expect(host.textContent).not.toMatch(/Modèles|Templates|ui-widgets-templates-tab/);
  });

  test("PackTabs has no Theme sub-tab", async () => {
    const { host } = await mount({ kind: "layouts" });
    expect([host.querySelector("#widget-tab-theme"), host.textContent?.includes("ui-theme-tab")]).toEqual([null, false]);
  });

  test("PackTabs of a layout pack starts with the Layouts tab, which shows the stage", async () => {
    const { host, props } = await mount({ kind: "layouts", active: 1 });
    const all = tabs(host);
    expect([all.map((el) => el.id), all[0].textContent, all[0].getAttribute("aria-controls")]).toEqual([
      ["widget-tab-0", "widget-tab-1", "widget-tab-2"],
      "ui-pack-layouts-tab",
      "stage-panel",
    ]);
    await act(async () => all[0].click());
    expect(props.onActivate).toHaveBeenLastCalledWith(null);
  });

  test("PackTabs of a layout pack without an active tab selects Layouts", async () => {
    const { host, props } = await mount({ kind: "layouts" });
    const all = tabs(host);
    expect([all[0].getAttribute("aria-selected"), all[0].tabIndex, all[1].tabIndex]).toEqual(["true", 0, -1]);
    await press(all[0], "ArrowRight");
    expect(props.onActivate).toHaveBeenLastCalledWith(1);
    await press(all[1], "ArrowLeft");
    expect(props.onActivate).toHaveBeenLastCalledWith(null);
    await press(all[0], "Delete");
    expect(props.onClose).not.toHaveBeenCalled();
  });

  test("PackTabs of a widget pack has no Layouts tab", async () => {
    const { host } = await mount();
    expect(host.querySelector("#widget-tab-0")).toBeNull();
  });

  test("PackTabs of a widget pack lists its widgets, Commun labelled Commun", async () => {
    const { host } = await mount();
    const all = tabs(host);
    expect(all).toHaveLength(2);
    expect(all[0].textContent).toContain(widgetLabel("_commun"));
    expect(all[1].textContent).toContain("titre");
  });

  test("PackTabs of a widget pack without an active tab makes its first widget tabbable", async () => {
    const { host } = await mount();
    const all = tabs(host);
    expect(all.map((el) => el.getAttribute("aria-selected"))).toEqual(["false", "false"]);
    expect(all.map((el) => el.tabIndex)).toEqual([0, -1]);
  });

  test("PackTabs of a widget pack without widgets has no tab", async () => {
    const { host } = await mount({ tabs: [] });
    expect([tabs(host).length, host.querySelector('[role="tablist"]')]).toEqual([0, null]);
  });

  test("PackTabs ArrowRight from the first widget activates the next, ArrowLeft wraps", async () => {
    const { host, props } = await mount();
    await press(tabs(host)[0], "ArrowRight");
    expect(props.onActivate).toHaveBeenLastCalledWith(2);
    expect(document.activeElement).toBe(tabs(host)[1]);
    await press(tabs(host)[1], "ArrowLeft");
    expect(props.onActivate).toHaveBeenLastCalledWith(1);
    await press(tabs(host)[0], "ArrowLeft");
    expect(props.onActivate).toHaveBeenLastCalledWith(2);
  });

  test("clic : onActivate(id)", async () => {
    const { host, props } = await mount({ active: 1 });
    await act(async () => tabs(host)[1].click());
    expect(props.onActivate).toHaveBeenLastCalledWith(2);
  });

  test("× : onClose(id), sans onActivate", async () => {
    const { host, props } = await mount();
    await act(async () => closeBtn(host, 1).click());
    expect(props.onClose).toHaveBeenCalledWith(2);
    expect(props.onActivate).not.toHaveBeenCalled();
  });

  test("clic du milieu (auxclick, button 1) : onClose(id)", async () => {
    const { host, props } = await mount();
    await act(async () => {
      tabs(host)[0].dispatchEvent(new MouseEvent("auxclick", { button: 1, bubbles: true, cancelable: true }));
    });
    expect(props.onClose).toHaveBeenCalledWith(1);
  });

  test("PackTabs marks the active tab only, each widget with its ×", async () => {
    const { host } = await mount({ active: 2 });
    const all = tabs(host);
    expect(all.map((el) => el.getAttribute("aria-selected"))).toEqual(["false", "true"]);
    expect(all.map((el) => el.tabIndex)).toEqual([-1, 0]);
    expect(host.querySelectorAll(".pack-tab-slot button[aria-hidden]")).toHaveLength(2);
  });

  test("PackTabs Home and End go to the first and the last widget", async () => {
    const { host, props } = await mount();
    await press(tabs(host)[0], "End");
    expect(props.onActivate).toHaveBeenLastCalledWith(2);
    await press(tabs(host)[1], "Home");
    expect(props.onActivate).toHaveBeenLastCalledWith(1);
  });

  test("Suppr sur un onglet widget : onClose(id)", async () => {
    const { host, props } = await mount();
    await press(tabs(host)[1], "Delete");
    expect(props.onClose).toHaveBeenCalledWith(2);
  });

  test("PackTabs keys it takes (arrows, Home, End, Delete, even on Layouts) never reach the window", async () => {
    const { host } = await mount({ kind: "layouts" });
    const seen: string[] = [];
    const l = (e: KeyboardEvent) => seen.push(e.key);
    window.addEventListener("keydown", l);
    try {
      for (const key of ["Delete", "Backspace", "ArrowRight", "ArrowLeft", "End", "Home"]) await press(tabs(host)[0], key);
      await press(tabs(host)[2], "Delete");
    } finally {
      window.removeEventListener("keydown", l);
    }
    expect(seen).toEqual([]);
  });

  test("PackTabs Backspace on a widget tab closes it like Delete; on Layouts nothing", async () => {
    const { host, props } = await mount({ kind: "layouts" });
    await press(tabs(host)[0], "Backspace");
    expect(props.onClose).not.toHaveBeenCalled();
    await press(tabs(host)[2], "Backspace");
    expect(props.onClose).toHaveBeenCalledWith(2);
  });

  test("une autre touche se propage", async () => {
    const { host } = await mount();
    const seen: string[] = [];
    const l = (e: KeyboardEvent) => seen.push(e.key);
    window.addEventListener("keydown", l);
    try {
      await press(tabs(host)[0], "x");
    } finally {
      window.removeEventListener("keydown", l);
    }
    expect(seen).toEqual(["x"]);
  });

  test("aria-controls de chaque onglet = widget-panel-<id>", async () => {
    const { host } = await mount();
    const all = tabs(host);
    expect(all[0].getAttribute("aria-controls")).toBe("widget-panel-1");
    expect(all[1].getAttribute("aria-controls")).toBe("widget-panel-2");
  });

  test("marque non enregistré sur le libellé (.unsaved-dot)", async () => {
    const { host } = await mount({ unsaved: new Set(["titre"]) });
    const all = tabs(host);
    expect(all[1].querySelector(".unsaved-dot")).not.toBeNull();
    expect(all[0].querySelector(".unsaved-dot")).toBeNull();
  });
});
