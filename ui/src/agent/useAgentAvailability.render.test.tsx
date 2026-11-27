// La disponibilité de l'agent vue de la page : demandée à l'ouverture du dialogue, suivie.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useAgentAvailability } = await import("./useAgentAvailability");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mount(asking: boolean) {
  const asked: number[] = [];
  const agent = { availability: async () => void asked.push(1) };
  let state: ReturnType<typeof useAgentAvailability> | null = null;
  const Probe = ({ asking: a }: { asking: boolean }) => {
    state = useAgentAvailability(agent, a);
    return null;
  };
  const root = createRoot(document.createElement("div"));
  const render = (a: boolean) => act(async () => root.render(createElement(Probe, { asking: a })));
  await render(asking);
  cleanup = async () => act(async () => root.unmount());
  return { asked, render, state: () => state! };
}

describe("useAgentAvailability", () => {
  test("useAgentAvailability asks nothing while the dialog is closed", async () => {
    const m = await mount(false);
    expect(m.asked).toHaveLength(0);
    expect(m.state().unusable).toBeNull();
  });

  test("useAgentAvailability asks the host each time the dialog opens", async () => {
    const m = await mount(false);
    await m.render(true);
    await m.render(false);
    await m.render(true);
    expect(m.asked).toHaveLength(2);
  });

  test("useAgentAvailability keeps the last answer received", async () => {
    const m = await mount(true);
    await act(async () => m.state().onAvailability("Agent introuvable : npx."));
    expect(m.state().unusable).toBe("Agent introuvable : npx.");
    await act(async () => m.state().onAvailability(null));
    expect(m.state().unusable).toBeNull();
  });
});
