// Rendu du panneau Widgets dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { TabApi, WidgetStatus } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act } = await import("react");
const { EditorView } = await import("@codemirror/view");
const { PREVIEW_DELAY_MS } = await import("./debounce");
const { deferred, editorText, memory, mountWith, pick, type, unmount, wait } = await import("./test/workbench");

afterEach(unmount);

const statuses: WidgetStatus[] = [
  { name: "kpi", schema: {}, diagnostics: [] },
  { name: "_commun", schema: null, diagnostics: [{ file: "_commun", line: 1, column: 1, error: true, message: "x" }] },
];

const api = {
  widgetStatus: async () => statuses,
  widgetSource: async () => "",
  previewWidget: async () => ({ svg: null, diagnostics: [] }),
} as unknown as TabApi;

describe("WidgetWorkbench", () => {
  test("liste : _commun en tête, chacun avec sa pastille d'état", async () => {
    const { host } = await mountWith(api);
    const items = [...host.querySelectorAll(".workbench-list li")];
    expect(items.map((li) => li.querySelector(".pill")?.className)).toEqual(["pill errors", "pill valid"]);
    expect(items[1]?.textContent).toContain("kpi");
  });
});

describe("WidgetWorkbench, édition", () => {
  test("une frappe en attente survit à un nouveau rendu du parent (rappels en ligne)", async () => {
    const sets: string[] = [];
    const changed: string[] = [];
    const fake = {
      widgetStatus: async () => [statuses[0]],
      widgetSource: async () => "#let a = 1\n",
      setWidget: async (name: string, source: string) => {
        sets.push(`${name}:${source}`);
        return { status: statuses[0], broken: [], saved: true };
      },
      previewWidget: async () => ({ svg: null, diagnostics: [] }),
    } as unknown as TabApi;
    const { host, rerender } = await mountWith(fake, { onChanged: () => changed.push("1") });
    await pick(host, "kpi");
    const content = host.querySelector(".typst-editor .cm-content");
    expect(content).not.toBeNull();
    const cm = EditorView.findFromDOM(content as HTMLElement);
    await act(async () => {
      cm?.dispatch({ changes: { from: 0, to: cm.state.doc.length, insert: "#let b = 2\n" }, userEvent: "input.type" });
    });
    // Le parent se rend de nouveau, avec de nouveaux rappels en ligne, avant le délai.
    await rerender({ onChanged: () => changed.push("2"), onError: () => {} });
    await act(async () => {
      await new Promise((r) => setTimeout(r, PREVIEW_DELAY_MS + 100));
    });
    expect(sets).toEqual(["kpi:#let b = 2\n"]);
    // Le rappel le plus récent est celui qui est appelé.
    expect(changed).toEqual(["2"]);
  });

  test("les diagnostics de _commun restent visibles sur le widget qui l'importe, avec leur fichier", async () => {
    const kpi: WidgetStatus = {
      name: "kpi",
      schema: null,
      diagnostics: [
        { file: "_commun", line: 3, column: 2, error: true, message: "unknown variable: foo" },
        { file: "kpi", line: 1, column: 1, error: true, message: "dans kpi" },
      ],
    };
    const fake = {
      widgetStatus: async () => [kpi],
      widgetSource: async () => "#let a = 1\n",
      previewWidget: async () => ({ svg: null, diagnostics: kpi.diagnostics }),
    } as unknown as TabApi;
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    const items = [...host.querySelectorAll(".workbench-diagnostics li")].map((li) => li.textContent);
    expect(items).toEqual(["_commun:3:2 unknown variable: foo", "kpi:1:1 dans kpi"]);
  });

  test("fermer pendant le délai envoie la dernière frappe", async () => {
    const { fake, log } = memory({ kpi: "a" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    await type(host, "ab");
    await unmount();
    expect(log).toEqual(["set kpi:ab"]);
  });

  test("changer de fichier pendant le délai : la frappe part, l'autre fichier s'affiche", async () => {
    const { fake, log } = memory({ kpi: "k", quote: "q" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    await type(host, "k2");
    await pick(host, "quote");
    expect(log).toEqual(["set kpi:k2"]);
    expect(editorText(host)).toBe("q");
  });

  test("supprimer pendant le délai : la frappe en attente ne recrée pas le fichier", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    await type(host, "k2");
    const del = host.querySelector<HTMLButtonElement>(".workbench-actions .danger");
    await act(async () => del?.click());
    await act(async () => {
      await new Promise((r) => setTimeout(r, PREVIEW_DELAY_MS + 100));
    });
    expect(log).toEqual(["remove kpi"]);
  });

  test("une réponse de source arrivée après une frappe ne ramène pas l'ancien texte", async () => {
    const { fake, log } = memory({ kpi: "T0" });
    const { host, rerender } = await mountWith(fake);
    await pick(host, "kpi");
    // Le pack change (notre écriture T1, vue plus tard) : la source est redemandée, réponse lente.
    const late = deferred<string>();
    const write = deferred<unknown>();
    (fake as unknown as { widgetSource: () => Promise<string> }).widgetSource = () => late.promise;
    await rerender({ revision: "r2" });
    // L'utilisateur tape T2 ; l'écriture part et reste en cours.
    (fake as unknown as { setWidget: (n: string, s: string) => Promise<unknown> }).setWidget = (n, s) => {
      log.push(`set ${n}:${s}`);
      return write.promise;
    };
    await type(host, "T2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(log).toEqual(["set kpi:T2"]);
    // La vieille réponse arrive : le texte tapé reste.
    await act(async () => late.resolve("T1"));
    expect(editorText(host)).toBe("T2");
    await act(async () => write.resolve({ status: statuses[0], broken: [], saved: true }));
    expect(editorText(host)).toBe("T2");
  });

  test("une réponse de source lente, sans écriture partie mais après une frappe, est ignorée", async () => {
    const { fake } = memory({ kpi: "T0" });
    const { host, rerender } = await mountWith(fake);
    await pick(host, "kpi");
    const late = deferred<string>();
    (fake as unknown as { widgetSource: () => Promise<string> }).widgetSource = () => late.promise;
    await rerender({ revision: "r2" });
    await type(host, "T2");
    await act(async () => late.resolve("T1"));
    expect(editorText(host)).toBe("T2");
  });

  test("le pack changé ailleurs (l'agent) : la source affichée est relue", async () => {
    const files = { kpi: "k" };
    const { fake } = memory(files);
    const { host, rerender } = await mountWith(fake);
    await pick(host, "kpi");
    expect(editorText(host)).toBe("k");
    files.kpi = "de l'agent";
    await rerender({ revision: "r2" });
    expect(editorText(host)).toBe("de l'agent");
  });

  test("Nouveau : un nom déjà pris est refusé, rien n'est écrit", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host } = await mountWith(fake);
    const create = [...host.querySelectorAll<HTMLButtonElement>(".workbench-actions .btn")][0];
    await act(async () => create?.click());
    const input = host.querySelector<HTMLInputElement>(".workbench-name input");
    expect(input).not.toBeNull();
    await act(async () => {
      const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
      set?.call(input, "kpi");
      input?.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(input?.getAttribute("aria-invalid")).toBe("true");
    await act(async () => host.querySelector("form.workbench-name")?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
    await wait(10);
    expect(log).toEqual([]);
    expect(host.querySelector(".workbench-name input")).not.toBeNull();
  });

  test("supprimer attend l'écriture déjà partie", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    const write = deferred<unknown>();
    (fake as unknown as { setWidget: (n: string, s: string) => Promise<unknown> }).setWidget = (n, s) => {
      log.push(`set ${n}:${s}`);
      return write.promise;
    };
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    await act(async () => host.querySelector<HTMLButtonElement>(".workbench-actions .danger")?.click());
    expect(log).toEqual(["set kpi:k2"]);
    await act(async () => write.resolve({ status: statuses[0], broken: [], saved: true }));
    await wait(10);
    expect(log).toEqual(["set kpi:k2", "remove kpi"]);
  });

  test("une écriture refusée : l'erreur est dite et le texte tapé reste, même après relecture", async () => {
    const { fake } = memory({ kpi: "k" });
    const errors: string[] = [];
    const { host, rerender } = await mountWith(fake, { onError: (m) => errors.push(m) });
    await pick(host, "kpi");
    (fake as unknown as { setWidget: () => Promise<unknown> }).setWidget = () => Promise.reject(new Error("trop grand"));
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(errors.length).toBe(1);
    expect(editorText(host)).toBe("k2");
    await rerender({ revision: "r2", onError: (m) => errors.push(m) });
    expect(editorText(host)).toBe("k2");
  });

  test("un aperçu ancien et lent n'écrase pas un plus récent", async () => {
    const { fake } = memory({ kpi: "k" });
    const previews: ReturnType<typeof deferred<{ svg: null; diagnostics: { file: string; line: null; column: null; error: boolean; message: string }[] }>>[] = [];
    (fake as unknown as { previewWidget: () => Promise<unknown> }).previewWidget = () => {
      const d = deferred<{ svg: null; diagnostics: { file: string; line: null; column: null; error: boolean; message: string }[] }>();
      previews.push(d);
      return d.promise;
    };
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(previews.length).toBe(2);
    const said = (message: string) => ({ svg: null, diagnostics: [{ file: "kpi", line: null, column: null, error: false, message }] });
    await act(async () => previews[1]?.resolve(said("récent")));
    await act(async () => previews[0]?.resolve(said("ancien")));
    const items = [...host.querySelectorAll(".workbench-diagnostics li")].map((li) => li.textContent);
    expect(items).toEqual(["kpi récent"]);
  });

  test("une frappe pas encore envoyée n'est pas écrasée par une relecture demandée après elle", async () => {
    const files = { kpi: "T0" };
    const { fake } = memory(files);
    const { host, rerender } = await mountWith(fake);
    await pick(host, "kpi");
    await type(host, "T2");
    files.kpi = "T1";
    await rerender({ revision: "r2" });
    expect(editorText(host)).toBe("T2");
  });

  test("deux écritures : la seconde attend la première, même lente", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const { host } = await mountWith(fake);
    await pick(host, "kpi");
    const writes: ReturnType<typeof deferred<unknown>>[] = [];
    (fake as unknown as { setWidget: (n: string, s: string) => Promise<unknown> }).setWidget = (n, s) => {
      log.push(`set ${n}:${s}`);
      const d = deferred<unknown>();
      writes.push(d);
      return d.promise;
    };
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    await type(host, "k3");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(log).toEqual(["set kpi:k2"]);
    await act(async () => writes[0]?.resolve({ status: statuses[0], broken: [], saved: true }));
    await wait(10);
    expect(log).toEqual(["set kpi:k2", "set kpi:k3"]);
    await act(async () => writes[1]?.resolve({ status: statuses[0], broken: [], saved: true }));
  });
});

describe("WidgetWorkbench, aperçu", () => {
  test("un widget d'élément est cadré sur sa boîte", async () => {
    const blobs: Blob[] = [];
    const create = URL.createObjectURL;
    URL.createObjectURL = (b: Blob | MediaSource) => {
      blobs.push(b as Blob);
      return "blob:apercu";
    };
    try {
      const fake = {
        widgetStatus: async () => [statuses[0]],
        widgetSource: async () => "#let a = 1\n",
        previewWidget: async () => ({
          svg: '<svg viewBox="0 0 1920 1080" width="1920pt" height="1080pt"></svg>',
          bbox: { x: 100, y: 200, w: 300, h: 40 },
          diagnostics: [],
        }),
      } as unknown as TabApi;
      const { host } = await mountWith(fake);
      await pick(host, "kpi");
      expect(host.querySelector(".workbench-preview img")?.getAttribute("src")).toBe("blob:apercu");
      const svg = await blobs.at(-1)?.text();
      const box = /viewBox="([^"]*)"/.exec(svg ?? "")?.[1]?.split(" ").map(Number) ?? [];
      // La boîte (100, 200, 300 × 40) avec sa marge, pas la slide entière.
      const m = 100 - (box[0] ?? 0);
      expect(m).toBeGreaterThan(0);
      expect(box).toEqual([100 - m, 200 - m, 300 + 2 * m, 40 + 2 * m]);
    } finally {
      URL.createObjectURL = create;
    }
  });
});
