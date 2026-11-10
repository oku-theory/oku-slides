// The deck tab's left side panel in happy-dom: its rail, its Packs, Themes and Inspector tabs, folding.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

// See ./a2ui/catalog.render.test.tsx: React loads after the DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { createSlot } = await import("./agent/slot");
const { SidePanel } = await import("./SidePanel");
const { t } = await import("./i18n");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Mounts the panel; `packs` and `themes` fill their slots, `inspector` gives the inspector's body. */
async function mount({ packs = false, themes = false, inspector = false, updates = false } = {}) {
  const calls: string[] = [];
  const slot = createSlot();
  if (packs) slot.set(createElement("div", { className: "packs-probe" }));
  const themesSlot = createSlot();
  if (themes) themesSlot.set(createElement("div", { className: "themes-probe" }));
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(SidePanel, {
        packs: slot,
        themes: themesSlot,
        inspector: inspector ? createElement("div", { className: "inspector-probe" }) : null,
        updates,
        onPacks: () => void calls.push("packs"),
        onThemes: () => void calls.push("themes"),
        onInspector: () => void calls.push("inspector"),
        onFold: () => void calls.push("fold"),
        onCloseBox: () => void calls.push("close-box"),
      }),
    ),
  );
  const toggle = (name: "packs" | "themes" | "inspector") => host.querySelector(`.side-rail .${name}-toggle`) as HTMLButtonElement;
  return { host, calls, slot, themesSlot, toggle };
}

test("SidePanel folded shows the rail without a body", async () => {
  const { host } = await mount();
  expect([!!host.querySelector(".side-rail"), !!host.querySelector(".side-body"), host.querySelector(".side-panel")?.classList.contains("open")]).toEqual([true, false, false]);
});

test("SidePanel folded packs toggle opens the packs", async () => {
  const { calls, toggle } = await mount();
  await act(async () => toggle("packs").click());
  expect(calls).toEqual(["packs"]);
});

test("SidePanel folded inspector toggle opens the inspector", async () => {
  const { calls, toggle } = await mount();
  await act(async () => toggle("inspector").click());
  expect(calls).toEqual(["inspector"]);
});

test("SidePanel with the packs slot filled shows the packs tab pressed", async () => {
  const { host, toggle } = await mount({ packs: true, inspector: true });
  expect([!!host.querySelector(".side-body .packs-probe"), !!host.querySelector(".inspector-probe"), toggle("packs").getAttribute("aria-pressed")]).toEqual([true, false, "true"]);
});

test("SidePanel with only the inspector shows the inspector tab pressed", async () => {
  const { host, toggle } = await mount({ inspector: true });
  expect([!!host.querySelector(".side-body .inspector-probe"), toggle("inspector").getAttribute("aria-pressed")]).toEqual([true, "true"]);
});

test("SidePanel pressed tab click folds the panel", async () => {
  const { calls, toggle } = await mount({ inspector: true });
  await act(async () => toggle("inspector").click());
  expect(calls).toEqual(["fold"]);
});

test("SidePanel pressed packs tab over a kept inspector closes the packs only", async () => {
  const { calls, toggle } = await mount({ packs: true, inspector: true });
  await act(async () => toggle("packs").click());
  expect(calls).toEqual(["close-box"]);
});

test("SidePanel pressed packs tab without a kept inspector folds the panel", async () => {
  const { calls, toggle } = await mount({ packs: true });
  await act(async () => toggle("packs").click());
  expect(calls).toEqual(["fold"]);
});

test("SidePanel Escape in the packs body closes the packs", async () => {
  const { host, calls } = await mount({ packs: true });
  await act(async () => {
    host.querySelector(".packs-probe")!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  });
  expect(calls).toEqual(["close-box"]);
});

test("SidePanel folding with the focus inside gives it to the rail", async () => {
  const { host, slot } = await mount({ packs: true });
  const inner = document.createElement("button");
  host.querySelector(".packs-probe")!.append(inner);
  inner.focus();
  await act(async () => slot.set(null));
  expect(document.activeElement?.classList.contains("packs-toggle")).toBe(true);
});

test("SidePanel inspector tab while packs show switches to the inspector", async () => {
  const { calls, toggle } = await mount({ packs: true, inspector: true });
  await act(async () => toggle("inspector").click());
  expect(calls).toEqual(["inspector"]);
});

test("SidePanel packs slot emptied shows the inspector again", async () => {
  const { host, slot } = await mount({ packs: true, inspector: true });
  await act(async () => slot.set(null));
  expect(!!host.querySelector(".side-body .inspector-probe")).toBe(true);
});

test("SidePanel folded themes toggle opens the themes", async () => {
  const { calls, toggle } = await mount();
  await act(async () => toggle("themes").click());
  expect(calls).toEqual(["themes"]);
});

test("SidePanel with the themes slot filled shows the themes tab pressed", async () => {
  const { host, toggle } = await mount({ themes: true, inspector: true });
  expect([!!host.querySelector(".side-body .themes-probe"), !!host.querySelector(".inspector-probe"), toggle("themes").getAttribute("aria-pressed")]).toEqual([true, false, "true"]);
});

test("SidePanel pressed themes tab over a kept inspector closes the box only", async () => {
  const { calls, toggle } = await mount({ themes: true, inspector: true });
  await act(async () => toggle("themes").click());
  expect(calls).toEqual(["close-box"]);
});

test("SidePanel pressed themes tab without a kept inspector folds the panel", async () => {
  const { calls, toggle } = await mount({ themes: true });
  await act(async () => toggle("themes").click());
  expect(calls).toEqual(["fold"]);
});

test("SidePanel Escape in the themes body closes the box", async () => {
  const { host, calls } = await mount({ themes: true });
  await act(async () => {
    host.querySelector(".themes-probe")!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  });
  expect(calls).toEqual(["close-box"]);
});

test("SidePanel themes closing with the focus inside gives it to the themes toggle", async () => {
  const { host, themesSlot } = await mount({ themes: true });
  const inner = document.createElement("button");
  host.querySelector(".themes-probe")!.append(inner);
  inner.focus();
  await act(async () => themesSlot.set(null));
  expect(document.activeElement?.classList.contains("themes-toggle")).toBe(true);
});

test("SidePanel themes toggle is named Themes", async () => {
  const { toggle } = await mount();
  expect(toggle("themes").getAttribute("aria-label")).toBe(t("ui-themes"));
});

test("SidePanel with both boxes filled shows the packs, the themes toggle unpressed", async () => {
  const { host, toggle } = await mount({ packs: true, themes: true });
  expect([!!host.querySelector(".side-body .packs-probe"), !!host.querySelector(".themes-probe"), toggle("themes").getAttribute("aria-pressed")]).toEqual([true, false, "false"]);
});

test("SidePanel with pack updates marks the packs toggle and says so", async () => {
  const { toggle } = await mount({ updates: true });
  expect([toggle("packs").classList.contains("has-update"), toggle("packs").getAttribute("aria-label")]).toEqual([true, t("ui-side-packs-update")]);
});

const foldToggle = (host: HTMLElement) => host.querySelector(".side-rail .fold-toggle") as HTMLButtonElement;

test("SidePanel open fold button folds the panel", async () => {
  const { host, calls } = await mount({ inspector: true });
  await act(async () => foldToggle(host).click());
  expect(calls).toEqual(["fold"]);
});

test("SidePanel open fold button is named Collapse", async () => {
  const { host } = await mount({ inspector: true });
  expect(foldToggle(host).getAttribute("aria-label")).toBe(t("ui-side-fold"));
});

test("SidePanel open fold button says the panel is expanded", async () => {
  const { host } = await mount({ inspector: true });
  expect(foldToggle(host).getAttribute("aria-expanded")).toBe("true");
});

test("SidePanel folded fold button is named Expand", async () => {
  const { host } = await mount();
  expect(foldToggle(host).getAttribute("aria-label")).toBe(t("ui-side-unfold"));
});

test("SidePanel folded fold button never opened opens the packs", async () => {
  const { host, calls } = await mount();
  await act(async () => foldToggle(host).click());
  expect(calls).toEqual(["packs"]);
});

test("SidePanel folded fold button reopens the tab shown last", async () => {
  const { host, calls, themesSlot } = await mount({ themes: true });
  await act(async () => themesSlot.set(null));
  await act(async () => foldToggle(host).click());
  expect(calls).toEqual(["themes"]);
});

test("SidePanel without pack updates names the packs toggle plainly", async () => {
  const { toggle } = await mount();
  expect([toggle("packs").classList.contains("has-update"), toggle("packs").getAttribute("aria-label")]).toEqual([false, t("ui-side-packs")]);
});

const { useSidePanel } = await import("./SidePanel");

/** Mounts `useSidePanel` with a fake agent and kept inspector state; returns its handles. */
async function mountHook({ inspector = false, packs = false, themes = false } = {}) {
  const calls: string[] = [];
  const slot = createSlot();
  if (packs) slot.set(createElement("div"));
  const themesSlot = createSlot();
  if (themes) themesSlot.set(createElement("div"));
  let api: ReturnType<typeof useSidePanel> | null = null;
  let setOpen: (v: boolean) => void = () => {};
  const agent = {
    showPacks: async () => void calls.push("show"),
    showThemes: async (slide: string | null) => void calls.push(`themes:${slide}`),
    surfaceAction: async (id: string, name: string) => void calls.push(`${id}:${name}`),
  };
  const { useState } = await import("react");
  const Probe = () => {
    const [open, set] = useState(inspector);
    setOpen = (v) => {
      calls.push(`inspector:${v}`);
      set(v);
    };
    api = useSidePanel({ agent, packs: slot, themes: themesSlot, inspector: open, setInspector: (v) => setOpen(v), say: (m) => void calls.push(`say:${m}`) });
    return null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Probe)));
  return { calls, slot, themesSlot, api: () => api!, toggleInspector: (v: boolean) => act(async () => setOpen(v)) };
}

test("useSidePanel openPacks folds the inspector and shows the packs", async () => {
  const { calls, api } = await mountHook({ inspector: true });
  await act(async () => api().openPacks());
  expect(calls).toEqual(["inspector:false", "show"]);
});

test("useSidePanel closeBoxes without the packs open sends nothing", async () => {
  const { calls, api } = await mountHook();
  await act(async () => api().closeBoxes());
  expect(calls).toEqual([]);
});

test("useSidePanel closeBoxes with the packs open dismisses them once", async () => {
  const { calls, api } = await mountHook({ packs: true });
  await act(async () => {
    api().closeBoxes();
    api().closeBoxes();
  });
  expect(calls).toEqual(["host-packs:dismiss"]);
});

test("useSidePanel inspector opened over the packs (shortcut) dismisses them", async () => {
  const { calls, toggleInspector } = await mountHook({ packs: true });
  await toggleInspector(true);
  expect(calls).toEqual(["inspector:true", "host-packs:dismiss"]);
});

test("useSidePanel openThemes folds the inspector and shows the picker on the slide", async () => {
  const { calls, api } = await mountHook({ inspector: true });
  await act(async () => api().openThemes("s-2"));
  expect(calls).toEqual(["inspector:false", "themes:s-2"]);
});

test("useSidePanel closeBoxes with the themes open dismisses the picker", async () => {
  const { calls, api } = await mountHook({ themes: true });
  await act(async () => api().closeBoxes());
  expect(calls).toEqual(["host-theme-picker:dismiss"]);
});

test("useSidePanel inspector opened over the themes (shortcut) dismisses them", async () => {
  const { calls, toggleInspector } = await mountHook({ themes: true });
  await toggleInspector(true);
  expect(calls).toEqual(["inspector:true", "host-theme-picker:dismiss"]);
});

test("useSidePanel themes opening over the packs dismisses the packs", async () => {
  const { calls, themesSlot } = await mountHook({ packs: true });
  await act(async () => themesSlot.set(createElement("div")));
  expect(calls).toEqual(["host-packs:dismiss"]);
});

test("useSidePanel packs opening over the themes dismisses the picker", async () => {
  const { calls, slot } = await mountHook({ themes: true });
  await act(async () => slot.set(createElement("div")));
  expect(calls).toEqual(["host-theme-picker:dismiss"]);
});

test("useSidePanel both boxes opening together dismisses the packs only", async () => {
  const { calls, slot, themesSlot } = await mountHook();
  await act(async () => {
    slot.set(createElement("div"));
    themesSlot.set(createElement("div"));
  });
  expect(calls).toEqual(["host-packs:dismiss"]);
});
