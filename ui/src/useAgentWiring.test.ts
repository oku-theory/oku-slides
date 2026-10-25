import { describe, expect, test } from "bun:test";
import type { SlideView } from "./api";
import { routeSurfaces } from "./agent/surfaces";
import { stageAnchors } from "./useAgentWiring";

const view = { slide: { id: "s-1" }, elements: [] } as unknown as SlideView;
const sheet = {
  id: "q", place: "sheet", live: false, question: true, messages: [], anchor: { slide: "s-1", owner: "e-1" },
} as const;

describe("fiches ancrées de l'agent", () => {
  test("stageAnchors : sous un widget de l'atelier, la scène est cachée et aucune fiche n'y est posée", () => {
    const shown = stageAnchors(view, "s-1", true);
    expect(shown).toBeNull();
    expect(routeSurfaces([{ ...sheet }], shown).loose.map((x) => x.id)).toEqual(["q"]);
  });

  test("stageAnchors : scène visible sur la slide affichée, les fiches s'y posent", () => {
    expect(stageAnchors(view, "s-1", false)?.slide).toBe("s-1");
  });

  test("stageAnchors : sur une autre slide que la vue, rien n'est posé", () => {
    expect(stageAnchors(view, "s-2", false)).toBeNull();
  });
});
