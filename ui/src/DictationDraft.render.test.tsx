// Rendu du brouillon des dictées dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { DictationDraft } = await import("./DictationDraft");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(text: string | null, { busy = false, insertTo = "dans le titre" as string | null } = {}) {
  const sent: string[] = [];
  const inserted: string[] = [];
  const changes: string[] = [];
  let discarded = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(DictationDraft, {
        text,
        dictations: 1,
        busy,
        insertTo,
        onInsert: (v: string) => inserted.push(v),
        onChange: (v: string) => changes.push(v),
        onSend: (v: string) => sent.push(v),
        onDiscard: () => discarded++,
      }),
    );
  });
  const area = () => host.querySelector("textarea") as HTMLTextAreaElement;
  const send = () => host.querySelector(".draft-send") as HTMLButtonElement;
  const insert = () => host.querySelector(".draft-insert") as HTMLButtonElement;
  return { host, area, send, insert, sent, inserted, changes, discarded: () => discarded };
}

const key = (el: Element, init: KeyboardEventInit) =>
  el.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init }));

describe("brouillon de dictée", () => {
  test("DictationDraft sans brouillon ne montre rien", async () => {
    const { host } = await mount(null);
    expect(host.querySelector(".draft")).toBeNull();
  });

  test("DictationDraft avec une dictée la montre, éditable", async () => {
    const { area } = await mount("rends ce titre plus court");
    expect(area().value).toBe("rends ce titre plus court");
  });

  test("DictationDraft n'envoie rien tant qu'on n'appuie pas sur Envoyer", async () => {
    const { sent } = await mount("rends ce titre");
    await act(async () => new Promise((r) => setTimeout(r, 20)));
    expect(sent).toEqual([]);
  });

  test("DictationDraft Envoyer envoie le texte sans ses blancs de bord", async () => {
    const { send, sent } = await mount("  plus court \n");
    await act(async () => send().click());
    expect(sent).toEqual(["plus court"]);
  });

  test("DictationDraft ⌘↩ dans le texte envoie", async () => {
    const { area, sent } = await mount("plus court");
    await act(async () => key(area(), { key: "Enter", metaKey: true }));
    expect(sent).toEqual(["plus court"]);
  });

  test("DictationDraft ↩ seul n'envoie pas", async () => {
    const { area, sent } = await mount("plus court");
    await act(async () => key(area(), { key: "Enter" }));
    expect(sent).toEqual([]);
  });

  test("DictationDraft vide ne peut pas être envoyé", async () => {
    const { area, send, sent } = await mount("   ");
    expect(send().disabled).toBe(true);
    await act(async () => key(area(), { key: "Enter", metaKey: true }));
    expect(sent).toEqual([]);
  });

  test("DictationDraft pendant un tour de l'agent attend pour envoyer", async () => {
    const { area, send, sent } = await mount("plus court", { busy: true });
    expect(send().disabled).toBe(true);
    await act(async () => key(area(), { key: "Enter", metaKey: true }));
    expect(sent).toEqual([]);
  });

  test("DictationDraft Effacer abandonne le brouillon", async () => {
    const { host, discarded } = await mount("plus court");
    await act(async () => (host.querySelector(".draft-discard") as HTMLButtonElement).click());
    expect(discarded()).toBe(1);
  });

  test("DictationDraft après une dictée place le curseur en fin de texte", async () => {
    const { area } = await mount("plus court");
    expect(document.activeElement).toBe(area());
    expect(area().selectionStart).toBe("plus court".length);
  });

  test("DictationDraft Insérer écrit le texte tel quel, sans l'agent", async () => {
    const { insert, inserted, sent } = await mount("  Budget 2027 \n");
    await act(async () => insert().click());
    expect(inserted).toEqual(["Budget 2027"]);
    expect(sent).toEqual([]);
  });

  test("DictationDraft ⌘⇧↩ insère", async () => {
    const { area, inserted, sent } = await mount("Budget 2027");
    await act(async () => key(area(), { key: "Enter", metaKey: true, shiftKey: true }));
    expect(inserted).toEqual(["Budget 2027"]);
    expect(sent).toEqual([]);
  });

  test("DictationDraft dit où Insérer écrira", async () => {
    const { host } = await mount("Budget 2027");
    expect(host.querySelector(".draft-insert-to")?.textContent).toBe("→ dans le titre");
  });

  test("DictationDraft sans zone où écrire ne peut pas insérer", async () => {
    const { insert, area, inserted } = await mount("Budget 2027", { insertTo: null });
    expect(insert().disabled).toBe(true);
    await act(async () => key(area(), { key: "Enter", metaKey: true, shiftKey: true }));
    expect(inserted).toEqual([]);
  });

  test("DictationDraft pendant un tour de l'agent peut toujours insérer", async () => {
    const { insert } = await mount("Budget 2027", { busy: true });
    expect(insert().disabled).toBe(false);
  });
});
