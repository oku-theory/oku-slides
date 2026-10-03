// Apparence appliquée à la racine du document, dans un DOM happy-dom.

import { beforeEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Appearance } from "./api";
import { applyAppearance, applyDefaultScheme } from "./appearance";

const papier: Appearance = {
  scheme: "light",
  slide: { bg: "#fffdf9", ink: "#2e2b26" },
  app: { desk: "#ece8e0" },
  variants: {},
  metrics: {},
  font_display: [],
  font_body: [],
  chart_palette: [],
};
const root = () => document.documentElement;

describe("applyAppearance", () => {
  beforeEach(() => {
    root().removeAttribute("style");
    root().removeAttribute("data-scheme");
    root().removeAttribute("data-derived");
  });

  test("applyAppearance au schéma du thème pose ses couleurs, sans dérivation", async () => {
    await applyAppearance(papier, "light");
    expect(root().style.getPropertyValue("--app-desk")).toBe("#ece8e0");
    expect(root().dataset.scheme).toBe("light");
    expect(root().hasAttribute("data-derived")).toBe(false);
  });

  test("applyAppearance à l'autre schéma sans variante dérive la palette", async () => {
    await applyAppearance(papier, "light");
    await applyAppearance(papier, "dark");
    expect(root().style.getPropertyValue("--app-desk")).toBe("");
    expect(root().dataset.derived).toBe("dark");
    expect(root().style.getPropertyValue("--t-light")).toBe("#fffdf9");
    expect(root().style.getPropertyValue("--t-dark")).toBe("#2e2b26");
    expect(root().style.colorScheme).toBe("dark");
  });

  test("applyAppearance à l'autre schéma prend la variante du thème", async () => {
    await applyAppearance({ ...papier, variants: { dark: { desk: "#1c1a17" } } }, "dark");
    expect(root().style.getPropertyValue("--app-desk")).toBe("#1c1a17");
    expect(root().hasAttribute("data-derived")).toBe(false);
  });
});

describe("applyDefaultScheme", () => {
  test("applyDefaultScheme sombre dérive les défauts clairs de la feuille de style", () => {
    root().removeAttribute("data-derived");
    applyDefaultScheme("dark");
    expect(root().dataset.derived).toBe("dark");
    expect(root().dataset.scheme).toBe("dark");
  });

  test("applyDefaultScheme clair garde les défauts tels quels", () => {
    applyDefaultScheme("dark");
    applyDefaultScheme("light");
    expect(root().hasAttribute("data-derived")).toBe(false);
  });
});
