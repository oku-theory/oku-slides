// Rendu de l'éditeur Typst dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { WidgetDiagnostic } from "./api";
import type { EditorCommands, EditorHistory } from "./TypstEditor";

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

type Extra = Partial<Parameters<typeof TypstEditor>[0]>;

async function mountWith(extra: Extra, text = "a") {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const render = (more: Extra = {}) =>
    act(async () =>
      root.render(
        createElement(TypstEditor, { text, diagnostics: [], label: "kpi", onChange: () => {}, ...extra, ...more }),
      ),
    );
  await render();
  const view = () => EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement)!;
  const typeText = (insert: string) =>
    act(async () =>
      view().dispatch({
        changes: { from: 0, to: view().state.doc.length, insert },
        userEvent: "input.type",
      }),
    );
  return { host, render, view, typeText };
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

/** Règles CSS injectées par CodeMirror (balises <style> ou feuilles adoptées). */
function injectedRules(): string {
  const sheets = [...document.querySelectorAll("style")].map((s) => s.textContent ?? "");
  const adopted = [...(document.adoptedStyleSheets ?? [])].flatMap((sh) => [...sh.cssRules].map((r) => r.cssText));
  return [...sheets, ...adopted].join("\n");
}

describe("TypstEditor, mise en page", () => {
  test("les lignes longues reviennent à la ligne au lieu de défiler", async () => {
    const { host } = await mount();
    expect(host.querySelector(".cm-content")!.classList.contains("cm-lineWrapping")).toBe(true);
  });
});

describe("TypstEditor, thème", () => {
  test("le curseur prend la couleur d'accent, pas le noir du thème de base", async () => {
    const { host } = await mount();
    const editor = host.querySelector(".cm-editor")!;
    const rules = injectedRules();
    const themed = [...editor.classList].filter((c) => c.startsWith("ͼ")).some((c) =>
      new RegExp(`\\.${c}\\.cm-editor \\.cm-content\\s*\\{[^}]*caret-color:\\s*var\\(--accent\\)`).test(rules),
    );
    expect(themed).toBe(true);
  });
});

describe("TypstEditor, commandes", () => {
  test("au montage : commandes posées et historique vide signalé", async () => {
    let commands: EditorCommands | null = null;
    const histories: EditorHistory[] = [];
    await mountWith({ onCommands: (c) => (commands = c), onHistory: (h) => histories.push(h) });
    expect(commands).not.toBeNull();
    expect(histories).toEqual([{ undo: 0, redo: 0 }]);
  });

  test("undo annule la frappe, onChange suit ; redo la refait", async () => {
    let commands: EditorCommands | null = null;
    const seen: string[] = [];
    const { typeText } = await mountWith({ onCommands: (c) => (commands = c), onChange: (t) => seen.push(t) });
    await typeText("ab");
    await act(async () => commands!.undo());
    expect(seen.at(-1)).toBe("a");
    await act(async () => commands!.redo());
    expect(seen.at(-1)).toBe("ab");
  });

  test("l'historique n'est signalé que s'il change (pas pour un déplacement du curseur)", async () => {
    const histories: EditorHistory[] = [];
    const { typeText, view } = await mountWith({ onHistory: (h) => histories.push(h) });
    await typeText("ab");
    const n = histories.length;
    await act(async () => view().dispatch({ selection: { anchor: 1 } }));
    expect(histories.length).toBe(n);
    expect(histories.at(-1)).toEqual({ undo: 1, redo: 0 });
  });

  test("démontage : commandes retirées", async () => {
    let commands: EditorCommands | null = null;
    await mountWith({ onCommands: (c) => (commands = c) });
    await act(async () => mounted!.root.unmount());
    mounted = null;
    expect(commands).toBeNull();
  });

  test("un remplacement (renommage) n'entre pas dans l'historique", async () => {
    const histories: EditorHistory[] = [];
    const { render } = await mountWith({ onHistory: (h) => histories.push(h) }, 'widget("kpi"');
    await render({ rewrite: { find: 'widget("kpi"', replace: 'widget("kpi2"' } });
    expect(histories.at(-1)).toEqual({ undo: 0, redo: 0 });
  });
});
