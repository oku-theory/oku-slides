import { describe, expect, test } from "bun:test";
import { GROUPS, byGroup, componentsOf, visible, type PlaybookEntry } from "./playbookEntries";

const entry = (component: string, group: PlaybookEntry["group"]): PlaybookEntry => ({
  component, variant: "default", note: "", group, agent: false, surface: `playbook-${component}-default`,
  messages: [{ version: "v0.9", updateComponents: { surfaceId: "s", components: [{ id: "root", component }] } }] as never,
});

describe("playbook entries", () => {
  test("visible with a filter keeps matching components, case-insensitive", () => {
    const all = [entry("Button", "atom"), entry("Badge", "atom"), entry("Row", "layout")];
    expect(visible(all, "bu").map((e) => e.component)).toEqual(["Button"]);
  });

  test("visible with a blank filter keeps everything", () => {
    expect(visible([entry("Row", "layout")], "  ")).toHaveLength(1);
  });

  test("byGroup returns the four groups in gallery order", () => {
    const groups = byGroup([entry("Row", "layout"), entry("Receipt", "host_only")]);
    expect(groups.map((g) => [g.title, g.entries.length])).toEqual([
      ["Layout", 1], ["Atoms", 0], ["Fields", 0], ["Host only", 1],
    ]);
  });

  test("componentsOf reads the updateComponents message", () => {
    expect(componentsOf(entry("Row", "layout"))).toEqual([{ id: "root", component: "Row" }]);
  });

  test("GROUPS names every group once", () => {
    expect(GROUPS.map((g) => g.group)).toEqual(["layout", "atom", "field", "host_only"]);
  });
});
