// Quelle surface reçoit les touches rapides.

import { describe, expect, test } from "bun:test";
import type { Surface } from "../api";
import { realSurface as real } from "../a2ui/fixtures/agentSurfaces";
import { quickTarget } from "./useQuickKeys";

const ask = (id: string): Surface => ({ ...real("ask-8"), id });
const settings = { ...real("show-6"), place: "thread", live: true, question: false } as Surface;

describe("quickTarget", () => {
  test("la fenêtre d'abord, puis la fiche", () => {
    const [modal, sheet] = [real("ask-2"), real("ask-1")];
    expect(quickTarget({ modal, sheet, drawer: real("show-6"), dock: [ask("ask-8")] })).toEqual({ surface: modal, kind: "modal" });
    expect(quickTarget({ sheet, drawer: real("show-6"), dock: [ask("ask-8")] })).toEqual({ surface: sheet, kind: "sheet" });
  });

  test("une question du dock passe avant le tiroir : elle bloque l'agent", () => {
    const q = ask("ask-8");
    expect(quickTarget({ drawer: real("show-6"), dock: [settings, q] })).toEqual({ surface: q, kind: "dock" });
  });

  test("plusieurs questions au dock : la dernière, au plus près du micro", () => {
    const last = ask("ask-12");
    expect(quickTarget({ dock: [settings, ask("ask-11"), last] })).toEqual({ surface: last, kind: "dock" });
  });

  test("des réglages seuls au dock ne prennent pas les touches", () => {
    expect(quickTarget({ dock: [settings] })).toBeNull();
    expect(quickTarget({ drawer: real("show-6"), dock: [settings] })).toEqual({ surface: real("show-6"), kind: "drawer" });
  });
});
