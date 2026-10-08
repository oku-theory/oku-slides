import { describe, expect, test } from "bun:test";
import { slideStep, slideTarget } from "./slideKeys";

const key = (k: string, mods: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean }> = {}) => ({
  key: k,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  ...mods,
});

describe("slideTarget", () => {
  test("slideTarget with next keys moves one slide forward", () => {
    for (const k of ["ArrowRight", "ArrowDown", "PageDown", " "]) expect(slideTarget(key(k), 1, 4)).toBe(2);
  });

  test("slideTarget with previous keys moves one slide back", () => {
    for (const k of ["ArrowLeft", "ArrowUp", "PageUp"]) expect(slideTarget(key(k), 1, 4)).toBe(0);
  });

  test("slideTarget with Home and End goes to the ends", () => {
    expect(slideTarget(key("Home"), 2, 4)).toBe(0);
    expect(slideTarget(key("End"), 0, 4)).toBe(3);
  });

  test("slideTarget at an end returns null", () => {
    expect(slideTarget(key("ArrowLeft"), 0, 4)).toBeNull();
    expect(slideTarget(key("ArrowRight"), 3, 4)).toBeNull();
  });

  test("slideTarget with a modifier or another key returns null", () => {
    expect(slideTarget(key("ArrowRight", { metaKey: true }), 0, 4)).toBeNull();
    expect(slideTarget(key("ArrowRight", { ctrlKey: true }), 0, 4)).toBeNull();
    expect(slideTarget(key("ArrowRight", { altKey: true }), 0, 4)).toBeNull();
    expect(slideTarget(key("a"), 0, 4)).toBeNull();
  });

  test("slideTarget with Space on a focused button leaves it to the button", () => {
    expect(slideTarget(key(" "), 1, 4, { space: false })).toBeNull();
  });

  test("slideTarget with arrows on a focused button still navigates", () => {
    expect(slideTarget(key("ArrowRight"), 1, 4, { space: false })).toBe(2);
  });
});

describe("slideTarget in the editor", () => {
  test("slideTarget without arrows leaves the arrows to the selection", () => {
    for (const k of ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"]) expect(slideTarget(key(k), 1, 4, { arrows: false })).toBeNull();
  });

  test("slideTarget without arrows still pages through the slides", () => {
    expect(slideTarget(key("PageDown"), 1, 4, { arrows: false })).toBe(2);
    expect(slideTarget(key("PageUp"), 1, 4, { arrows: false })).toBe(0);
  });
});

describe("slideStep", () => {
  test("slideStep without slides returns null", () => {
    expect(slideStep(0, 1, 0)).toBeNull();
  });

  test("slideStep from an unknown slide starts at the first", () => {
    expect(slideStep(-1, 1, 4)).toBe(1);
  });
});
