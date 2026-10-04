// Miniature de la slide d'un reçu, résolue par empreinte (t-450).

import { describe, expect, test } from "bun:test";
import { resolveSlideThumb, type SlideThumbs } from "./slideThumbs";

/** Des miniatures : `cache` par empreinte, `slides` (identifiant → empreinte actuelle). */
function thumbs(cache: Record<string, string>, slides: Record<string, string>): SlideThumbs & { asked: string[] } {
  const asked: string[] = [];
  return {
    asked,
    cached: (hash) => cache[hash] ?? null,
    svg: (id, hash) => {
      asked.push(`${id}@${hash}`);
      return `url:${id}@${hash}`;
    },
    current: (id) => slides[id],
  };
}

describe("resolveSlideThumb", () => {
  test("resolveSlideThumb avec l'empreinte du reçu en cache rend cette image, même si la slide a changé depuis", () => {
    const t = thumbs({ h1: "blob:h1" }, { "s-4": "h2" });
    expect(resolveSlideThumb(t, { id: "s-4", hash: "h1" })).toEqual({ url: "blob:h1", changed: false });
    expect(t.asked).toEqual([]);
  });

  test("resolveSlideThumb hors cache sur une slide inchangée la demande à son empreinte", () => {
    const t = thumbs({}, { "s-4": "h1" });
    expect(resolveSlideThumb(t, { id: "s-4", hash: "h1" })).toEqual({ url: "url:s-4@h1", changed: false });
    expect(t.asked).toEqual(["s-4@h1"]);
  });

  test("resolveSlideThumb hors cache sur une slide modifiée depuis rend son état actuel, marqué", () => {
    const t = thumbs({}, { "s-4": "h2" });
    expect(resolveSlideThumb(t, { id: "s-4", hash: "h1" })).toEqual({ url: "url:s-4@h2", changed: true });
    expect(t.asked).toEqual(["s-4@h2"]);
  });

  test("resolveSlideThumb hors cache sur une slide supprimée ne rend rien", () => {
    expect(resolveSlideThumb(thumbs({}, {}), { id: "s-4", hash: "h1" })).toBeNull();
  });

  test("resolveSlideThumb sans miniatures fournies ne rend rien", () => {
    expect(resolveSlideThumb(null, { id: "s-4", hash: "h1" })).toBeNull();
  });

  test("resolveSlideThumb dont l'image n'est pas encore reçue ne rend rien", () => {
    const t: SlideThumbs = { cached: () => null, svg: () => null, current: () => "h1" };
    expect(resolveSlideThumb(t, { id: "s-4", hash: "h1" })).toBeNull();
  });
});
