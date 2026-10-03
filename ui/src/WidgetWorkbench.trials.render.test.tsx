// Panneau Widgets : diagnostics d'un essai (réponse d'une écriture) face à la relecture du pack.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { WidgetChange } from "./api";

const { PREVIEW_DELAY_MS } = await import("./debounce");
const { memory, mountWith, pick, stub, type, unmount, wait } = await import("./test/workbench");

afterEach(unmount);

const rendu = { file: "kpi", line: 1, column: 1, error: true, message: "rendu" };
const pill = (host: HTMLElement) => host.querySelector(".workbench-list .pill")?.className;

describe("WidgetWorkbench, essai sans enregistrement", () => {
  test("mode deck, essai refusé (rien d'enregistré) : une relecture du pack ne lui colle pas ses diagnostics", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host, rerender } = await mountWith(fake);
    await pick(host, "kpi");
    stub(fake, "setWidget", async (): Promise<WidgetChange> => ({
      status: { name: "kpi", schema: null, diagnostics: [rendu] },
      broken: [],
      saved: false,
    }));
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    expect(pill(host)).toBe("pill errors");
    // Le pack change pour une autre raison : ses statuts sont ceux du pack, sans l'essai.
    await rerender({ revision: "r1" });
    await wait(10);
    expect(pill(host)).toBe("pill valid");
  });
});

describe("WidgetWorkbench, diagnostics du rendu (mode modèles)", () => {
  const rendu = { file: "kpi", line: 1, column: 1, error: true, message: "rendu du modèle" };

  /** kpi écrit : la réponse porte un diagnostic du rendu, que la sonde (`widget_status`) ignore. */
  async function written() {
    const files: Record<string, string> = { kpi: "k" };
    const { fake } = memory(files);
    const { host, rerender } = await mountWith(fake);
    await pick(host, "kpi");
    // Mode modèles : la version est enregistrée, même si le rendu des modèles échoue.
    stub(fake, "setWidget", async (name: string, source: string): Promise<WidgetChange> => {
      files[name] = source;
      return { status: { name: "kpi", schema: null, diagnostics: [rendu] }, broken: [], saved: true };
    });
    await type(host, "k2");
    await wait(PREVIEW_DELAY_MS + 50);
    const listed = () => [...host.querySelectorAll(".workbench-diagnostics li")].map((li) => li.textContent);
    const pill = () => host.querySelector(".workbench-list .pill")?.className;
    return { host, rerender, listed, pill };
  }

  test("la relecture du pack changé par l'écriture garde ses diagnostics du rendu", async () => {
    const { rerender, listed, pill } = await written();
    expect(pill()).toBe("pill errors");
    await rerender({ revision: "r1" });
    await wait(10);
    expect(pill()).toBe("pill errors");
    expect(listed()).toEqual(["kpi:1:1 rendu du modèle"]);
  });

  test("un pack changé de nouveau ailleurs (l'agent) : ils ne valent plus", async () => {
    const { rerender, listed, pill } = await written();
    await rerender({ revision: "r1" });
    await wait(10);
    await rerender({ revision: "r2" });
    await wait(10);
    expect(pill()).toBe("pill valid");
    expect(listed()).toEqual([]);
  });
});
