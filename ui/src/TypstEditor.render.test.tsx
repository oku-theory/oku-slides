// Rendu de l'éditeur Typst dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { WidgetDiagnostic } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const TypstEditor = (await import("./TypstEditor")).default;

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const diag: WidgetDiagnostic = { file: "kpi", line: 1, column: 1, error: true, message: "boom" };

async function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const render = (diagnostics: WidgetDiagnostic[], diagnosticsFor?: string) =>
    act(async () =>
      root.render(createElement(TypstEditor, { text: "#let a = 1", diagnostics, diagnosticsFor, label: "kpi", onChange: () => {} })),
    );
  await render([]);
  return { host, render };
}

const markers = (host: HTMLElement) => host.querySelectorAll(".cm-diag-marker").length;

describe("TypstEditor", () => {
  test("des diagnostics calculés pour un autre texte que le courant sont ignorés", async () => {
    const { host, render } = await mount();
    const cm = EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement);
    await act(async () => {
      cm?.dispatch({ changes: { from: 0, to: cm.state.doc.length, insert: "#let b = 2" }, userEvent: "input.type" });
    });
    await render([diag], "#let a = 1");
    expect(markers(host)).toBe(0);
    await render([{ ...diag }], "#let b = 2");
    expect(markers(host)).toBe(1);
  });
});
