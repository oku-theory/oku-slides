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
});
