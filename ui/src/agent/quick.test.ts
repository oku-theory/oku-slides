// Touches rapides des surfaces de l'agent et position de la fiche.

import { describe, expect, test } from "bun:test";
import { quickAction, quickKey, sheetPosition } from "./quick";
import type { Surface } from "../api";
import { realSurface as real } from "../a2ui/fixtures/agentSurfaces";

const surface = (components: unknown[], data?: unknown, more: unknown[] = []): Surface => ({
  id: "ask-1",
  place: "sheet",
  live: false,
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

const rewrite = surface([
  { id: "pick-0", component: "Button", label: "Choisir 1", primary: true, action: { event: { name: "rewrite", context: { kept: 0, text: "Un" } } } },
  { id: "pick-1", component: "Button", label: "Choisir 2", action: { event: { name: "rewrite", context: { kept: 1, text: "Deux" } } } },
]);

const variants = surface([
  { id: "v0", component: "SlideThumb", svg: "", label: "1", action: { event: { name: "variant", context: { index: 0 } } } },
  { id: "v1", component: "SlideThumb", svg: "", label: "2", action: { event: { name: "variant", context: { index: 1 } } } },
]);

describe("touches des surfaces", () => {
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
  test("sous la boîte, alignée à gauche, à l'échelle", () => {
    expect(sheetPosition({ x: 100, y: 200, w: 400, h: 80 }, 0.5, false)).toEqual({ left: 50, top: 148 });
  });
  test("en arabe, alignée sur le bord droit de la boîte", () => {
    expect(sheetPosition({ x: 100, y: 200, w: 400, h: 80 }, 0.5, true)).toEqual({ left: 250, top: 148 });
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
