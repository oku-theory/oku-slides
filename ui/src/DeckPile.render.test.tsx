// A tab's pile: the deck's slides, or a theme tab's specimens.

import { afterAll, afterEach, beforeAll, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: DeckPile, pileSection } = await import("./DeckPile");
const { setMessages } = await import("./i18n");
const { resetMessages } = await import("./test/messages");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});
beforeAll(async () => {
  const en = await Bun.file(new URL("../../crates/oku-slides-i18n/src/locales/en/ui.ftl", import.meta.url)).text();
  setMessages("en", [en]);
});
afterAll(resetMessages);

const deckSnapshot = (): Snapshot =>
  ({
    editing: null, history: null, read_only: false,
    slides: [
      { id: "s1", kind: "essentiel/title", title: "Bonjour", hash: "a" },
      { id: "s2", kind: "essentiel/content", title: null, hash: "b" },
    ],
  }) as unknown as Snapshot;

/** A theme tab: the specimen deck, one slide per layout of Essentiel. */
const themeTabSnapshot = (): Snapshot =>
  ({
    editing: null, history: null, read_only: false, theming: {},
    slides: [
      { id: "layout-blank", kind: "essentiel/blank", title: null, hash: "a" },
      { id: "layout-title", kind: "essentiel/title", title: "Titre", hash: "b" },
    ],
  }) as unknown as Snapshot;

async function renderPile(snap: Snapshot, themeTab = false) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const noop = () => {};
  await act(async () =>
    root.render(
      createElement(DeckPile, { snap, themeTab, current: snap.slides[0]?.id ?? null, svg: () => null, onSelect: noop, onMove: noop, onAdd: noop, onRemove: noop, axis: "x" }),
    ),
  );
  return host;
}

/** Buttons named `name`, by their text or, icon only, by their `aria-label`. */
const buttons = (host: HTMLElement, name: RegExp) =>
  [...host.querySelectorAll("button")].filter((b) => name.test(b.textContent ?? "") || name.test(b.getAttribute("aria-label") ?? ""));

describe("DeckPile", () => {
  test("DeckPile on the deck offers add, delete and drag", async () => {
    const host = await renderPile(deckSnapshot());
    expect(buttons(host, /New slide/)).toHaveLength(1);
    expect(host.querySelectorAll(".thumb-remove")).toHaveLength(2);
    expect(host.querySelectorAll(".thumb-name")).toHaveLength(0);
  });

  test("DeckPile in a theme tab is fixed: no add, delete or drag, slides named after their layout", async () => {
    const host = await renderPile(themeTabSnapshot(), true);
    expect([buttons(host, /New slide/).length, host.querySelectorAll(".thumb-remove").length]).toEqual([0, 0]);
    expect([...host.querySelectorAll(".thumb")].map((b) => b.getAttribute("title"))).toEqual([null, null]);
    expect([...host.querySelectorAll(".thumb-name")].map((e) => e.textContent)).toEqual(["blank", "title"]);
  });

  test("DeckPile read only offers nothing to change", async () => {
    const host = await renderPile({ ...deckSnapshot(), read_only: true });
    expect([buttons(host, /New slide/).length, host.querySelectorAll(".thumb-remove").length]).toEqual([0, 0]);
  });
});

describe("pileSection", () => {
  test("pileSection gives layouts in a theme tab", () => {
    expect(pileSection(true)).toBe("layouts");
  });

  test("pileSection gives slides outside a theme tab", () => {
    expect(pileSection(false)).toBe("slides");
  });
});
