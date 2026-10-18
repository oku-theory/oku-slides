// La dictée d'un onglet : le brouillon envoyé à l'agent (gardé si la consigne échoue, t-211)
// et le texte dicté au curseur pendant l'édition.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useDictation } = await import("./useDictation");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

/** Monte la dictée avec une consigne qui réussit ou échoue (`ok`). */
async function mount(ok: boolean, editing: string | null = null) {
  const prompts: string[] = [];
  let out: ReturnType<typeof useDictation> | null = null;
  const Probe = () => {
    out = useDictation({
      snap: null,
      editing,
      setDraft: async () => {},
      prompt: async (text: string) => {
        prompts.push(text);
        return ok;
      },
    });
    return null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  await act(async () => out!.actions.onDictated("Ajoute une slide", "draft"));
  return { state: () => out!, prompts };
}

test("useDictation un envoi réussi ferme le brouillon", async () => {
  const d = await mount(true);
  await act(async () => d.state().actions.sendDraft("Ajoute une slide"));
  expect(d.prompts).toEqual(["Ajoute une slide"]);
  expect(d.state().draft.state.text).toBeNull();
});

test("useDictation un envoi qui échoue garde le texte dicté", async () => {
  const d = await mount(false);
  await act(async () => d.state().actions.sendDraft("Ajoute une slide"));
  expect(d.prompts).toEqual(["Ajoute une slide"]);
  expect(d.state().draft.state.text).toBe("Ajoute une slide");
});

test("useDictation hors édition n'écrit nulle part", async () => {
  const d = await mount(true);
  expect(d.state().insertText("Budget 2027")).toBe(false);
});

test("useDictation pendant l'édition écrit au curseur", async () => {
  const d = await mount(true, "e1\u001ftitle");
  let ok = false;
  await act(async () => void (ok = d.state().insertText("Budget 2027")));
  expect([ok, d.state().dictated]).toEqual([true, { seq: 1, text: "Budget 2027" }]);
});
