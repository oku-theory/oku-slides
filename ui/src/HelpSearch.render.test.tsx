// Rendu de la recherche de l'aide dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const HelpSearch = (await import("./HelpSearch")).default;
const { t } = await import("./i18n");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const HITS = [
  { slide: "s-grille", title: "Grille, modèles, thème", excerpt: "Grille : 12 × 6 cases" },
  { slide: "s-editer", title: "Éditer à la main", excerpt: "…sur la grille." },
  { slide: "s-fichiers", title: "Fichiers", excerpt: null },
];

async function mount(shown: string[], queries: string[] = []) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const props = {
    search: async (q: string) => {
      queries.push(q);
      return q.includes("zzz") ? [] : HITS;
    },
    onShow: (id: string) => shown.push(id),
    delay: 0,
  };
  await act(async () => root.render(createElement(HelpSearch, props)));
  return host;
}

const input = (host: HTMLElement) => host.querySelector<HTMLInputElement>("input")!;

async function type(host: HTMLElement, text: string) {
  const el = input(host);
  await act(async () => {
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    set.call(el, text);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
  // La recherche part après le délai (0 ici) puis sa réponse arrive.
  await act(async () => new Promise((r) => setTimeout(r, 10)));
}

async function key(host: HTMLElement, k: string) {
  await act(async () => {
    input(host).dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
  });
}

const options = (host: HTMLElement) => [...host.querySelectorAll('[role="option"]')];

describe("HelpSearch", () => {
  test("HelpSearch input is a labelled combobox", async () => {
    const host = await mount([]);
    expect(input(host).getAttribute("role")).toBe("combobox");
    expect(input(host).getAttribute("aria-label")).toBe(t("ui-help-search"));
  });

  test("HelpSearch typing lists the hits with their title", async () => {
    const host = await mount([]);
    await type(host, "grille");
    expect(options(host).map((o) => o.querySelector("strong")?.textContent)).toEqual(["Grille, modèles, thème", "Éditer à la main", "Fichiers"]);
    expect(options(host)[2].querySelector(".caption")).toBeNull();
  });

  test("HelpSearch Enter shows the first hit and closes the list", async () => {
    const shown: string[] = [];
    const host = await mount(shown);
    await type(host, "grille");
    await key(host, "Enter");
    expect(shown).toEqual(["s-grille"]);
    expect(options(host)).toHaveLength(0);
  });

  test("HelpSearch ArrowDown then Enter shows the second hit", async () => {
    const shown: string[] = [];
    const host = await mount(shown);
    await type(host, "grille");
    await key(host, "ArrowDown");
    await key(host, "Enter");
    expect(shown).toEqual(["s-editer"]);
  });

  test("HelpSearch click on a hit shows its slide", async () => {
    const shown: string[] = [];
    const host = await mount(shown);
    await type(host, "grille");
    await act(async () => (options(host)[1] as HTMLElement).dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    expect(shown).toEqual(["s-editer"]);
  });

  test("HelpSearch Escape clears the query", async () => {
    const host = await mount([]);
    await type(host, "grille");
    await key(host, "Escape");
    expect(input(host).value).toBe("");
    expect(options(host)).toHaveLength(0);
  });

  test("HelpSearch without hits says so", async () => {
    const host = await mount([]);
    await type(host, "zzz");
    expect(host.querySelector('[role="status"]')?.textContent).toBe(t("ui-help-search-empty"));
  });

  test("HelpSearch with a blank query does not search", async () => {
    const queries: string[] = [];
    const host = await mount([], queries);
    await type(host, "   ");
    expect(queries).toEqual([]);
    expect(options(host)).toHaveLength(0);
  });
});
