import { describe, expect, test } from "bun:test";
import { filterChoices, widgetChoices, type WidgetChoice } from "./widgetChoices";

const kpi = { name: "kpi", label: "KPI", description: "Une valeur en grand" };

describe("widgetChoices", () => {
  test("widgetChoices without pack widgets lists the native kinds in card order", () => {
    expect(widgetChoices([]).map((c) => c.key)).toEqual(["text", "card", "shape", "chart", "table"]);
  });

  test("widgetChoices with pack widgets puts them after the native kinds", () => {
    expect(widgetChoices([kpi]).map((c) => c.key)).toEqual(["text", "card", "shape", "chart", "table", "widget:kpi"]);
  });

  test("widgetChoices with a pack widget named like a native kind keeps the native one only", () => {
    const cards = widgetChoices([{ name: "card", label: "Carte du pack", description: null }]).filter((c) => c.name === "card");
    expect(cards.map((c) => c.pick)).toEqual([{ native: "card" }]);
  });

  test("widgetChoices for a pack widget without label shows its name", () => {
    expect(widgetChoices([{ name: "frise", label: null, description: null }]).find((c) => c.key === "widget:frise")?.label).toBe("frise");
  });

  test("widgetChoices for a pack widget without description does not match the word null", () => {
    expect(filterChoices(widgetChoices([{ name: "frise", label: null, description: null }]), "null")).toEqual([]);
  });
});

/** Un choix de test : `label` et `description` lisibles, sans Fluent. */
const choice = (name: string, label: string, description: string): WidgetChoice => ({
  key: `widget:${name}`,
  name,
  label,
  description,
  pick: { widget: { name, label, description } },
});

const list = [choice("kpi", "Indicateur", "Une valeur en grand"), choice("frise", "Frise", "Des étapes datées")];

describe("filterChoices", () => {
  test("filterChoices with an empty query keeps every choice", () => {
    expect(filterChoices(list, "  ")).toEqual(list);
  });

  test("filterChoices matches the label", () => {
    expect(filterChoices(list, "indic").map((c) => c.name)).toEqual(["kpi"]);
  });

  test("filterChoices matches the technical name", () => {
    expect(filterChoices(list, "KPI").map((c) => c.name)).toEqual(["kpi"]);
  });

  test("filterChoices matches the description", () => {
    expect(filterChoices(list, "étapes").map((c) => c.name)).toEqual(["frise"]);
  });

  test("filterChoices ignores case and accents", () => {
    expect(filterChoices(list, "DATEES").map((c) => c.name)).toEqual(["frise"]);
  });

  test("filterChoices requires every word somewhere", () => {
    expect(filterChoices(list, "valeur frise")).toEqual([]);
  });
});
