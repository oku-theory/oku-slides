// Fiche, tiroir et fenêtre de l'agent, avec leurs touches rapides, dans un DOM happy-dom.

import "../test/dom";
import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";
import type { Surface } from "../api";
import { realSurface as real } from "../a2ui/fixtures/agentSurfaces";
import type { QuickAction } from "./quick";
import type { QuickTarget } from "./useQuickKeys";

// Voir ../a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AgentSheet } = await import("./AgentSheet");
const { VariantDrawer } = await import("./VariantDrawer");
const { ConfirmModal } = await import("./ConfirmModal");
const { useQuickKeys, ARMING_MS } = await import("./useQuickKeys");
const { AgentThread } = await import("./AgentThread");
const { routeSurfaces } = await import("./surfaces");
const { fromA2ui } = await import("./quick");

const COMPONENT = { sheet: AgentSheet, drawer: VariantDrawer, modal: ConfirmModal };

/** Comme App : la surface affichée, et les touches rapides branchées sur elle (une question du dock : dans le fil). */
function Harness({ target, send }: { target: QuickTarget; send: (s: Surface, a: QuickAction) => void }) {
  useQuickKeys(target, true, send);
  if (target.kind === "dock") {
    const { turns, loose, dock } = routeSurfaces([target.surface], null);
    return createElement(AgentThread, {
      state: "working",
      turns,
      loose,
      dock,
      onAction: (s, a) => send(s, fromA2ui(a)),
      onNewConversation: () => {},
      onError: () => {},
    });
  }
  return createElement(COMPONENT[target.kind], { key: target.surface.id, surface: target.surface, onAction: send, onError: () => {} });
}

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

/** Monte la surface dans un parent qui compte les pointeurs reçus ; renvoie les actions envoyées. */
async function mount(kind: QuickTarget["kind"], surface: Surface) {
  const sent: [string, QuickAction][] = [];
  let parentPointers = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const send = (s: Surface, a: QuickAction) => sent.push([s.id, a]);
  const render = (target: QuickTarget) =>
    act(async () => root.render(createElement("div", { onPointerDown: () => parentPointers++ }, createElement(Harness, { target, send }))));
  await render({ kind, surface });
  // Le fil charge ses surfaces à la demande (lazy) : laisser Suspense les résoudre.
  await act(async () => new Promise((r) => setTimeout(r, 0)));
  return { host, sent, parentPointers: () => parentPointers, rerender: render };
}

/** Une touche, par sa position (`code`) et son nom (`key`), sur `target` (sinon le corps du document). */
async function press(key: string, code: string, target: Element = document.body, mods: { metaKey?: boolean } = {}) {
  const e = new KeyboardEvent("keydown", { key, code, bubbles: true, cancelable: true, ...mods });
  await act(async () => {
    target.dispatchEvent(e);
  });
  return e;
}

describe("fiche d'une question ancrée", () => {
  test("ne prend pas le focus ; une touche tapée n'importe où choisit (2 en AZERTY)", async () => {
    const { host, sent } = await mount("sheet", real("ask-1"));
    expect(host.contains(document.activeElement)).toBe(false);
    await press("é", "Digit2");
    expect(sent).toEqual([["ask-1", { name: "choose", context: { choice: "b" } }]]);
  });

  test("Échap abandonne", async () => {
    const { sent } = await mount("sheet", real("ask-1"));
    await press("Escape", "Escape");
    expect(sent).toEqual([["ask-1", { name: "dismiss", context: {} }]]);
  });

  test("une touche tapée dans un champ reste au champ", async () => {
    const { sent } = await mount("sheet", real("ask-1"));
    const field = document.createElement("input");
    document.body.append(field);
    try {
      await press("2", "Digit2", field);
      expect(sent).toEqual([]);
    } finally {
      field.remove();
    }
  });

  test("⌘2 et ⌘S ne sont pas pris : les raccourcis de l'app restent", async () => {
    const { sent } = await mount("sheet", real("ask-1"));
    const e = await press("s", "KeyS", document.body, { metaKey: true });
    await press("2", "Digit2", document.body, { metaKey: true });
    expect(e.defaultPrevented).toBe(false);
    expect(sent).toEqual([]);
  });

  test("un évènement déjà traité (Échap d'une dictée) est ignoré", async () => {
    const { sent } = await mount("sheet", real("ask-1"));
    const e = new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true, cancelable: true });
    e.preventDefault();
    await act(async () => {
      document.body.dispatchEvent(e);
    });
    expect(sent).toEqual([]);
  });

  test("Entrée sur un bouton qui a le focus reste son clic", async () => {
    const { host, sent } = await mount("sheet", real("ask-4"));
    const keep = [...host.querySelectorAll("button")].find((b) => b.textContent?.includes("original")) as HTMLElement;
    expect(keep).toBeDefined();
    await press("Enter", "Enter", keep);
    expect(sent).toEqual([]);
  });

  test("E va au champ de retouche", async () => {
    const { host } = await mount("sheet", real("ask-4"));
    await press("e", "KeyE");
    expect(document.activeElement).toBe(host.querySelector("input"));
  });

  test("un clic envoie l'action A2UI du bouton", async () => {
    const { host, sent } = await mount("sheet", real("ask-4"));
    const pick = host.querySelector("button[data-primary]") as HTMLElement;
    await act(async () => pick.click());
    expect(sent).toEqual([["ask-4", { name: "rewrite", context: { kept: 0, text: "Nos paris pour 2027" } }]]);
  });

  test("un clic dans la fiche ne désélectionne pas l'élément dessous", async () => {
    const { host, parentPointers } = await mount("sheet", real("ask-1"));
    await act(async () => {
      host.querySelector('[role="dialog"]')?.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    });
    expect(parentPointers()).toBe(0);
  });
});

describe("question du dock", () => {
  test("1–4 choisissent l'option correspondante, où que soit le curseur (2 en AZERTY)", async () => {
    const { sent } = await mount("dock", real("ask-8"));
    await press("é", "Digit2");
    expect(sent).toEqual([["ask-8", { name: "choose", context: { choice: "b" } }]]);
  });

  test("une touche tapée dans un champ texte reste au champ", async () => {
    const { sent } = await mount("dock", real("ask-8"));
    const field = document.createElement("textarea");
    document.body.append(field);
    try {
      await press("1", "Digit1", field);
      await press("Enter", "Enter", field);
      expect(sent).toEqual([]);
    } finally {
      field.remove();
    }
  });

  test("Entrée sur l'option qui a le focus la choisit, une seule fois", async () => {
    const { host, sent } = await mount("dock", real("ask-8"));
    const chip = host.querySelectorAll<HTMLButtonElement>(".agent-dock .chip")[1];
    chip.focus();
    // Entrée sur un bouton : le navigateur le clique ; la touche rapide s'efface.
    const e = await press("Enter", "Enter", chip);
    expect(e.defaultPrevented).toBe(false);
    await act(async () => chip.click());
    expect(sent).toEqual([["ask-8", { name: "choose", context: { choice: "b" } }]]);
  });

  test("Échap reste à l'app (désélectionner) : la question ne s'abandonne que par son bouton", async () => {
    const { sent } = await mount("dock", real("ask-8"));
    const e = await press("Escape", "Escape");
    expect(e.defaultPrevented).toBe(false);
    expect(sent).toEqual([]);
  });
});

describe("tiroir des variantes", () => {
  test("2 choisit la seconde variante", async () => {
    const { host, sent } = await mount("drawer", real("show-6"));
    expect(host.querySelectorAll(".slide-thumb img")).toHaveLength(2);
    await press("2", "Digit2");
    expect(sent).toEqual([["show-6", { name: "variant", context: { index: 1 } }]]);
  });
});

describe("fenêtre de confirmation", () => {
  test("modale nommée ; prend le focus", async () => {
    const { host } = await mount("modal", real("ask-2"));
    const dialog = host.querySelector('[role="alertdialog"]');
    expect(dialog?.getAttribute("aria-modal")).toBe("true");
    expect(dialog?.getAttribute("aria-label")).toBe("ui-agent-confirm");
    expect(document.activeElement).toBe(dialog);
  });

  test("Entrée et les chiffres ne confirment pas : il faut un geste délibéré", async () => {
    const { sent } = await mount("modal", real("perm-3"));
    now += ARMING_MS + 1;
    await press("Enter", "Enter");
    await press("1", "Digit1");
    expect(sent).toEqual([]);
  });

  test("Échap annule, mais pas dans les premières 500 ms", async () => {
    const { sent } = await mount("modal", real("ask-2"));
    await press("Escape", "Escape");
    expect(sent).toEqual([]);
    now += ARMING_MS + 1;
    await press("Escape", "Escape");
    expect(sent).toEqual([["ask-2", { name: "dismiss", context: {} }]]);
  });

  test("un clic dans les premières 500 ms est ignoré ; ensuite il répond", async () => {
    const { host, sent } = await mount("modal", real("ask-2"));
    const yes = host.querySelector("button[data-primary]") as HTMLElement;
    await act(async () => yes.click());
    expect(sent).toEqual([]);
    now += ARMING_MS + 1;
    await act(async () => yes.click());
    expect(sent).toEqual([["ask-2", { name: "confirm", context: { confirmed: true } }]]);
  });

  test("Tab tourne à l'intérieur de la fenêtre", async () => {
    const { host } = await mount("modal", real("ask-2"));
    const buttons = [...host.querySelectorAll<HTMLElement>(".agent-modal button")];
    buttons.at(-1)?.focus();
    await press("Tab", "Tab", buttons.at(-1));
    expect(document.activeElement).toBe(buttons[0]);
  });

  test("une fenêtre qui en remplace une autre prend le focus", async () => {
    const { host, rerender } = await mount("modal", real("ask-2"));
    const first = host.querySelector('[role="alertdialog"]');
    await rerender({ kind: "modal", surface: real("perm-3") });
    const second = host.querySelector('[role="alertdialog"]');
    expect(second).not.toBe(first);
    expect(document.activeElement).toBe(second);
  });

  test("rend le focus à l'élément qui l'avait en se fermant", async () => {
    const before = document.createElement("button");
    document.body.append(before);
    try {
      before.focus();
      await mount("modal", real("ask-2"));
      expect(document.activeElement).not.toBe(before);
      const m = mounted;
      mounted = null;
      await act(async () => m?.root.unmount());
      m?.host.remove();
      expect(document.activeElement).toBe(before);
    } finally {
      before.remove();
    }
  });
});

describe("réglages du fil", () => {
  test("une surface show-… vivante rend dans le dock, ses touches ignorées de l'app", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    const live = { ...real("show-6"), place: "thread", live: true, turn: 1 } as Surface;
    const { turns, loose, dock } = routeSurfaces([live], null);
    await act(async () => {
      root.render(createElement(AgentThread, { state: "idle", turns, loose, dock, onAction: () => {}, onNewConversation: () => {}, onError: () => {} }));
    });
    await act(async () => new Promise((r) => setTimeout(r, 0)));
    expect(host.querySelector('.agent-dock.agent-keys [data-surface-id="show-6"]')).not.toBeNull();
  });
});
