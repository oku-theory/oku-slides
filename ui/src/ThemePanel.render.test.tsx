// The theme panel of a theme tab, rendered in a happy-dom DOM.

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
  "ui-theme-name = Theme name\nui-theme-unsaved = Unsaved theme changes\nui-tab-theme = Theme\n",
]);

const PAPIER_ID = "2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31";
const PRO_ID = "7e1b9c3d-2f4a-4d6e-a8b0-5c7d9e1f3b42";
const themes = [
  { id: PAPIER_ID, version: "v1", name: "Papier", namespace: "papier", extends: [], builtin: true, default: true },
  { id: PRO_ID, version: "v1", name: "Pro", namespace: "pro", extends: [], builtin: true, default: false },
];

/** A theme tab's snapshot, theme `name` as its draft. */
const snapEditingTheme = (name: string, o: { dirty?: boolean; base?: string; namespace?: string } = {}): Snapshot =>
  ({
    editing: null,
    theming: {
      theme: {
        base: { id: o.base ?? PAPIER_ID, version: "v1" },
        namespace: o.namespace ?? "papier",
        name,
        child: true,
        extends: [],
        dirty: o.dirty ?? false,
        revision: "r1",
      },
      session_version: null,
      read_only: false,
      leased: true,
      display_name: name,
    },
    themes, history: null, undo_label: null, redo_label: null, slides: [],
  }) as unknown as Snapshot;

/** Une api en mémoire qui note les changements du thème édité. */
function fakeApi() {
  const { fake } = memory({});
  const calls: unknown[][] = [];
  stub(fake, "hostTheme", async () => ({ id: HOST_THEME, place: "thread", live: true, turn: null, messages: [] }) as unknown as Surface);
  stub(fake, "renameEditedTheme", async (name: string) => (calls.push(["rename", name]), snapEditingTheme(name)));
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
  test("ThemePanel renames the edited theme and offers no base picker", async () => {
    const { host, calls } = await mount(snapEditingTheme("Papier"));
    expect(host.querySelector("select")).toBeNull();
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

  test("ThemePanel is a region labelled by its title, with the host-theme surface", async () => {
    const { host } = await mount(snapEditingTheme("Papier"));
    const panel = host.querySelector("#theme-panel");
    const title = panel?.getAttribute("aria-labelledby");
    expect([panel?.getAttribute("role"), title && host.querySelector(`#${title}`)?.textContent]).toEqual(["region", "Theme"]);
    expect(host.querySelector(`[data-surface-id="${HOST_THEME}"]`)).not.toBeNull();
  });

  test("ThemePanel on a base outside the library shows the draft's name", async () => {
    const { host } = await mount(snapEditingTheme("Reçu", { base: "0b1c2d3e-0000-4000-8000-000000000009", namespace: "recu" }));
    expect(byLabel<HTMLInputElement>(host, "Theme name").value).toBe("Reçu");
    expect(host.querySelector("select")).toBeNull();
  });

  test("ThemePanel outside a theme tab shows no name field", async () => {
    const { host } = await mount({ ...snapEditingTheme("Papier"), theming: null } as Snapshot);
    expect(host.querySelector("input")).toBeNull();
  });
});
