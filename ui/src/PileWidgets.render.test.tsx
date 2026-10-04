// Rendu de la section « Widgets » de la pile dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { TabApi, WidgetChange } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { PREVIEW_DELAY_MS } = await import("./debounce");
const { default: PileWidgets } = await import("./PileWidgets");
const { useWidgetFiles } = await import("./useWidgetFiles");
const { skeleton } = await import("./widgetList");
const { deferred, memory, stub, wait } = await import("./test/widgetApi");

type Root = ReturnType<typeof createRoot>;
type Files = ReturnType<typeof useWidgetFiles>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

interface Over {
  confirm?: (m: string) => boolean;
  focus?: number;
}

async function mount(fake: TabApi, over: Over = {}) {
  const opened: string[] = [];
  const removed: string[] = [];
  const errors: string[] = [];
  const renamed: string[] = [];
  const box: { files: Files | null } = { files: null };
  const Probe = ({ focus }: { focus: number }) => {
    const files = useWidgetFiles(fake, "p", "r", {
      changed: () => {},
      error: (m) => errors.push(m),
      renamed: (from, to) => renamed.push(`${from}>${to}`),
    });
    box.files = files;
    return createElement(PileWidgets, {
      files,
      current: null,
      onOpen: (n: string) => opened.push(n),
      onRemoved: (n: string) => removed.push(n),
      focus,
      confirm: over.confirm,
    });
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Probe, { focus: over.focus ?? 0 })));
  const rerender = (focus: number) => act(async () => root.render(createElement(Probe, { focus })));
  return { host, opened, removed, errors, renamed, box, rerender };
}

const rows = (host: HTMLElement) => [...host.querySelectorAll<HTMLButtonElement>(".pile-widgets-file")];
const row = (host: HTMLElement, label: string) => rows(host).find((b) => b.textContent?.includes(label)) as HTMLButtonElement;
const toggle = (host: HTMLElement) => host.querySelector<HTMLButtonElement>(".pile-section-toggle") as HTMLButtonElement;
const nameInput = (host: HTMLElement) => host.querySelector<HTMLInputElement>(".pile-widgets-name input");
const add = (host: HTMLElement) => host.querySelector<HTMLButtonElement>(".pile-section-head .token") as HTMLButtonElement;
async function submitName(host: HTMLElement, value: string) {
  const input = nameInput(host);
  await act(async () => {
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
    set?.call(input, value);
    input?.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () =>
    host.querySelector("form.pile-widgets-name")?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })),
  );
  return input;
}

const escape = (host: HTMLElement) =>
  act(async () => nameInput(host)?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));

describe("PileWidgets", () => {
  test("liste : Commun en tête, pastilles d'état, marque non enregistré", async () => {
    const { fake } = memory({ kpi: "k", _commun: "c" });
    stub(fake, "widgetStatus", async () => [
      { name: "kpi", schema: {}, diagnostics: [] },
      { name: "_commun", schema: null, diagnostics: [{ file: "_commun", line: 1, column: 1, error: true, message: "x" }] },
    ]);
    const { host, box } = await mount(fake);
    expect(rows(host).map((b) => b.querySelector(".pill")?.className)).toEqual(["pill errors", "pill valid"]);
    expect(rows(host)[1].textContent).toContain("kpi");
    expect(host.querySelector(".unsaved-dot")).toBeNull();
    stub(fake, "setWidget", () => Promise.reject(new Error("trop grand")));
    await act(async () => box.files?.edit("kpi", "k2"));
    await wait(PREVIEW_DELAY_MS + 50);
    expect(row(host, "kpi").querySelector(".unsaved-dot")).not.toBeNull();
    expect(row(host, "ui-widgets-common").querySelector(".unsaved-dot")).toBeNull();
  });

  test("clic sur un fichier : onOpen(nom)", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host, opened } = await mount(fake);
    await act(async () => row(host, "kpi").click());
    expect(opened).toEqual(["kpi"]);
  });

  test("+ puis un nom libre : créé avec son squelette, onOpen(nom)", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host, opened } = await mount(fake);
    await act(async () => add(host).click());
    await submitName(host, "abc");
    await wait(10);
    expect(log).toEqual([`set abc:${skeleton("abc")}`]);
    expect(opened).toEqual(["abc"]);
  });

  test("+ avec un nom pris : aria-invalid, rien n'est écrit", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host, opened } = await mount(fake);
    await act(async () => add(host).click());
    const input = await submitName(host, "kpi");
    expect(input?.getAttribute("aria-invalid")).toBe("true");
    await wait(10);
    expect(log).toEqual([]);
    expect(opened).toEqual([]);
    expect(nameInput(host)).not.toBeNull();
  });

  test("avant le premier statut, + est désactivé", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const listing = deferred<unknown>();
    stub(fake, "widgetStatus", () => listing.promise);
    const { host } = await mount(fake);
    expect(add(host).disabled).toBe(true);
    await act(async () => listing.resolve([{ name: "kpi", schema: null, diagnostics: [] }]));
    expect(add(host).disabled).toBe(false);
    await act(async () => add(host).click());
    expect((await submitName(host, "kpi"))?.getAttribute("aria-invalid")).toBe("true");
    await wait(10);
    expect(log).toEqual([]);
  });

  test("`_commun` : + le crée s'il manque ; pas en double ; pas par un renommage", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => row(host, "kpi").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect((await submitName(host, "_commun"))?.getAttribute("aria-invalid")).toBe("true");
    await escape(host);
    await act(async () => add(host).click());
    await submitName(host, "_commun");
    await wait(10);
    expect(log).toEqual([`set _commun:${skeleton("_commun")}`]);
    await act(async () => add(host).click());
    expect((await submitName(host, "_commun"))?.getAttribute("aria-invalid")).toBe("true");
    await wait(10);
    expect(log.length).toBe(1);
  });

  test("double-clic sur un nom : champ ; valider → renameWidget", async () => {
    const { fake, log, } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => row(host, "kpi").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(nameInput(host)?.value).toBe("kpi");
    await submitName(host, "kpi2");
    await wait(10);
    expect(log).toEqual(["rename kpi>kpi2"]);
    expect(nameInput(host)).toBeNull();
  });

  test("F2 sur le bouton : même champ", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => row(host, "kpi").dispatchEvent(new KeyboardEvent("keydown", { key: "F2", bubbles: true, cancelable: true })));
    expect(nameInput(host)?.value).toBe("kpi");
  });

  test("double-clic sur Commun : pas de champ", async () => {
    const { fake } = memory({ _commun: "c" });
    const { host } = await mount(fake);
    await act(async () => row(host, "ui-widgets-common").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(nameInput(host)).toBeNull();
    await act(async () => row(host, "ui-widgets-common").dispatchEvent(new KeyboardEvent("keydown", { key: "F2", bubbles: true })));
    expect(nameInput(host)).toBeNull();
  });

  test("Échap abandonne le champ et rend le focus à la ligne", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => row(host, "kpi").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    await escape(host);
    expect(nameInput(host)).toBeNull();
    expect(document.activeElement).toBe(row(host, "kpi"));
    expect(log).toEqual([]);
  });

  test("valider un renommage rend le focus à la ligne renommée", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => row(host, "kpi").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    await submitName(host, "kpi2");
    await wait(10);
    expect(document.activeElement).toBe(row(host, "kpi2"));
  });

  test("perdre le focus ne perd pas le nom saisi", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => add(host).click());
    const input = nameInput(host);
    await submitName(host, "");
    await act(async () => {
      const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
      set?.call(input, "abc");
      input?.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => {
      input?.blur();
      input?.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    });
    expect(nameInput(host)?.value).toBe("abc");
  });

  test("supprimer : confirmation refusée → rien ; acceptée → removeWidget, onRemoved(nom)", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const asked: string[] = [];
    let answer = false;
    const { host, removed } = await mount(fake, { confirm: (m) => (asked.push(m), answer) });
    const button = () => host.querySelector<HTMLButtonElement>(".pile-widgets-remove") as HTMLButtonElement;
    await act(async () => button().click());
    await wait(10);
    expect(asked).toEqual(["ui-widgets-confirm-delete"]);
    expect(log).toEqual([]);
    expect(removed).toEqual([]);
    answer = true;
    await act(async () => button().click());
    await wait(10);
    expect(log).toEqual(["remove kpi"]);
    expect(removed).toEqual(["kpi"]);
  });

  test("suppression refusée par la session (modèle qui l'utilise) : erreur dite, onRemoved non appelé", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "removeWidget", () => Promise.reject(new Error("utilisé par un modèle")));
    const { host, removed, errors } = await mount(fake, { confirm: () => true });
    await act(async () => host.querySelector<HTMLButtonElement>(".pile-widgets-remove")?.click());
    await wait(10);
    expect(errors.length).toBe(1);
    expect(errors[0]).toContain("utilisé par un modèle");
    expect(removed).toEqual([]);
  });

  test("création refusée : onOpen non appelé", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "setWidget", async (name: string): Promise<WidgetChange> => ({
      status: { name, schema: null, diagnostics: [{ file: name, line: 1, column: 1, error: true, message: "rendu cassé" }] },
      broken: [],
      saved: false,
    }));
    const { host, opened, errors } = await mount(fake);
    await act(async () => add(host).click());
    await submitName(host, "abc");
    await wait(10);
    expect(opened).toEqual([]);
    expect(errors).toEqual(["rendu cassé"]);
    expect(rows(host).map((b) => b.textContent)).toEqual(["kpi"]);
  });

  test("pack vide : texte ui-widgets-empty", async () => {
    const { fake } = memory({});
    const { host } = await mount(fake);
    expect(host.querySelector(".pile-widgets .caption")?.textContent).toBe("ui-widgets-empty");
  });

  test("focus change : le titre de la section prend le focus", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host, rerender } = await mount(fake);
    expect(document.activeElement).not.toBe(toggle(host));
    await rerender(1);
    expect(document.activeElement).toBe(toggle(host));
  });

  test("focus change : une section repliée se rouvre", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host, rerender } = await mount(fake);
    await act(async () => toggle(host).click());
    expect(rows(host).length).toBe(0);
    await rerender(1);
    expect(rows(host).length).toBe(1);
  });

  test("repliée : + rouvre la section sur le champ du nom", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => toggle(host).click());
    await act(async () => add(host).click());
    expect(nameInput(host)).not.toBeNull();
  });

  test("repliée : le bouton Nouveau widget reste là", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => toggle(host).click());
    expect(add(host)).not.toBeNull();
  });

  test("le bouton Supprimer est atteignable au clavier", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    const button = host.querySelector<HTMLButtonElement>(".pile-widgets-remove") as HTMLButtonElement;
    expect(button.hasAttribute("hidden")).toBe(false);
    expect(button.tabIndex).not.toBe(-1);
    expect(button.getAttribute("tabindex")).toBeNull();
  });
});

/** Saisit `value` dans le champ du nom, comme une frappe, sans valider. */
async function typeName(host: HTMLElement, value: string) {
  const input = nameInput(host);
  await act(async () => {
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
    set?.call(input, value);
    input?.dispatchEvent(new Event("input", { bubbles: true }));
  });
  return nameInput(host) as HTMLInputElement;
}

/** Le message relié au champ par `aria-describedby`, s'il y en a un. */
const issue = (host: HTMLElement) => {
  const id = nameInput(host)?.getAttribute("aria-describedby");
  return id ? host.querySelector(`#${CSS.escape(id)}`)?.textContent ?? null : null;
};

describe("PileWidgets, nom saisi", () => {
  test("une majuscule tapée devient une minuscule, sans message", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => add(host).click());
    const input = await typeName(host, "Carte");
    expect(input.value).toBe("carte");
    expect(input.getAttribute("aria-invalid")).toBe("false");
    expect(issue(host)).toBeNull();
  });

  test("accents et espaces deviennent un nom valide, créé sous ce nom", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host, opened } = await mount(fake);
    await act(async () => add(host).click());
    await submitName(host, "Météo du jour");
    await wait(10);
    expect(log).toEqual([`set meteo-du-jour:${skeleton("meteo-du-jour")}`]);
    expect(opened).toEqual(["meteo-du-jour"]);
  });

  test("un nom pris affiche son message, relié au champ", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => add(host).click());
    const input = await typeName(host, "Kpi");
    expect(input.getAttribute("aria-invalid")).toBe("true");
    expect(issue(host)).toBe("ui-widgets-name-taken");
  });

  test("un nom qui ne commence pas par une lettre affiche son message", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => add(host).click());
    await typeName(host, "2d");
    expect(issue(host)).toBe("ui-widgets-name-letter");
  });

  test("renommer normalise aussi la saisie", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mount(fake);
    await act(async () => row(host, "kpi").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    const input = await typeName(host, "Grand KPI");
    expect(input.value).toBe("grand-kpi");
  });

  test("pack vide : le texte du pack vide est masqué pendant la saisie d'un nom", async () => {
    const { fake } = memory({});
    const { host } = await mount(fake);
    expect(host.textContent).toContain("ui-widgets-empty");
    await act(async () => add(host).click());
    expect(host.textContent).not.toContain("ui-widgets-empty");
  });
});
