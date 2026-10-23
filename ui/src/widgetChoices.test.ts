import { describe, expect, test } from "bun:test";
import type { AddPackView, AddWidget } from "./packTypes";
import { filterChoices, widgetSections, type WidgetChoice } from "./widgetChoices";

const widget = (name: string, label = name, description: string | null = null): AddWidget => ({ name, label, description, seed: { params: {} } });
const pack = (namespace: string, name: string, widgets: AddWidget[]): AddPackView => ({ namespace, name, widgets });

describe("widgetSections", () => {
  test("widgetSections with two packs keeps pack and widget order", () => {
    const sections = widgetSections([
      pack("base", "Base", [widget("base/text"), widget("base/card")]),
      pack("acme", "Acme", [widget("acme/kpi")]),
    ]);
    expect(sections.map((s) => [s.name, s.choices.map((c) => c.key)])).toEqual([
      ["Base", ["base/text", "base/card"]],
      ["Acme", ["acme/kpi"]],
    ]);
  });

  test("widgetSections carries the widget, its seed included", () => {
    const kpi = widget("acme/kpi", "KPI", "Une valeur en grand");
    expect(widgetSections([pack("acme", "Acme", [kpi])])[0].choices[0]).toEqual({
      key: "acme/kpi",
      name: "acme/kpi",
      label: "KPI",
      description: "Une valeur en grand",
      widget: kpi,
    });
  });

  test("widgetSections for a widget without description does not match the word null", () => {
    const choices = widgetSections([pack("acme", "Acme", [widget("frise")])]).flatMap((s) => s.choices);
    expect(filterChoices(choices, "null")).toEqual([]);
  });
});

/** Un choix de test : `label` et `description` lisibles, sans Fluent. */
const choice = (name: string, label: string, description: string): WidgetChoice => ({
  key: name,
  name,
  label,
  description,
  widget: widget(name, label, description),
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
