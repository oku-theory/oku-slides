// A view broadcast by the host: shown, then the active tab's snapshot read again.
import { describe, expect, test } from "bun:test";
import { showBroadcast } from "./workspaceBroadcast";
import type { TabInfo, WorkspaceView } from "./workspaceApi";

const tab = (dirty: boolean): TabInfo => ({
  id: "t-1",
  title: "deck",
  dirty,
  pristine: false,
  theme: null,
  document: { kind: "deck", path: null, deck_uid: null },
});
const view = (generation: number, dirty: boolean): WorkspaceView => ({
  generation,
  focused: "w-1",
  agent_detached: false,
  import_pick: null,
  windows: [{ label: "w-1", active: "t-1", tabs: [tab(dirty)] }],
});

describe("showBroadcast", () => {
  test("Workspace re-reads the active tab snapshot when its TabInfo changes", () => {
    const shown: WorkspaceView[] = [];
    let bumps = 0;
    const deps = { alive: { current: true }, setView: (v: WorkspaceView) => shown.push(v), bump: () => (bumps += 1) };
    showBroadcast(view(2, true), deps);
    expect(shown.map((v) => v.windows[0]?.tabs[0]?.dirty)).toEqual([true]);
    expect(bumps).toBe(1);
  });

  test("showBroadcast after unmount shows nothing", () => {
    let calls = 0;
    const deps = { alive: { current: false }, setView: () => (calls += 1), bump: () => (calls += 1) };
    showBroadcast(view(2, true), deps);
    expect(calls).toBe(0);
  });
});
