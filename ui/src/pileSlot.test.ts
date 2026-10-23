import { describe, expect, test } from "bun:test";
import { dropSlot } from "./pileSlot";

// Trois miniatures de 100 px, 20 px d'écart, le long de l'axe.
const rects = [0, 120, 240].map((p) => ({ top: p, left: p, width: 100, height: 100 }));

describe("dropSlot", () => {
  test("dropSlot vertical above the middle of the second thumb returns 1", () => {
    expect(dropSlot(rects, 140, "y", false)).toBe(1);
  });

  test("dropSlot vertical past the last middle returns the count", () => {
    expect(dropSlot(rects, 300, "y", false)).toBe(3);
  });

  test("dropSlot horizontal left of the second middle returns 1", () => {
    expect(dropSlot(rects, 160, "x", false)).toBe(1);
  });

  test("dropSlot horizontal right to left reads the strip from the right", () => {
    // De droite à gauche, la première miniature est la plus à droite.
    const rtl = [...rects].reverse();
    expect(dropSlot(rtl, 200, "x", true)).toBe(1);
  });

  test("dropSlot horizontal right to left past the leftmost middle returns the count", () => {
    expect(dropSlot([...rects].reverse(), 10, "x", true)).toBe(3);
  });
});
