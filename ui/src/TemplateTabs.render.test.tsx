// Rendu de la barre des sous-onglets du mode modèles dans un DOM happy-dom.

import { afterEach, describe, expect, mock, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const TemplateTabs = (await import("./TemplateTabs")).default;
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
    tabs: TABS,
    active: null as number | "theme" | null,
    unsaved: new Set<string>(),
    onActivate: mock((_id: number | "theme" | null) => {}),
    onClose: mock((_id: number) => {}),
    ...over,
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(TemplateTabs, props as never));
  });
  return { host, props };
}

const tabs = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>('[role="tab"]')];
const closeBtn = (host: HTMLElement, i: number) =>
  host.querySelectorAll<HTMLElement>(".template-tab-slot button[aria-hidden]")[i];
const press = (el: HTMLElement, key: string) =>
  act(async () => {
    el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  });

describe("TemplateTabs", () => {
  test("Modèles en premier, puis Thème, puis les widgets, Commun libellé « Commun »", async () => {
    const { host } = await mount();
    const all = tabs(host);
    expect(all).toHaveLength(4);
    expect(all[0].id).toBe("widget-tab-0");
    expect(all[1].id).toBe("widget-tab-theme");
    expect(all[2].textContent).toContain(widgetLabel("_commun"));
    expect(all[3].textContent).toContain("titre");
  });

  test("clic sur Thème : onActivate(\"theme\") ; Thème n'a pas de ×", async () => {
    const { host, props } = await mount();
    await act(async () => tabs(host)[1].click());
    expect(props.onActivate).toHaveBeenLastCalledWith("theme");
    expect(tabs(host)[1].closest(".template-tab-slot")).toBeNull();
  });

  test("TemplateTabs ArrowRight from Templates activates Theme tab", async () => {
    const { host, props } = await mount();
    await press(tabs(host)[0], "ArrowRight");
    expect(props.onActivate).toHaveBeenLastCalledWith("theme");
    expect(document.activeElement).toBe(tabs(host)[1]);
    await press(tabs(host)[1], "ArrowRight");
    expect(props.onActivate).toHaveBeenLastCalledWith(1);
    await press(tabs(host)[2], "ArrowLeft");
    expect(props.onActivate).toHaveBeenLastCalledWith("theme");
  });

  test("Thème actif : aria-selected, tabIndex 0 ; Suppr sur Thème ne ferme rien", async () => {
    const { host, props } = await mount({ active: "theme" });
    const all = tabs(host);
    expect(all.map((el) => el.getAttribute("aria-selected"))).toEqual(["false", "true", "false", "false"]);
    expect(all[1].tabIndex).toBe(0);
    await press(all[1], "Delete");
    expect(props.onClose).not.toHaveBeenCalled();
  });

  test("clic : onActivate(id) ; clic sur Modèles : onActivate(null)", async () => {
    const { host, props } = await mount({ active: 1 });
    await act(async () => tabs(host)[3].click());
    expect(props.onActivate).toHaveBeenLastCalledWith(2);
    await act(async () => tabs(host)[0].click());
    expect(props.onActivate).toHaveBeenLastCalledWith(null);
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
      tabs(host)[2].dispatchEvent(new MouseEvent("auxclick", { button: 1, bubbles: true, cancelable: true }));
    });
    expect(props.onClose).toHaveBeenCalledWith(1);
  });

  test("aria-selected et tabIndex 0 sur l'actif seulement ; Modèles n'a pas de ×", async () => {
    const { host } = await mount({ active: 2 });
    const all = tabs(host);
    expect(all.map((el) => el.getAttribute("aria-selected"))).toEqual(["false", "false", "false", "true"]);
    expect(all.map((el) => el.tabIndex)).toEqual([-1, -1, -1, 0]);
    expect(host.querySelectorAll(".template-tab-slot button[aria-hidden]")).toHaveLength(2);
    expect(all[0].closest(".template-tab-slot")).toBeNull();
  });

  test("← depuis Thème revient à Modèles ; ← depuis Modèles va au dernier widget", async () => {
    const { host, props } = await mount();
    await press(tabs(host)[1], "ArrowLeft");
    expect(props.onActivate).toHaveBeenLastCalledWith(null);
    expect(document.activeElement).toBe(tabs(host)[0]);
    await press(tabs(host)[0], "ArrowLeft");
    expect(props.onActivate).toHaveBeenLastCalledWith(2);
  });

  test("Suppr sur un onglet widget : onClose(id) ; sur Modèles : rien", async () => {
    const { host, props } = await mount();
    await press(tabs(host)[0], "Delete");
    expect(props.onClose).not.toHaveBeenCalled();
    await press(tabs(host)[3], "Delete");
    expect(props.onClose).toHaveBeenCalledWith(2);
  });

  test("touches prises par la barre (flèches, Début, Fin, Suppr, même sur Modèles) : jamais vues de la fenêtre", async () => {
    const { host } = await mount();
    const seen: string[] = [];
    const l = (e: KeyboardEvent) => seen.push(e.key);
    window.addEventListener("keydown", l);
    try {
      for (const key of ["Delete", "ArrowRight", "ArrowLeft", "End", "Home"]) await press(tabs(host)[0], key);
      await press(tabs(host)[3], "Delete");
    } finally {
      window.removeEventListener("keydown", l);
    }
    expect(seen).toEqual([]);
  });

  test("⌫ (Backspace) sur un onglet widget : onClose(id), comme Suppr ; sur Modèles : rien", async () => {
    const { host, props } = await mount();
    await press(tabs(host)[0], "Backspace");
    expect(props.onClose).not.toHaveBeenCalled();
    await press(tabs(host)[3], "Backspace");
    expect(props.onClose).toHaveBeenCalledWith(2);
  });

  test("⌫ (Backspace) sur Modèles : pris par la barre, jamais vu de la fenêtre", async () => {
    const { host } = await mount();
    const seen: string[] = [];
    const l = (e: KeyboardEvent) => seen.push(e.key);
    window.addEventListener("keydown", l);
    try {
      await press(tabs(host)[0], "Backspace");
    } finally {
      window.removeEventListener("keydown", l);
    }
    expect(seen).toEqual([]);
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
    expect(all[0].getAttribute("aria-controls")).toBe("templates-panel");
    expect(all[1].getAttribute("aria-controls")).toBe("theme-panel");
    expect(all[2].getAttribute("aria-controls")).toBe("widget-panel-1");
    expect(all[3].getAttribute("aria-controls")).toBe("widget-panel-2");
  });

  test("marque non enregistré sur le libellé (.unsaved-dot)", async () => {
    const { host } = await mount({ unsaved: new Set(["titre"]) });
    const all = tabs(host);
    expect(all[3].querySelector(".unsaved-dot")).not.toBeNull();
    expect(all[2].querySelector(".unsaved-dot")).toBeNull();
  });
});
