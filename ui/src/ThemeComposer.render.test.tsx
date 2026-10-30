// A theme tab's notes column: the theme panel, the agent thread, then the composer (t-1277).
// The column is rebuilt by hand here, so this checks pack.css only, not that App renders these
// children in a theme tab; the e2e covers that.

import { afterEach, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import "./test/dom";

// See ./a2ui/catalog.render.test.tsx: React loads after the DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { DictationDraft } = await import("./DictationDraft");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement; style: HTMLStyleElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
  m.style.remove();
});

/** The notes column as App lays it out in a theme tab, with pack.css applied and `text` drafted. */
async function mountThemeNotes(text: string) {
  const style = document.createElement("style");
  style.textContent = readFileSync(`${import.meta.dir}/pack.css`, "utf8");
  document.head.append(style);
  const host = document.createElement("aside");
  host.className = "notes theme-active";
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host, style };
  await act(async () => {
    root.render([
      createElement("section", { key: "thread", className: "agent-thread" }),
      createElement(DictationDraft, {
        key: "draft",
        text,
        dictations: 0,
        typing: 0,
        busy: false,
        ready: true,
        shortcuts: false,
        onChange: () => {},
        images: [],
        thumbs: new Map<string, string>(),
        onPasteImages: () => {},
        onRemoveImage: () => {},
        onSend: () => {},
        onDiscard: () => {},
      }),
      // ThemePanels portals the theme panel in after App's own children.
      createElement("section", { key: "panel", className: "theme-panel" }),
    ]);
  });
  const shown = (selector: string) => {
    const el = host.querySelector(selector);
    if (!el) throw new Error(`${selector} is missing`);
    return getComputedStyle(el).display !== "none";
  };
  const order = (selector: string) => {
    const el = host.querySelector(selector);
    if (!el) throw new Error(`${selector} is missing`);
    return Number(getComputedStyle(el).order || 0);
  };
  return { host, shown, order };
}

describe("theme tab composer", () => {
  test("theme tab notes show the composer with its draft", async () => {
    const notes = await mountThemeNotes("Passe les titres en bleu.");
    expect(notes.shown(".draft")).toBe(true);
    expect(notes.host.querySelector(".draft")?.textContent).toContain("Passe les titres en bleu.");
  });

  test("theme tab notes show the agent thread below the theme panel", async () => {
    const notes = await mountThemeNotes("x");
    expect(notes.shown(".agent-thread")).toBe(true);
    expect(notes.shown(".theme-panel")).toBe(true);
    expect(notes.order(".agent-thread")).toBeGreaterThan(notes.order(".theme-panel"));
  });

  test("theme tab notes place the composer below the thread", async () => {
    const notes = await mountThemeNotes("x");
    expect(notes.order(".draft")).toBeGreaterThan(notes.order(".agent-thread"));
  });
});
