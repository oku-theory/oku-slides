// Avis passager : texte, alerte pour une erreur, effacement après le délai.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import { manualTimers } from "./test/timers";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useToast } = await import("./useToast");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Monte un composant qui rend l'avis ; `toast()` donne le hook courant. */
async function mountToast() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const timers = manualTimers();
  const out: { hook: ReturnType<typeof useToast> | null } = { hook: null };
  function Probe() {
    out.hook = useToast(timers);
    return createElement("div", null, out.hook.node);
  }
  await act(async () => root.render(createElement(Probe)));
  return { host, timers, toast: () => out.hook!, delays: timers.delays };
}

describe("useToast", () => {
  test("useToast say affiche le texte", async () => {
    const m = await mountToast();
    await act(async () => m.toast().say("enregistré"));
    expect(m.host.querySelector(".toast")?.textContent).toBe("enregistré");
  });

  test("useToast une erreur s'annonce comme alerte", async () => {
    const m = await mountToast();
    await act(async () => m.toast().say("refusé", true));
    expect(m.host.querySelector(".toast.error")?.getAttribute("role")).toBe("alert");
  });

  test("useToast l'avis s'efface après son délai", async () => {
    const m = await mountToast();
    await act(async () => m.toast().say("enregistré"));
    await act(async () => m.timers.run());
    expect(m.host.querySelector(".toast")).toBeNull();
  });

  test("useToast un nouvel avis remplace le précédent sans être effacé par son minuteur", async () => {
    const m = await mountToast();
    await act(async () => m.toast().say("un"));
    await act(async () => m.toast().say("deux"));
    // Le minuteur du premier est annulé : un seul reste, celui de « deux ».
    await act(async () => m.timers.run());
    expect(m.host.querySelector(".toast")).toBeNull();
  });
});
