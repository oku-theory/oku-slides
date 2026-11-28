import { describe, expect, test } from "bun:test";
import { stripKeyNav } from "./stripKeyNav";

const key = (k: string) => ({ key: k });

describe("stripKeyNav", () => {
  test("stripKeyNav arrows move and wrap", () => {
    expect(stripKeyNav(key("ArrowRight"), 0, 3, false)).toBe(1);
    expect(stripKeyNav(key("ArrowRight"), 2, 3, false)).toBe(0);
    expect(stripKeyNav(key("ArrowLeft"), 0, 3, false)).toBe(2);
    expect(stripKeyNav(key("ArrowLeft"), 2, 3, false)).toBe(1);
  });

  test("stripKeyNav home and end go to the ends", () => {
    expect(stripKeyNav(key("Home"), 2, 3, false)).toBe(0);
    expect(stripKeyNav(key("End"), 0, 3, true)).toBe(2);
  });

  test("stripKeyNav rtl mirrors the arrows", () => {
    expect(stripKeyNav(key("ArrowLeft"), 0, 3, true)).toBe(1);
    expect(stripKeyNav(key("ArrowRight"), 0, 3, true)).toBe(2);
  });

  test("stripKeyNav other keys and an empty strip give null", () => {
    expect(stripKeyNav(key("ArrowUp"), 0, 3, false)).toBeNull();
    expect(stripKeyNav(key("a"), 0, 3, false)).toBeNull();
    expect(stripKeyNav(key("ArrowRight"), 0, 0, false)).toBeNull();
  });

  test("stripKeyNav with no current tab starts from an end", () => {
    expect(stripKeyNav(key("ArrowRight"), -1, 3, false)).toBe(0);
    expect(stripKeyNav(key("ArrowLeft"), -1, 3, false)).toBe(2);
  });
});
