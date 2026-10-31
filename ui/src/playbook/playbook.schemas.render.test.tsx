// Every playbook example passes the catalog: the processor and each component's zod schema.

import { describe, expect, test } from "bun:test";
import "../test/dom";
import { MessageProcessor } from "@a2ui/web_core/v0_9";
import fixture from "../a2ui/fixtures/playbook.json";

// Bun does not evaluate static imports in written order: the catalog loads after `../test/dom`.
const { CATALOG_ID, catalog } = await import("../a2ui/catalog");
const { componentsOf } = await import("./playbookEntries");
type Entry = import("./playbookEntries").PlaybookEntry;
const entries = fixture as unknown as Entry[];

describe("playbook fixture", () => {
  test("covers exactly the catalog's components", () => {
    const shown = [...new Set(entries.map((e) => e.component))].sort();
    expect(shown).toEqual([...catalog.components.keys()].sort());
  });

  test("every example builds a surface with a root", () => {
    const missing = entries.filter((e) => {
      const processor = new MessageProcessor([catalog]);
      processor.processMessages(e.messages as Parameters<typeof processor.processMessages>[0]);
      const model = processor.model.surfacesMap.get(e.surface);
      const ok = model?.catalog.id === CATALOG_ID && model.componentsModel.get("root") !== undefined;
      model?.dispose();
      return !ok;
    });
    expect(missing.map((e) => `${e.component}/${e.variant}`)).toEqual([]);
  });

  test("every component's props pass its zod schema", () => {
    const failures = entries.flatMap((e) =>
      (componentsOf(e) as { id: string; component: string }[])
        .filter(({ id: _id, component, ...props }) => !catalog.components.get(component)?.schema.safeParse(props).success)
        .map((c) => `${e.component}/${e.variant} ${c.id}`),
    );
    expect(failures).toEqual([]);
  });
});
