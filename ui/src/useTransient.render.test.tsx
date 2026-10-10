// Valeur passagère : montrée, effacée après son délai, ou tout de suite.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import { manualTimers } from "./test/timers";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useTransient } = await import("./useTransient");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Monte un composant qui rend la valeur ; `hook()` donne le hook courant. */
async function mountTransient() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const timers = manualTimers();
  const out: { hook: ReturnType<typeof useTransient<string>> | null } = { hook: null };
  function Probe() {
    out.hook = useTransient<string>(timers);
    return createElement("p", null, out.hook[0]);
  }
  await act(async () => root.render(createElement(Probe)));
  return { host, timers, hook: () => out.hook! };
}

describe("useTransient", () => {
  test("useTransient show rend la valeur pour le délai demandé", async () => {
    const m = await mountTransient();
    await act(async () => m.hook()[1]("refusé", 6000));
    expect([m.host.textContent, m.timers.delays]).toEqual(["refusé", [6000]]);
  });

  test("useTransient la valeur s'efface après son délai", async () => {
    const m = await mountTransient();
    await act(async () => m.hook()[1]("refusé", 6000));
    await act(async () => m.timers.run());
    expect(m.host.textContent).toBe("");
  });

  test("useTransient null efface tout de suite", async () => {
    const m = await mountTransient();
    await act(async () => m.hook()[1]("refusé", 6000));
    await act(async () => m.hook()[1](null));
    expect(m.host.textContent).toBe("");
  });

  test("useTransient le minuteur d'une valeur remplacée ne l'efface pas", async () => {
    const m = await mountTransient();
    await act(async () => m.hook()[1]("un", 6000));
    // Le premier minuteur, gardé avant d'être remplacé : il part en retard.
    const late = m.timers.pending()[0]!;
    await act(async () => m.hook()[1]("deux", 6000));
    await act(async () => late());
    expect(m.host.textContent).toBe("deux");
  });
});
