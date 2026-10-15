import "./test/dom";
import { expect, test } from "bun:test";
import type { FieldView, TextStyle } from "./api";

// Chargé après le DOM de test (voir test/dom.ts).
const { editorGeometry } = await import("./editorGeometry");

const style: TextStyle = { size: 40, leading: 24, tracking: 0, font: ["Newsreader"], weight: 500, fill: "#2e2b26", align: "left", case: null, bg: null };
const field = (extra: Partial<FieldView> = {}): FieldView => ({
  owner: "e", owner_kind: "element", widget: "text", field: "body", kind: "rich", label: null,
  text: "Un bloc", empty: false, side: null, x: 160, y: 300, w: 120, h: 27, ax: 160, aw: 720, style, ...extra,
});
const numbers = (css: Record<string, unknown>) =>
  Object.fromEntries(Object.entries(css).filter(([, v]) => typeof v === "number")) as Record<string, number>;

test("editorGeometry without layout gives finite boxes from the fallback metrics", () => {
  for (const f of [field(), field({ kind: "plain" }), field({ empty: true, h: 0, w: 0, side: "before" })]) {
    const css = numbers(editorGeometry(f, f.text, 0.5, [f]) as Record<string, unknown>);
    for (const [k, v] of Object.entries(css)) expect(Number.isFinite(v), k).toBe(true);
  }
});

test("editorGeometry of a rich field spans its zone, where Typst wraps", () => {
  const f = field();
  const css = editorGeometry(f, "Un bloc", 1, [f]);
  expect(css.left).toBe(160 - 5);
  expect(css.width).toBe(720 + 2 * 5);
});

test("editorGeometry paints its background on the text lines only, not its padding", () => {
  const f = field();
  const css = editorGeometry(f, "Un bloc", 1, [f]);
  expect(css.backgroundClip).toBe("content-box");
});

test("editorGeometry sets the optical size to the Typst size in pt, not the screen size", () => {
  const f = field();
  const css = editorGeometry(f, "Un bloc", 0.5, [f]);
  expect(css.fontVariationSettings).toBe("'opsz' 40");
});

test("editorGeometry carries the zone's weight, colour, case, leading and tracking to the editor (t-272)", () => {
  const f = field({ style: { ...style, weight: 700, fill: "#aa3300", case: "upper", tracking: 2 } });
  const css = editorGeometry(f, "Un bloc", 0.5, [f]);
  expect(css.fontWeight).toBe(700);
  expect(css.color).toBe("#aa3300");
  expect(css.textTransform).toBe("uppercase");
  expect(css.letterSpacing).toBe(1);
  expect(css.fontSize).toBe(20);
  expect(typeof css.lineHeight).toBe("string");
  expect(String(css.lineHeight)).toMatch(/px$/);
  expect(String(css.fontFamily)).toContain("Newsreader");
});
