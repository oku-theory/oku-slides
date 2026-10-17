// Rendu du dialogue d'import d'un .pptx dans un DOM happy-dom.

import { afterAll, afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { ImportTarget, PackInfo, ThemeInfo } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const ImportDialog = (await import("./ImportDialog")).default;
const { ARMING_MS } = await import("./modalFrame");
const { setMessages } = await import("./i18n");

afterAll(resetMessages);
setMessages("fr", [
  [
    "ui-import-title = Importer une présentation PowerPoint",
    "ui-import-hifi = Copie fidèle",
    "ui-import-retrofit = Utiliser mes modèles",
    "ui-import-theme-default = Thème par défaut",
    "ui-import-no-packs = Aucun pack de modèles dans la bibliothèque.",
    "ui-import-tune = Peaufiner avec l'agent",
    "ui-import-tune-note = L'agent compare chaque slide à l'original.",
    "ui-import-tune-note-retrofit = L'agent compare chaque slide et dessine les widgets manquants.",
    "ui-import-submit = Importer",
    "ui-import-busy = Import en cours…",
    "ui-close-cancel = Annuler",
    "ui-picker-set-mine = { $name } · à moi",
    "ui-picker-set-received = { $name } · reçu",
  ].join("\n") + "\n",
]);

const PACKS: PackInfo[] = [
  { id: "p-base", version: "v1", name: "Base", description: "", builtin: true, default: false, received: false, count: 3 },
  { id: "p-pro", version: "v1", name: "Pro", description: "", builtin: true, default: true, received: false, count: 5 },
  { id: "p-moi", version: "v2", name: "Maison", description: "", builtin: false, default: false, received: false, count: 2 },
  { id: "p-recu", version: "v1", name: "Client", description: "", builtin: false, default: false, received: true, count: 4 },
];
const THEMES: ThemeInfo[] = [
  { id: "t-papier", version: "v1", name: "Papier", builtin: true, default: true, deck_only: false },
  { id: "t-nuit", version: "v1", name: "Nuit", builtin: true, default: false, deck_only: false },
  { id: "t-import", version: "v1", name: "Alto", builtin: false, default: false, deck_only: true },
];

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
let now = 0;
beforeEach(() => {
  now = 1_000_000;
  spyOn(Date, "now").mockImplementation(() => now);
});
afterEach(async () => {
  (Date.now as unknown as { mockRestore: () => void }).mockRestore();
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

interface Calls {
  imports: { target: ImportTarget; tune: boolean }[];
  closed: number;
}

interface Options {
  packs?: () => Promise<PackInfo[]>;
  busy?: boolean;
  error?: string | null;
}

async function mount({ packs = () => Promise.resolve(PACKS), busy = false, error = null }: Options = {}) {
  const calls: Calls = { imports: [], closed: 0 };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const api = { templateSets: packs };
  const render = (state: { busy: boolean; error: string | null }) =>
    act(async () =>
      root.render(
        createElement(ImportDialog, {
          path: "/Users/moi/Présentations/Alto Wealth.pptx",
          api,
          themes: THEMES,
          ...state,
          onImport: (target: ImportTarget, tune: boolean) => void calls.imports.push({ target, tune }),
          onClose: () => void calls.closed++,
        }),
      ),
    );
  await render({ busy, error });
  const radio = (value: string) => host.querySelector<HTMLInputElement>(`input[type="radio"][value="${value}"]`)!;
  const selects = () => [...host.querySelectorAll("select")] as HTMLSelectElement[];
  const tune = () => host.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
  const button = (label: string) => [...host.querySelectorAll("button")].find((b) => b.textContent === label)!;
  const dialog = () => host.querySelector<HTMLElement>('[role="dialog"]')!;
  return { host, calls, radio, selects, tune, button, dialog, render };
}

/** Clique après la garde des 500 ms. */
async function click(el: HTMLElement) {
  now += ARMING_MS + 1;
  await act(async () => el.click());
}

async function choose(select: HTMLSelectElement, value: string) {
  await act(async () => {
    select.value = value;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
}

/** Une touche sur l'élément qui a le focus (ou le document) : elle passe par la fenêtre. */
async function press(key: string, shiftKey = false) {
  const target = document.activeElement ?? document.body;
  await act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { key, shiftKey, bubbles: true, cancelable: true })));
}

test("ImportDialog s'ouvre en copie fidèle, sans pack ni thème, peaufinage coché", async () => {
  const { host, radio, selects, tune, dialog } = await mount();
  expect(dialog().getAttribute("aria-modal")).toBe("true");
  expect(host.querySelector("h2")?.textContent).toBe("Importer une présentation PowerPoint");
  expect(host.querySelector(".import-file")?.textContent).toBe("Alto Wealth.pptx");
  expect(radio("hifi").checked).toBe(true);
  expect(selects()).toHaveLength(0);
  expect(tune().checked).toBe(true);
  expect(tune().disabled).toBe(false);
});

test("ImportDialog prend le focus à l'ouverture", async () => {
  const { dialog } = await mount();
  expect(document.activeElement).toBe(dialog());
});

test("ImportDialog en retrofit montre les packs (défaut choisi, reçus et à moi nommés)", async () => {
  const { radio, selects } = await mount();
  await click(radio("retrofit"));
  const [pack] = selects();
  expect([...pack!.options].map((o) => o.textContent)).toEqual(["Base", "Pro", "Maison · à moi", "Client · reçu"]);
  expect(pack!.value).toBe("p-pro");
});

test("ImportDialog en retrofit liste les thèmes sans le défaut ni un thème du deck seul", async () => {
  const { radio, selects } = await mount();
  await click(radio("retrofit"));
  const theme = selects()[1]!;
  expect([...theme.options].map((o) => o.textContent)).toEqual(["Thème par défaut", "Nuit"]);
  expect(theme.value).toBe("");
});

test("ImportDialog sans aucun pack le dit en retrofit et n'importe pas", async () => {
  const { host, radio, button } = await mount({ packs: () => Promise.resolve([]) });
  await click(radio("retrofit"));
  expect(host.querySelector(".import-target .caption.import-empty")?.textContent).toBe("Aucun pack de modèles dans la bibliothèque.");
  expect(button("Importer").disabled).toBe(true);
});

test("ImportDialog montre l'erreur de la liste des packs seulement en retrofit", async () => {
  const { host, radio } = await mount({ packs: () => Promise.reject(new Error("bibliothèque illisible")) });
  expect(host.querySelector('[role="alert"]')).toBeNull();
  await click(radio("retrofit"));
  expect(host.querySelector('[role="alert"]')?.textContent).toBe("bibliothèque illisible");
  await click(radio("hifi"));
  expect(host.querySelector('[role="alert"]')).toBeNull();
});

test("ImportDialog explique le peaufinage selon le mode", async () => {
  const { host, radio } = await mount();
  const note = () => host.querySelector("#import-tune-note")?.textContent;
  expect(note()).toBe("L'agent compare chaque slide à l'original.");
  await click(radio("retrofit"));
  expect(note()).toBe("L'agent compare chaque slide et dessine les widgets manquants.");
});

test("ImportDialog revenu en copie fidèle cache de nouveau pack et thème", async () => {
  const { radio, selects } = await mount();
  await click(radio("retrofit"));
  await click(radio("hifi"));
  expect(selects()).toHaveLength(0);
});

test("ImportDialog Importer en copie fidèle envoie hifi et le peaufinage, sans se fermer lui-même", async () => {
  const { calls, button } = await mount();
  await click(button("Importer"));
  expect(calls.imports).toEqual([{ target: { mode: "hifi" }, tune: true }]);
  expect(calls.closed).toBe(0);
});

test("ImportDialog Importer en retrofit envoie le pack et le thème choisis, sans peaufinage", async () => {
  const { calls, radio, selects, tune, button } = await mount();
  await click(radio("retrofit"));
  await choose(selects()[0]!, "p-moi");
  await choose(selects()[1]!, "t-nuit");
  await click(tune());
  await click(button("Importer"));
  expect(calls.imports).toEqual([{ target: { mode: "retrofit", pack: "p-moi", theme: "t-nuit" }, tune: false }]);
});

test("ImportDialog retrofit sur le thème par défaut envoie theme null", async () => {
  const { calls, radio, button } = await mount();
  await click(radio("retrofit"));
  await click(button("Importer"));
  expect(calls.imports[0]!.target).toEqual({ mode: "retrofit", pack: "p-pro", theme: null });
});

test("ImportDialog ignore un clic des 500 premières ms", async () => {
  const { calls, button } = await mount();
  await act(async () => button("Importer").click());
  expect(calls.imports).toHaveLength(0);
});

test("ImportDialog se dit armé (data-armed) une fois la garde passée", async () => {
  const { dialog } = await mount();
  expect(dialog().hasAttribute("data-armed")).toBe(false);
  await act(async () => new Promise((r) => setTimeout(r, ARMING_MS + 20)));
  expect(dialog().getAttribute("data-armed")).toBe("true");
});

test("ImportDialog pendant l'import : occupé, choix désactivés, focus sur la fenêtre", async () => {
  const { host, calls, tune, button, dialog, render } = await mount();
  button("Importer").focus();
  await render({ busy: true, error: null });
  expect(host.querySelector('[role="status"]')?.textContent).toBe("Import en cours…");
  expect(dialog().getAttribute("aria-busy")).toBe("true");
  expect(button("Importer").disabled).toBe(true);
  expect(tune().disabled).toBe(true);
  expect(document.activeElement).toBe(dialog());
  await click(button("Importer"));
  expect(calls.imports).toHaveLength(0);
});

test("ImportDialog pendant l'import : Annuler et Échap ferment, l'import continue ailleurs", async () => {
  const { calls, button } = await mount({ busy: true });
  expect(button("Annuler").disabled).toBe(false);
  await click(button("Annuler"));
  await press("Escape");
  expect(calls.closed).toBe(2);
});

test("ImportDialog montre le refus de l'hôte", async () => {
  const { host, button } = await mount({ error: "Ce fichier n'est pas une présentation PowerPoint." });
  expect(host.querySelector('[role="alert"]')?.textContent).toBe("Ce fichier n'est pas une présentation PowerPoint.");
  expect(button("Importer").disabled).toBe(false);
});

test("ImportDialog Échap et Annuler ferment sans importer", async () => {
  const { calls, button } = await mount();
  now += ARMING_MS + 1;
  await press("Escape");
  await click(button("Annuler"));
  expect(calls.closed).toBe(2);
  expect(calls.imports).toHaveLength(0);
});

test("ImportDialog ignore Échap pendant la garde, sans le laisser à l'app", async () => {
  const { calls } = await mount();
  let reached = false;
  const app = () => (reached = true);
  window.addEventListener("keydown", app);
  await press("Escape");
  window.removeEventListener("keydown", app);
  expect(calls.closed).toBe(0);
  expect(reached).toBe(false);
});

test("ImportDialog n'importe pas sur Entrée", async () => {
  const { calls, dialog } = await mount();
  await act(async () => void dialog().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })));
  expect(calls.imports).toHaveLength(0);
});

test("ImportDialog en copie fidèle : Tab tourne du dernier bouton à la radio cochée, Maj+Tab en sens inverse", async () => {
  const { radio, button } = await mount();
  button("Importer").focus();
  await press("Tab");
  expect(document.activeElement).toBe(radio("hifi"));
  await press("Tab", true);
  expect(document.activeElement).toBe(button("Importer"));
});

test("ImportDialog en retrofit : Tab saute la radio non cochée", async () => {
  const { radio, button } = await mount();
  await click(radio("retrofit"));
  button("Importer").focus();
  await press("Tab");
  expect(document.activeElement).toBe(radio("retrofit"));
  await press("Tab", true);
  expect(document.activeElement).toBe(button("Importer"));
});

test("ImportDialog : un focus perdu revient dans la fenêtre au Tab", async () => {
  const { radio } = await mount();
  (document.activeElement as HTMLElement | null)?.blur();
  await press("Tab");
  expect(document.activeElement).toBe(radio("hifi"));
});
