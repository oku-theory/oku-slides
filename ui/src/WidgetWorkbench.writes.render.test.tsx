// Panneau Widgets : écritures, leur file d'attente et leurs suites.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { WidgetChange } from "./api";

const { PREVIEW_DELAY_MS } = await import("./debounce");
const { skeleton } = await import("./widgetList");
const { act } = await import("react");
const { action, deferred, editorText, memory, mountWith, pick, stub, submitName, type, unmount, wait } = await import("./test/workbench");

afterEach(unmount);

describe("WidgetWorkbench, après une écriture", () => {
  test("un seul relevé des statuts et un seul aperçu, le parent relisant le pack", async () => {
    const { fake, calls } = memory({ kpi: "k" });
    let version = 0;
    // Comme l'app : l'écriture relit le snapshot, dont la version du pack a changé.
    const { host, rerender } = await mountWith(fake, { onChanged: () => (version += 1) });
    await pick(host, "kpi");
    calls.length = 0;
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(version).toBe(1);
    await rerender({ revision: "r1" });
    await wait(10);
    expect(calls.filter((c) => c === "status")).toEqual(["status"]);
    expect(calls.filter((c) => c === "preview")).toEqual(["preview"]);
  });

  test("les statuts affichés sont ceux que rend l'écriture (mode deck, rendu en échec : rien d'enregistré)", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    stub(fake, "setWidget", async (): Promise<WidgetChange> => ({
      status: { name: "kpi", schema: null, diagnostics: [{ file: "kpi", line: 1, column: 1, error: true, message: "rendu" }] },
      broken: [],
      saved: false,
    }));
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(host.querySelector(".workbench-list .pill")?.className).toBe("pill errors");
  });
});

describe("WidgetWorkbench, file des opérations", () => {
  test("une création attend la suppression en cours", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    const removing = deferred<unknown>();
    stub(fake, "removeWidget", (name: string) => {
      log.push(`remove ${name}`);
      return removing.promise;
    });
    await act(async () => action(host, 2)?.click());
    await act(async () => action(host, 0)?.click());
    await submitName(host, "abc");
    await wait(10);
    expect(log).toEqual(["remove kpi"]);
    await act(async () => removing.resolve([]));
    await wait(10);
    expect(log.map((l) => l.split(":")[0])).toEqual(["remove kpi", "set abc"]);
  });

  test("une écriture attend le renommage en cours, dans l'ordre", async () => {
    const { fake, log } = memory({ kpi: "k", quote: "q" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    const renaming = deferred<unknown>();
    stub(fake, "renameWidget", (from: string, to: string) => {
      log.push(`rename ${from}>${to}`);
      return renaming.promise;
    });
    await act(async () => action(host, 1)?.click());
    await submitName(host, "kpi2");
    // Pendant le renommage, une frappe dans un autre fichier.
    await pick(host, "quote");
    await type(host, "q2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(log).toEqual(["rename kpi>kpi2"]);
    await act(async () => renaming.resolve([]));
    await wait(10);
    expect(log).toEqual(["rename kpi>kpi2", "set quote:q2"]);
  });

  test("deux suppressions de suite : la seconde attend la première", async () => {
    const { fake, log } = memory({ a: "1", b: "2" });
    const { host } = await mountWith(fake);
    const removing: ReturnType<typeof deferred<unknown>>[] = [];
    stub(fake, "removeWidget", (name: string) => {
      log.push(`remove ${name}`);
      const d = deferred<unknown>();
      removing.push(d);
      return d.promise;
    });
    await pick(host, "a");
    await act(async () => action(host, 2)?.click());
    await pick(host, "b");
    await act(async () => action(host, 2)?.click());
    expect(log).toEqual(["remove a"]);
    await act(async () => removing[0]?.resolve([]));
    await wait(10);
    expect(log).toEqual(["remove a", "remove b"]);
  });
});

describe("WidgetWorkbench, non enregistré", () => {
  const unsaved = (host: HTMLElement) =>
    [...host.querySelectorAll(".workbench-file")].filter((b) => b.querySelector(".workbench-unsaved")).map((b) => b.textContent);

  test("une écriture refusée marque le fichier, jusqu'à la prochaine réussie", async () => {
    const { fake } = memory({ kpi: "k", quote: "q" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    const ok = fake.setWidget;
    stub(fake, "setWidget", () => Promise.reject(new Error("trop grand")));
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(unsaved(host)).toEqual(["kpi"]);
    const mark = host.querySelector(".workbench-unsaved");
    expect(mark?.getAttribute("aria-label")).toBe("ui-widgets-unsaved");
    // Le libellé est dit en clair pour le fichier ouvert.
    expect(host.querySelector(".picker-head .caption")?.textContent).toBe("ui-widgets-unsaved");
    // Toujours refusé : la marque reste.
    await type(host, "k3");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(unsaved(host)).toEqual(["kpi"]);
    // Le fichier ouvert change : la marque reste sur kpi, le libellé ne vaut que pour lui.
    await pick(host, "quote");
    expect(unsaved(host)).toEqual(["kpi"]);
    expect(host.querySelector(".picker-head .caption")?.textContent).toBe("");
    await pick(host, "kpi");
    stub(fake, "setWidget", ok);
    await type(host, "k4");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(unsaved(host)).toEqual([]);
    expect(host.querySelector(".picker-head .caption")?.textContent).toBe("");
  });

  test("mode deck, essai refusé (saved: false) : marqué non enregistré, texte tapé gardé à la relecture", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host, rerender } = await mountWith(fake);
    await pick(host, "kpi");
    stub(fake, "setWidget", async (name: string, source: string): Promise<WidgetChange> => {
      log.push(`set ${name}:${source}`);
      return { status: { name, schema: null, diagnostics: [{ file: name, line: 1, column: 1, error: true, message: "rendu" }] }, broken: [], saved: false };
    });
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(log).toEqual(["set kpi:k2"]);
    expect(unsaved(host)).toEqual(["kpi"]);
    // Le pack relu (une autre raison) : la source enregistrée est restée « k », le texte tapé reste.
    await rerender({ revision: "r1" });
    await wait(10);
    expect(editorText(host)).toBe("k2");
    expect(unsaved(host)).toEqual(["kpi"]);
  });

  test("Nouveau refusé (saved: false) : pas de fichier fantôme, l'erreur est dite", async () => {
    const { fake } = memory({ kpi: "k" });
    const errors: string[] = [];
    const { host } = await mountWith(fake, { onError: (m) => errors.push(m) });
    stub(fake, "setWidget", async (name: string): Promise<WidgetChange> => ({
      status: { name, schema: null, diagnostics: [{ file: name, line: 1, column: 1, error: true, message: "rendu cassé" }] },
      broken: [],
      saved: false,
    }));
    await act(async () => action(host, 0)?.click());
    await submitName(host, "abc");
    await wait(10);
    expect([...host.querySelectorAll(".workbench-file")].map((b) => b.textContent)).toEqual(["kpi"]);
    expect(errors).toEqual(["rendu cassé"]);
  });

  test("supprimer un fichier non enregistré efface sa marque", async () => {
    const { fake } = memory({ kpi: "k", quote: "q" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    const ok = fake.setWidget;
    stub(fake, "setWidget", () => Promise.reject(new Error("non")));
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    stub(fake, "setWidget", ok);
    await act(async () => action(host, 2)?.click());
    await wait(10);
    // Recréé sous le même nom : pas de marque héritée.
    await act(async () => action(host, 0)?.click());
    await submitName(host, "kpi");
    await wait(10);
    expect(unsaved(host)).toEqual([]);
  });
});

describe("WidgetWorkbench, nom", () => {
  test("avant le premier statut, Nouveau attend : les noms pris ne sont pas encore connus", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const listing = deferred<unknown>();
    stub(fake, "widgetStatus", () => listing.promise);
    const { host } = await mountWith(fake);
    expect(action(host, 0)?.disabled).toBe(true);
    await act(async () => listing.resolve([{ name: "kpi", schema: null, diagnostics: [] }]));
    expect(action(host, 0)?.disabled).toBe(false);
    await act(async () => action(host, 0)?.click());
    const input = await submitName(host, "kpi");
    expect(input?.getAttribute("aria-invalid")).toBe("true");
    await wait(10);
    expect(log).toEqual([]);
  });

  test("`_commun` : Nouveau le crée s'il manque, avec son squelette ; pas en double, pas par un renommage", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    await act(async () => action(host, 1)?.click());
    expect((await submitName(host, "_commun"))?.getAttribute("aria-invalid")).toBe("true");
    await act(async () => host.querySelector<HTMLInputElement>(".workbench-name input")?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    await act(async () => action(host, 0)?.click());
    await submitName(host, "_commun");
    await wait(10);
    expect(log).toEqual([`set _commun:${skeleton("_commun")}`]);
    await act(async () => action(host, 0)?.click());
    expect((await submitName(host, "_commun"))?.getAttribute("aria-invalid")).toBe("true");
    await wait(10);
    expect(log.length).toBe(1);
  });
});

describe("WidgetWorkbench, frappe pendant un renommage", () => {
  const decl = (name: string, body: string) => `#let widget = widget("${name}", ${body})`;

  /** kpi ouvert, son renommage en kpi2 lancé et retenu ; `release` le laisse aboutir. */
  async function renaming() {
    const { fake, log } = memory({ kpi: decl("kpi", "a") });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    const gate = deferred<void>();
    const rename = fake.renameWidget;
    stub(fake, "renameWidget", async (from: string, to: string) => {
      await gate.promise;
      return rename(from, to);
    });
    await act(async () => action(host, 1)?.click());
    await submitName(host, "kpi2");
    const release = async () => {
      await act(async () => gate.resolve());
      await wait(PREVIEW_DELAY_MS + 50);
    };
    return { host, log, release };
  }

  test("frappe encore en attente à la fin du renommage : écrite sous le nouveau nom, texte gardé", async () => {
    const { host, log, release } = await renaming();
    await type(host, decl("kpi", "tapé"));
    await release();
    expect(log.filter((l) => l.startsWith("set kpi:"))).toEqual([]);
    expect(log.at(-1)).toBe(`set kpi2:${decl("kpi2", "tapé")}`);
    expect(editorText(host)).toBe(decl("kpi2", "tapé"));
  });

  test("frappe partie pendant le renommage : redirigée vers le nouveau nom, texte gardé", async () => {
    const { host, log, release } = await renaming();
    await type(host, decl("kpi", "tapé"));
    await wait(PREVIEW_DELAY_MS + 50);
    await release();
    expect(log.filter((l) => l.startsWith("set kpi:"))).toEqual([]);
    expect(log).toContain(`set kpi2:${decl("kpi2", "tapé")}`);
    expect(editorText(host)).toBe(decl("kpi2", "tapé"));
  });

  test("sans frappe : l'éditeur montre le nom déclaré renommé, sans écriture de plus", async () => {
    const { fake, log } = memory({ kpi: decl("kpi", "a"), quote: "q" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    // La source relue de kpi2 arrive alors que le renommage occupe encore la file : périmée.
    await act(async () => action(host, 1)?.click());
    await submitName(host, "kpi2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(log).toEqual(["rename kpi>kpi2"]);
    expect(editorText(host)).toBe(decl("kpi2", "a"));
  });

  test("renommage refusé : la frappe reste écrite sous l'ancien nom", async () => {
    const { fake, log } = memory({ kpi: decl("kpi", "a") });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    const gate = deferred<void>();
    stub(fake, "renameWidget", async () => {
      await gate.promise;
      throw new Error("utilisé");
    });
    await act(async () => action(host, 1)?.click());
    await submitName(host, "kpi2");
    await type(host, decl("kpi", "tapé"));
    await wait(PREVIEW_DELAY_MS + 50);
    await act(async () => gate.resolve());
    await wait(10);
    expect(log).toEqual([`set kpi:${decl("kpi", "tapé")}`]);
    expect(editorText(host)).toBe(decl("kpi", "tapé"));
  });
});
