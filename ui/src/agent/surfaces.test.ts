import { describe, expect, test } from "bun:test";
import { agentBusy, byPlace, isAsking, routeSurfaces, suggestedPrompt, surfacesReducer } from "./surfaces";
import type { Surface } from "../api";

const s = (id: string, place: Surface["place"]): Surface => ({ id, place, messages: [] });

describe("surfaces de l'agent", () => {
  test("une surface s'ajoute puis se ferme", () => {
    const list = surfacesReducer([], { kind: "surface", surface: s("ask-1", "sheet") });
    expect(list.map((x) => x.id)).toEqual(["ask-1"]);
    expect(surfacesReducer(list, { kind: "close_surface", id: "ask-1" })).toEqual([]);
  });

  test("une surface de même identifiant est remplacée à sa place", () => {
    const before = [s("a", "postit"), s("show-1", "postit"), s("c", "postit")];
    const list = surfacesReducer(before, {
      kind: "surface",
      surface: { ...s("show-1", "postit"), anchor: { slide: "a", element: "b" } },
    });
    expect(list.map((x) => x.id)).toEqual(["a", "show-1", "c"]);
    expect(list[1].anchor).toEqual({ slide: "a", element: "b" });
  });

  test("fermer une surface inconnue ne change rien", () => {
    const list = [s("a", "postit")];
    expect(surfacesReducer(list, { kind: "close_surface", id: "z" })).toEqual(list);
  });

  test("byPlace garde l'ordre d'arrivée", () => {
    const list = [s("a", "postit"), s("b", "sheet"), s("c", "postit")];
    expect(byPlace(list, "postit").map((x) => x.id)).toEqual(["a", "c"]);
  });

  test("isAsking : une question ouverte, pas un post-it ni un tiroir", () => {
    expect(isAsking([s("a", "postit"), s("b", "drawer")])).toBe(false);
    expect(isAsking([s("a", "postit"), s("c", "sheet")])).toBe(true);
    expect(isAsking([s("m", "modal")])).toBe(true);
    expect(isAsking([s("t", "thread")])).toBe(true);
    expect(isAsking([])).toBe(false);
  });

  test("changed et focus ne touchent pas aux surfaces", () => {
    const list = [s("a", "postit")];
    expect(surfacesReducer(list, { kind: "changed" })).toBe(list);
    expect(surfacesReducer(list, { kind: "focus", slide: "s-1", element: null })).toBe(list);
  });

  test("surfaces remplace la liste entière", () => {
    const list = [s("a", "postit"), s("b", "sheet")];
    expect(surfacesReducer(list, { kind: "surfaces", list: [s("c", "modal")] }).map((x) => x.id)).toEqual(["c"]);
  });

  test("surfaces vide ferme tout", () => {
    expect(surfacesReducer([s("a", "postit"), s("b", "sheet")], { kind: "surfaces", list: [] })).toEqual([]);
  });

  test("agentBusy : un tour en cours sans question ouverte", () => {
    expect(agentBusy("working", [s("p", "postit")])).toBe(true);
    expect(agentBusy("starting", [])).toBe(true);
    expect(agentBusy("working", [s("q", "sheet")])).toBe(false);
    expect(agentBusy("idle", [])).toBe(false);
  });

  const sheet = (id: string, slide: string, element: string): Surface => ({ ...s(id, "sheet"), anchor: { slide, element } });
  const shown = { slide: "s-1", targets: ["e-1", "@title"] };

  test("routeSurfaces : une fiche dont l'élément est à l'écran va sur la scène", () => {
    const r = routeSurfaces([sheet("q", "s-1", "e-1")], shown);
    expect(r.stage?.id).toBe("q");
    expect(r.thread).toEqual([]);
  });

  test("routeSurfaces : une fiche d'une autre slide, d'un élément disparu ou sans ancre va au fil", () => {
    const list = [sheet("a", "s-2", "e-1"), sheet("b", "s-1", "e-9"), s("c", "sheet")];
    const r = routeSurfaces(list, shown);
    expect(r.stage).toBeUndefined();
    expect(r.thread.map((x) => x.id)).toEqual(["a", "b", "c"]);
  });

  test("routeSurfaces : sans slide affichée, toutes les fiches vont au fil", () => {
    expect(routeSurfaces([sheet("q", "s-1", "e-1")], null).thread.map((x) => x.id)).toEqual(["q"]);
  });

  test("routeSurfaces : une seule fiche sur la scène, la plus ancienne ; les suivantes au fil", () => {
    const r = routeSurfaces([sheet("a", "s-1", "e-1"), sheet("b", "s-1", "@title")], shown);
    expect(r.stage?.id).toBe("a");
    expect(r.thread.map((x) => x.id)).toEqual(["b"]);
  });

  test("routeSurfaces : la fenêtre la plus ancienne d'abord (une question en attente n'est pas remplacée)", () => {
    const r = routeSurfaces([s("m1", "modal"), s("m2", "modal")], shown);
    expect(r.modal?.id).toBe("m1");
  });

  test("routeSurfaces : le tiroir le plus récent ; fil et post-its dans l'ordre d'arrivée", () => {
    const r = routeSurfaces([s("d1", "drawer"), s("t", "thread"), s("p", "postit"), s("d2", "drawer")], shown);
    expect(r.drawer?.id).toBe("d2");
    expect(r.thread.map((x) => x.id)).toEqual(["t", "p"]);
  });

  test("isAsking : des suites proposées ne sont pas une question", () => {
    expect(isAsking([s("n", "suggestions")])).toBe(false);
  });

  test("routeSurfaces : les suites vont au fil, à leur place d'arrivée", () => {
    const r = routeSurfaces([s("p", "postit"), s("n", "suggestions")], shown);
    expect(r.thread.map((x) => x.id)).toEqual(["p", "n"]);
  });

  test("suggestedPrompt : la consigne d'une suite cliquée", () => {
    expect(suggestedPrompt({ name: "suggest", context: { prompt: "En barres" } })).toBe("En barres");
  });

  test("suggestedPrompt : rien pour une autre action ou une consigne vide", () => {
    expect(suggestedPrompt({ name: "choose", context: { prompt: "x" } })).toBeNull();
    expect(suggestedPrompt({ name: "suggest", context: { prompt: "  " } })).toBeNull();
    expect(suggestedPrompt({ name: "suggest", context: {} })).toBeNull();
  });
});
