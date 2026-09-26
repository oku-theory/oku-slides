import { describe, expect, test } from "bun:test";
import { Latest } from "./latest";

describe("Latest", () => {
  test("accept : réponses dans l'ordre des demandes, toutes appliquées", () => {
    const l = new Latest();
    const a = l.begin();
    const b = l.begin();
    expect(l.accept(a)).toBe(true);
    expect(l.accept(b)).toBe(true);
  });

  test("accept : réponse ancienne arrivée après une plus récente, écartée", () => {
    const l = new Latest();
    const old = l.begin();
    const recent = l.begin();
    expect(l.accept(recent)).toBe(true);
    expect(l.accept(old)).toBe(false);
  });

  test("accept : la même réponse deux fois, écartée la seconde", () => {
    const l = new Latest();
    const a = l.begin();
    expect(l.accept(a)).toBe(true);
    expect(l.accept(a)).toBe(false);
  });
});
