// Portée des raccourcis d'App : pendant l'édition d'un texte, seuls ⌘' et ⌘⇧G passent (t-364).

import { describe, expect, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: TextEditor } = await import("./TextEditor");
const { appKeyScope } = await import("./appKeyScope");

const FREE = { modal: false, dialog: false, editing: false };

/** Monte un éditeur de texte, envoie `init` à son contenu et rend l'événement tel que la fenêtre l'a vu. */
async function keyInEditor(init: KeyboardEventInit): Promise<KeyboardEvent | null> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const nop = () => {};
  const props = { initial: "a", plain: false, label: "Texte", style: {}, palette: [], onChange: nop, onCommit: nop, onCancel: nop, onNext: nop };
  let seen: KeyboardEvent | null = null;
  const onKey = (e: KeyboardEvent) => void (seen = e);
  window.addEventListener("keydown", onKey);
  try {
    await act(async () => root.render(createElement(TextEditor, props)));
    const content = host.querySelector(".cm-content") as HTMLElement;
    await act(async () => void content.dispatchEvent(new KeyboardEvent("keydown", { ...init, bubbles: true, cancelable: true })));
    return seen;
  } finally {
    window.removeEventListener("keydown", onKey);
    await act(async () => root.unmount());
    host.remove();
  }
}

describe("appKeyScope", () => {
  test("appKeyScope cmd quote in the text editor reaches App as a display shortcut", async () => {
    const e = await keyInEditor({ key: "'", code: "Quote", metaKey: true });
    expect([e?.defaultPrevented, e && appKeyScope({ ...FREE, editing: true }, e)]).toEqual([false, "display"]);
  });

  test("appKeyScope cmd shift G in the text editor reaches App as a display shortcut", async () => {
    const e = await keyInEditor({ key: "G", code: "KeyG", metaKey: true, shiftKey: true });
    expect([e?.defaultPrevented, e && appKeyScope({ ...FREE, editing: true }, e)]).toEqual([false, "display"]);
  });

  test("appKeyScope cmd z in the text editor never reaches the window", async () => {
    expect(await keyInEditor({ key: "z", code: "KeyZ", metaKey: true })).toBeNull();
  });

  test("appKeyScope cmd z while editing is not App's", () => {
    const e = { key: "z", metaKey: true, ctrlKey: false, shiftKey: false, target: document.body };
    expect(appKeyScope({ ...FREE, editing: true }, e)).toBe("none");
  });

  test("appKeyScope cmd quote in a plain input is a display shortcut", () => {
    const input = document.createElement("input");
    const e = { key: "'", metaKey: true, ctrlKey: false, shiftKey: false, target: input };
    expect(appKeyScope(FREE, e)).toBe("display");
  });

  test.each([
    ["modal", { ...FREE, modal: true }],
    ["dialog", { ...FREE, dialog: true }],
  ])("appKeyScope cmd quote with a %s open is ignored", (_name, state) => {
    expect(appKeyScope(state, { key: "'", metaKey: true, ctrlKey: false, shiftKey: false, target: document.body })).toBe("none");
  });

  test("appKeyScope any key outside text entry is App's", () => {
    expect(appKeyScope(FREE, { key: "z", metaKey: true, ctrlKey: false, shiftKey: false, target: document.body })).toBe("all");
  });
});
