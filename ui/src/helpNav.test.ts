import { describe, expect, test } from "bun:test";
import { helpStep, helpTarget } from "./helpNav";

const key = (k: string, mods: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean }> = {}) => ({
  key: k,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  ...mods,
});

describe("helpTarget", () => {
  test("helpTarget with next keys moves one slide forward", () => {
    for (const k of ["ArrowRight", "ArrowDown", "PageDown", " "]) expect(helpTarget(key(k), 1, 4)).toBe(2);
  });

  test("helpTarget with previous keys moves one slide back", () => {
    for (const k of ["ArrowLeft", "ArrowUp", "PageUp"]) expect(helpTarget(key(k), 1, 4)).toBe(0);
  });

  test("helpTarget with Home and End goes to the ends", () => {
    expect(helpTarget(key("Home"), 2, 4)).toBe(0);
    expect(helpTarget(key("End"), 0, 4)).toBe(3);
  });

  test("helpTarget at an end returns null", () => {
    expect(helpTarget(key("ArrowLeft"), 0, 4)).toBeNull();
    expect(helpTarget(key("ArrowRight"), 3, 4)).toBeNull();
  });

  test("helpTarget with a modifier or another key returns null", () => {
    expect(helpTarget(key("ArrowRight", { metaKey: true }), 0, 4)).toBeNull();
    expect(helpTarget(key("ArrowRight", { ctrlKey: true }), 0, 4)).toBeNull();
    expect(helpTarget(key("ArrowRight", { altKey: true }), 0, 4)).toBeNull();
    expect(helpTarget(key("a"), 0, 4)).toBeNull();
  });

  test("helpTarget with Space on a focused button leaves it to the button", () => {
    expect(helpTarget(key(" "), 1, 4, true)).toBeNull();
  });

  test("helpTarget with arrows on a focused button still navigates", () => {
    expect(helpTarget(key("ArrowRight"), 1, 4, true)).toBe(2);
  });
});

describe("helpStep", () => {
  test("helpStep without slides returns null", () => {
    expect(helpStep(0, 1, 0)).toBeNull();
  });

  test("helpStep from an unknown slide starts at the first", () => {
    expect(helpStep(-1, 1, 4)).toBe(1);
  });
});
