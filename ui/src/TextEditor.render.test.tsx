// Éditeur de texte de la slide : la palette des pastilles suit le thème.

import { describe, expect, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const { undo } = await import("@codemirror/commands");
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
  /** Monte un éditeur `rich` sur `initial` ; rend sa vue, le texte affiché et de quoi démonter. */
  async function mountRich(initial: string) {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const nop = () => {};
    const props = { initial, plain: false, label: "Texte", style: {}, palette: ["#2f5d8a"], onChange: nop, onCommit: nop, onCancel: nop, onNext: nop };
    await act(async () => root.render(createElement(TextEditor, props)));
    const view = EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement)!;
    // Sans focus, CodeMirror n'écrit plus la sélection DOM : happy-dom ne relance pas selectionchange en pleine mise à jour.
    view.contentDOM.blur();
    const shown = () => [...host.querySelectorAll(".cm-line")].map((l) => l.textContent).join("\n");
    const select = (anchor: number, head = anchor) => act(async () => view.dispatch({ selection: { anchor, head } }));
    const unmount = async () => {
      await act(async () => root.unmount());
      host.remove();
    };
    return { host, view, shown, select, unmount };
  }

  test("TextEditor curseur hors des constructions : marques cachées, gras et lien mis en forme (t-272)", async () => {
    const e = await mountRich("**a** [l](https://x.y) ==h=={1} fin");
    try {
      expect(e.shown()).toBe("a l h fin");
      expect(e.host.querySelector(".cm-md-bold")?.textContent).toBe("a");
      expect(e.host.querySelector(".cm-md-link")?.textContent).toBe("l");
      expect(e.host.querySelector(".cm-md-highlight")?.getAttribute("style")).toContain("#2f5d8a");
    } finally {
      await e.unmount();
    }
  });

  test("TextEditor curseur dans le gras : ses marques se montrent, les autres restent cachées (t-272)", async () => {
    const e = await mountRich("**a** *b*");
    try {
      await e.select(3);
      expect(e.shown()).toBe("**a** b");
      await e.select(9);
      expect(e.shown()).toBe("a *b*");
    } finally {
      await e.unmount();
    }
  });

  test("TextEditor listes hors du curseur : puce et rang de l'item à la place du préfixe (t-272)", async () => {
    const e = await mountRich("- un\n1. a\n1. b\nfin");
    try {
      expect(e.shown()).toBe("• un\n1. a\n2. b\nfin");
      await e.select(1);
      expect(e.shown().split("\n")[0]).toBe("- un");
    } finally {
      await e.unmount();
    }
  });

  test("TextEditor quasi-rendu : le document reste le markdown source, l'annulation ignore les déplacements (t-272)", async () => {
    const e = await mountRich("**a** b");
    try {
      await act(async () => e.view.dispatch({ changes: { from: 7, insert: "c" }, selection: { anchor: 8 }, userEvent: "input.type" }));
      await e.select(3);
      await e.select(0);
      expect(e.view.state.doc.toString()).toBe("**a** bc");
      await act(async () => void undo(e.view));
      expect(e.view.state.doc.toString()).toBe("**a** b");
    } finally {
      await e.unmount();
    }
  });

  test("TextEditor copie d'une sélection aux marques cachées : le presse-papiers reçoit le markdown source (t-272)", async () => {
    const e = await mountRich("x **a** y");
    try {
      await e.select(0, 9);
      const data = new DataTransfer();
      const copy = new ClipboardEvent("copy", { clipboardData: data, bubbles: true, cancelable: true });
      await act(async () => void e.view.contentDOM.dispatchEvent(copy));
      expect(data.getData("text/plain")).toBe("x **a** y");
    } finally {
      await e.unmount();
    }
  });
});
