// Rendu du sélecteur de thème dans un DOM happy-dom.

import { afterAll, afterEach, expect, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const ThemeMenu = (await import("./ThemeMenu")).default;
const { setMessages } = await import("./i18n");

afterAll(resetMessages);
setMessages("fr", ["ui-toolbar-theme = Thème\nui-toolbar-theme-embedded = Thème du fichier\n"]);

const info = (namespace: string, name: string, over: Record<string, unknown> = {}) => ({
  id: `id-${namespace}`, version: "v1", name, namespace, extends: [], builtin: true, default: false, ...over,
});
const themes = [info("papier", "Papier", { default: true }), info("pro", "Pro"), info("neutre", "Neutre")];

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(theme: string | null, themeName: string | null = null) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const picked: string[] = [];
  await act(async () => root.render(createElement(ThemeMenu, { theme, themeName, themes, label: "Thème", onPick: (ns: string) => picked.push(ns) })));
  return { select: host.querySelector("select")!, picked };
}

test("ThemeMenu without deck-only themes lists library themes by namespace", async () => {
  const { select } = await mount("pro");
  expect([...select.options].map((o) => o.getAttribute("value"))).toEqual(["papier", "pro", "neutre"]);
  expect([...select.options].map((o) => o.textContent)).toEqual(["Papier", "Pro", "Neutre"]);
  expect(select.value).toBe("pro");
  expect(select.querySelector("optgroup")).toBeNull();
});

test("choisir Pro appelle onPick avec son espace de noms", async () => {
  const { select, picked } = await mount("papier");
  await act(async () => {
    select.value = "pro";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(picked).toEqual(["pro"]);
});

test("thème épinglé absent de la bibliothèque : option désactivée, sélectionnée, sous son nom", async () => {
  const { select } = await mount("maison", "Maison");
  const first = select.options[0]!;
  expect([first.textContent, first.disabled]).toEqual(["Maison", true]);
  expect(select.value).toBe("maison");
  expect(select.options.length).toBe(4);
});

test("thème absent et sans nom : l'étiquette traduite du thème du fichier", async () => {
  const { select } = await mount("maison");
  expect(select.options[0]!.textContent).toBe("Thème du fichier");
  expect(select.options[0]!.disabled).toBe(true);
});
