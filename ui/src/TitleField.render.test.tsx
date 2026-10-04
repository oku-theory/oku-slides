// Rendu du titre modifiable sur place dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const TitleField = (await import("./TitleField")).default;

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(committed: string[]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const props = { value: "Feuille de route {{annee}}", display: "Feuille de route 2027", onCommit: (v: string) => committed.push(v) };
  await act(async () => root.render(createElement(TitleField, props)));
  return host;
}

async function edit(host: HTMLElement): Promise<HTMLInputElement> {
  await act(async () => host.querySelector<HTMLButtonElement>("button")!.click());
  return host.querySelector<HTMLInputElement>("input")!;
}

async function type(input: HTMLInputElement, text: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, text);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function key(input: HTMLInputElement, k: string) {
  await act(async () => {
    input.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
  });
}

describe("TitleField", () => {
  test("TitleField at rest shows the display title", async () => {
    const host = await mount([]);
    expect(host.querySelector("button")?.textContent).toBe("Feuille de route 2027");
  });

  test("TitleField click edits the source value", async () => {
    const input = await edit(await mount([]));
    expect(input.value).toBe("Feuille de route {{annee}}");
  });

  test("TitleField Enter commits a changed title", async () => {
    const committed: string[] = [];
    const host = await mount(committed);
    const input = await edit(host);
    await type(input, "Bilan");
    await key(input, "Enter");
    expect(committed).toEqual(["Bilan"]);
    expect(host.querySelector("input")).toBeNull();
  });

  test("TitleField Enter on an unchanged title commits nothing", async () => {
    const committed: string[] = [];
    const input = await edit(await mount(committed));
    await key(input, "Enter");
    expect(committed).toEqual([]);
  });

  test("TitleField Escape cancels the edit", async () => {
    const committed: string[] = [];
    const host = await mount(committed);
    const input = await edit(host);
    await type(input, "Bilan");
    await key(input, "Escape");
    expect(committed).toEqual([]);
    expect(host.querySelector("button")?.textContent).toBe("Feuille de route 2027");
  });

  test("TitleField keys do not reach the app shortcuts", async () => {
    const host = await mount([]);
    const input = await edit(host);
    let reached = false;
    const listener = () => (reached = true);
    window.addEventListener("keydown", listener);
    await key(input, "Delete");
    window.removeEventListener("keydown", listener);
    expect(reached).toBe(false);
  });
});
