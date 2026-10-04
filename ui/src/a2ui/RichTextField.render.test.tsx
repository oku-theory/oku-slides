// Champ riche d'une carte : markdown mis en forme, validé en paragraphes.

import { describe, expect, test } from "bun:test";
import "../test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { undo } = await import("@codemirror/commands");
const { EditorView } = await import("@codemirror/view");
const { mountSurface, messagesOf } = await import("../test/surface");
const { RichTextField } = await import("./RichTextField");

const PARAS = [{ runs: [{ t: "Un", color: "#ff0000" }] }, { runs: [{ t: "Deux" }] }];
const rich = () =>
  messagesOf(
    [{ id: "root", component: "RichTextField", label: "Corps", value: { path: "/v" }, source: { path: "/s" }, palette: [],
       action: { event: { name: "param", context: { value: { path: "/s" } } } } }],
    { v: "Un\nDeux", s: PARAS },
  );

async function editor() {
  const m = await mountSurface(rich());
  const content = m.host.querySelector(".cm-content") as HTMLElement;
  const view = EditorView.findFromDOM(content)!;
  const type = (text: string) => act(async () => view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } }));
  const blur = () => act(async () => content.dispatchEvent(new FocusEvent("blur")));
  const key = (k: string, init: KeyboardEventInit = {}) =>
    act(async () => content.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, ...init })));
  return { ...m, content, view, type, blur, key };
}

describe("RichTextField", () => {
  test("montre une ligne par paragraphe", async () => {
    const m = await editor();
    expect(m.host.querySelectorAll(".cm-line").length).toBe(2);
  });

  test("met le gras en forme", async () => {
    const m = await editor();
    await m.type("**Un**");
    expect(m.host.querySelector(".cm-line .cm-md-bold")).not.toBeNull();
  });

  test("à la perte du focus, envoie des paragraphes", async () => {
    const m = await editor();
    await m.type("Un\nTrois");
    await m.blur();
    expect((m.actions.at(-1)?.context?.value as typeof PARAS)[1]).toEqual({ runs: [{ t: "Trois" }] });
  });

  test("une ligne inchangée garde sa couleur", async () => {
    const m = await editor();
    await m.type("Un\nTrois");
    await m.blur();
    expect((m.actions.at(-1)?.context?.value as typeof PARAS)[0]).toEqual(PARAS[0]);
  });

  test("Échap rétablit le texte", async () => {
    const m = await editor();
    await m.type("Autre");
    await m.key("Escape");
    expect(m.view.state.doc.toString()).toBe("Un\nDeux");
  });

  test("Échap n'envoie rien", async () => {
    const m = await editor();
    await m.type("Autre");
    await m.key("Escape");
    expect(m.actions).toHaveLength(0);
  });

  test("⌘↩ valide", async () => {
    const m = await editor();
    m.content.focus();
    await m.type("Un\nTrois");
    const mod = /Mac/.test(navigator.platform) ? { metaKey: true } : { ctrlKey: true };
    await m.key("Enter", mod);
    expect(m.actions).toHaveLength(1);
  });

  test("après Échap, annuler ne rejoue pas la saisie abandonnée", async () => {
    const m = await editor();
    await m.type("Autre");
    await m.key("Escape");
    undo(m.view);
    expect(m.view.state.doc.toString()).toBe("Un\nDeux");
  });

  test("après une nouvelle valeur de la session, annuler ne rejoue pas l'ancienne saisie", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const props = { label: "Corps", source: PARAS, palette: [], onCommit: () => {} };
    try {
      await act(async () => root.render(createElement(RichTextField, { ...props, value: "Un\nDeux" })));
      const view = EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement)!;
      await act(async () => view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: "Autre" } }));
      await act(async () => root.render(createElement(RichTextField, { ...props, value: "Trois" })));
      undo(EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement)!);
      expect(EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement)!.state.doc.toString()).toBe("Trois");
    } finally {
      await act(async () => root.unmount());
      host.remove();
    }
  });
});
