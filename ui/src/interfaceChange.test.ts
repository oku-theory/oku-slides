import { describe, expect, test } from "bun:test";
import type { InterfaceChange } from "./api";
import { applyInterface } from "./interfaceChange";

const NONE: InterfaceChange = { scheme: null, locale: null, guides: null, folded: null, pile: null, notes: null, code: null };

/** The live table the page measures: 900 px wide, its pile at the default 236 px. */
const TABLE = { table: 900, pile: 236 };

/** Les commandes appelées, dans l'ordre ; `failing` : changeLocale refuse. */
function record(failing = false) {
  const calls: string[] = [];
  const cmd = {
    setFolded: (section: string, fold: boolean) => void calls.push(`folded:${section}:${fold}`),
    setCode: (on: boolean) => void calls.push(`code:${on}`),
    columns: { pile: null, notes: 500 },
    setColumns: (c: { pile: number | null; notes: number | null }) => void calls.push(`columns:${c.pile}:${c.notes}`),
    measure: () => TABLE,
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

  test("applyInterface folds slides and keeps layouts", async () => {
    const { calls, cmd } = record();
    applyInterface({ ...NONE, folded: { slides: true, layouts: null } }, cmd);
    await Promise.resolve();
    expect(calls).toEqual(["folded:slides:true"]);
  });

  test("applyInterface pile beyond live bound is clamped", async () => {
    // 900 px of table: the pile gives way before the sheet, 900 - 300 - 320 = 280 at most.
    const { calls, cmd } = record();
    applyInterface({ ...NONE, pile: 360 }, cmd);
    await Promise.resolve();
    expect(calls).toEqual(["columns:280:500"]);
  });

  test("applyInterface notes beyond live bound is clamped against the new pile", async () => {
    const { calls, cmd } = record();
    applyInterface({ ...NONE, pile: 200, notes: 5000 }, cmd);
    await Promise.resolve();
    expect(calls).toEqual([`columns:200:${900 - 200 - 320}`]);
  });

  test("applyInterface unmeasured table keeps the asked widths", async () => {
    // No table yet (loading): the static bounds Rust checked hold, and the grid's CSS clamps the display.
    const { calls, cmd } = record();
    cmd.measure = () => ({ table: 0, pile: 0 });
    applyInterface({ ...NONE, pile: 300, notes: 900 }, cmd);
    await Promise.resolve();
    expect(calls).toEqual(["columns:300:900"]);
  });

  test("applyInterface code opens the code tab", async () => {
    const { calls, cmd } = record();
    applyInterface({ ...NONE, code: true }, cmd);
    await Promise.resolve();
    expect(calls).toEqual(["code:true"]);
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
