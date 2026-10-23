// La pile d'un onglet : slides du deck, ou layouts du pack édité (mode pack).

import { afterAll, afterEach, beforeAll, describe, expect, test } from "bun:test";
import "./test/dom";
import type { PackKind, Snapshot } from "./api";
import { ESSENTIEL } from "./builtinKinds";

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
      { id: "s1", kind: ESSENTIEL.title, title: "Bonjour", hash: "a" },
      { id: "s2", kind: ESSENTIEL.content, title: null, hash: "b" },
    ],
  }) as unknown as Snapshot;

/** Un onglet en mode pack sur un pack de `kind` : une slide par layout pour un pack de layouts. */
const snapshotInPackMode = (kind: PackKind): Snapshot =>
  ({
    editing: { pack: "e", name: "Essentiel", kind, builtin: true, deck_only: false },
    history: null, read_only: false,
    slides: kind === "layouts"
      ? [
          { id: "layout-blank", kind: ESSENTIEL.blank, title: null, hash: "a" },
          { id: "layout-title", kind: ESSENTIEL.title, title: "Titre", hash: "b" },
        ]
      : [],
  }) as unknown as Snapshot;

async function renderPile(snap: Snapshot) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const noop = () => {};
  await act(async () =>
    root.render(
      createElement(DeckPile, { snap, current: snap.slides[0]?.id ?? null, svg: () => null, onSelect: noop, onMove: noop, onAdd: noop, onRemove: noop, axis: "x" }),
    ),
  );
  return host;
}

/** Boutons nommés `name`, par leur texte ou, pictogramme seul, par leur `aria-label`. */
const buttons = (host: HTMLElement, name: RegExp) =>
  [...host.querySelectorAll("button")].filter((b) => name.test(b.textContent ?? "") || name.test(b.getAttribute("aria-label") ?? ""));

describe("DeckPile", () => {
  test("pack mode hides the add-slide button", async () => {
    const host = await renderPile(snapshotInPackMode("layouts"));
    expect(buttons(host, /Nouvelle slide|New slide/)).toHaveLength(0);
  });

  test("DeckPile in pack mode offers no delete and no drag", async () => {
    const host = await renderPile(snapshotInPackMode("layouts"));
    expect(host.querySelectorAll(".thumb-remove")).toHaveLength(0);
    expect([...host.querySelectorAll(".thumb")].map((b) => b.getAttribute("title"))).toEqual([null, null]);
  });

  test("DeckPile in pack mode names each slide after its layout", async () => {
    const host = await renderPile(snapshotInPackMode("layouts"));
    expect([...host.querySelectorAll(".thumb-name")].map((e) => e.textContent)).toEqual(["blank", "title"]);
    expect(host.querySelector("nav")?.getAttribute("aria-label")).toBe("Layouts");
  });

  test("DeckPile on the deck offers add, delete and drag", async () => {
    const host = await renderPile(deckSnapshot());
    expect(buttons(host, /New slide/)).toHaveLength(1);
    expect(host.querySelectorAll(".thumb-remove")).toHaveLength(2);
    expect(host.querySelectorAll(".thumb-name")).toHaveLength(0);
  });

  test("DeckPile read only offers nothing to change", async () => {
    const host = await renderPile({ ...deckSnapshot(), read_only: true });
    expect([buttons(host, /New slide/).length, host.querySelectorAll(".thumb-remove").length]).toEqual([0, 0]);
  });
});

describe("pileSection", () => {
  test("pileSection gives slides on the deck, layouts for a layout pack, none for a widget pack", () => {
    expect([pileSection(deckSnapshot()), pileSection(snapshotInPackMode("layouts")), pileSection(snapshotInPackMode("widgets"))]).toEqual([
      "slides",
      "layouts",
      null,
    ]);
  });
});
