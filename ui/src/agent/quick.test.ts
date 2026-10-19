// Touches rapides des surfaces de l'agent et position de la fiche.

import { describe, expect, test } from "bun:test";
import { quickAction, quickKey, sheetPlacement } from "./quick";
import type { Surface } from "../api";
import { realSurface as real } from "../a2ui/fixtures/agentSurfaces";

const surface = (components: unknown[], data?: unknown, more: unknown[] = []): Surface => ({
  id: "ask-1",
  place: "sheet",
  live: false,
  question: true,
  messages: [
    { version: "v0.9", createSurface: { surfaceId: "ask-1", catalogId: "x" } },
    { version: "v0.9", updateComponents: { surfaceId: "ask-1", components } },
    ...(data === undefined ? [] : [{ version: "v0.9", updateDataModel: { surfaceId: "ask-1", value: data } }]),
    ...more,
  ] as unknown as Surface["messages"],
});

const chips = (n: number) =>
  surface([
    {
      id: "options",
      component: "ChoiceChips",
      options: Array.from({ length: n }, (_, i) => ({ id: String.fromCharCode(97 + i), label: `${i}` })),
      value: { path: "/choice" },
      action: { event: { name: "choose", context: { choice: { path: "/choice" } } } },
    },
  ]);

/** Deux pastilles, `choice` déjà dans le modèle. */
const chosen = (choice: string): Surface => {
  const base = chips(2);
  const update = { version: "v0.9", updateDataModel: { surfaceId: "ask-1", value: { choice } } };
  return { ...base, messages: [...base.messages, update] as unknown as Surface["messages"] };
};

const rewrite = surface([
  { id: "pick-0", component: "Button", label: "Choisir 1", primary: true, action: { event: { name: "rewrite", context: { kept: 0, text: "Un" } } } },
  { id: "pick-1", component: "Button", label: "Choisir 2", action: { event: { name: "rewrite", context: { kept: 1, text: "Deux" } } } },
]);

const variants = surface([
  { id: "v0", component: "SlideThumb", svg: "", label: "1", action: { event: { name: "variant", context: { index: 0 } } } },
  { id: "v1", component: "SlideThumb", svg: "", label: "2", action: { event: { name: "variant", context: { index: 1 } } } },
]);

const themes = surface([
  { id: "c0", component: "ThemeChoice", name: "Papier", slide: "s-1", swatches: [], active: true, action: { event: { name: "theme", context: { theme: "id-1" } } } },
  { id: "c1", component: "ThemeChoice", name: "Pro", slide: "s-1", swatches: [], active: false, action: { event: { name: "theme", context: { theme: "id-2" } } } },
  { id: "close", component: "Button", label: "Fermer", action: { event: { name: "dismiss", context: {} } } },
]);

describe("touches des surfaces", () => {
  test("un chiffre choisit le thème correspondant", () => {
    expect(quickAction(themes, "2")).toEqual({ name: "theme", context: { theme: "id-2" } });
  });

  test("le bouton Fermer n'est pas numéroté", () => {
    expect(quickAction(themes, "3")).toBeNull();
  });

  test("un chiffre choisit l'option correspondante", () => {
    expect(quickAction(chips(2), "2")).toEqual({ name: "choose", context: { choice: "b" } });
  });
  test("un chiffre choisit la proposition de reformulation", () => {
    expect(quickAction(rewrite, "2")).toEqual({ name: "rewrite", context: { kept: 1, text: "Deux" } });
  });
  test("un chiffre choisit la variante", () => {
    expect(quickAction(variants, "1")).toEqual({ name: "variant", context: { index: 0 } });
  });
  test("Entrée déclenche le bouton principal", () => {
    expect(quickAction(rewrite, "Enter")).toEqual({ name: "rewrite", context: { kept: 0, text: "Un" } });
  });
  test("Entrée sans bouton principal ne fait rien", () => expect(quickAction(variants, "Enter")).toBeNull());
  // Aucune question à pastilles n'a d'option par défaut (`ask_choice` part de `""`) : Entrée ne
  // choisit jamais à la place de l'utilisateur, même si le modèle contient déjà une option.
  test("Entrée sur des pastilles ne choisit rien, même une option déjà dans le modèle", () => {
    expect(quickAction(chosen("b"), "Enter")).toBeNull();
    expect(quickAction(chosen(""), "Enter")).toBeNull();
  });
  test("Échap abandonne", () => expect(quickAction(chips(2), "Escape")).toEqual({ name: "dismiss", context: {} }));
  test("un chiffre hors des options ne fait rien", () => expect(quickAction(chips(2), "4")).toBeNull());
  test("seuls 1 à 4 sont des touches rapides", () => {
    expect(quickAction(chips(5), "4")).toEqual({ name: "choose", context: { choice: "d" } });
    expect(quickAction(chips(5), "5")).toBeNull();
  });
  test("des pastilles sans liste d'options ne lèvent pas", () => {
    expect(quickAction(surface([{ id: "o", component: "ChoiceChips", value: { path: "/c" }, action: { event: { name: "choose" } } }]), "1")).toBeNull();
  });
  test("une valeur liée au modèle est lue dans le modèle, comme au clic", () => {
    const s = surface(
      [{ id: "ok", component: "Button", label: "OK", primary: true, action: { event: { name: "rewrite", context: { kept: "edited", text: { path: "/draft" } } } } }],
      { draft: "Mon texte" },
    );
    expect(quickAction(s, "Enter")).toEqual({ name: "rewrite", context: { kept: "edited", text: "Mon texte" } });
  });
  test("une valeur liée absente du modèle n'envoie rien plutôt qu'une action incomplète", () => {
    const s = surface([{ id: "ok", component: "Button", label: "OK", primary: true, action: { event: { name: "x", context: { v: { path: "/nulle" } } } } }], {});
    expect(quickAction(s, "Enter")).toBeNull();
  });
  test("une mise à jour des composants remplace la version précédente", () => {
    const s = surface(rewriteComponents(), undefined, [
      { version: "v0.9", updateComponents: { surfaceId: "ask-1", components: [{ id: "pick-1", component: "Button", label: "B", primary: true, action: { event: { name: "rewrite", context: { kept: 1 } } } }] } },
    ]);
    expect(quickAction(s, "Enter")?.context).toEqual({ kept: 0 });
    expect(quickAction(s, "2")?.context).toEqual({ kept: 1 });
  });
});

function rewriteComponents() {
  return [
    { id: "pick-0", component: "Button", label: "A", primary: true, action: { event: { name: "rewrite", context: { kept: 0 } } } },
    { id: "pick-1", component: "Button", label: "B", action: { event: { name: "rewrite", context: { kept: 9 } } } },
  ];
}

describe("touches des surfaces produites par la session", () => {
  test("choix : le nom et le contexte viennent de l'action des pastilles", () => {
    expect(quickAction(real("ask-1"), "1")).toEqual({ name: "choose", context: { choice: "a" } });
  });
  test("confirmation : Entrée confirme, 2 refuse", () => {
    expect(quickAction(real("ask-2"), "Enter")).toEqual({ name: "confirm", context: { confirmed: true } });
    expect(quickAction(real("ask-2"), "2")).toEqual({ name: "confirm", context: { confirmed: false } });
  });
  test("permission : les chiffres suivent l'ordre affiché, l'index reste celui de l'agent", () => {
    expect(quickAction(real("perm-3"), "1")).toEqual({ name: "permission", context: { index: 1 } });
    expect(quickAction(real("perm-3"), "2")).toEqual({ name: "permission", context: { index: 2 } });
    expect(quickAction(real("perm-3"), "3")).toEqual({ name: "permission", context: { index: 0 } });
  });
  test("reformulation : 2 garde la seconde proposition, 3 garde l'original", () => {
    expect(quickAction(real("ask-4"), "2")).toEqual({ name: "rewrite", context: { kept: 1, text: "Ce que nous visons" } });
    expect(quickAction(real("ask-4"), "3")).toEqual({ name: "rewrite", context: { kept: "original", text: "Nos objectifs pour 2027" } });
  });
  test("variantes : 2 choisit la seconde ; « Fermer » n'est pas numéroté", () => {
    expect(quickAction(real("show-6"), "2")).toEqual({ name: "variant", context: { index: 1 } });
    expect(quickAction(real("show-6"), "3")).toBeNull();
  });
});

describe("position de la fiche", () => {
  const anchor = { left: 50, top: 100, width: 200, height: 40 };
  const sheet = { width: 120, height: 80 };
  const zone = { left: 0, top: 0, right: 600, bottom: 400 };
  test("avant mesure : sous la boîte, alignée à gauche", () => {
    expect(sheetPlacement(anchor, null, null, false)).toEqual({ left: 50, top: 148, above: false });
  });
  test("avant mesure, en arabe : au bord droit de la boîte", () => {
    expect(sheetPlacement(anchor, null, null, true)).toEqual({ left: 250, top: 148, above: false });
  });
  test("mesurée, la place en dessous suffit : sous la boîte", () => {
    expect(sheetPlacement(anchor, sheet, zone, false)).toEqual({ left: 50, top: 148, above: false });
  });
  test("en arabe, mesurée : bord droit de la fiche sur le bord droit de la boîte", () => {
    expect(sheetPlacement(anchor, sheet, zone, true)).toEqual({ left: 130, top: 148, above: false });
  });
  test("près du bas : retournée au-dessus de la boîte", () => {
    const low = { ...anchor, top: 300 };
    expect(sheetPlacement(low, sheet, zone, false)).toEqual({ left: 50, top: 212, above: true });
  });
  test("ni dessus ni dessous : du côté le plus grand, recadrée dans la zone", () => {
    const tall = { width: 120, height: 390 };
    expect(sheetPlacement(anchor, tall, zone, false)).toEqual({ left: 50, top: 10, above: false });
    expect(sheetPlacement({ ...anchor, top: 300 }, tall, zone, false)).toEqual({ left: 50, top: 0, above: true });
  });
  test("près d'un bord : recadrée horizontalement dans la zone", () => {
    expect(sheetPlacement({ ...anchor, left: 550 }, sheet, zone, false).left).toBe(480);
    expect(sheetPlacement({ ...anchor, left: -40, width: 60 }, sheet, zone, true).left).toBe(0);
    expect(sheetPlacement({ ...anchor, left: -40 }, sheet, { ...zone, left: -20 }, false).left).toBe(-20);
  });
  test("plus large que la zone : collée à son bord gauche", () => {
    expect(sheetPlacement(anchor, { width: 700, height: 80 }, zone, false).left).toBe(0);
  });
});

const ev = (key: string, code: string, mods: Partial<Record<"metaKey" | "ctrlKey" | "altKey" | "repeat" | "isComposing", boolean>> = {}) => ({
  key,
  code,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  repeat: false,
  ...mods,
});

describe("touche rapide d'un évènement clavier", () => {
  test("les chiffres par leur position : 1–4 en AZERTY", () => {
    expect(quickKey(ev("é", "Digit2"))).toBe("2");
    expect(quickKey(ev("3", "Numpad3"))).toBe("3");
    expect(quickKey(ev("(", "Digit5"))).toBeNull();
  });
  test("E par sa position, même sur un clavier cyrillique", () => expect(quickKey(ev("у", "KeyE"))).toBe("e"));
  test("Entrée et Échap par leur nom", () => {
    expect(quickKey(ev("Enter", "NumpadEnter"))).toBe("Enter");
    expect(quickKey(ev("Escape", "Escape"))).toBe("Escape");
  });
  test("modificateurs, répétition et composition ne sont pas des touches rapides", () => {
    expect(quickKey(ev("1", "Digit1", { metaKey: true }))).toBeNull();
    expect(quickKey(ev("e", "KeyE", { ctrlKey: true }))).toBeNull();
    expect(quickKey(ev("1", "Digit1", { altKey: true }))).toBeNull();
    expect(quickKey(ev("Enter", "Enter", { repeat: true }))).toBeNull();
    expect(quickKey(ev("Enter", "Enter", { isComposing: true }))).toBeNull();
  });
});
