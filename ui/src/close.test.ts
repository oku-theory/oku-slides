import { describe, expect, mock, test } from "bun:test";
import type { Snapshot, TabApi, TabInfo } from "./api";
import { choiceOf, mayClose, saveForClose } from "./close";

const labels = { save: "Enregistrer", discard: "Ne pas enregistrer", cancel: "Annuler" };

describe("fermeture d'un onglet modifié", () => {
  test("choiceOf lit le libellé cliqué et les réponses système", () => {
    expect(choiceOf("Enregistrer", labels)).toBe("save");
    expect(choiceOf("Ne pas enregistrer", labels)).toBe("discard");
    expect(choiceOf("Yes", labels)).toBe("save");
    expect(choiceOf("No", labels)).toBe("discard");
    expect(choiceOf("Annuler", labels)).toBe("cancel");
    expect(choiceOf("Cancel", labels)).toBe("cancel");
  });

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

  test("saveForClose : modèles modifiés enregistrés, puis le deck retrouvé", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => snap({ dirty: true, editing: { set: "s", name: "S", builtin: false } }),
      saveTemplates: async () => (calls.push("saveTemplates"), snap({ editing: { set: "s", name: "S", builtin: false } })),
      closeTemplates: async () => (calls.push("closeTemplates"), snap({})),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(calls).toEqual(["saveTemplates", "closeTemplates"]);
  });

  test("saveForClose : modèles fermés mais deck resté modifié, l'enregistrement a lieu", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => snap({ dirty: true, editing: { set: "s", name: "S", builtin: false } }),
      saveTemplates: async () => (calls.push("saveTemplates"), snap({ editing: { set: "s", name: "S", builtin: false } })),
      closeTemplates: async () => (calls.push("closeTemplates"), snap({ dirty: true, path: "/x.oku-slides.pdf" })),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(calls).toEqual(["saveTemplates", "closeTemplates", "save"]);
  });

  test("saveForClose : nouveau deck, on renonce au choix du fichier", async () => {
    const save = mock(async () => snap({}));
    const api = { snapshot: async () => snap({ dirty: true }), save } as unknown as TabApi;
    const pick = mock(async () => null);
    expect(await saveForClose(api, pick)).toBe(false);
    expect(save).not.toHaveBeenCalled();
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
  const tab = (dirty: boolean): TabInfo => ({ id: "t1", title: "essai", dirty, editing_templates: false, path: null, pristine: !dirty, deck_uid: null });

  test("onglet propre : keep, sans poser de question", async () => {
    const ask = mock(async () => "cancel" as const);
    expect(await mayClose(tab(false), {} as unknown as TabApi, ask)).toBe("keep");
    expect(ask).not.toHaveBeenCalled();
  });

  test("discard : ferme en abandonnant le travail", async () => {
    const ask = mock(async () => "discard" as const);
    expect(await mayClose(tab(true), {} as unknown as TabApi, ask)).toBe("discard");
  });

  test("save réussi : keep", async () => {
    const ask = mock(async () => "save" as const);
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save: mock(async () => snap({})) } as unknown as TabApi;
    expect(await mayClose(tab(true), api, ask)).toBe("keep");
  });

  test("save qui échoue (renoncé au choix du fichier) : null", async () => {
    const ask = mock(async () => "save" as const);
    const api = { snapshot: async () => snap({ dirty: true }), save: mock(async () => snap({})) } as unknown as TabApi;
    const pick = mock(async () => null);
    expect(await mayClose(tab(true), api, ask, pick)).toBeNull();
  });

  test("cancel : null, rien n'est tenté", async () => {
    const ask = mock(async () => "cancel" as const);
    const api = { snapshot: mock(async () => snap({})) } as unknown as TabApi;
    expect(await mayClose(tab(true), api, ask)).toBeNull();
    expect(api.snapshot).not.toHaveBeenCalled();
  });
});
