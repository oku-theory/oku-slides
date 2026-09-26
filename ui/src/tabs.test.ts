import { describe, expect, test } from "bun:test";
import { cycle, finalIndex, nth, outside, slotAt, tabShortcut } from "./tabs";

const key = (code: string, key: string, mods: Partial<{ metaKey: boolean; ctrlKey: boolean; shiftKey: boolean; altKey: boolean; repeat: boolean }> = {}) => ({
  code,
  key,
  metaKey: false,
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  repeat: false,
  ...mods,
});

describe("barre d'onglets", () => {
  test("slotAt compte les milieux à gauche du pointeur", () => {
    expect(slotAt([50, 150, 250], 10)).toBe(0);
    expect(slotAt([50, 150, 250], 150)).toBe(1);
    expect(slotAt([50, 150, 250], 400)).toBe(3);
  });

  test("slotAt en RTL : « avant » se lit à droite du pointeur", () => {
    // Même ordre logique (index 0, 1, 2), mais affichés de droite à gauche :
    // le premier onglet (index 0) est le plus à droite (mid 250).
    expect(slotAt([250, 150, 50], 400, true)).toBe(0);
    expect(slotAt([250, 150, 50], 150, true)).toBe(1);
    expect(slotAt([250, 150, 50], 10, true)).toBe(3);
  });

  test("finalIndex tient compte de l'onglet retiré avant sa place", () => {
    expect(finalIndex(0, 3)).toBe(2);
    expect(finalIndex(2, 0)).toBe(0);
    expect(finalIndex(1, 1)).toBe(1);
  });

  test("cycle boucle dans les deux sens et reste sur place si rien ne va", () => {
    expect(cycle(["a", "b", "c"], "c", 1)).toBe("a");
    expect(cycle(["a", "b", "c"], "a", -1)).toBe("c");
    expect(cycle(["a"], "a", 1)).toBe("a");
    expect(cycle(["a", "b"], "z", 1)).toBeUndefined();
    expect(cycle([], "a", 1)).toBeUndefined();
  });

  test("nth donne l'onglet n, et 9 le dernier", () => {
    expect(nth(["a", "b", "c"], 2)).toBe("b");
    expect(nth(["a", "b", "c"], 9)).toBe("c");
    expect(nth(["a", "b", "c"], 5)).toBeUndefined();
  });

  test("tabShortcut reconnaît ⌘T, ⌘W, ⌘⇧N et ⌘1 sur Mac", () => {
    expect(tabShortcut(key("KeyT", "t", { metaKey: true }), true)).toEqual({ kind: "new-tab" });
    expect(tabShortcut(key("KeyW", "w", { metaKey: true }), true)).toEqual({ kind: "close-tab" });
    expect(tabShortcut(key("KeyN", "N", { metaKey: true, shiftKey: true }), true)).toEqual({ kind: "new-window" });
    expect(tabShortcut(key("Digit1", "1", { metaKey: true }), true)).toEqual({ kind: "nth", n: 1 });
  });

  test("tabShortcut en AZERTY : chiffres par touche physique, lettres par caractère", () => {
    // ⌘1 : la touche « & / 1 » donne `key` « & ».
    expect(tabShortcut(key("Digit1", "&", { metaKey: true }), true)).toEqual({ kind: "nth", n: 1 });
    // ⌘Z (annuler) se tape sur la touche physique `KeyW` : jamais fermer l'onglet.
    expect(tabShortcut(key("KeyW", "z", { metaKey: true }), true)).toBeNull();
    // ⌘W se tape sur la touche physique `KeyZ`.
    expect(tabShortcut(key("KeyZ", "w", { metaKey: true }), true)).toEqual({ kind: "close-tab" });
  });

  test("tabShortcut ignore les répétitions, Alt, ⌘⇧T et ⌘⇧W", () => {
    expect(tabShortcut(key("KeyW", "w", { metaKey: true, repeat: true }), true)).toBeNull();
    expect(tabShortcut(key("KeyT", "t", { metaKey: true, altKey: true }), true)).toBeNull();
    expect(tabShortcut(key("KeyT", "T", { metaKey: true, shiftKey: true }), true)).toBeNull();
    expect(tabShortcut(key("KeyW", "W", { metaKey: true, shiftKey: true }), true)).toBeNull();
  });

  test("tabShortcut prend Ctrl hors de macOS et ignore ⌘ seul", () => {
    expect(tabShortcut(key("KeyT", "t", { ctrlKey: true }), false)).toEqual({ kind: "new-tab" });
    expect(tabShortcut(key("KeyT", "t", { metaKey: true }), false)).toBeNull();
    expect(tabShortcut(key("Digit2", "2", { metaKey: true }), false)).toBeNull();
  });

  test("tabShortcut : Ctrl+Tab et Ctrl+Maj+Tab changent d'onglet partout", () => {
    expect(tabShortcut(key("Tab", "Tab", { ctrlKey: true }), true)).toEqual({ kind: "cycle", delta: 1 });
    expect(tabShortcut(key("Tab", "Tab", { ctrlKey: true, shiftKey: true }), true)).toEqual({ kind: "cycle", delta: -1 });
  });

  test("tabShortcut : ⌘⇧1 (Maj) n'est pas ⌘1", () => {
    expect(tabShortcut(key("Digit1", "1", { metaKey: true, shiftKey: true }), true)).toBeNull();
  });

  test("tabShortcut : ⌘N sans Maj ne fait rien (seul ⌘⇧N ouvre une fenêtre)", () => {
    expect(tabShortcut(key("KeyN", "n", { metaKey: true }), true)).toBeNull();
  });

  test("outside : hors de la fenêtre dès le bord", () => {
    expect(outside(10, 10, 100, 100)).toBe(false);
    expect(outside(-1, 10, 100, 100)).toBe(true);
    expect(outside(10, 100, 100, 100)).toBe(true);
  });
});
