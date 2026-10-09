// Le brouillon de dictée envoyé à l'agent : gardé si la consigne échoue (t-211).

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
async function mount(ok: boolean) {
  const prompts: string[] = [];
  let out: ReturnType<typeof useDictation> | null = null;
  const Probe = () => {
    out = useDictation({
      snap: null,
      view: null,
      selected: null,
      editing: null,
      apply: async () => true,
      addElement: () => {},
      say: () => {},
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
