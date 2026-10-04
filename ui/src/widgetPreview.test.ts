import { describe, expect, test } from "bun:test";
import { croppedSvg } from "./widgetPreview";

const SVG = '<svg viewBox="0 0 1920 1080" width="1920pt" height="1080pt" xmlns="http://www.w3.org/2000/svg"><path d="M0 0"/></svg>';

/** Un attribut de la balise `<svg>` racine. */
function attr(svg: string, name: string): string | undefined {
  const tag = /<svg\b[^>]*>/.exec(svg)?.[0] ?? "";
  return new RegExp(`\\s${name}="([^"]*)"`).exec(tag)?.[1];
}

describe("croppedSvg", () => {
  test("un widget d'élément : la vue est sa boîte, sans marge", () => {
    const svg = croppedSvg(SVG, { x: 124, y: 150, w: 736, h: 30 });
    expect(attr(svg, "viewBox")).toBe("124 150 736 30");
    expect(attr(svg, "width")).toBe("736pt");
    expect(attr(svg, "height")).toBe("30pt");
    // Le contenu ne change pas.
    expect(svg).toContain('<path d="M0 0"/>');
  });

  test("un widget de slide : toute la slide", () => {
    expect(croppedSvg(SVG, null)).toBe(SVG);
  });
});
