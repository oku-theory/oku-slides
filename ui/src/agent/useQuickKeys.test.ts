// Quelle surface reçoit les touches rapides.

import { describe, expect, test } from "bun:test";
import type { Surface } from "../api";
import { realSurface as real } from "../a2ui/fixtures/agentSurfaces";
import { agentBusy, routeSurfaces } from "./surfaces";
import { answerTarget, quickTarget } from "./useQuickKeys";

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

describe("answerTarget", () => {
  // Une consigne (dictée ou tapée) répond à la question qui reçoit les touches 1–4, jamais à une autre.
  const sheet = real("ask-1");
  const onStage = sheet.anchor ? { slide: sheet.anchor.slide, targets: [sheet.anchor.element] } : null;
  // Comme `App` : la fiche qui reçoit les touches est celle de la scène (`stage`).
  const target = (list: Surface[], shown = onStage) => {
    const routed = routeSurfaces(list, shown);
    return answerTarget(quickTarget({ ...routed, sheet: routed.stage }));
  };

  test("une fiche sur la slide affichée et une question au dock : la fiche, ouverte avant", () => {
    expect(target([sheet, ask("ask-8")])).toBe("ask-1");
  });

  test("une fiche hors de la slide affichée ne reçoit ni touches ni consigne : la question du dock", () => {
    expect(target([sheet, ask("ask-8")], null)).toBe("ask-8");
    expect(target([sheet], null)).toBeNull();
  });

  test("une fenêtre et une question au dock : la fenêtre", () => {
    expect(target([ask("ask-8"), real("ask-2")])).toBe("ask-2");
  });

  test("plusieurs questions au dock : la dernière, comme les touches", () => {
    expect(target([ask("ask-11"), ask("ask-12")])).toBe("ask-12");
  });

  test("sans question qui reçoit les touches, la consigne ne répond à rien", () => {
    expect(target([settings, real("show-6")])).toBeNull();
    expect(answerTarget(null)).toBeNull();
  });
});

describe("agentBusy", () => {
  // « L'agent attend l'utilisateur » = une question reçoit la consigne (`answerTarget`) : ⌘⇧D et
  // Échap y répondent ; sinon, pendant un tour, ils l'arrêtent. Une question invisible n'attend pas.
  const sheet = real("ask-1");
  const onStage = sheet.anchor ? { slide: sheet.anchor.slide, targets: [sheet.anchor.element] } : null;
  const busy = (list: Surface[], shown = onStage) => {
    const routed = routeSurfaces(list, shown);
    return agentBusy("working", answerTarget(quickTarget({ ...routed, sheet: routed.stage })));
  };

  test("une question qui reçoit les touches : l'agent attend (fiche, fenêtre, dock)", () => {
    expect(busy([sheet])).toBe(false);
    expect(busy([real("ask-2")])).toBe(false);
    expect(busy([ask("ask-8")])).toBe(false);
  });

  test("seule une fiche hors de la slide affichée attend : le tour est en cours (⌘⇧D l'arrête)", () => {
    expect(busy([sheet], null)).toBe(true);
  });

  test("un tiroir ou des réglages ne sont pas une question ; au repos, rien n'est en cours", () => {
    expect(busy([real("show-6"), settings])).toBe(true);
    expect(busy([])).toBe(true);
    expect(agentBusy("starting", null)).toBe(true);
    expect(agentBusy("idle", null)).toBe(false);
  });
});
