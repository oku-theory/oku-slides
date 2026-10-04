// Pastilles des cas d'exemple d'un widget : choisir, ajouter, renommer, supprimer.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: ExamplePills, currentCase, newCaseName } = await import("./ExamplePills");
const { setMessages } = await import("./i18n");
const { resetMessages } = await import("./test/messages");
const { memory, stub, wait } = await import("./test/widgetApi");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
// Les autres fichiers de la passe attendent les identifiants bruts.
afterEach(resetMessages);
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const snap = (examples: Record<string, string[]>) => ({ examples }) as unknown as Snapshot;

interface Shown { names: string[]; current: string }

/** Monte les pastilles de `kpi` ; `sent` note les appels des cas, `selected` les choix, `toasts` les refus. */
async function mount(fake: TabApi, first: Shown) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  const selected: string[] = [];
  const toasts: string[] = [];
  // Comme `run` de l'App : attend, et dit l'erreur en toast.
  const run = async (p: Promise<Snapshot>) => {
    try {
      return await p;
    } catch (e) {
      toasts.push(String(e));
      return null;
    }
  };
  const render = (p: Shown) =>
    act(async () =>
      root.render(createElement(ExamplePills, { api: fake, widget: "kpi", names: p.names, current: p.current, onSelect: (n: string) => selected.push(n), run })),
    );
  await render(first);
  const pills = () => [...host.querySelectorAll<HTMLButtonElement>(".example-pill")];
  const pill = (name: string) => pills().find((b) => b.textContent === name)!;
  return { host, render, pills, pill, selected, toasts };
}

/** Une api dont les cas sont notés dans `sent` ; chaque appel rend `after`. */
function casesApi(after: Record<string, string[]>) {
  const { fake } = memory({ kpi: "k" });
  const sent: unknown[][] = [];
  for (const key of ["addExample", "renameExample", "removeExample"] as const)
    stub(fake, key, async (...a: unknown[]) => (sent.push([key, ...a]), snap(after)));
  return { fake, sent };
}

const type = (input: HTMLInputElement, value: string) =>
  act(async () => {
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    set.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
const key = (el: Element, k: string) => act(async () => void el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true })));
const dblclick = (el: Element) => act(async () => void el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));

describe("ExamplePills", () => {
  test("une pastille par cas, dans un groupe nommé ; la courante est enfoncée", async () => {
    const { fake } = casesApi({});
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Long" });
    expect(m.host.querySelector("[role=group]")?.getAttribute("aria-label")).toBe("ui-examples");
    expect(m.pills().map((b) => [b.textContent, b.getAttribute("aria-pressed")])).toEqual([
      ["Exemple", "false"],
      ["Long", "true"],
    ]);
  });

  test("un clic choisit le cas", async () => {
    const { fake } = casesApi({});
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Exemple" });
    await act(async () => m.pill("Long").click());
    expect(m.selected).toEqual(["Long"]);
  });

  test("+ Cas ajoute un cas au nom libre, copie du courant, puis le choisit", async () => {
    setMessages("fr", ["ui-examples-new-name = Cas { $n }"]);
    const { fake, sent } = casesApi({ kpi: ["Exemple", "Cas 3", "Cas 2"] });
    const m = await mount(fake, { names: ["Exemple", "Cas 2"], current: "Cas 2" });
    await act(async () => m.host.querySelector<HTMLButtonElement>(".example-add")!.click());
    await wait(10);
    expect(sent).toEqual([["addExample", "kpi", "Cas 3", "Cas 2"]]);
    expect(m.selected).toEqual(["Cas 3"]);
  });

  test("+ Cas refusé : dit en toast, rien n'est choisi", async () => {
    const { fake } = casesApi({});
    stub(fake, "addExample", async () => {
      throw new Error("trop de cas");
    });
    const m = await mount(fake, { names: ["Exemple"], current: "Exemple" });
    await act(async () => m.host.querySelector<HTMLButtonElement>(".example-add")!.click());
    await wait(10);
    expect([m.toasts, m.selected]).toEqual([["Error: trop de cas"], []]);
  });

  test("double-clic : un champ au nom du cas ; Entrée renomme et garde le cas courant", async () => {
    const { fake, sent } = casesApi({ kpi: ["Exemple", "Court"] });
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Long" });
    await dblclick(m.pill("Long"));
    const input = m.host.querySelector<HTMLInputElement>(".example-name input")!;
    expect(input.value).toBe("Long");
    await type(input, "Court");
    await key(input, "Enter");
    await wait(10);
    expect(sent).toEqual([["renameExample", "kpi", "Long", "Court"]]);
    expect(m.selected).toEqual(["Court"]);
    expect(m.host.querySelector(".example-name")).toBeNull();
  });

  test("renommer un autre cas que le courant ne change pas le choix", async () => {
    const { fake, sent } = casesApi({ kpi: ["Exemple", "Court"] });
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Exemple" });
    await dblclick(m.pill("Long"));
    await type(m.host.querySelector<HTMLInputElement>(".example-name input")!, "Court");
    await key(m.host.querySelector(".example-name input")!, "Enter");
    await wait(10);
    expect(sent).toEqual([["renameExample", "kpi", "Long", "Court"]]);
    expect(m.selected).toEqual([]);
  });

  test("Échap annule le renommage", async () => {
    const { fake, sent } = casesApi({});
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Long" });
    await dblclick(m.pill("Long"));
    const input = m.host.querySelector<HTMLInputElement>(".example-name input")!;
    await type(input, "Court");
    await key(input, "Escape");
    expect(sent).toEqual([]);
    expect(m.host.querySelector(".example-name")).toBeNull();
    expect(m.pill("Long")).toBeTruthy();
  });

  test("nom inchangé : rien n'est envoyé", async () => {
    const { fake, sent } = casesApi({});
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Long" });
    await dblclick(m.pill("Long"));
    await key(m.host.querySelector(".example-name input")!, "Enter");
    expect(sent).toEqual([]);
    expect(m.host.querySelector(".example-name")).toBeNull();
  });

  test("renommage refusé (nom pris) : l'erreur de l'hôte en toast, le cas garde son nom", async () => {
    const { fake } = casesApi({});
    stub(fake, "renameExample", async () => {
      throw new Error("nom déjà pris");
    });
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Long" });
    await dblclick(m.pill("Long"));
    const input = m.host.querySelector<HTMLInputElement>(".example-name input")!;
    await type(input, "Exemple");
    await key(input, "Enter");
    await wait(10);
    expect([m.toasts, m.selected]).toEqual([["Error: nom déjà pris"], []]);
    expect(m.pill("Long")).toBeTruthy();
  });

  test("un seul cas : pas de bouton de suppression", async () => {
    const { fake } = casesApi({});
    const m = await mount(fake, { names: ["Exemple"], current: "Exemple" });
    expect(m.host.querySelector(".example-remove")).toBeNull();
  });

  test("supprimer demande confirmation dans la page ; Annuler ne supprime rien", async () => {
    const { fake, sent } = casesApi({});
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Long" });
    const removes = [...m.host.querySelectorAll<HTMLButtonElement>(".example-remove")];
    expect(removes.map((b) => b.getAttribute("aria-label"))).toEqual(["ui-examples-delete-named", "ui-examples-delete-named"]);
    await act(async () => removes[1].click());
    const confirm = m.host.querySelector(".example-confirm")!;
    expect(confirm.textContent).toContain("ui-examples-confirm-delete");
    await act(async () => confirm.querySelector<HTMLButtonElement>(".example-confirm-cancel")!.click());
    expect(sent).toEqual([]);
    expect(m.host.querySelector(".example-confirm")).toBeNull();
  });

  test("Échap dans la confirmation annule", async () => {
    const { fake, sent } = casesApi({});
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Long" });
    await act(async () => m.host.querySelectorAll<HTMLButtonElement>(".example-remove")[1].click());
    await key(m.host.querySelector(".example-confirm")!, "Escape");
    expect(sent).toEqual([]);
    expect(m.host.querySelector(".example-confirm")).toBeNull();
  });

  test("supprimer confirmé : removeExample, puis le premier cas si c'était le courant", async () => {
    const { fake, sent } = casesApi({ kpi: ["Exemple"] });
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Long" });
    await act(async () => m.host.querySelectorAll<HTMLButtonElement>(".example-remove")[1].click());
    await act(async () => m.host.querySelector<HTMLButtonElement>(".example-confirm-ok")!.click());
    await wait(10);
    expect(sent).toEqual([["removeExample", "kpi", "Long"]]);
    expect(m.selected).toEqual(["Exemple"]);
    expect(m.host.querySelector(".example-confirm")).toBeNull();
  });

  test("supprimer un autre cas que le courant ne change pas le choix", async () => {
    const { fake, sent } = casesApi({ kpi: ["Exemple"] });
    const m = await mount(fake, { names: ["Exemple", "Long"], current: "Exemple" });
    await act(async () => m.host.querySelectorAll<HTMLButtonElement>(".example-remove")[1].click());
    await act(async () => m.host.querySelector<HTMLButtonElement>(".example-confirm-ok")!.click());
    await wait(10);
    expect(sent).toEqual([["removeExample", "kpi", "Long"]]);
    expect(m.selected).toEqual([]);
  });
});

describe("currentCase", () => {
  test("le cas gardé s'il existe, sinon le premier", () => {
    expect([currentCase(["Exemple", "Long"], "Long"), currentCase(["Exemple", "Long"], "Absent"), currentCase(["Exemple"], undefined)]).toEqual([
      "Long",
      "Exemple",
      "Exemple",
    ]);
  });
});

describe("newCaseName", () => {
  test("le premier « Cas N » libre, à partir du nombre de cas + 1", () => {
    setMessages("fr", ["ui-examples-new-name = Cas { $n }"]);
    expect([newCaseName(["Exemple"]), newCaseName(["Exemple", "Cas 3", "Cas 4"])]).toEqual(["Cas 2", "Cas 5"]);
  });
});
