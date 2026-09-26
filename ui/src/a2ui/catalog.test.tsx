import { describe, expect, test } from "bun:test";
import { MessageProcessor } from "@a2ui/web_core/v0_9";
import { CATALOG_ID, panelCatalog } from "./catalog";

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
  const processor = new MessageProcessor([panelCatalog]);
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
