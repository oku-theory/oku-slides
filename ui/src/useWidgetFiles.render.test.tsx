// État partagé des fichiers de widgets : statuts, frappes, file des opérations.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { PackKind, TabApi, WidgetChange } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { PREVIEW_DELAY_MS } = await import("./debounce");
const { skeleton } = await import("./widgetList");
const { useWidgetFiles } = await import("./useWidgetFiles");
const { deferred, memory, stub, wait } = await import("./test/widgetApi");

type Files = ReturnType<typeof useWidgetFiles>;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

interface Out { files: Files | null; errors: string[]; changed: string[]; renamed: string[] }

/** Monte le hook ; `render` le relance (pack, revision, rappels). */
async function mount(fake: TabApi, kind: PackKind = "widgets") {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: Out = { files: null, errors: [], changed: [], renamed: [] };
  function Probe({ pack, rev, tag }: { pack: string | null; rev: string; tag: string }) {
    out.files = useWidgetFiles(fake, pack, kind, rev, {
      changed: () => out.changed.push(tag),
      error: (m) => out.errors.push(m),
      renamed: (f, t) => out.renamed.push(`${f}>${t}`),
    });
    return null;
  }
  const render = (pack: string | null = "p", rev = "r", tag = "a") =>
    act(async () => root.render(createElement(Probe, { pack, rev, tag })));
  await render();
  return { out, render, unmount: () => act(async () => root.unmount()) };
}

const typed = async (out: Out, name: string, text: string) => {
  await act(async () => out.files!.edit(name, text));
};
const later = () => wait(PREVIEW_DELAY_MS + 50);
const decl = (n: string, b: string) => `#let widget = widget("${n}", ${b})`;

/** `key` de l'api retenu jusqu'à `release`. */
function held(fake: TabApi, key: "setWidget" | "removeWidget" | "renameWidget") {
  const gate = deferred<void>();
  const real = (fake as unknown as Record<string, (...a: unknown[]) => Promise<unknown>>)[key];
  stub(fake, key, async (...a: never[]) => {
    await gate.promise;
    return real(...a);
  });
  return { release: () => act(async () => gate.resolve()) };
}

describe("useWidgetFiles, statuts", () => {
  // 1
  test("lit les statuts, _commun en tête, avec leur revision", async () => {
    const { fake } = memory({ kpi: "k", _commun: "c" });
    const { out } = await mount(fake);
    expect(out.files?.statuses?.map((s) => s.name)).toEqual(["_commun", "kpi"]);
    expect(out.files?.statusesFor).toBe("r");
  });

  // 2
  test("hors mode pack (pack null) : rien n'est lu", async () => {
    const { fake, calls } = memory({ kpi: "k" });
    const { out, render } = await mount(fake);
    calls.length = 0;
    await render(null);
    expect(out.files?.statuses).toBeNull();
    expect(calls).toEqual([]);
  });

  // 2b
  test("une relecture ancienne arrivée après une plus récente est ignorée", async () => {
    const { fake } = memory({ kpi: "k" });
    const slow = deferred<unknown>();
    let first = true;
    const status = fake.widgetStatus;
    stub(fake, "widgetStatus", () => {
      if (!first) return status();
      first = false;
      return slow.promise;
    });
    const { out, render } = await mount(fake);
    await render("p", "r2");
    await act(async () => slow.resolve([]));
    expect(out.files?.statuses?.map((s) => s.name)).toEqual(["kpi"]);
    expect(out.files?.statusesFor).toBe("r2");
  });

  // 2c
  test("un autre pack : frappe en attente annulée, tables remises à zéro", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out, render } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    expect(out.files!.checked.size).toBe(1);
    expect(out.files!.writes.size).toBe(1);
    log.length = 0;
    await typed(out, "kpi", "k3");
    await render("autre");
    await later();
    expect(log).toEqual([]);
    expect(out.files!.checked.size).toBe(0);
    expect(out.files!.writes.size).toBe(0);
    expect(out.files!.unsaved.size).toBe(0);
  });

  // 2d
  test("une relecture en vol quand le pack devient null laisse les statuts à null", async () => {
    const { fake } = memory({ kpi: "k" });
    const slow = deferred<unknown>();
    stub(fake, "widgetStatus", () => slow.promise);
    const { out, render } = await mount(fake);
    await render(null);
    await act(async () => slow.resolve([{ name: "kpi", schema: null, diagnostics: [] }]));
    expect(out.files?.statuses).toBeNull();
    expect(out.files?.statusesFor).toBeNull();
  });

  // 2e
  test("une écriture en vol pendant un changement de pack : ni statuts fusionnés, ni changed", async () => {
    const { fake } = memory({ kpi: "k" });
    const { out, render } = await mount(fake);
    const h = held(fake, "setWidget");
    await typed(out, "kpi", "k2");
    await later();
    await render("autre");
    await h.release();
    await wait(10);
    expect(out.changed).toEqual([]);
    expect(out.files!.statuses?.map((s) => s.name)).toEqual(["kpi"]);
    expect(out.files!.checked.size).toBe(0);
    expect(out.files!.writes.size).toBe(0);
  });
});

describe("useWidgetFiles, frappes et file", () => {
  // 3
  test("frappes dans deux fichiers : chacune écrite, dans l'ordre d'envoi", async () => {
    const { fake, log } = memory({ a: "a", b: "b" });
    const { out } = await mount(fake);
    const h = held(fake, "setWidget");
    await typed(out, "a", "a1");
    await typed(out, "b", "b1");
    await act(async () => out.files!.flush());
    await wait(10);
    expect(log).toEqual([]);
    await h.release();
    await wait(10);
    expect(log).toEqual(["set a:a1", "set b:b1"]);
  });

  // 4
  test("deux écritures du même fichier : la seconde ne part qu'après la première, même lente", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const gates = [deferred<void>(), deferred<void>()];
    let n = 0;
    stub(fake, "setWidget", async (name: string, source: string): Promise<WidgetChange> => {
      log.push(`set ${name}:${source}`);
      await gates[n++].promise;
      return { status: { name, schema: null, diagnostics: [] }, broken: [], saved: true };
    });
    await typed(out, "kpi", "k1");
    await later();
    await typed(out, "kpi", "k2");
    await later();
    expect(log).toEqual(["set kpi:k1"]);
    await act(async () => gates[0].resolve());
    await wait(10);
    expect(log).toEqual(["set kpi:k1", "set kpi:k2"]);
    await act(async () => gates[1].resolve());
  });

  // 5
  test("une écriture refusée marque le fichier et dit l'erreur ; la suivante réussie efface la marque", async () => {
    const { fake } = memory({ kpi: "k" });
    const set = fake.setWidget;
    stub(fake, "setWidget", async () => { throw new Error("non"); });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    expect(out.files!.unsaved.has("kpi")).toBe(true);
    expect(out.errors).toEqual(["non"]);
    stub(fake, "setWidget", set);
    await typed(out, "kpi", "k3");
    await later();
    expect(out.files!.unsaved.has("kpi")).toBe(false);
  });

  // 6
  test("essai refusé (saved: false) : non enregistré, statuts de l'essai, pas de changed", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "setWidget", async (): Promise<WidgetChange> => ({
      status: { name: "kpi", schema: null, diagnostics: [{ file: "kpi", line: 1, column: 1, error: true, message: "rendu" }] },
      broken: [],
      saved: false,
    }));
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    expect(out.files!.unsaved.has("kpi")).toBe(true);
    expect(out.files!.statuses?.[0].diagnostics.map((d) => d.message)).toEqual(["rendu"]);
    expect(out.files!.checked.get("kpi")).toBe("k2");
    expect(out.changed).toEqual([]);
  });

  // 7
  test("une écriture enregistrée compte dans writes et appelle changed", async () => {
    const { fake } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    expect(out.files!.writes.get("kpi")).toBe(1);
    expect(out.changed).toEqual(["a"]);
  });

  // 8
  test("remove pendant le délai : la frappe en attente ne recrée pas le fichier", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await act(async () => { await out.files!.remove("kpi"); });
    await later();
    expect(log).toEqual(["remove kpi"]);
  });

  // 9
  test("remove attend l'écriture déjà partie", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const h = held(fake, "setWidget");
    await typed(out, "kpi", "k2");
    await later();
    let removing: Promise<boolean> | null = null;
    await act(async () => { removing = out.files!.remove("kpi"); });
    await wait(10);
    expect(log).toEqual([]);
    await h.release();
    await act(async () => { await removing; });
    expect(log).toEqual(["set kpi:k2", "remove kpi"]);
  });

  // 10
  test("create attend la suppression en cours", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const h = held(fake, "removeWidget");
    let created: Promise<boolean> | null = null;
    await act(async () => { void out.files!.remove("kpi"); created = out.files!.create("abc"); });
    await wait(10);
    expect(log).toEqual([]);
    await h.release();
    await act(async () => { await created; });
    expect(log).toEqual(["remove kpi", `set abc:${skeleton("abc", "widgets")}`]);
  });

  test("useWidgetFiles create in a layout pack writes a slide widget skeleton", async () => {
    const { fake, log } = memory({});
    const { out } = await mount(fake, "layouts");
    await act(async () => { await out.files!.create("hero"); });
    expect(log).toEqual([`set hero:${skeleton("hero", "layouts")}`]);
  });

  // 11
  test("deux suppressions de suite : la seconde ne part qu'après la première", async () => {
    const { fake, log } = memory({ a: "a", b: "b" });
    const { out } = await mount(fake);
    const remove = fake.removeWidget;
    const gates = [deferred<void>(), deferred<void>()];
    let n = 0;
    stub(fake, "removeWidget", async (name: string) => {
      log.push(`remove ${name}`);
      await gates[n++].promise;
      return remove(name).then((l) => (log.pop(), l));
    });
    await act(async () => { void out.files!.remove("a"); void out.files!.remove("b"); });
    await wait(10);
    expect(log).toEqual(["remove a"]);
    await act(async () => gates[0].resolve());
    await wait(10);
    expect(log).toEqual(["remove a", "remove b"]);
    await act(async () => gates[1].resolve());
  });

  // 12
  test("une écriture d'un autre fichier attend le renommage en cours", async () => {
    const { fake, log } = memory({ kpi: decl("kpi", "a"), quote: "q" });
    const { out } = await mount(fake);
    const h = held(fake, "renameWidget");
    await act(async () => { void out.files!.rename("kpi", "kpi2"); });
    await typed(out, "quote", "q2");
    await later();
    expect(log).toEqual([]);
    await h.release();
    await wait(10);
    expect(log).toEqual(["rename kpi>kpi2", "set quote:q2"]);
  });

  // 13
  test("create refusé (saved: false) : pas de fichier fantôme, l'erreur est dite, faux", async () => {
    const { fake } = memory({});
    stub(fake, "setWidget", async (): Promise<WidgetChange> => ({
      status: { name: "abc", schema: null, diagnostics: [{ file: "abc", line: 1, column: 1, error: true, message: "rendu" }] },
      broken: [],
      saved: false,
    }));
    const { out } = await mount(fake);
    let ok = true;
    await act(async () => { ok = await out.files!.create("abc"); });
    expect(ok).toBe(false);
    expect(out.errors).toEqual(["rendu"]);
    expect(out.files!.statuses?.some((s) => s.name === "abc")).toBe(false);
  });

  // 14
  test("supprimer un fichier non enregistré efface sa marque ; le recréer n'en hérite pas", async () => {
    const { fake } = memory({ kpi: "k" });
    const set = fake.setWidget;
    stub(fake, "setWidget", async () => { throw new Error("non"); });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    stub(fake, "setWidget", set);
    await act(async () => { await out.files!.remove("kpi"); });
    expect(out.files!.unsaved.has("kpi")).toBe(false);
    await act(async () => { await out.files!.create("kpi"); });
    expect(out.files!.unsaved.has("kpi")).toBe(false);
  });

  // 15
  test("une frappe en attente survit à un nouveau rendu du parent : seuls les nouveaux rappels servent", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out, render } = await mount(fake);
    await typed(out, "kpi", "k2");
    await render("p", "r", "b");
    await later();
    expect(log).toEqual(["set kpi:k2"]);
    expect(out.changed).toEqual(["b"]);
  });

  // 16
  test("démontage pendant le délai : la frappe part quand même", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out, unmount } = await mount(fake);
    await typed(out, "kpi", "k2");
    await unmount();
    mounted = null;
    await wait(10);
    expect(log).toEqual(["set kpi:k2"]);
  });

  // 16b
  test("settle attend la frappe en attente et l'écriture partie", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const h = held(fake, "setWidget");
    await typed(out, "kpi", "k2");
    let done = false;
    const settling = out.files!.settle().then(() => { done = true; });
    await wait(10);
    expect(done).toBe(false);
    await h.release();
    await act(async () => settling);
    expect(log).toEqual(["set kpi:k2"]);
  });
});

describe("useWidgetFiles, renommage", () => {
  /** kpi lu, son renommage en kpi2 lancé et retenu. */
  async function renaming() {
    const { fake, log } = memory({ kpi: decl("kpi", "a") });
    const r = await mount(fake);
    await act(async () => r.out.files!.loaded("kpi", decl("kpi", "a")));
    const h = held(fake, "renameWidget");
    let done: Promise<boolean> | null = null;
    await act(async () => { done = r.out.files!.rename("kpi", "kpi2"); });
    const release = async () => {
      await h.release();
      await act(async () => { await done; });
      await later();
    };
    return { ...r, fake, log, release };
  }

  // 17
  test("frappe en attente à la fin du renommage : écrite sous le nouveau nom, remplacement non silencieux", async () => {
    const { out, log, release } = await renaming();
    await typed(out, "kpi", decl("kpi", "tapé"));
    await release();
    expect(log.filter((l) => l.startsWith("set kpi:"))).toEqual([]);
    expect(log.at(-1)).toBe(`set kpi2:${decl("kpi2", "tapé")}`);
    expect(out.files!.rewrites.get("kpi2")).toEqual({ find: 'widget("kpi"', replace: 'widget("kpi2"' });
  });

  // 18
  test("frappe partie pendant le renommage : redirigée vers le nouveau nom", async () => {
    const { out, log, release } = await renaming();
    await typed(out, "kpi", decl("kpi", "tapé"));
    await later();
    await release();
    expect(log.filter((l) => l.startsWith("set kpi:"))).toEqual([]);
    expect(log).toContain(`set kpi2:${decl("kpi2", "tapé")}`);
  });

  // 19
  test("sans frappe : remplacement silencieux, aucune écriture, texte connu", async () => {
    const { out, log, release } = await renaming();
    await release();
    expect(log).toEqual(["rename kpi>kpi2"]);
    expect(out.files!.rewrites.get("kpi2")?.silent).toBe(true);
    expect(out.files!.checked.get("kpi2")).toBe(decl("kpi2", "a"));
    expect(out.renamed).toEqual(["kpi>kpi2"]);
  });

  // 19b
  test("marqué non enregistré mais rien tapé depuis : remplacement non silencieux, texte connu sous le nouveau nom", async () => {
    const { fake } = memory({ kpi: decl("kpi", "a") });
    const set = fake.setWidget;
    stub(fake, "setWidget", async () => { throw new Error("non"); });
    const { out } = await mount(fake);
    await typed(out, "kpi", decl("kpi", "b"));
    await later();
    stub(fake, "setWidget", set);
    await act(async () => { await out.files!.rename("kpi", "kpi2"); });
    expect(out.files!.rewrites.get("kpi2")?.silent).toBeUndefined();
    expect(out.files!.unsaved.has("kpi2")).toBe(true);
  });

  // 20
  test("renommage refusé : la frappe reste écrite sous l'ancien nom, renamed non appelé", async () => {
    const { fake, log } = memory({ kpi: decl("kpi", "a") });
    const { out } = await mount(fake);
    const gate = deferred<void>();
    stub(fake, "renameWidget", async () => {
      await gate.promise;
      throw new Error("utilisé");
    });
    let done: Promise<boolean> | null = null;
    await act(async () => { done = out.files!.rename("kpi", "kpi2"); });
    await typed(out, "kpi", decl("kpi", "tapé"));
    await later();
    await act(async () => gate.resolve());
    let ok = true;
    await act(async () => { ok = await done!; });
    await wait(10);
    expect(ok).toBe(false);
    expect(log).toEqual([`set kpi:${decl("kpi", "tapé")}`]);
    expect(out.renamed).toEqual([]);
  });

  // 19c
  test("texte inconnu avant le renommage : remplacement non silencieux, rien de connu sous le nouveau nom", async () => {
    const { fake } = memory({ kpi: decl("kpi", "a") });
    const { out } = await mount(fake);
    await act(async () => { await out.files!.rename("kpi", "kpi2"); });
    expect(out.files!.rewrites.get("kpi2")).toEqual({ find: 'widget("kpi"', replace: 'widget("kpi2"' });
    expect(out.files!.checked.has("kpi2")).toBe(false);
  });

  // 20b (l'ordre « renamed avant les statuts » est vérifié par son effet,
  // l'absence d'élagage du sous-onglet renommé : T8 n°9)
  test("renommage réussi : renamed une fois, statuts sous le nouveau nom", async () => {
    const { fake } = memory({ kpi: decl("kpi", "a") });
    const { out } = await mount(fake);
    await act(async () => { await out.files!.rename("kpi", "kpi2"); });
    expect(out.renamed).toEqual(["kpi>kpi2"]);
    expect(out.files!.statuses?.map((s) => s.name)).toEqual(["kpi2"]);
  });
});

describe("useWidgetFiles, essais et relectures", () => {
  // 21
  test("relecture après une écriture : diagnostics du rendu gardés ; pack changé de nouveau : plus", async () => {
    const { fake } = memory({ kpi: "k" });
    const set = fake.setWidget;
    stub(fake, "setWidget", async (n: string, s: string): Promise<WidgetChange> => {
      const c = await set(n, s);
      return { ...c, status: { ...c.status, diagnostics: [{ file: "kpi", line: 1, column: 1, error: true, message: "rendu" }] } };
    });
    const { out, render } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    await render("p", "r1");
    await wait(10);
    expect(out.files!.statuses?.[0].diagnostics.length).toBe(1);
    await render("p", "r2");
    await wait(10);
    expect(out.files!.statuses?.[0].diagnostics.length).toBe(0);
  });

  // 21b
  test("deux fichiers écrits avant la relecture : chacun garde ses diagnostics du rendu", async () => {
    const { fake } = memory({ a: "a", b: "b" });
    const set = fake.setWidget;
    stub(fake, "setWidget", async (n: string, s: string): Promise<WidgetChange> => {
      const c = await set(n, s);
      return { ...c, status: { ...c.status, diagnostics: [{ file: n, line: 1, column: 1, error: true, message: `rendu ${n}` }] } };
    });
    const { out, render } = await mount(fake);
    await typed(out, "a", "a2");
    await typed(out, "b", "b2");
    await act(async () => out.files!.flush());
    await wait(10);
    await render("p", "r1");
    await wait(10);
    expect(out.files!.statuses?.map((s) => s.diagnostics.length)).toEqual([1, 1]);
  });

  test("relecture relancée par une écriture : les diagnostics de l'essai d'avant sont gardés", async () => {
    const { fake } = memory({ a: "a", b: "b" });
    const set = fake.setWidget;
    stub(fake, "setWidget", async (n: string, s: string): Promise<WidgetChange> => {
      const c = await set(n, s);
      return { ...c, status: { ...c.status, diagnostics: [{ file: n, line: 1, column: 1, error: true, message: `rendu ${n}` }] } };
    });
    const { out, render } = await mount(fake);
    // L'écriture de a, puis la relecture de r1 retenue.
    await typed(out, "a", "a2");
    await act(async () => out.files!.flush());
    await wait(10);
    const held = deferred<void>();
    const status = fake.widgetStatus;
    let first = true;
    stub(fake, "widgetStatus", async () => {
      if (first) {
        first = false;
        await held.promise;
      }
      return status();
    });
    await render("p", "r1");
    // L'écriture de b relance la relecture : la première est écartée.
    await typed(out, "b", "b2");
    await act(async () => out.files!.flush());
    await wait(10);
    await act(async () => held.resolve());
    await wait(10);
    expect(out.files!.statuses?.map((s) => [s.name, s.diagnostics.length])).toEqual([["a", 1], ["b", 1]]);
  });

  // 22
  test("essai refusé : une relecture du pack ne lui colle pas ses diagnostics", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "setWidget", async (): Promise<WidgetChange> => ({
      status: { name: "kpi", schema: null, diagnostics: [{ file: "kpi", line: 1, column: 1, error: true, message: "rendu" }] },
      broken: [],
      saved: false,
    }));
    const { out, render } = await mount(fake);
    await typed(out, "kpi", "k2");
    await later();
    await render("p", "r1");
    await wait(10);
    expect(out.files!.statuses?.[0].diagnostics.length).toBe(0);
  });

  // 23
  test("stale : périmée après une frappe du même fichier, pas après une frappe d'un autre", async () => {
    const { fake } = memory({ a: "a", b: "b" });
    const { out } = await mount(fake);
    await act(async () => out.files!.loaded("b", "b"));
    const askedB = out.files!.edits("b");
    await typed(out, "a", "a2");
    await later();
    expect(out.files!.stale("b", askedB, "b-agent")).toBe(false);
    const askedA = out.files!.edits("a");
    await typed(out, "a", "a3");
    expect(out.files!.stale("a", askedA, "a-agent")).toBe(true);
  });

  // 24
  test("stale : la source déjà affichée est périmée (notre propre texte)", async () => {
    const { fake } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    await act(async () => out.files!.loaded("kpi", "k"));
    expect(out.files!.stale("kpi", out.files!.edits("kpi"), "k")).toBe(true);
  });
});

describe("useWidgetFiles, revue finale", () => {
  /** Monte, puis retient la relecture lancée par la revision `r2` ; elle rendra la liste d'alors. */
  async function heldRefresh(files: Record<string, string>) {
    const { fake, log } = memory(files);
    const r = await mount(fake);
    const real = fake.widgetStatus;
    const before = await real();
    const slow = deferred<void>();
    let first = true;
    stub(fake, "widgetStatus", async () => {
      if (!first) return real();
      first = false;
      await slow.promise;
      return before;
    });
    await r.render("p", "r2");
    return { ...r, fake, log, resolve: () => act(async () => slow.resolve()) };
  }

  test("relecture en vol puis renommage : la liste d'avant ne revient pas, statuts relus pour la revision", async () => {
    const { out, resolve } = await heldRefresh({ kpi: decl("kpi", "a") });
    await act(async () => { await out.files!.rename("kpi", "kpi2"); });
    await resolve();
    await wait(10);
    expect(out.files!.statuses?.map((s) => s.name)).toEqual(["kpi2"]);
    expect(out.files!.statusesFor).toBe("r2");
  });

  test("relecture en vol puis suppression : le widget supprimé ne réapparaît pas", async () => {
    const { out, resolve } = await heldRefresh({ kpi: "k", chart: "c" });
    await act(async () => { await out.files!.remove("chart"); });
    await resolve();
    await wait(10);
    expect(out.files!.statuses?.map((s) => s.name)).toEqual(["kpi"]);
    expect(out.files!.statusesFor).toBe("r2");
  });

  test("relecture en vol puis création : le widget créé reste", async () => {
    const { out, resolve } = await heldRefresh({ kpi: "k" });
    await act(async () => { await out.files!.create("abc"); });
    await resolve();
    await wait(10);
    expect(out.files!.statuses?.map((s) => s.name)).toEqual(["abc", "kpi"]);
    expect(out.files!.statusesFor).toBe("r2");
  });

  test("settle attend aussi une opération mise en file pendant l'attente", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const set = fake.setWidget;
    const gates = { kpi: deferred<void>(), abc: deferred<void>() };
    stub(fake, "setWidget", async (name: string, source: string) => {
      await gates[name as "kpi" | "abc"].promise;
      return set(name, source);
    });
    await typed(out, "kpi", "k2");
    let done = false;
    const settling = out.files!.settle().then(() => { done = true; });
    await act(async () => { void out.files!.create("abc"); });
    await act(async () => gates.kpi.resolve());
    await wait(10);
    expect(done).toBe(false);
    await act(async () => gates.abc.resolve());
    await act(async () => settling);
    expect(done).toBe(true);
    expect(log).toEqual(["set kpi:k2", `set abc:${skeleton("abc", "widgets")}`]);
  });

  test("remove refusé pendant le délai : la frappe annulée revient, écrite ensuite", async () => {
    const { fake, log } = memory({ kpi: "k" });
    stub(fake, "removeWidget", async () => { throw new Error("utilisé"); });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    let ok = true;
    await act(async () => { ok = await out.files!.remove("kpi"); });
    await wait(10);
    expect(ok).toBe(false);
    expect(out.errors).toEqual(["utilisé"]);
    expect(log).toEqual(["set kpi:k2"]);
    expect(out.files!.unsaved.has("kpi")).toBe(false);
  });

  test("remove refusé, frappe plus récente entre-temps : l'ancienne frappe n'est pas réécrite", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const gate = deferred<void>();
    stub(fake, "removeWidget", async () => {
      await gate.promise;
      throw new Error("utilisé");
    });
    const { out } = await mount(fake);
    await typed(out, "kpi", "k2");
    let removing: Promise<boolean> | null = null;
    await act(async () => { removing = out.files!.remove("kpi"); });
    await typed(out, "kpi", "k3");
    await act(async () => gate.resolve());
    await act(async () => { await removing; });
    await later();
    expect(log).toEqual(["set kpi:k3"]);
  });
});

describe("useWidgetFiles, l'agent écrit", () => {
  test("forget abandonne une écriture déjà en file", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    const h = held(fake, "setWidget");
    let creating!: Promise<boolean>;
    await act(async () => {
      creating = out.files!.create("autre");
    });
    await typed(out, "kpi", "tapé");
    await act(async () => out.files!.flush("kpi"));
    expect(out.files!.pending("kpi")).toBe(false); // envoyée : en file derrière `create`, pas encore partie
    await act(async () => out.files!.forget("kpi"));
    await h.release();
    await act(async () => { await creating; });
    await later();
    expect(log.some((l) => l.startsWith("set kpi:"))).toBe(false);
  });

  test("keep renvoie le dernier texte tenté et non enregistré", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const real = fake.setWidget;
    let refuse = true;
    stub(fake, "setWidget", async (name: string, text: string): Promise<WidgetChange> => {
      if (!refuse) return real(name, text);
      log.push(`refused ${name}:${text}`);
      return { status: { name, schema: null, diagnostics: [] }, broken: [], saved: false };
    });
    const { out } = await mount(fake);
    await typed(out, "kpi", "tapé");
    await later();
    expect(out.files!.pending("kpi")).toBe(true);
    refuse = false;
    await act(async () => out.files!.keep("kpi"));
    await later();
    expect(log).toEqual(["refused kpi:tapé", "set kpi:tapé"]);
    expect(out.files!.pending("kpi")).toBe(false);
  });

  test("keep : une écriture en vol écrasée ensuite par l'agent est renvoyée", async () => {
    const mem = memory({ kpi: "k" });
    const real = mem.fake.setWidget;
    const gate = deferred<void>();
    let first = true;
    stub(mem.fake, "setWidget", async (name: string, text: string) => {
      const change = await real(name, text);
      if (first) {
        first = false;
        // La session a écrit la frappe, puis l'agent a écrasé le fichier ; la réponse n'est pas encore revenue.
        mem.files.kpi = "agent";
        await gate.promise;
      }
      return change;
    });
    const { out } = await mount(mem.fake);
    await typed(out, "kpi", "tapé");
    await act(async () => out.files!.flush("kpi"));
    expect(out.files!.pending("kpi")).toBe(true); // écriture en vol
    await act(async () => out.files!.keep("kpi"));
    await act(async () => gate.resolve());
    await later();
    expect(mem.files.kpi).toBe("tapé");
  });

  test("pending : frappe en attente", async () => {
    const { fake } = memory({ kpi: "k" });
    const { out } = await mount(fake);
    expect(out.files!.pending("kpi")).toBe(false);
    await typed(out, "kpi", "tapé"); // < PREVIEW_DELAY_MS : encore en attente
    expect(out.files!.pending("kpi")).toBe(true);
  });
});
