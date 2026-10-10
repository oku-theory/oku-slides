// Le panneau des agents vu de la page : la dernière surface reçue, l'ouverture et le choix.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import type { Surface } from "../api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useAgentPicker } = await import("./useAgentPicker");
const { PICKER } = await import("../a2ui/fixtures/agentPicker");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mount({ pickerFails = null, selectFails = null }: { pickerFails?: string | null; selectFails?: string | null } = {}) {
  const pickers: number[] = [];
  const selected: string[] = [];
  const errors: string[] = [];
  const agent = {
    picker: async () => {
      pickers.push(1);
      if (pickerFails !== null) throw new Error(pickerFails);
    },
    selectAgent: async (id: string) => {
      selected.push(id);
      if (selectFails !== null) throw new Error(selectFails);
    },
  };
  let picker: ReturnType<typeof useAgentPicker> | null = null;
  const Probe = () => {
    picker = useAgentPicker(agent, (m) => void errors.push(m));
    return null;
  };
  const root = createRoot(document.createElement("div"));
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => act(async () => root.unmount());
  return { pickers, selected, errors, picker: () => picker! };
}

/** Laisse les promesses se résoudre et React les rendre. */
const settle = () => act(async () => new Promise((r) => setTimeout(r, 0)));

describe("useAgentPicker", () => {
  test("useAgentPicker starts without a surface and keeps the last one received", async () => {
    const m = await mount();
    expect(m.picker().surface).toBeNull();
    await act(async () => m.picker().onPicker(PICKER));
    expect(m.picker().surface).toBe(PICKER as Surface);
  });

  test("useAgentPicker open asks the host for the picker", async () => {
    const m = await mount();
    await act(async () => m.picker().open());
    expect(m.pickers).toHaveLength(1);
  });

  test("useAgentPicker open failure is reported", async () => {
    const m = await mount({ pickerFails: "pas de pile" });
    await act(async () => m.picker().open());
    await settle();
    expect(m.errors).toEqual(["pas de pile"]);
  });

  test("useAgentPicker accepted choice sends select_agent and asks nothing more", async () => {
    const m = await mount();
    await act(async () => m.picker().select("claude"));
    await settle();
    expect([m.selected, m.pickers, m.errors]).toEqual([["claude"], [], []]);
  });

  test("useAgentPicker refused choice is said and the picker is asked again (shown and saved choice agree)", async () => {
    const m = await mount({ selectFails: "refus" });
    await act(async () => m.picker().select("claude"));
    await settle();
    expect([m.selected, m.errors, m.pickers.length]).toEqual([["claude"], ["refus"], 1]);
  });
});
