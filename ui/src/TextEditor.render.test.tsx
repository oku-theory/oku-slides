// Éditeur de texte de la slide : la palette des pastilles suit le thème.

import { describe, expect, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const { default: TextEditor } = await import("./TextEditor");

describe("TextEditor", () => {
  test("palette changée pendant l'édition : la pastille prend la nouvelle couleur, la saisie reste", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const nop = () => {};
    const props = { initial: "==a==", plain: false, label: "Texte", style: {}, onChange: nop, onCommit: nop, onCancel: nop, onNext: nop };
    const highlight = () => (host.querySelector(".cm-md-highlight") as HTMLElement | null)?.getAttribute("style") ?? "";
    try {
      await act(async () => root.render(createElement(TextEditor, { ...props, palette: ["#ff0000"] })));
      expect(highlight()).toContain("#ff0000");
      const view = EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement)!;
      await act(async () => view.dispatch({ changes: { from: view.state.doc.length, insert: " b" } }));
      await act(async () => root.render(createElement(TextEditor, { ...props, palette: ["#00ff00"] })));
      expect(highlight()).toContain("#00ff00");
      expect(EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement)!.state.doc.toString()).toBe("==a== b");
    } finally {
      await act(async () => root.unmount());
      host.remove();
    }
  });

  /** Monte un éditeur de titre ; rend le compte des validations et de quoi émettre un `blur`. */
  async function mountEditor() {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const counts = { commits: 0 };
    const nop = () => {};
    const props = { initial: "a", plain: true, label: "Titre", style: {}, palette: [], onChange: nop, onCommit: () => void counts.commits++, onCancel: nop, onNext: nop };
    await act(async () => root.render(createElement(TextEditor, props)));
    const blur = () => act(async () => host.querySelector(".cm-content")!.dispatchEvent(new FocusEvent("blur")));
    const unmount = async () => {
      await act(async () => root.unmount());
      host.remove();
    };
    return { counts, blur, unmount };
  }

  test("TextEditor blur quand la fenêtre perd le focus : l'édition continue (t-654)", async () => {
    const e = await mountEditor();
    const hasFocus = document.hasFocus;
    try {
      document.hasFocus = () => false;
      await e.blur();
      expect(e.counts.commits).toBe(0);
    } finally {
      document.hasFocus = hasFocus;
      await e.unmount();
    }
  });

  test("TextEditor blur dans la fenêtre : l'édition se termine (t-654)", async () => {
    const e = await mountEditor();
    const hasFocus = document.hasFocus;
    try {
      document.hasFocus = () => true;
      await e.blur();
      expect(e.counts.commits).toBe(1);
    } finally {
      document.hasFocus = hasFocus;
      await e.unmount();
    }
  });
});
