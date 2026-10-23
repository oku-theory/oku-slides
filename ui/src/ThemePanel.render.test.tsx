// Rendu du panneau du thème (mode pack) dans un DOM happy-dom.

import { afterAll, afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { Snapshot, Surface, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const ThemePanel = (await import("./ThemePanel")).default;
const { HOST_THEME } = await import("./useHostSettings");
const { setMessages } = await import("./i18n");
const { memory, stub, wait } = await import("./test/widgetApi");

afterAll(resetMessages);
setMessages("en", [
  "ui-theme-picker = Theme\nui-theme-name = Theme name\nui-theme-unsaved = Unsaved theme changes\nui-toolbar-theme-embedded = File theme\n",
]);

const PAPIER_ID = "2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31";
const PRO_ID = "7e1b9c3d-2f4a-4d6e-a8b0-5c7d9e1f3b42";
const OTHER_ID = "11111111-1111-4111-8111-111111111111";
const themes = [
  { id: PAPIER_ID, version: "v1", name: "Papier", namespace: "papier", extends: [], builtin: true, default: true },
  { id: PRO_ID, version: "v1", name: "Pro", namespace: "pro", extends: [], builtin: true, default: false },
];

/** Un instantané en mode pack, le thème `name` en brouillon. */
const snapEditingTheme = (name: string, o: { dirty?: boolean; base?: string; namespace?: string } = {}): Snapshot =>
  ({
    editing: {
      pack: "p",
      base: "b",
      theme: {
        base: { id: o.base ?? PAPIER_ID, version: "v1" },
        namespace: o.namespace ?? "papier",
        name,
        child: true,
        extends: [],
        dirty: o.dirty ?? false,
        revision: "r1",
      },
    },
    themes, history: null, undo_label: null, redo_label: null, slides: [],
  }) as unknown as Snapshot;

/** Une api en mémoire qui note les changements du thème édité. */
function fakeApi() {
  const { fake } = memory({});
  const calls: unknown[][] = [];
  stub(fake, "hostTheme", async () => ({ id: HOST_THEME, place: "thread", live: true, turn: null, messages: [] }) as unknown as Surface);
  stub(fake, "renameEditedTheme", async (name: string) => (calls.push(["rename", name]), snapEditingTheme(name)));
  stub(fake, "setEditedTheme", async (id: string) => (calls.push(["set", id]), snapEditingTheme("Pro", { base: id, namespace: "pro" })));
  return { api: fake as TabApi, calls };
}

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(snap: Snapshot) {
  const { api, calls } = fakeApi();
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  const run = async (p: Promise<Snapshot>) => p.catch(() => null);
  await act(async () => root.render(createElement(ThemePanel, { api, snap, run, say: () => {} })));
  await wait(10);
  return { host, calls };
}

/** Le contrôle dont l'étiquette accessible est `name`. */
function byLabel<T extends HTMLElement>(host: HTMLElement, name: string): T {
  const label = [...host.querySelectorAll("label")].find((l) => l.querySelector(".caption")?.textContent === name);
  const el = label?.querySelector("select, input") ?? host.querySelector(`[aria-label="${name}"]`);
  if (!el) throw new Error(`aucun contrôle « ${name} »`);
  return el as T;
}

async function typeInto(input: HTMLInputElement, text: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, text);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

describe("ThemePanel", () => {
  test("ThemePanel lists themes and renames the edited theme", async () => {
    const { host, calls } = await mount(snapEditingTheme("Papier"));
    const select = byLabel<HTMLSelectElement>(host, "Theme");
    expect(select.value).toBe("papier");
    expect([...select.options].map((o) => o.textContent)).toEqual(["Papier", "Pro"]);
    const name = byLabel<HTMLInputElement>(host, "Theme name");
    expect(name.value).toBe("Papier");
    await typeInto(name, "Maison");
    await act(async () => {
      name.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    });
    await wait(10);
    expect(calls).toEqual([["rename", "Maison"]]);
  });

  test("Entrée puis la sortie du champ : un seul renommage", async () => {
    const { host, calls } = await mount(snapEditingTheme("Papier"));
    const name = byLabel<HTMLInputElement>(host, "Theme name");
    await typeInto(name, "Maison");
    await act(async () => {
      name.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    });
    await act(async () => name.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    await wait(10);
    expect(calls).toEqual([["rename", "Maison"]]);
  });

  test("choisir un autre thème appelle setEditedTheme", async () => {
    const { host, calls } = await mount(snapEditingTheme("Papier"));
    const select = byLabel<HTMLSelectElement>(host, "Theme");
    await act(async () => {
      select.value = "pro";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await wait(10);
    expect(calls).toEqual([["set", PRO_ID]]);
  });

  test("un nom inchangé ou vide n'envoie rien", async () => {
    const { host, calls } = await mount(snapEditingTheme("Papier"));
    const name = byLabel<HTMLInputElement>(host, "Theme name");
    await act(async () => name.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    await typeInto(name, "   ");
    await act(async () => name.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    await wait(10);
    expect(calls).toEqual([]);
    expect(name.value).toBe("Papier");
  });

  test("brouillon modifié : la marque non enregistré", async () => {
    const clean = await mount(snapEditingTheme("Papier"));
    expect(clean.host.querySelector(".unsaved-dot")).toBeNull();
    await act(async () => mounted!.root.unmount());
    mounted!.host.remove();
    mounted = null;
    const dirty = await mount(snapEditingTheme("Papier", { dirty: true }));
    expect(dirty.host.querySelector('.unsaved-dot[aria-label="Unsaved theme changes"]')).not.toBeNull();
  });

  test("ThemePanel with a base absent from the library uses its namespace as menu value", async () => {
    const { host } = await mount(snapEditingTheme("Maison", { base: OTHER_ID, namespace: "maison" }));
    const select = byLabel<HTMLSelectElement>(host, "Theme");
    expect(select.value).toBe("maison");
    expect(select.options[0]!.textContent).toBe("Maison");
    expect(select.options[0]!.disabled).toBe(true);
  });

  test("le panneau porte l'id du tabpanel et la surface host-theme", async () => {
    const { host } = await mount(snapEditingTheme("Papier"));
    const panel = host.querySelector("#theme-panel");
    expect(panel?.getAttribute("role")).toBe("tabpanel");
    expect(panel?.getAttribute("aria-labelledby")).toBe("widget-tab-theme");
    expect(host.querySelector(`[data-surface-id="${HOST_THEME}"]`)).not.toBeNull();
  });
});
