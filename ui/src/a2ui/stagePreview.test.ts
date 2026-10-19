// Le magasin de l'aperçu sur la scène (variantes, thèmes).

import { describe, expect, test } from "bun:test";
import { createStagePreviewStore } from "./stagePreview";

describe("createStagePreviewStore", () => {
  test("show puis get rend la valeur posée", () => {
    const store = createStagePreviewStore();
    store.show({ slide: "s-1", url: "data:a" });
    expect(store.get()).toEqual({ slide: "s-1", url: "data:a" });
  });

  test("show remplace l'aperçu montré", () => {
    const store = createStagePreviewStore();
    store.show({ url: "data:a" });
    store.show({ url: "data:b" });
    expect(store.get()?.url).toBe("data:b");
  });

  test("hide de l'aperçu montré rend la slide", () => {
    const store = createStagePreviewStore();
    store.show({ url: "data:a" });
    store.hide({ url: "data:a" });
    expect(store.get()).toBeNull();
  });

  test("hide d'un autre aperçu garde celui montré", () => {
    const store = createStagePreviewStore();
    store.show({ url: "data:b" });
    store.hide({ url: "data:a" });
    expect(store.get()?.url).toBe("data:b");
  });

  test("hide de la même image pour une autre slide garde l'aperçu", () => {
    const store = createStagePreviewStore();
    store.show({ slide: "s-1", url: "data:a" });
    store.hide({ slide: "s-2", url: "data:a" });
    expect(store.get()?.slide).toBe("s-1");
  });

  test("un changement prévient les abonnés", () => {
    const store = createStagePreviewStore();
    let calls = 0;
    store.subscribe(() => calls++);
    store.show({ slide: "s-1", url: "data:a" });
    store.hide({ slide: "s-1", url: "data:a" });
    expect(calls).toBe(2);
  });

  test("la même valeur ne prévient personne", () => {
    const store = createStagePreviewStore();
    let calls = 0;
    store.show({ slide: "s-1", url: "data:a" });
    store.subscribe(() => calls++);
    store.show({ slide: "s-1", url: "data:a" });
    expect(calls).toBe(0);
  });

  test("un abonné désabonné n'est plus prévenu", () => {
    const store = createStagePreviewStore();
    let calls = 0;
    const off = store.subscribe(() => calls++);
    off();
    store.show({ slide: "s-1", url: "data:a" });
    expect(calls).toBe(0);
  });
});
