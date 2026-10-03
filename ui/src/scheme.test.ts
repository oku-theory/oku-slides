import { describe, expect, test } from "bun:test";
import type { Appearance } from "./api";
import { applyScheme, chosenScheme, cssRgb, onSchemeChange, resolveScheme, slideEnds, uiPalette } from "./scheme";

const appearance = (over: Partial<Appearance>): Appearance => ({
  scheme: "light",
  slide: {},
  app: { desk: "#ece8e0" },
  variants: {},
  metrics: {},
  font_display: [],
  font_body: [],
  chart_palette: [],
  ...over,
});

describe("resolveScheme", () => {
  test("resolveScheme sans choix suit le système", () => {
    expect(resolveScheme(null, true)).toBe("dark");
    expect(resolveScheme(null, false)).toBe("light");
  });

  test("resolveScheme avec un choix l'impose au système", () => {
    expect(resolveScheme("light", true)).toBe("light");
    expect(resolveScheme("dark", false)).toBe("dark");
  });
});

describe("uiPalette", () => {
  test("uiPalette du schéma du thème donne ses couleurs app", () => {
    expect(uiPalette(appearance({}), "light")).toEqual({ desk: "#ece8e0" });
  });

  test("uiPalette de l'autre schéma donne la variante du thème", () => {
    const a = appearance({ variants: { dark: { desk: "#1c1a17" } } });
    expect(uiPalette(a, "dark")).toEqual({ desk: "#1c1a17" });
  });

  test("uiPalette sans variante pour ce schéma rend null (à dériver)", () => {
    expect(uiPalette(appearance({}), "dark")).toBeNull();
  });
});

describe("slideEnds", () => {
  test("slideEnds range fond et encre du plus clair au plus foncé", () => {
    expect(slideEnds({ bg: "#fffdf9", ink: "#2e2b26" })).toEqual({ light: "#fffdf9", dark: "#2e2b26" });
    expect(slideEnds({ bg: "#101820", ink: "#f0f0f0" })).toEqual({ light: "#f0f0f0", dark: "#101820" });
  });

  test("slideEnds sans couleurs lisibles rend null", () => {
    expect(slideEnds({ bg: "transparent" })).toBeNull();
  });
});

describe("cssRgb", () => {
  test("cssRgb lit hex, rgb() et color(srgb)", () => {
    expect(cssRgb("#1c1a17")).toEqual([28, 26, 23]);
    expect(cssRgb("rgb(28, 26, 23)")).toEqual([28, 26, 23]);
    expect(cssRgb("color(srgb 0.1 0.2 1)")).toEqual([26, 51, 255]);
  });

  test("cssRgb d'une couleur inconnue rend null", () => {
    expect(cssRgb("oklab(0.5 0 0)")).toBeNull();
  });
});

describe("choix du schéma", () => {
  test("applyScheme retient le choix de l'hôte et prévient", () => {
    let calls = 0;
    const off = onSchemeChange(() => calls++);
    applyScheme("dark");
    off();
    applyScheme(null);
    expect(calls).toBe(1);
    expect(chosenScheme()).toBeNull();
  });
});
