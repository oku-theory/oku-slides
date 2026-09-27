import { describe, expect, test } from "bun:test";
import {
  contrast,
  EDITOR_DARK,
  EDITOR_LIGHT,
  EDITOR_MIN_CONTRAST,
  editorBackground,
  LIGHT_BACKGROUND,
  LIGHT_INK,
  luminance,
} from "./color";

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

describe("contrast", () => {
  test("contrast noir sur blanc : 21, dans les deux sens", () => {
    expect(contrast(0, 1)).toBeCloseTo(21, 5);
    expect(contrast(1, 0)).toBeCloseTo(21, 5);
  });

  test("contrast d'une couleur avec elle-même : 1", () => {
    expect(contrast(0.3, 0.3)).toBeCloseTo(1, 5);
  });
});

describe("editorBackground", () => {
  test("editorBackground sans couleur de texte garde le fond de la slide", () => {
    expect(editorBackground(null, "#f7f5f0")).toBe("#f7f5f0");
  });

  test("editorBackground sans couleur de texte ni fond : aucun fond", () => {
    expect(editorBackground(null, null)).toBeUndefined();
  });

  test("editorBackground avec un texte qui n'est pas une couleur garde le fond", () => {
    expect(editorBackground("currentColor", "#f7f5f0")).toBe("#f7f5f0");
  });

  test("editorBackground assez contrasté garde le fond de la slide", () => {
    expect(editorBackground("#2e2b26", "#fffdf9")).toBe("#fffdf9");
  });

  test("editorBackground texte clair sur fond clair : fond sombre", () => {
    expect(editorBackground("#ffffff", "#f7f5f0")).toBe(EDITOR_DARK);
  });

  test("editorBackground texte sombre sur fond sombre : fond clair", () => {
    expect(editorBackground("#1d1c1a", "#2e2b26")).toBe(EDITOR_LIGHT);
  });

  test("editorBackground sans fond connu : selon le texte", () => {
    expect(editorBackground("#ffffff", null)).toBe(EDITOR_DARK);
    expect(editorBackground("#000000", null)).toBe(EDITOR_LIGHT);
  });

  test("editorBackground seuils nommés : contraste 3, texte clair au-delà de 0,4", () => {
    expect(EDITOR_MIN_CONTRAST).toBe(3);
    expect(LIGHT_INK).toBe(0.4);
  });
});
