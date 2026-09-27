import { describe, expect, test } from "bun:test";
import { agentUi } from "./ui";

describe("contexte de la consigne", () => {
  test("un champ sélectionné (@title) devient field", () => {
    expect(agentUi("s-1", "@title", null)).toEqual({ slide: "s-1", element: null, field: "title", editing: false });
  });

  test("un élément en édition", () => {
    expect(agentUi("s-1", "e-2", "e-2")).toEqual({ slide: "s-1", element: "e-2", field: null, editing: true });
  });

  test("un champ en édition", () => {
    expect(agentUi("s-1", "@title", "@title")).toEqual({ slide: "s-1", element: null, field: "title", editing: true });
  });

  test("une édition ailleurs que la sélection n'est pas l'édition en cours", () => {
    expect(agentUi("s-1", "e-2", "e-3").editing).toBe(false);
  });

  test("un @ seul n'est ni un champ ni un élément", () => {
    expect(agentUi("s-1", "@", null)).toEqual({ slide: "s-1", element: null, field: null, editing: false });
  });

  test("rien de sélectionné", () => {
    expect(agentUi(null, null, null)).toEqual({ slide: null, element: null, field: null, editing: false });
  });
});
