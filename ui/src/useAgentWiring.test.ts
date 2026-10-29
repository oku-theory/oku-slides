import { describe, expect, test } from "bun:test";
import type { SlideView } from "./api";
import { routeSurfaces } from "./agent/surfaces";
import { stageAnchors } from "./useAgentWiring";

const view = { slide: { id: "s-1" }, elements: [] } as unknown as SlideView;
const sheet = {
  id: "q", place: "sheet", live: false, question: true, messages: [], anchor: { slide: "s-1", owner: "e-1" },
} as const;

describe("the agent's anchored cards", () => {
  test("stageAnchors in a pack tab places no card on the stage", () => {
    const shown = stageAnchors(view, "s-1", true);
    expect(shown).toBeNull();
    expect(routeSurfaces([{ ...sheet }], shown).loose.map((x) => x.id)).toEqual(["q"]);
  });

  test("stageAnchors on the slide shown places the cards there", () => {
    expect(stageAnchors(view, "s-1", false)?.slide).toBe("s-1");
  });

  test("stageAnchors on another slide than the view places nothing", () => {
    expect(stageAnchors(view, "s-2", false)).toBeNull();
  });
});
