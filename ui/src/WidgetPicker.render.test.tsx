// Rendu de la grille d'ajout dans un DOM happy-dom.

import { afterAll, afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { WidgetPreview } from "./api";
import type { WidgetChoice } from "./widgetChoices";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const WidgetPicker = (await import("./WidgetPicker")).default;
const { ARMING_MS } = await import("./modalFrame");
const { setMessages } = await import("./i18n");
const { widgetChoices } = await import("./widgetChoices");

afterAll(resetMessages);
setMessages("fr", [
  [
    "ui-add-kind = { $kind ->",
    "    [card] Carte",
    "    [shape] Forme",
    "    [chart] Graphique",
    "    [table] Tableau",
    "   *[text] Texte",
    "}",
    "ui-add-kind-description = { $kind ->",
    "    [card] Un titre et un court texte dans un cadre",
    "    [shape] Une forme colorée",
    "    [chart] Un graphique tiré d'un tableau",
    "    [table] Un tableau de données",
    "   *[text] Un paragraphe de texte",
    "}",
    "ui-widget-picker-empty = Aucun widget pour « { $query } ».",
    "ui-picker-close = Fermer",
  ].join("\n") + "\n",
]);

const PACK = [
  { name: "kpi", label: "Indicateur", description: "Une valeur en grand" },
  { name: "frise", label: "Frise", description: "Des étapes datées" },
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

interface Options {
  preview?: (name: string) => Promise<WidgetPreview>;
  canPick?: (c: WidgetChoice) => boolean;
  pack?: typeof PACK;
}

const SVG = { svg: "<svg/>", diagnostics: [] } as unknown as WidgetPreview;

async function mount({ preview = async () => SVG, canPick = () => true, pack = PACK }: Options = {}) {
  const picked: string[] = [];
  const asked: string[] = [];
  let closed = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const render = (list: typeof PACK, pickable = canPick) =>
    act(async () =>
      root.render(
        createElement(WidgetPicker, {
          choices: widgetChoices(list),
          canPick: pickable,
          themeKey: "t",
          preview: (name: string) => (asked.push(name), preview(name)),
          onPick: (c: WidgetChoice) => void picked.push(c.key),
          onClose: () => void closed++,
        }),
      ),
    );
  await render(pack);
  await act(async () => new Promise((r) => setTimeout(r, 0)));
  const names = () => [...host.querySelectorAll('[role="option"] .template-name')].map((n) => n.textContent);
  const filter = () => host.querySelector<HTMLInputElement>('input[type="search"]')!;
  const selected = () => host.querySelector('[aria-selected="true"] .template-name')?.textContent;
  const button = (label: string) => host.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
  return { host, picked, asked, closed: () => closed, names, filter, selected, button, rerender: render };
}

/** Attend la fin de la garde des 500 ms (minuteur réel du cadre, et horloge simulée). */
async function arm() {
  now += ARMING_MS + 1;
  await act(async () => new Promise((r) => setTimeout(r, ARMING_MS + 20)));
}

async function type(el: HTMLInputElement, text: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, text);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function key(el: HTMLElement, k: string) {
  await act(async () => {
    el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
  });
}

test("WidgetPicker opened shows the native kinds then the pack widgets", async () => {
  const { names } = await mount();
  expect(names()).toEqual(["Texte", "Carte", "Forme", "Graphique", "Tableau", "Indicateur", "Frise"]);
});

test("WidgetPicker opened focuses the filter field", async () => {
  const { filter } = await mount();
  expect(document.activeElement).toBe(filter());
});

test("WidgetPicker filtered by a description word keeps the matching cards", async () => {
  const { filter, names } = await mount();
  await type(filter(), "etapes");
  expect(names()).toEqual(["Frise"]);
});

test("WidgetPicker filtered with no match says so", async () => {
  const { filter, host } = await mount();
  await type(filter(), "zzz");
  expect(host.querySelector(".widget-picker-empty")?.textContent).toBe("Aucun widget pour « zzz ».");
});

test("WidgetPicker Enter after the guard picks the focused filtered card", async () => {
  const { filter, picked } = await mount();
  await type(filter(), "valeur");
  await arm();
  await key(filter(), "Enter");
  expect(picked).toEqual(["widget:kpi"]);
});

test("WidgetPicker Enter during the guard picks nothing", async () => {
  const { filter, picked } = await mount();
  await act(async () => new Promise((r) => setTimeout(r, 0)));
  await key(filter(), "Enter");
  expect(picked).toEqual([]);
});

test("WidgetPicker arrow right moves the focus to the next card", async () => {
  const { filter, selected } = await mount();
  await key(filter(), "ArrowRight");
  expect(selected()).toBe("Carte");
});

test("WidgetPicker card refused by canPick is disabled", async () => {
  const { host } = await mount({ canPick: (c) => c.key !== "text" });
  expect(host.querySelector<HTMLButtonElement>('[role="option"]')!.disabled).toBe(true);
});

test("WidgetPicker pack widget shows its rendered preview in an img", async () => {
  const { host } = await mount();
  expect(host.querySelectorAll('[role="option"] img')).toHaveLength(2);
});

test("WidgetPicker previews are asked for each pack widget, in order", async () => {
  const { asked } = await mount();
  expect(asked).toEqual(["kpi", "frise"]);
});

test("WidgetPicker pack widget whose preview fails keeps the generic icon, the others load", async () => {
  const { host } = await mount({ preview: async (name) => (name === "kpi" ? Promise.reject(new Error("rendu")) : SVG) });
  const faces = [...host.querySelectorAll('[role="option"] .widget-choice-face')].slice(5);
  expect(faces.map((f) => (f.querySelector("img") ? "img" : "icon"))).toEqual(["icon", "img"]);
});

test("WidgetPicker pack widget named like an Object property without preview keeps the icon", async () => {
  const { host } = await mount({
    pack: [{ name: "constructor", label: null, description: null }],
    preview: () => Promise.reject(new Error("rendu")),
  });
  expect(host.querySelectorAll('[role="option"] img')).toHaveLength(0);
});

test("WidgetPicker Enter on the close button does not pick a card", async () => {
  const { button, picked } = await mount();
  await arm();
  await key(button("Fermer"), "Enter");
  expect(picked).toEqual([]);
});

test("WidgetPicker arrow right with the caret inside the filter text keeps the card focus", async () => {
  const { filter, selected } = await mount();
  await type(filter(), "ta");
  const before = selected();
  filter().setSelectionRange(1, 1);
  await key(filter(), "ArrowRight");
  expect([before, selected()]).toEqual(["Graphique", "Graphique"]);
});

test("WidgetPicker arrow right with the caret at the end of the filter moves the card focus", async () => {
  const { filter, selected } = await mount();
  await type(filter(), "a");
  filter().setSelectionRange(1, 1);
  await key(filter(), "ArrowRight");
  expect(selected()).toBe("Carte");
});

test("WidgetPicker opened with the first card refused focuses the first pickable one", async () => {
  const { selected } = await mount({ canPick: (c) => c.key !== "text" });
  expect(selected()).toBe("Carte");
});

test("WidgetPicker arrows skip refused cards", async () => {
  const { filter, selected } = await mount({ canPick: (c) => c.key !== "card" });
  await key(filter(), "ArrowRight");
  expect(selected()).toBe("Forme");
});

test("WidgetPicker focus moved then filtered resets to the first match even if still shown", async () => {
  const { filter, selected } = await mount();
  for (let i = 0; i < 4; i++) await key(filter(), "ArrowRight");
  await type(filter(), "ta");
  expect(selected()).toBe("Graphique");
});

test("WidgetPicker focused card kept when the same choices come again", async () => {
  const { filter, selected, rerender } = await mount();
  for (let i = 0; i < 6; i++) await key(filter(), "ArrowRight");
  await rerender([...PACK]);
  expect(selected()).toBe("Frise");
});

test("WidgetPicker focused card that becomes refused falls back to the first pickable card", async () => {
  const { filter, selected, rerender } = await mount();
  for (let i = 0; i < 2; i++) await key(filter(), "ArrowRight");
  await rerender(PACK, (c) => c.key !== "shape");
  expect(selected()).toBe("Texte");
});

test("WidgetPicker with no pickable card marks none as focused", async () => {
  const { filter, selected } = await mount({ canPick: () => false });
  expect([selected(), filter().getAttribute("aria-activedescendant")]).toEqual([undefined, null]);
});

test("WidgetPicker arrow left with the caret at the start moves the focus back", async () => {
  const { filter, selected } = await mount();
  await key(filter(), "ArrowRight");
  await key(filter(), "ArrowLeft");
  expect(selected()).toBe("Texte");
});

test("WidgetPicker arrow left on the first card keeps it", async () => {
  const { filter, selected } = await mount();
  await key(filter(), "ArrowLeft");
  expect(selected()).toBe("Texte");
});

/** Pose les cartes en rangées de `columns` (happy-dom n'a pas de mise en page). */
function rows(host: HTMLElement, columns: number) {
  host.querySelectorAll<HTMLElement>('[role="option"]').forEach((el, i) => {
    Object.defineProperty(el, "offsetTop", { configurable: true, get: () => Math.floor(i / columns) * 100 });
  });
}

test("WidgetPicker arrow down moves the focus one row below", async () => {
  const { filter, host, selected } = await mount();
  rows(host, 3);
  await key(filter(), "ArrowRight");
  await key(filter(), "ArrowDown");
  expect(selected()).toBe("Tableau");
});

test("WidgetPicker arrow up moves the focus one row above", async () => {
  const { filter, host, selected } = await mount();
  rows(host, 3);
  for (let i = 0; i < 4; i++) await key(filter(), "ArrowRight");
  await key(filter(), "ArrowUp");
  expect(selected()).toBe("Carte");
});

test("WidgetPicker arrow down onto a refused card goes on down, or stays", async () => {
  const { filter, host, selected } = await mount({ canPick: (c) => c.key !== "table" });
  rows(host, 3);
  await key(filter(), "ArrowRight");
  await key(filter(), "ArrowDown");
  expect(selected()).toBe("Carte");
});

test("WidgetPicker arrow down from the filter is taken from the text field", async () => {
  const { filter } = await mount();
  const down = new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true });
  await act(async () => filter().dispatchEvent(down));
  expect(down.defaultPrevented).toBe(true);
});

test("WidgetPicker arrow down with text in the filter still moves a row", async () => {
  const { filter, host, selected } = await mount();
  await type(filter(), "un");
  filter().setSelectionRange(0, 0);
  rows(host, 3);
  await key(filter(), "ArrowDown");
  expect(selected()).toBe((host.querySelectorAll('[role="option"] .template-name')[3] as HTMLElement).textContent);
});

test("WidgetPicker shift arrow down keeps the card focus and the text field's own behaviour", async () => {
  const { filter, host, selected } = await mount();
  rows(host, 3);
  const down = new KeyboardEvent("keydown", { key: "ArrowDown", shiftKey: true, bubbles: true, cancelable: true });
  await act(async () => filter().dispatchEvent(down));
  expect([selected(), down.defaultPrevented]).toEqual(["Texte", false]);
});

test("WidgetPicker pointer resting still while the grid scrolls does not take the focus", async () => {
  const { filter, host, selected } = await mount();
  const card = host.querySelectorAll('[role="option"]')[4];
  const at = { bubbles: true, clientX: 10, clientY: 10 };
  await act(async () => card.dispatchEvent(new MouseEvent("mousemove", at)));
  await key(filter(), "ArrowLeft");
  await act(async () => card.dispatchEvent(new MouseEvent("mousemove", at)));
  expect(selected()).toBe("Graphique");
});

test("WidgetPicker mouse down on a card keeps the focus in the filter", async () => {
  const { host } = await mount();
  const down = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
  host.querySelectorAll('[role="option"]')[2].dispatchEvent(down);
  expect(down.defaultPrevented).toBe(true);
});

test("WidgetPicker pointer moving over a card focuses it", async () => {
  const { host, selected } = await mount();
  await act(async () => host.querySelectorAll('[role="option"]')[4].dispatchEvent(new MouseEvent("mousemove", { bubbles: true })));
  expect(selected()).toBe("Tableau");
});

test("WidgetPicker empty filter result is announced outside the list", async () => {
  const { filter, host } = await mount();
  await type(filter(), "zzz");
  expect([host.querySelector('[role="listbox"] .widget-picker-empty') !== null, host.querySelector('[role="status"]')?.textContent]).toEqual([
    false,
    "Aucun widget pour « zzz ».",
  ]);
});

test("WidgetPicker focused card removed while open falls back to the first card", async () => {
  const { filter, selected, rerender } = await mount();
  for (let i = 0; i < 6; i++) await key(filter(), "ArrowRight");
  await rerender([]);
  expect(selected()).toBe("Texte");
});

test("WidgetPicker filter field points at the focused card for screen readers", async () => {
  const { filter, host } = await mount();
  await key(filter(), "ArrowRight");
  const id = filter().getAttribute("aria-activedescendant");
  expect(id && host.querySelector(`#${id} .template-name`)?.textContent).toBe("Carte");
});

test("WidgetPicker click on a card after the guard picks it", async () => {
  const { host, picked } = await mount();
  now += ARMING_MS + 1;
  await act(async () => host.querySelectorAll<HTMLButtonElement>('[role="option"]')[3].click());
  expect(picked).toEqual(["chart"]);
});

test("WidgetPicker click on a card during the guard picks nothing", async () => {
  const { host, picked } = await mount();
  await act(async () => host.querySelectorAll<HTMLButtonElement>('[role="option"]')[3].click());
  expect(picked).toEqual([]);
});

test("WidgetPicker Escape after the guard closes it", async () => {
  const { filter, closed } = await mount();
  await arm();
  await key(filter(), "Escape");
  expect(closed()).toBe(1);
});

test("WidgetPicker close button after the guard closes it", async () => {
  const { button, closed } = await mount();
  now += ARMING_MS + 1;
  await act(async () => button("Fermer").click());
  expect(closed()).toBe(1);
});

test("WidgetPicker pointer down on the scrim closes it", async () => {
  const { host, closed } = await mount();
  await act(async () => host.querySelector(".picker-scrim")!.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true })));
  expect(closed()).toBe(1);
});

test("WidgetPicker keys typed in the filter do not reach the app", async () => {
  const { filter } = await mount();
  let reached = 0;
  const onKey = () => void reached++;
  window.addEventListener("keydown", onKey);
  await key(filter(), "Backspace");
  window.removeEventListener("keydown", onKey);
  expect(reached).toBe(0);
});
