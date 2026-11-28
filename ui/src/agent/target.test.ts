import { describe, expect, test } from "bun:test";
import type { TabDocument, TabInfo, WorkspaceView } from "../api";
import { agentTarget } from "./target";

const view = (focused: string) => ({
  generation: 1,
  focused,
  agent_detached: true,
  import_picks: [],
  windows: [
    { label: "w-1", active: "t-1", tabs: [] },
    { label: "w-2", active: "t-5", tabs: [] },
  ],
});

describe("agentTarget", () => {
  test("agentTarget suit l'onglet actif de la fenêtre au premier plan", () => {
    expect(agentTarget(view("w-2"))).toBe("t-5");
  });
  test("agentTarget fenêtre au premier plan inconnue : null", () => {
    expect(agentTarget(view("w-9"))).toBeNull();
  });
  test("agent target knowledge tab maps owner", () => {
    const tab = (id: string, document: TabDocument): TabInfo => ({ id, title: id, dirty: false, dirty_revision: null, pristine: false, theme: null, document, created: null });
    const v: WorkspaceView = {
      generation: 1,
      focused: "w-2",
      agent_detached: true,
      import_picks: [],
      windows: [
        { label: "w-1", active: "t-1", tabs: [tab("t-1", { kind: "deck", path: null, deck_uid: null })] },
        { label: "w-2", active: "t-2", tabs: [tab("t-2", { kind: "knowledge", deck_tab: "t-1" })] },
      ],
    };
    expect(agentTarget(v)).toBe("t-1");
  });
});
