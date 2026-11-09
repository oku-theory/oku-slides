// The deck tab's left side panel in happy-dom: its rail, its Packs and Inspector tabs, folding.

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

/** Mounts the panel; `packs` fills the packs slot, `inspector` gives the inspector's body. */
async function mount({ packs = false, inspector = false, updates = false } = {}) {
  const calls: string[] = [];
  const slot = createSlot();
  if (packs) slot.set(createElement("div", { className: "packs-probe" }));
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(SidePanel, {
        packs: slot,
        inspector: inspector ? createElement("div", { className: "inspector-probe" }) : null,
        updates,
        onPacks: () => void calls.push("packs"),
        onInspector: () => void calls.push("inspector"),
        onFold: () => void calls.push("fold"),
        onClosePacks: () => void calls.push("close-packs"),
      }),
    ),
  );
  const toggle = (name: "packs" | "inspector") => host.querySelector(`.side-rail .${name}-toggle`) as HTMLButtonElement;
  return { host, calls, slot, toggle };
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
  expect(calls).toEqual(["close-packs"]);
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
  expect(calls).toEqual(["close-packs"]);
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

test("SidePanel with pack updates marks the packs toggle and says so", async () => {
  const { toggle } = await mount({ updates: true });
  expect([toggle("packs").classList.contains("has-update"), toggle("packs").getAttribute("aria-label")]).toEqual([true, t("ui-side-packs-update")]);
});

test("SidePanel without pack updates names the packs toggle plainly", async () => {
  const { toggle } = await mount();
  expect([toggle("packs").classList.contains("has-update"), toggle("packs").getAttribute("aria-label")]).toEqual([false, t("ui-side-packs")]);
});

const { useSidePanel } = await import("./SidePanel");

/** Mounts `useSidePanel` with a fake agent and kept inspector state; returns its handles. */
async function mountHook({ inspector = false, packs = false } = {}) {
  const calls: string[] = [];
  const slot = createSlot();
  if (packs) slot.set(createElement("div"));
  let api: ReturnType<typeof useSidePanel> | null = null;
  let setOpen: (v: boolean) => void = () => {};
  const agent = {
    showPacks: async () => void calls.push("show"),
    surfaceAction: async (id: string, name: string) => void calls.push(`${id}:${name}`),
  };
  const { useState } = await import("react");
  const Probe = () => {
    const [open, set] = useState(inspector);
    setOpen = (v) => {
      calls.push(`inspector:${v}`);
      set(v);
    };
    api = useSidePanel({ agent, packs: slot, inspector: open, setInspector: (v) => setOpen(v), say: (m) => void calls.push(`say:${m}`) });
    return null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Probe)));
  return { calls, slot, api: () => api!, toggleInspector: (v: boolean) => act(async () => setOpen(v)) };
}

test("useSidePanel openPacks folds the inspector and shows the packs", async () => {
  const { calls, api } = await mountHook({ inspector: true });
  await act(async () => api().openPacks());
  expect(calls).toEqual(["inspector:false", "show"]);
});

test("useSidePanel closePacks without the packs open sends nothing", async () => {
  const { calls, api } = await mountHook();
  await act(async () => api().closePacks());
  expect(calls).toEqual([]);
});

test("useSidePanel closePacks with the packs open dismisses them once", async () => {
  const { calls, api } = await mountHook({ packs: true });
  await act(async () => {
    api().closePacks();
    api().closePacks();
  });
  expect(calls).toEqual(["host-packs:dismiss"]);
});

test("useSidePanel inspector opened over the packs (shortcut) dismisses them", async () => {
  const { calls, toggleInspector } = await mountHook({ packs: true });
  await toggleInspector(true);
  expect(calls).toEqual(["inspector:true", "host-packs:dismiss"]);
});
