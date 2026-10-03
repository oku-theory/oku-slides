import { describe, expect, test } from "bun:test";
import { croppedSvg } from "./widgetPreview";

const SVG = '<svg viewBox="0 0 1920 1080" width="1920pt" height="1080pt" xmlns="http://www.w3.org/2000/svg"><path d="M0 0"/></svg>';

/** Un attribut de la balise `<svg>` racine. */
function attr(svg: string, name: string): string | undefined {
  const tag = /<svg\b[^>]*>/.exec(svg)?.[0] ?? "";
  return new RegExp(`\\s${name}="([^"]*)"`).exec(tag)?.[1];
}

describe("croppedSvg", () => {
  test("un widget d'élément : la vue est sa boîte, avec une marge", () => {
    const svg = croppedSvg(SVG, { x: 124, y: 150, w: 736, h: 30 });
    const [x, y, w, h] = (attr(svg, "viewBox") ?? "").split(" ").map(Number);
    // La marge est la même des quatre côtés, et la boîte y tient.
    const m = 124 - (x ?? 0);
    expect(m).toBeGreaterThan(0);
    expect([x, y, w, h]).toEqual([124 - m, 150 - m, 736 + 2 * m, 30 + 2 * m]);
    expect(attr(svg, "width")).toBe(`${736 + 2 * m}pt`);
    expect(attr(svg, "height")).toBe(`${30 + 2 * m}pt`);
    // Le contenu ne change pas.
    expect(svg).toContain('<path d="M0 0"/>');
  });

  test("un widget de slide : toute la slide", () => {
    expect(croppedSvg(SVG, null)).toBe(SVG);
  });
});
