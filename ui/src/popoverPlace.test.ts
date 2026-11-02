import { describe, expect, test } from "bun:test";
import { popoverPlace } from "./popoverPlace";

describe("popoverPlace", () => {
  test("popoverPlace : sous la cellule, ramené à gauche plutôt que de déborder du tableau", () => {
    const box = { left: 100, top: 50, width: 300 } as DOMRect;
    expect(popoverPlace(box, { left: 150, bottom: 90 } as DOMRect, 200)).toEqual({ left: 50, top: 40 });
    expect(popoverPlace(box, { left: 350, bottom: 120 } as DOMRect, 200)).toEqual({ left: 100, top: 70 });
    expect(popoverPlace(box, { left: 100, bottom: 90 } as DOMRect, 400)).toEqual({ left: 0, top: 40 });
  });

  test("popoverPlace with room below puts the box under its anchor", () => {
    const box = { left: 0, top: 0, width: 800, height: 600 } as DOMRect;
    expect(popoverPlace(box, { left: 100, top: 80, bottom: 100 } as DOMRect, 200, 150)).toEqual({ left: 100, top: 100 });
  });

  test("popoverPlace with an anchor near the bottom puts the box above it", () => {
    const box = { left: 0, top: 0, width: 800, height: 600 } as DOMRect;
    expect(popoverPlace(box, { left: 100, top: 560, bottom: 590 } as DOMRect, 200, 150)).toEqual({ left: 100, top: 410 });
  });

  test("popoverPlace with no room above nor below keeps the box inside the viewport", () => {
    const box = { left: 0, top: 0, width: 800, height: 300 } as DOMRect;
    expect(popoverPlace(box, { left: 100, top: 100, bottom: 250 } as DOMRect, 200, 280)).toEqual({ left: 100, top: 0 });
  });
});
