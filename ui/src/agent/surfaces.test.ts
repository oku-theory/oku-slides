import { describe, expect, test } from "bun:test";
import { HOST_PACKS, HOST_VOICE, byPlace, isHostSettings, isRequestedBox, isQuestion, routeStaged, routeSurfaces, sendThenPrompt, suggestedPrompt, surfacesReducer } from "./surfaces";
import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import type { Surface } from "../api";
import { realSurface as real } from "../a2ui/fixtures/agentSurfaces";

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
      surface: { ...s("show-1", "thread"), anchor: { slide: "a", owner: "b" } },
    });
    expect(list.map((x) => x.id)).toEqual(["a", "show-1", "c"]);
    expect(list[1].anchor).toEqual({ slide: "a", owner: "b" });
  });

  test("fermer une surface inconnue ne change rien", () => {
    const list = [s("a", "thread")];
    expect(surfacesReducer(list, { kind: "close_surface", id: "z" })).toEqual(list);
  });

  test("byPlace garde l'ordre d'arrivée", () => {
    const list = [s("a", "thread"), s("b", "sheet"), s("c", "thread")];
    expect(byPlace(list, "thread").map((x) => x.id)).toEqual(["a", "c"]);
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

  test("une resynchronisation garde l'objet d'une surface inchangée : sa vue n'est pas reconstruite", () => {
    const [a, b] = [s("a", "thread"), { ...s("b", "thread"), live: true }];
    const [sameA, frozenB] = surfacesReducer([a, b], { kind: "surfaces", list: [structuredClone(a), { ...structuredClone(b), live: false }] });
    expect(sameA).toBe(a);
    // Changée (ici figée) : le nouvel objet.
    expect(frozenB).not.toBe(b);
    expect(frozenB.live).toBe(false);
  });

  test("surfaces vide ferme tout", () => {
    expect(surfacesReducer([s("a", "thread"), s("b", "sheet")], { kind: "surfaces", list: [] })).toEqual([]);
  });

  const sheet = (id: string, slide: string, owner: string, field?: string): Surface => ({ ...s(id, "sheet"), anchor: { slide, owner, field } });
  const onStage = new Set(["e-1", "s-1/title"]);
  const shown = { slide: "s-1", has: (a: { owner: string; field?: string }) => onStage.has(a.field ? `${a.owner}/${a.field}` : a.owner) };

  test("routeStaged la fiche posée sur la scène n'est pas dans le fil", () => {
    const routed = routeStaged([sheet("s-a", "s1", "e1"), sheet("s-b", "s2", "e2")], "s-a");
    expect(routed.loose.map((x) => x.id)).toEqual(["s-b"]);
  });

  test("routeStaged la fiche posée est rendue comme fiche de la scène", () => {
    const routed = routeStaged([sheet("s-a", "s1", "e1")], "s-a");
    expect(routed.stage?.id).toBe("s-a");
  });

  test("routeStaged sans fiche posée : toutes les fiches vont au fil", () => {
    expect(routeStaged([sheet("s-a", "s1", "e1")], null).loose.map((x) => x.id)).toEqual(["s-a"]);
  });

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
    const r = routeSurfaces([sheet("a", "s-1", "e-1"), sheet("b", "s-1", "s-1", "title")], shown);
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
  const sheet = ts("ask-4", { place: "sheet", live: true, anchor: { slide: "s-1", owner: "e-1" } });
  expect(routeSurfaces([sheet], null).loose).toEqual([sheet]);
});

test("isQuestion lit le champ typé `question`, pas le préfixe de l'identifiant", () => {
  expect(isQuestion(ts("ask-1", { live: true, question: false }))).toBe(false);
  expect(isQuestion(ts("confirm-1", { live: true, question: true }))).toBe(true);
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
  test("une suite est une nouvelle consigne : elle ne répond à aucune question en attente", async () => {
    const sent: [string, string | null | undefined][] = [];
    await sendThenPrompt(suggest, async () => true, (t, answer) => sent.push([t, answer]));
    expect(sent).toEqual([["En barres", null]]);
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

test("isHostSettings ne compte pas le sélecteur de thème", () => {
  expect(isHostSettings(real("host-theme-picker"))).toBe(false);
});

test("isHostSettings compte les réglages de l'hôte", () => {
  expect(isHostSettings(real("host-widget"))).toBe(true);
});

/** Le panneau des packs : la fixture n'en a pas, ses messages ne comptent pas ici. */
const packsPanel = () => ({ ...real("host-widget"), id: HOST_PACKS });

test("isHostSettings with the packs panel gives false", () => {
  expect(isHostSettings(packsPanel())).toBe(false);
});

test("isRequestedBox with the packs panel or the theme picker gives true", () => {
  expect([packsPanel(), ...["host-theme-picker", "host-widget", "ask-1"].map(real)].map(isRequestedBox)).toEqual([true, true, false, false]);
});

/** La boîte des réglages de la dictée et de l'agent. */
const voiceBox = () => ({ ...real("host-widget"), id: HOST_VOICE });

test("isRequestedBox with the voice box is true", () => {
  expect(isRequestedBox(voiceBox())).toBe(true);
});

test("isHostSettings with the voice box is false", () => {
  expect(isHostSettings(voiceBox())).toBe(false);
});
