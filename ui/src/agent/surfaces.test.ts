import { describe, expect, test } from "bun:test";
import { agentBusy, byPlace, isAsking, isQuestion, routeSurfaces, sendThenPrompt, suggestedPrompt, surfacesReducer } from "./surfaces";
import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import type { Surface } from "../api";

const s = (id: string, place: Surface["place"]): Surface => ({ id, place, live: false, question: false, messages: [] });

describe("surfaces de l'agent", () => {
  test("une surface s'ajoute puis se ferme", () => {
    const list = surfacesReducer([], { kind: "surface", surface: s("ask-1", "sheet") });
    expect(list.map((x) => x.id)).toEqual(["ask-1"]);
    expect(surfacesReducer(list, { kind: "close_surface", id: "ask-1" })).toEqual([]);
  });

  test("une surface de même identifiant est remplacée à sa place", () => {
    const before = [s("a", "thread"), s("show-1", "thread"), s("c", "thread")];
    const list = surfacesReducer(before, {
      kind: "surface",
      surface: { ...s("show-1", "thread"), anchor: { slide: "a", element: "b" } },
    });
    expect(list.map((x) => x.id)).toEqual(["a", "show-1", "c"]);
    expect(list[1].anchor).toEqual({ slide: "a", element: "b" });
  });

  test("fermer une surface inconnue ne change rien", () => {
    const list = [s("a", "thread")];
    expect(surfacesReducer(list, { kind: "close_surface", id: "z" })).toEqual(list);
  });

  test("byPlace garde l'ordre d'arrivée", () => {
    const list = [s("a", "thread"), s("b", "sheet"), s("c", "thread")];
    expect(byPlace(list, "thread").map((x) => x.id)).toEqual(["a", "c"]);
  });

  test("isAsking : une question ouverte, pas un tiroir", () => {
    expect(isAsking([s("show-1", "thread"), s("b", "drawer")])).toBe(false);
    expect(isAsking([s("a", "thread"), s("c", "sheet")])).toBe(true);
    expect(isAsking([s("m", "modal")])).toBe(true);
    expect(isAsking([])).toBe(false);
  });

  test("isAsking : des réglages vivants du fil ne sont pas une question", () => {
    expect(isAsking([{ ...s("show-1", "thread"), live: true }])).toBe(false);
  });

  test("isAsking : une question vivante du fil en est une, figée non", () => {
    expect(isAsking([{ ...s("ask-1", "thread"), live: true, question: true }])).toBe(true);
    expect(isAsking([{ ...s("ask-1", "thread"), question: true }])).toBe(false);
  });

  test("changed et focus ne touchent pas aux surfaces", () => {
    const list = [s("a", "thread")];
    expect(surfacesReducer(list, { kind: "changed" })).toBe(list);
    expect(surfacesReducer(list, { kind: "focus", slide: "s-1", element: null })).toBe(list);
  });

  test("surfaces remplace la liste entière", () => {
    const list = [s("a", "thread"), s("b", "sheet")];
    expect(surfacesReducer(list, { kind: "surfaces", list: [s("c", "modal")] }).map((x) => x.id)).toEqual(["c"]);
  });

  test("surfaces vide ferme tout", () => {
    expect(surfacesReducer([s("a", "thread"), s("b", "sheet")], { kind: "surfaces", list: [] })).toEqual([]);
  });

  test("agentBusy : un tour en cours sans question ouverte", () => {
    expect(agentBusy("working", [s("p", "thread")])).toBe(true);
    expect(agentBusy("starting", [])).toBe(true);
    expect(agentBusy("working", [s("q", "sheet")])).toBe(false);
    expect(agentBusy("idle", [])).toBe(false);
  });

  const sheet = (id: string, slide: string, element: string): Surface => ({ ...s(id, "sheet"), anchor: { slide, element } });
  const shown = { slide: "s-1", targets: ["e-1", "@title"] };

  test("routeSurfaces : une fiche dont l'élément est à l'écran va sur la scène", () => {
    const r = routeSurfaces([sheet("q", "s-1", "e-1")], shown);
    expect(r.stage?.id).toBe("q");
    expect(r.loose).toEqual([]);
  });

  test("routeSurfaces : une fiche d'une autre slide, d'un élément disparu ou sans ancre va au fil", () => {
    const list = [sheet("a", "s-2", "e-1"), sheet("b", "s-1", "e-9"), s("c", "sheet")];
    const r = routeSurfaces(list, shown);
    expect(r.stage).toBeUndefined();
    expect(r.loose.map((x) => x.id)).toEqual(["a", "b", "c"]);
  });

  test("routeSurfaces : sans slide affichée, toutes les fiches vont au fil", () => {
    expect(routeSurfaces([sheet("q", "s-1", "e-1")], null).loose.map((x) => x.id)).toEqual(["q"]);
  });

  test("routeSurfaces : une seule fiche sur la scène, la plus ancienne ; les suivantes au fil", () => {
    const r = routeSurfaces([sheet("a", "s-1", "e-1"), sheet("b", "s-1", "@title")], shown);
    expect(r.stage?.id).toBe("a");
    expect(r.loose.map((x) => x.id)).toEqual(["b"]);
  });

  test("routeSurfaces : la fenêtre la plus ancienne d'abord (une question en attente n'est pas remplacée)", () => {
    const r = routeSurfaces([s("m1", "modal"), s("m2", "modal")], shown);
    expect(r.modal?.id).toBe("m1");
  });

  test("routeSurfaces : le tiroir le plus récent ; fil et surfaces dans l'ordre d'arrivée", () => {
    const r = routeSurfaces([s("d1", "drawer"), s("t", "thread"), s("p", "thread"), s("d2", "drawer")], shown);
    expect(r.drawer?.id).toBe("d2");
    expect(r.loose.map((x) => x.id)).toEqual(["t", "p"]);
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

test("surface_patch ajoute les messages et garde les précédents tels quels", () => {
  const s = { id: "turn-1", place: "thread", live: false, turn: 1, messages: [{ version: "v0.9", createSurface: { surfaceId: "turn-1", catalogId: "x" } }] } as unknown as Surface;
  const patch = { version: "v0.9", updateDataModel: { surfaceId: "turn-1", path: "/narration", value: "Je lis" } } as unknown as A2uiMessage;
  const [next] = surfacesReducer([s], { kind: "surface_patch", id: "turn-1", messages: [patch] });
  expect(next.messages.length).toBe(2);
  expect(next.messages[0]).toBe(s.messages[0]);
});

/** Une surface du fil (`thread`), figée par défaut. */
const ts = (id: string, extra: Partial<Surface>): Surface => ({ id, place: "thread", live: false, question: false, messages: [], ...extra });

test("routeSurfaces met au dock les surfaces vivantes du fil, la question en dernier", () => {
  const params = ts("show-1", { live: true, turn: 1 });
  const ask = ts("ask-2", { live: true, turn: 1, question: true });
  const r = routeSurfaces([ask, params], null);
  expect(r.dock.map((x) => x.id)).toEqual(["show-1", "ask-2"]);
});

test("routeSurfaces range les surfaces figées par tour, celle du tour d'abord", () => {
  const r = routeSurfaces([ts("report-3", { turn: 1 }), ts("turn-2", { turn: 2 }), ts("turn-1", { turn: 1 })], null);
  expect(r.turns.map((t) => t.turn)).toEqual([1, 2]);
  expect(r.turns[0].surfaces.map((x) => x.id)).toEqual(["turn-1", "report-3"]);
});

test("routeSurfaces garde visible une surface figée sans tour", () => {
  const lost = ts("render-4", {});
  expect(routeSurfaces([lost], null).loose).toEqual([lost]);
});

test("routeSurfaces garde les fiches hors scène dans le fil", () => {
  const sheet = ts("ask-4", { place: "sheet", live: true, anchor: { slide: "s-1", element: "e-1" } });
  expect(routeSurfaces([sheet], null).loose).toEqual([sheet]);
});

test("isAsking voit une question vivante du fil, pas des réglages", () => {
  expect(isAsking([ts("ask-1", { live: true, turn: 1, question: true })])).toBe(true);
  expect(isAsking([ts("show-1", { live: true, turn: 1 })])).toBe(false);
  expect(isAsking([ts("report-1", { turn: 1 })])).toBe(false);
});

test("isQuestion lit le champ typé `question`, pas le préfixe de l'identifiant", () => {
  expect(isQuestion(ts("ask-1", { live: true, question: false }))).toBe(false);
  expect(isQuestion(ts("confirm-1", { live: true, question: true }))).toBe(true);
  expect(isAsking([ts("ask-1", { live: true, turn: 1, question: false })])).toBe(false);
  expect(isAsking([ts("x-1", { live: true, turn: 1, question: true })])).toBe(true);
});

test("surface_patch d'une surface inconnue ne change rien", () => {
  const list = [ts("turn-1", { turn: 1 })];
  expect(surfacesReducer(list, { kind: "surface_patch", id: "nope", messages: [] })).toEqual(list);
});

describe("sendThenPrompt", () => {
  const suggest = { name: "suggest", context: { prompt: "En barres" } };
  test("envoie la consigne une fois l'action acceptée", async () => {
    const sent: string[] = [];
    await sendThenPrompt(suggest, async () => true, (t) => sent.push(t));
    expect(sent).toEqual(["En barres"]);
  });
  test("n'envoie rien quand le hub refuse l'action", async () => {
    const sent: string[] = [];
    await sendThenPrompt(suggest, async () => false, (t) => sent.push(t));
    expect(sent).toEqual([]);
  });
  test("une autre action n'envoie jamais de consigne", async () => {
    const sent: string[] = [];
    await sendThenPrompt({ name: "choose", context: { prompt: "x" } }, async () => true, (t) => sent.push(t));
    expect(sent).toEqual([]);
  });
});
