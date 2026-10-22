import { describe, expect, mock, test } from "bun:test";
import type { Snapshot, TabApi, TabInfo } from "./api";
import { mayClose, saveForClose } from "./close";

describe("fermeture d'un onglet modifié", () => {
  const snap = (s: Partial<Snapshot>) => ({ dirty: false, path: null, name: "essai", editing: null, ...s }) as Snapshot;

  test("saveForClose : deck propre, rien n'est enregistré", async () => {
    const save = mock(async () => snap({}));
    const api = { snapshot: async () => snap({}), save } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(save).not.toHaveBeenCalled();
  });

  test("saveForClose : deck avec fichier, enregistré sur place", async () => {
    const save = mock(async () => snap({ path: "/x.oku-slides.pdf" }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(save).toHaveBeenCalledWith(null, false);
  });

  test("closing a tab in pack mode with changes asks to save the pack", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => snap({ dirty: true, editing: { pack: "s", name: "S", kind: "layouts", builtin: false } }),
      savePack: async () => (calls.push("savePack"), snap({ editing: { pack: "s", name: "S", kind: "layouts", builtin: false } })),
      closePack: async () => (calls.push("closePack"), snap({})),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(calls).toEqual(["savePack", "closePack"]);
  });

  test("saveForClose : historique ouvert, fermé d'abord : c'est le deck retrouvé qui s'enregistre", async () => {
    const calls: string[] = [];
    const history = { index: 0 } as unknown as Snapshot["history"];
    const api = {
      snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf", history }),
      historyClose: async () => (calls.push("historyClose"), snap({ dirty: true, path: "/x.oku-slides.pdf", history: null })),
      save: async () => (calls.push("save"), snap({ path: "/x.oku-slides.pdf" })),
    } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(calls).toEqual(["historyClose", "save"]);
  });

  test("saveForClose : pack fermé mais deck resté modifié, l'enregistrement a lieu", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => snap({ dirty: true, editing: { pack: "s", name: "S", kind: "layouts", builtin: false } }),
      savePack: async () => (calls.push("savePack"), snap({ editing: { pack: "s", name: "S", kind: "layouts", builtin: false } })),
      closePack: async () => (calls.push("closePack"), snap({ dirty: true, path: "/x.oku-slides.pdf" })),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(calls).toEqual(["savePack", "closePack", "save"]);
  });

  test("saveForClose : nouveau deck, enregistré au dossier par défaut sans dialogue", async () => {
    const save = mock(async () => snap({ path: "/Documents/Oku Slides/essai.oku-slides.pdf" }));
    const api = { snapshot: async () => snap({ dirty: true }), save } as unknown as TabApi;
    const pick = mock(async () => null);
    expect(await saveForClose(api, pick)).toBe(true);
    expect(save).toHaveBeenCalledWith(null, false);
    expect(pick).not.toHaveBeenCalled();
  });

  test("saveForClose : fichier remplacé par un autre deck, on renonce : pas fermé", async () => {
    const save = mock(async () => snap({ dirty: true, path: "/x.oku-slides.pdf", disk_replaced: true }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api, undefined, async () => "cancel")).toBe(false);
    expect(save).toHaveBeenCalledTimes(1);
  });

  test("saveForClose : fichier remplacé, « Écraser » : enregistré, fermé", async () => {
    const calls: unknown[] = [];
    const save = async (p: string | null, o = false) =>
      (calls.push([p, o]), snap({ path: "/x.oku-slides.pdf", disk_replaced: calls.length === 1 }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api, undefined, async () => "overwrite")).toBe(true);
    expect(calls).toEqual([[null, false], [null, true]]);
  });
});

describe("mayClose", () => {
  const snap = (s: Partial<Snapshot>) => ({ dirty: false, path: null, name: "essai", editing: null, ...s }) as Snapshot;
  const tab = (dirty: boolean): TabInfo => ({ id: "t1", title: "essai", dirty, editing_pack: false, path: null, pristine: !dirty, deck_uid: null });

  test("onglet propre : keep, rien n'est enregistré", async () => {
    const api = { snapshot: mock(async () => snap({})) } as unknown as TabApi;
    expect(await mayClose(tab(false), api)).toBe("keep");
    expect(api.snapshot).not.toHaveBeenCalled();
  });

  test("onglet modifié : enregistré sans question, keep", async () => {
    const save = mock(async () => snap({ path: "/x.oku-slides.pdf" }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await mayClose(tab(true), api)).toBe("keep");
    expect(save).toHaveBeenCalledTimes(1);
  });

  test("onglet modifié dont l'enregistrement est refusé (fichier remplacé) : null, pas fermé", async () => {
    const save = mock(async () => snap({ dirty: true, path: "/x.oku-slides.pdf", disk_replaced: true }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await mayClose(tab(true), api, undefined, async () => "cancel")).toBeNull();
  });
});
