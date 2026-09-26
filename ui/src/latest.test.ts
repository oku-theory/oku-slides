import { describe, expect, test } from "bun:test";
import { Latest, newest } from "./latest";

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

describe("newest", () => {
  const v = (generation: number) => ({ generation });

  test("newest : sans vue montrée, la nouvelle est prise", () => {
    expect(newest(null, v(3))).toEqual(v(3));
  });

  test("newest : vue plus récente que celle montrée, prise", () => {
    expect(newest(v(3), v(4))).toEqual(v(4));
  });

  test("newest : vue plus ancienne (réponse doublée par un événement), écartée", () => {
    const shown = v(5);
    expect(newest(shown, v(4))).toBe(shown);
  });

  test("newest : même numéro, prise (relecture sans changement)", () => {
    const next = v(5);
    expect(newest(v(5), next)).toBe(next);
  });
});
