import { describe, expect, test } from "bun:test";
import { LIGHT_BACKGROUND, luminance } from "./color";

describe("luminance", () => {
  test("blanc : 1, noir : 0", () => {
    expect(luminance("#ffffff")).toBeCloseTo(1, 5);
    expect(luminance("#000000")).toBeCloseTo(0, 5);
  });

  test("couleur courte #rgb : comme sa forme longue", () => {
    expect(luminance("#fff")).toBeCloseTo(1, 5);
    expect(luminance("#abc")).toBeCloseTo(luminance("#aabbcc")!, 10);
  });

  test("texte qui n'est pas une couleur : null", () => {
    expect(luminance("rouge")).toBeNull();
    expect(luminance("#abcd")).toBeNull();
  });
});

describe("LIGHT_BACKGROUND", () => {
  test("gris 60 % (luma de ink-on dans Typst) : exactement le seuil", () => {
    expect(luminance("#999999")).toBeCloseTo(LIGHT_BACKGROUND, 10);
  });
});
