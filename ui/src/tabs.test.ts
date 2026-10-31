import { describe, expect, test } from "bun:test";
import { arrivalIndex, coalesce, cycle, finalIndex, menuAbove, menuBelow, nth, othersToClose, outside, slotAt, tabShortcut } from "./tabs";
import type { TabDocument, TabInfo } from "./workspaceApi";

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

  test("tabShortcut reconnaît ⌘? (QWERTY : ⌘⇧/) et ⌘/ pour l'aide", () => {
    expect(tabShortcut(key("Slash", "?", { metaKey: true, shiftKey: true }), true)).toEqual({ kind: "help" });
    expect(tabShortcut(key("Slash", "/", { metaKey: true }), true)).toEqual({ kind: "help" });
  });

  test("tabShortcut en AZERTY : ⌘? se tape sur la touche physique KeyM", () => {
    expect(tabShortcut(key("KeyM", "?", { metaKey: true, shiftKey: true }), true)).toEqual({ kind: "help" });
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

  test("menuAbove pose le menu au-dessus du pointeur", () => {
    expect(menuAbove(100, 700, 240, 160, 1440, 900)).toEqual({ x: 100, y: 540 });
  });

  test("menuBelow puts the menu under the pointer", () => {
    expect(menuBelow(100, 200, 240, 160, 1440, 900)).toEqual({ x: 100, y: 200 });
  });

  test("menuBelow keeps a menu near the bottom inside the window", () => {
    expect(menuBelow(1400, 850, 240, 160, 1440, 900)).toEqual({ x: 1192, y: 732 });
  });

  test("menuAbove garde le menu dans la fenêtre à droite et en haut", () => {
    expect(menuAbove(1400, 100, 240, 160, 1440, 900)).toEqual({ x: 1192, y: 8 });
  });

  test("coalesce ne lance qu'un appel à la fois et relance une fois ceux arrivés entre-temps", async () => {
    let calls = 0;
    const done: (() => void)[] = [];
    const { run } = coalesce(() => {
      calls += 1;
      return new Promise<void>((r) => done.push(r));
    });
    run();
    run();
    run();
    expect(calls).toBe(1);
    done[0]();
    await Promise.resolve();
    await Promise.resolve();
    expect(calls).toBe(2);
  });

  test("coalesce sans appel entre-temps ne relance pas", async () => {
    let calls = 0;
    const { run } = coalesce(async () => {
      calls += 1;
    });
    run();
    await Promise.resolve();
    await Promise.resolve();
    expect(calls).toBe(1);
  });

  test("coalesce continue après un appel en échec", async () => {
    let calls = 0;
    const { run } = coalesce(async () => {
      calls += 1;
      throw new Error("hôte occupé");
    });
    run();
    await Promise.resolve();
    await Promise.resolve();
    run();
    expect(calls).toBe(2);
  });

  test("coalesce idle attend la relance comprise", async () => {
    const order: string[] = [];
    const done: (() => void)[] = [];
    const { run, idle } = coalesce(() => new Promise<void>((r) => done.push(() => (order.push("appel"), r()))));
    run();
    run();
    const waited = idle().then(() => order.push("idle"));
    done[0]();
    await new Promise((r) => setTimeout(r, 0));
    done[1]();
    await waited;
    expect(order).toEqual(["appel", "appel", "idle"]);
  });

  test("arrivalIndex place l'onglet arrivé parmi les autres", () => {
    expect(arrivalIndex([50, 150, 250], 100)).toBe(1);
  });

  test("arrivalIndex au-delà du dernier onglet laisse l'onglet en fin de barre", () => {
    expect(arrivalIndex([50, 150, 250], 400)).toBeNull();
  });
});

describe("othersToClose", () => {
  const PACK = "a7e11e40-0000-4000-8000-000000000001";
  const tab = (id: string, document: TabDocument = { kind: "deck", path: null, deck_uid: null }): TabInfo => ({
    id, title: id, dirty: false, pristine: false, theme: null, document,
  });
  const linked = (id: string, owner: string) => tab(id, { kind: "deck_pack", deck_tab: owner, pack: PACK });

  test("closeOthers from a deck-pack tab keeps its owner", () => {
    const tabs = [tab("t-1"), linked("t-2", "t-1"), linked("t-3", "t-1"), tab("t-4")];
    expect(othersToClose(tabs, "t-2")).toEqual(["t-3", "t-4"]);
  });

  test("closeOthers from a deck tab closes its deck-pack tabs", () => {
    const tabs = [tab("t-1"), linked("t-2", "t-1"), tab("t-3")];
    expect(othersToClose(tabs, "t-1")).toEqual(["t-2", "t-3"]);
  });
});
