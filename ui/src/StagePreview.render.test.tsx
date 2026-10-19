// L'aperçu survolé (thème, variante) sur la scène du deck.

import { afterEach, describe, expect, test } from "bun:test";
import type { Root } from "react-dom/client";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: Stage } = await import("./Stage");
const { createStagePreviewStore } = await import("./a2ui/stagePreview");
const { fieldKey } = await import("./stageFields");

type StageProps = Parameters<typeof Stage>[0];

const style = { size: 20, leading: 6, tracking: 0, font: ["Inter"], weight: 400, fill: null, align: "left", case: null, bg: null };
// La vue de Stage.render.test.tsx : une slide « s », un élément c1 avec un champ title.
const view = {
  slide: { id: "s", kind: "content", params: { blocks: [{ id: "c1", kind: "acme/card", params: { title: "Titre" } }] } },
  page: 1, hash: "h", width: 1920, height: 1080, zone: null,
  targets: [{ id: "c1", kind: "acme/card", free: false, x: 0, y: 0, w: 400, h: 200 }],
  fields: [{
    owner: "c1", owner_kind: "element", widget: "acme/card", field: "title", list: null, item: null, column: "title",
    kind: "plain", label: null, text: "Titre", empty: false, side: null,
    x: 10, y: 60, w: 100, h: 20, ax: 10, aw: 300, style,
  }],
  items: [], lists: [],
} as unknown as StageProps["view"];

let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(more: Partial<StageProps>) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(Stage, {
        view, svgUrl: null, selected: null, editing: null, palette: [],
        onSelect: () => {}, onEdit: () => {}, onApply: () => {}, onNotice: () => {},
        ...more,
      } as StageProps),
    );
  });
  return host;
}

describe("aperçu sur la scène", () => {
  test("l'aperçu de la slide montrée se pose par-dessus elle", async () => {
    const store = createStagePreviewStore();
    const host = await mount({ hoverPreview: store });
    await act(async () => store.show({ slide: "s", url: "data:image/svg+xml;utf8,%3Csvg%2F%3E" }));
    expect(host.querySelector(".stage-hover-preview")?.getAttribute("src")).toBe("data:image/svg+xml;utf8,%3Csvg%2F%3E");
  });

  test("l'aperçu d'une autre slide n'est pas montré", async () => {
    const store = createStagePreviewStore();
    const host = await mount({ hoverPreview: store });
    await act(async () => store.show({ slide: "s-2", url: "data:a" }));
    expect(host.querySelector(".stage-hover-preview")).toBeNull();
  });

  test("pendant l'édition d'un texte, l'aperçu n'est pas montré", async () => {
    const store = createStagePreviewStore();
    store.show({ slide: "s", url: "data:a" });
    const host = await mount({ hoverPreview: store, editing: fieldKey({ owner: "c1", field: "title" }) });
    expect(host.querySelector(".stage-hover-preview")).toBeNull();
  });

  test("retirer l'aperçu l'enlève de la scène", async () => {
    const store = createStagePreviewStore();
    const host = await mount({ hoverPreview: store });
    await act(async () => store.show({ slide: "s", url: "data:a" }));
    await act(async () => store.hide({ slide: "s", url: "data:a" }));
    expect(host.querySelector(".stage-hover-preview")).toBeNull();
  });

  test("en lecture seule, l'aperçu d'un thème n'est pas montré", async () => {
    const store = createStagePreviewStore();
    store.show({ slide: "s", url: "data:a" });
    const host = await mount({ hoverPreview: store, readOnly: true });
    expect(host.querySelector(".stage-hover-preview")).toBeNull();
  });

  test("en mode modèles, l'aperçu d'un thème n'est pas montré", async () => {
    const store = createStagePreviewStore();
    store.show({ slide: "s", url: "data:a" });
    const host = await mount({ hoverPreview: store, templates: true });
    expect(host.querySelector(".stage-hover-preview")).toBeNull();
  });

  test("pendant l'édition d'un texte, l'aperçu d'une variante reste montré", async () => {
    const store = createStagePreviewStore();
    store.show({ url: "data:v" });
    const host = await mount({ hoverPreview: store, editing: fieldKey({ owner: "c1", field: "title" }) });
    expect(host.querySelector(".stage-hover-preview")?.getAttribute("src")).toBe("data:v");
  });

  test("en lecture seule, l'aperçu d'une variante reste montré", async () => {
    const store = createStagePreviewStore();
    store.show({ url: "data:v" });
    const host = await mount({ hoverPreview: store, readOnly: true });
    expect(host.querySelector(".stage-hover-preview")?.getAttribute("src")).toBe("data:v");
  });

  test("en mode modèles, l'aperçu d'une variante reste montré", async () => {
    const store = createStagePreviewStore();
    store.show({ url: "data:v" });
    const host = await mount({ hoverPreview: store, templates: true });
    expect(host.querySelector(".stage-hover-preview")?.getAttribute("src")).toBe("data:v");
  });
});
