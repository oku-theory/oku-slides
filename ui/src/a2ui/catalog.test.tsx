import { describe, expect, test } from "bun:test";
import { MessageProcessor } from "@a2ui/web_core/v0_9";
import { CATALOG_ID, catalog, sixDigits } from "./catalog";
import { buildSurface } from "./SurfaceView";
import agentSurfaces from "./fixtures/agent-surfaces.json";

/** Ce que la session renvoie pour la requête `panel` en édition des modèles. */
const panelMessages = (slide = "t-1") => {
  const ctx = (extra: Record<string, unknown>) => ({ slide, ...extra });
  const field = (id: string, label: string, path: string, name: string) => ({
    id, component: "TextField", label, value: { path },
    action: { event: { name, context: ctx({ value: { path } }) } },
  });
  const stepper = (id: string, label: string, path: string) => ({
    id, component: "Stepper", label, value: { path }, min: 1, max: 24,
    action: { event: { name: "set_grid", context: ctx({ columns: { path: "/grid/columns" }, rows: { path: "/grid/rows" } }) } },
  });
  return [
    { version: "v0.9", createSurface: { surfaceId: "panel", catalogId: CATALOG_ID } },
    {
      version: "v0.9",
      updateComponents: {
        surfaceId: "panel",
        components: [
          { id: "root", component: "Column", children: ["template", "grid"] },
          { id: "template", component: "Card", child: "template-body" },
          { id: "template-body", component: "Column", children: ["template-title", "template-name", "template-description"] },
          { id: "template-title", component: "Text", text: "Ce modèle", variant: "h4" },
          field("template-name", "Nom", "/template/name", "rename_template"),
          field("template-description", "Description", "/template/description", "describe_template"),
          { id: "grid", component: "Card", child: "grid-body" },
          { id: "grid-body", component: "Column", children: ["grid-title", "grid-columns", "grid-rows"] },
          { id: "grid-title", component: "Text", text: "Grille", variant: "h4" },
          stepper("grid-columns", "Colonnes", "/grid/columns"),
          stepper("grid-rows", "Lignes", "/grid/rows"),
        ],
      },
    },
    {
      version: "v0.9",
      updateDataModel: { surfaceId: "panel", value: { template: { name: "Texte", description: "" }, grid: { columns: 12, rows: 6 } } },
    },
  ];
};

function surfaceOf(messages: unknown[]) {
  const processor = new MessageProcessor([catalog]);
  processor.processMessages(messages as Parameters<typeof processor.processMessages>[0]);
  return processor.model.surfacesMap.get("panel");
}

describe("catalogue du panneau", () => {
  test("accepte la surface que produit la session", () => {
    expect(surfaceOf(panelMessages())?.catalog.id).toBe(CATALOG_ID);
  });

  test("refuse une propriété hors du catalogue", () => {
    const messages = panelMessages();
    messages[1] = {
      version: "v0.9",
      updateComponents: { surfaceId: "panel", components: [{ id: "root", component: "Text", text: "a", colour: "red" } as never] },
    };
    expect(() => surfaceOf(messages)).toThrow(/Text/);
  });

  test("refuse un compteur sans valeur", () => {
    const messages = panelMessages();
    messages[1] = {
      version: "v0.9",
      updateComponents: { surfaceId: "panel", components: [{ id: "root", component: "Stepper", label: "n" } as never] },
    };
    expect(() => surfaceOf(messages)).toThrow(/Stepper/);
  });

  test("garde les composants et le modèle de données de la session", () => {
    const surface = surfaceOf(panelMessages());
    expect(surface?.componentsModel.get("grid-columns")?.type).toBe("Stepper");
    expect(surface?.dataModel.get("/template/name")).toBe("Texte");
    expect(surface?.dataModel.get("/grid/columns")).toBe(12);
  });
});

const surface = (components: unknown[], data: unknown = {}) => [
  { version: "v0.9", createSurface: { surfaceId: "s", catalogId: CATALOG_ID } },
  { version: "v0.9", updateComponents: { surfaceId: "s", components } },
  { version: "v0.9", updateDataModel: { surfaceId: "s", value: data } },
];

/** Les types de composants de la surface `s`, par identifiant. */
function typesOf(components: unknown[], data: unknown = {}) {
  const processor = new MessageProcessor([catalog]);
  processor.processMessages(surface(components, data) as Parameters<typeof processor.processMessages>[0]);
  const model = processor.model.surfacesMap.get("s");
  return Object.fromEntries((components as { id: string }[]).map((c) => [c.id, model?.componentsModel.get(c.id)?.type]));
}

const V2 = ["Column", "Row", "Card", "Text", "TextField", "Stepper", "Button", "ChoiceChips", "Slider", "ColorField", "Select", "SlideThumb", "DiffText"];

describe("catalogue v2", () => {
  test("déclare chacun des composants du catalogue Rust", () => {
    expect([...catalog.components.keys()].sort()).toEqual([...V2].sort());
  });

  test("les surfaces produites par la session Rust sont acceptées", () => {
    for (const s of agentSurfaces as { id: string; messages: unknown[] }[]) {
      const processor = new MessageProcessor([catalog]);
      processor.processMessages(s.messages as Parameters<typeof processor.processMessages>[0]);
      const model = processor.model.surfacesMap.get(s.id);
      expect(model?.catalog.id).toBe(CATALOG_ID);
      expect(model?.componentsModel.get("root")).toBeDefined();
    }
  });

  test("accepte les composants de l'agent", () => {
    expect(
      typesOf([
        { id: "root", component: "Column", children: ["row", "chips", "diff", "thumb", "slider", "color", "select", "ok"] },
        { id: "row", component: "Row", children: [] },
        { id: "chips", component: "ChoiceChips", options: [{ id: "a", label: "A" }, { id: "b", label: "B", hint: "h" }], value: { path: "/choice" },
          action: { event: { name: "choose", context: { choice: { path: "/choice" } } } } },
        { id: "diff", component: "DiffText", segments: [{ kind: "keep", text: "Nos " }, { kind: "del", text: "objectifs" }, { kind: "ins", text: "paris" }] },
        { id: "thumb", component: "SlideThumb", svg: "<svg/>", label: "1 · A", action: { event: { name: "variant", context: { index: 0 } } } },
        { id: "slider", component: "Slider", label: "Espacement", value: { path: "/f0" }, min: 0, max: 64, step: 4 },
        { id: "color", component: "ColorField", label: "Accent", value: { path: "/f1" }, palette: ["#c2573a", "#fff"] },
        { id: "select", component: "Select", label: "Thème", value: { path: "/f2" }, options: [{ id: "pro", label: "Pro" }] },
        { id: "ok", component: "Button", label: "OK", primary: true, action: { event: { name: "confirm", context: { confirmed: true } } } },
      ], { choice: "", f0: 16, f1: "#c2573a", f2: "pro" }),
    ).toEqual({
      root: "Column", row: "Row", chips: "ChoiceChips", diff: "DiffText", thumb: "SlideThumb",
      slider: "Slider", color: "ColorField", select: "Select", ok: "Button",
    });
  });

  const action = { event: { name: "x" } };
  test.each([
    ["un segment de nature inconnue", { component: "DiffText", segments: [{ kind: "moved", text: "x" }] }, /DiffText/],
    ["un bouton sans action", { component: "Button", label: "OK" }, /Button/],
    ["une rangée sans enfants", { component: "Row" }, /Row/],
    ["des pastilles sans options", { component: "ChoiceChips", value: "a" }, /ChoiceChips/],
    ["une option avec une clé en trop", { component: "ChoiceChips", value: "a", options: [{ id: "a", label: "A", icon: "x" }] }, /ChoiceChips/],
    ["un curseur sans borne", { component: "Slider", label: "l", value: 1, max: 2 }, /Slider/],
    ["une couleur sans palette", { component: "ColorField", label: "l", value: "#000000" }, /ColorField/],
    ["une palette qui n'est pas en hexadécimal", { component: "ColorField", label: "l", value: "#000000", palette: ["#fff url(x)"] }, /ColorField/],
    ["une liste sans options", { component: "Select", label: "l", value: "a" }, /Select/],
    ["une miniature sans SVG", { component: "SlideThumb", label: "l", action }, /SlideThumb/],
  ])("refuse %s", (_, component, error) => {
    expect(() => typesOf([{ id: "root", ...component }])).toThrow(error);
  });
});

describe("sixDigits", () => {
  test.each([
    ["#abc", "#aabbcc"],
    ["#ABCD", "#aabbcc"],
    ["#C2573A", "#c2573a"],
    ["#c2573a80", "#c2573a"],
    ["rouge", "#000000"],
    [undefined, "#000000"],
  ])("%s → %s", (input, expected) => expect(sixDigits(input)).toBe(expected));
});

describe("buildSurface", () => {
  test("une surface invalide rend l'erreur, pas d'exception", () => {
    const built = buildSurface(surface([{ id: "root", component: "Nope?" , text: 1 }, { id: "x", component: "Text" }]) as never, "s", () => {});
    expect(built.surface).toBeNull();
    expect(built.error).toMatch(/Text/);
  });

  test("un identifiant absent des messages ne rend rien", () => {
    const built = buildSurface(surface([{ id: "root", component: "Text", text: "a" }]) as never, "autre", () => {});
    expect(built).toEqual({ surface: null, error: null });
  });

  test("aucun message, aucune surface", () => {
    expect(buildSurface([], "s", () => {})).toEqual({ surface: null, error: null });
  });
});
