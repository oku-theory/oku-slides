import { describe, expect, test } from "bun:test";
import type { InterfaceChange } from "./api";
import { applyInterface } from "./interfaceChange";

const NONE: InterfaceChange = { scheme: null, locale: null, guides: null };

/** Les commandes appelées, dans l'ordre ; `failing` : changeLocale refuse. */
function record(failing = false) {
  const calls: string[] = [];
  const cmd = {
    changeScheme: async (c: string | null) => void calls.push(`scheme:${c}`),
    changeLocale: async (c: string | null) => {
      if (failing) throw new Error("refusée");
      calls.push(`locale:${c}`);
    },
    setGhosts: (on: boolean) => void calls.push(`guides:${on}`),
    fail: (e: unknown) => void calls.push(`fail:${(e as Error).message}`),
  };
  return { calls, cmd };
}

describe("applyInterface", () => {
  test.each([
    [{ ...NONE, scheme: "dark" }, ["scheme:dark"]],
    [{ ...NONE, scheme: "system" }, ["scheme:null"]],
    [{ ...NONE, locale: "de" }, ["locale:de"]],
    [{ ...NONE, locale: "system" }, ["locale:null"]],
    [{ ...NONE, guides: true }, ["guides:true"]],
    [NONE, []],
  ] as [InterfaceChange, string[]][])("applyInterface %o appelle %o", async (change, expected) => {
    const { calls, cmd } = record();
    applyInterface(change, cmd);
    await Promise.resolve();
    expect(calls).toEqual(expected);
  });

  test("applyInterface un champ absent de l'évènement ne change rien", async () => {
    const { calls, cmd } = record();
    applyInterface({ guides: true } as unknown as InterfaceChange, cmd);
    await Promise.resolve();
    expect(calls).toEqual(["guides:true"]);
  });

  test("applyInterface une commande refusée dit son erreur", async () => {
    const { calls, cmd } = record(true);
    applyInterface({ ...NONE, locale: "de" }, cmd);
    await new Promise((r) => setTimeout(r, 0));
    expect(calls).toEqual(["fail:refusée"]);
  });
});
