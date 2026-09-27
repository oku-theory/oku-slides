import { describe, expect, test } from "bun:test";

import { mergedNotice, openGuarded, saveGuarded } from "./merge";

const snap = (over: Partial<Record<string, unknown>> = {}) =>
  ({ path: "/d/deck.oku-slides.pdf", name: "deck", dirty: true, disk_replaced: false, ...over }) as never;

describe("saveGuarded", () => {
  test("saveGuarded with an unchanged file saves once", async () => {
    const calls: unknown[] = [];
    const api = { save: async (p: string | null, o = false) => (calls.push([p, o]), snap({ dirty: false })) } as never;
    const out = await saveGuarded(api, snap(), false);
    expect(calls).toEqual([[null, false]]);
    expect(out).not.toBeNull();
  });

  test("saveGuarded on a replaced file and overwrite chosen saves with overwrite", async () => {
    const calls: unknown[] = [];
    let first = true;
    const api = {
      save: async (p: string | null, o = false) => {
        calls.push([p, o]);
        const r = first ? snap({ disk_replaced: true }) : snap({ dirty: false });
        first = false;
        return r;
      },
    } as never;
    await saveGuarded(api, snap(), false, async () => "overwrite");
    expect(calls).toEqual([[null, false], [null, true]]);
  });

  test("saveGuarded on a replaced file and save as chosen saves to the picked path", async () => {
    const calls: unknown[] = [];
    let first = true;
    const api = {
      save: async (p: string | null, o = false) => {
        calls.push([p, o]);
        const r = first ? snap({ disk_replaced: true }) : snap({ dirty: false });
        first = false;
        return r;
      },
    } as never;
    await saveGuarded(api, snap(), false, async () => "save-as", async () => "/d/autre.oku-slides.pdf");
    expect(calls).toEqual([[null, false], ["/d/autre.oku-slides.pdf", true]]);
  });

  test("saveGuarded on a replaced file and save as the same file saves with overwrite", async () => {
    const calls: unknown[] = [];
    const api = {
      save: async (p: string | null, o = false) => (calls.push([p, o]), snap({ disk_replaced: calls.length === 1 })),
    } as never;
    const out = await saveGuarded(api, snap(), false, async () => "save-as", async () => "/d/deck.oku-slides.pdf");
    expect(calls).toEqual([[null, false], ["/d/deck.oku-slides.pdf", true]]);
    expect(out).not.toBeNull();
  });

  test("saveGuarded returns null when the last save still reports a replaced file", async () => {
    const api = { save: async () => snap({ disk_replaced: true }) } as never;
    expect(await saveGuarded(api, snap(), false, async () => "overwrite")).toBeNull();
    expect(await saveGuarded(api, snap(), false, async () => "save-as", async () => "/d/deck.oku-slides.pdf")).toBeNull();
  });

  test("saveGuarded on a replaced file and cancel returns null", async () => {
    const api = { save: async () => snap({ disk_replaced: true }) } as never;
    expect(await saveGuarded(api, snap(), false, async () => "cancel")).toBeNull();
  });

  test("saveGuarded on a replaced file asks with the file name only", async () => {
    const asked: string[] = [];
    const api = { save: async () => snap({ disk_replaced: true }) } as never;
    await saveGuarded(api, snap(), false, async (file) => (asked.push(file), "cancel"));
    expect(asked).toEqual(["deck.oku-slides.pdf"]);
  });

  test("saveGuarded as a new name saves with overwrite, confirmed by the dialog", async () => {
    const calls: unknown[] = [];
    const api = { save: async (p: string | null, o = false) => (calls.push([p, o]), snap({ dirty: false })) } as never;
    const out = await saveGuarded(api, snap(), true, undefined, async () => "/d/deck.oku-slides.pdf");
    expect(calls).toEqual([["/d/deck.oku-slides.pdf", true]]);
    expect(out).not.toBeNull();
  });

  test("saveGuarded without a file and no path picked saves nothing", async () => {
    const calls: unknown[] = [];
    const api = { save: async (p: string | null) => (calls.push(p), snap()) } as never;
    expect(await saveGuarded(api, snap({ path: null }), false, undefined, async () => null)).toBeNull();
    expect(calls).toEqual([]);
  });
});

describe("openGuarded", () => {
  const calls: string[] = [];
  const asked: string[][] = [];
  const deps = (tab: string | null, choice: "merge" | "apart" | "cancel", notice: string | null = "Fusionné") => ({
    sameDeck: async () => ({ tab, title: tab ? "Roadmap" : null }),
    openTab: async () => void calls.push("open"),
    merge: async (t: string) => (calls.push(`merge:${t}`), { notice }),
    activate: async (t: string) => void calls.push(`activate:${t}`),
    ask: async (file: string, title: string) => (asked.push([file, title]), choice),
  });

  test("openGuarded without a tab holding the deck just opens", async () => {
    calls.length = 0;
    expect(await openGuarded("w-1", "/d/x.oku-slides.pdf", deps(null, "merge"))).toBeNull();
    expect(calls).toEqual(["open"]);
  });

  test("openGuarded and merge chosen merges into that tab then activates it", async () => {
    calls.length = 0;
    const out = await openGuarded("w-1", "/d/x.oku-slides.pdf", deps("t-3", "merge"));
    expect(calls).toEqual(["merge:t-3", "activate:t-3"]);
    expect(out).toEqual({ tab: "t-3", text: "Fusionné" });
  });

  test("openGuarded asks with the file name and the tab title", async () => {
    asked.length = 0;
    await openGuarded("w-1", "/d/x.oku-slides.pdf", deps("t-3", "cancel"));
    expect(asked).toEqual([["x.oku-slides.pdf", "Roadmap"]]);
  });

  test("openGuarded and merge without a notice returns null", async () => {
    calls.length = 0;
    expect(await openGuarded("w-1", "/d/x.oku-slides.pdf", deps("t-3", "merge", null))).toBeNull();
  });

  test("openGuarded and apart chosen opens a new tab", async () => {
    calls.length = 0;
    expect(await openGuarded("w-1", "/d/x.oku-slides.pdf", deps("t-3", "apart"))).toBeNull();
    expect(calls).toEqual(["open"]);
  });

  test("openGuarded and cancel does nothing", async () => {
    calls.length = 0;
    expect(await openGuarded("w-1", "/d/x.oku-slides.pdf", deps("t-3", "cancel"))).toBeNull();
    expect(calls).toEqual([]);
  });
});

describe("mergedNotice", () => {
  const win = (active: string, ...tabs: string[]) => ({ active, tabs: tabs.map((id) => ({ id })) });
  const merged = { tab: "t-3", text: "Fusionné" };

  test("mergedNotice without a merge shows nothing", () => {
    expect(mergedNotice(null, win("t-1", "t-1"))).toBeNull();
  });

  test("mergedNotice waits until the merged tab of this window is active", () => {
    expect(mergedNotice(merged, win("t-1", "t-1", "t-3"))).toBeNull();
    expect(mergedNotice(merged, win("t-3", "t-1", "t-3"))).toBe("Fusionné");
  });

  test("mergedNotice for a tab of another window shows it in this window", () => {
    expect(mergedNotice(merged, win("t-1", "t-1", "t-2"))).toBe("Fusionné");
  });
});
