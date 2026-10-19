// Le magasin de l'aperçu de thème sur la scène.

import { describe, expect, test } from "bun:test";
import { createStagePreviewStore } from "./stagePreview";

describe("createStagePreviewStore", () => {
  test("set puis get rend la valeur posée", () => {
    const store = createStagePreviewStore();
    store.set({ slide: "s-1", url: "data:a" });
    expect(store.get()).toEqual({ slide: "s-1", url: "data:a" });
  });

  test("un changement prévient les abonnés", () => {
    const store = createStagePreviewStore();
    let calls = 0;
    store.subscribe(() => calls++);
    store.set({ slide: "s-1", url: "data:a" });
    store.set(null);
    expect(calls).toBe(2);
  });

  test("la même valeur ne prévient personne", () => {
    const store = createStagePreviewStore();
    let calls = 0;
    store.set({ slide: "s-1", url: "data:a" });
    store.subscribe(() => calls++);
    store.set({ slide: "s-1", url: "data:a" });
    expect(calls).toBe(0);
  });

  test("un abonné désabonné n'est plus prévenu", () => {
    const store = createStagePreviewStore();
    let calls = 0;
    const off = store.subscribe(() => calls++);
    off();
    store.set({ slide: "s-1", url: "data:a" });
    expect(calls).toBe(0);
  });
});
