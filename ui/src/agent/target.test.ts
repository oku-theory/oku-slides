import { describe, expect, test } from "bun:test";
import { agentTarget } from "./target";

const view = (focused: string) => ({
  generation: 1,
  focused,
  agent_detached: true,
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
});
