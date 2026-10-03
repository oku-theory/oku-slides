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

async function mount(text = "#let a = 1") {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const render = (diagnostics: WidgetDiagnostic[], diagnosticsFor?: string) =>
    act(async () =>
      root.render(createElement(TypstEditor, { text, diagnostics, diagnosticsFor, label: "kpi", onChange: () => {} })),
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

  test("un diagnostic en fin de ligne reste souligné", async () => {
    const { host, render } = await mount("#let a = (\n");
    await render([{ ...diag, line: 1, column: 11 }]);
    expect(host.querySelector(".cm-diag-underline")?.textContent).toBe("(");
  });

  test("un diagnostic sur une ligne vide est marqué à sa place", async () => {
    const { host, render } = await mount("#let a = 1\n\n#let b = 2");
    await render([{ ...diag, line: 2, column: 1 }]);
    expect(host.querySelectorAll(".cm-diag-point").length).toBe(1);
    // La marque est dans la ligne 2, vide.
    const lines = [...host.querySelectorAll(".cm-line")];
    expect(lines[1]?.querySelector(".cm-diag-point")).not.toBeNull();
  });

  test("remplacement : appliqué à chaque nouvel objet, pas à celui présent à l'ouverture", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    const seen: string[] = [];
    const render = (rewrite: { find: string; replace: string } | null) =>
      act(async () =>
        root.render(
          createElement(TypstEditor, { text: "#let a = 1", diagnostics: [], label: "kpi", onChange: (t) => seen.push(t), rewrite }),
        ),
      );
    const old = { find: "a", replace: "z" };
    await render(old);
    expect(host.querySelector(".cm-content")?.textContent).toBe("#let a = 1");
    await render({ find: "a = 1", replace: "b = 2" });
    expect(host.querySelector(".cm-content")?.textContent).toBe("#let b = 2");
    expect(seen).toEqual(["#let b = 2"]);
  });

  test("remplacement discret : le texte change sans onChange", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    const seen: string[] = [];
    const render = (rewrite: { find: string; replace: string; silent?: boolean } | null) =>
      act(async () =>
        root.render(
          createElement(TypstEditor, { text: "#let a = 1", diagnostics: [], label: "kpi", onChange: (t) => seen.push(t), rewrite }),
        ),
      );
    await render(null);
    await render({ find: "a", replace: "b", silent: true });
    expect(host.querySelector(".cm-content")?.textContent).toBe("#let b = 1");
    expect(seen).toEqual([]);
  });
});
