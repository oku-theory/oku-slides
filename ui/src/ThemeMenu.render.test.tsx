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
setMessages("fr", ["ui-toolbar-theme = Thème\nui-toolbar-theme-embedded = Thème du fichier\nui-toolbar-theme-deck-group = Cette présentation\n"]);

const PAPIER = "2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31";
const PRO = "7e1b9c3d-2f4a-4d6e-a8b0-5c7d9e1f3b42";
const OTHER = "11111111-1111-4111-8111-111111111111";
const IMPORTED = "22222222-2222-4222-8222-222222222222";
const importedTheme = { id: IMPORTED, version: "v3", name: "Importé", builtin: false, default: false, deck_only: true };
const themes = [
  { id: PAPIER, version: "v1", name: "Papier", builtin: true, default: true },
  { id: PRO, version: "v1", name: "Pro", builtin: true, default: false },
];

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(
  theme: { id: string; version: string } | null,
  themeName: string | null = null,
  deckThemes: (typeof importedTheme)[] = [],
) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const picked: string[] = [];
  await act(async () => root.render(createElement(ThemeMenu, { theme, themeName, themes, deckThemes, onPick: (id: string) => picked.push(id) })));
  return { select: host.querySelector("select")!, picked };
}

test("deux thèmes : les noms sont les options et le thème du deck est choisi", async () => {
  const { select } = await mount({ id: PAPIER, version: "v1" });
  expect([...select.options].map((o) => o.textContent)).toEqual(["Papier", "Pro"]);
  expect(select.value).toBe(PAPIER);
});

test("choisir Pro appelle onPick avec son id", async () => {
  const { select, picked } = await mount({ id: PAPIER, version: "v1" });
  await act(async () => {
    select.value = PRO;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(picked).toEqual([PRO]);
});

test("thème inconnu de la bibliothèque : option désactivée, sélectionnée, sous son nom", async () => {
  const { select } = await mount({ id: OTHER, version: "v9" }, "Maison");
  const first = select.options[0]!;
  expect(first.textContent).toBe("Maison");
  expect(first.disabled).toBe(true);
  expect(select.value).toBe(OTHER);
  expect(select.options.length).toBe(3);
});

test("thème inconnu et sans nom : l'étiquette traduite du thème du fichier", async () => {
  const { select } = await mount({ id: OTHER, version: "v9" });
  expect(select.options[0]!.textContent).toBe("Thème du fichier");
  expect(select.options[0]!.disabled).toBe(true);
});

test("thèmes de la présentation : un groupe traduit, son option active", async () => {
  const { select } = await mount({ id: PAPIER, version: "v1" }, null, [importedTheme]);
  const group = select.querySelector("optgroup")!;
  expect(group.label).toBe("Cette présentation");
  const option = group.querySelector("option")!;
  expect([option.value, option.textContent, option.disabled]).toEqual([IMPORTED, "Importé", false]);
});

test("choisir le thème de la présentation appelle onPick avec son id", async () => {
  const { select, picked } = await mount({ id: PAPIER, version: "v1" }, null, [importedTheme]);
  await act(async () => {
    select.value = IMPORTED;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(picked).toEqual([IMPORTED]);
});

test("thème du deck parmi ceux de la présentation : choisi, sans option désactivée", async () => {
  const { select } = await mount({ id: IMPORTED, version: "v3" }, "Importé", [importedTheme]);
  expect(select.value).toBe(IMPORTED);
  expect([...select.options].some((o) => o.disabled)).toBe(false);
  expect(select.options.length).toBe(3);
});

test("sans thème de la présentation : aucun groupe", async () => {
  const { select } = await mount({ id: PAPIER, version: "v1" });
  expect(select.querySelector("optgroup")).toBeNull();
});
