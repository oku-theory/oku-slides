import { describe, expect, test } from "bun:test";
import { filterChoices, widgetChoices, type WidgetChoice } from "./widgetChoices";

const kpi = { name: "acme/kpi", label: "KPI", description: "Une valeur en grand" };

describe("widgetChoices", () => {
  test("widgetChoices without pack widgets lists the native kinds in card order", () => {
    expect(widgetChoices([]).map((c) => c.key)).toEqual(["base/text", "base/card", "base/shape", "base/chart", "base/table"]);
  });

  test("widgetChoices with pack widgets puts them after the native kinds", () => {
    expect(widgetChoices([kpi]).map((c) => c.key)).toEqual(["base/text", "base/card", "base/shape", "base/chart", "base/table", "widget:acme/kpi"]);
  });

  test("widgetChoices with the base card widget keeps the native card only", () => {
    const cards = widgetChoices([{ name: "base/card", label: "Carte", description: null }]).filter((c) => c.name.endsWith("card"));
    expect(cards.map((c) => c.pick)).toEqual([{ native: "card" }]);
  });

  test("le filtre « base » ne retient aucune carte native", () => {
    expect(filterChoices(widgetChoices([]), "base")).toEqual([]);
  });

  test("widgetChoices for a pack widget without label shows its name", () => {
    expect(widgetChoices([{ name: "deck/frise", label: null, description: null }]).find((c) => c.key === "widget:deck/frise")?.label).toBe("deck/frise");
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
