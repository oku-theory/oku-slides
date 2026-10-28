// Closing a tab: saveForClose and mayClose, deck tabs, pack tabs and theme tabs.
import { afterAll, beforeAll, describe, expect, mock, test } from "bun:test";
import type { Snapshot, TabApi, TabDocument, TabInfo } from "./api";
import { mayClose, saveForClose } from "./close";
import { setMessages } from "./i18n";
import type { PackMode, ThemeTabView } from "./packTypes";
import { resetMessages } from "./test/messages";

const ftl = (lang: string) => Bun.file(new URL(`../../crates/oku-slides-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();
beforeAll(async () => {
  setMessages("en", [await ftl("en")]);
});
// The other files of the run expect raw ids.
afterAll(resetMessages);

const snap = (s: Partial<Snapshot>) => ({ dirty: false, path: null, name: "essai", editing: null, ...s }) as Snapshot;
/** A pack tab's editing view. */
const editing = (o: Partial<PackMode> = {}) =>
  ({ pack: "s", name: "S", kind: "layouts", builtin: false, read_only: false, leased: true, display_name: "S", ...o }) as PackMode;
const yes = () => true;

describe("saveForClose", () => {
  test("saveForClose of a clean deck saves nothing", async () => {
    const save = mock(async () => snap({}));
    const api = { snapshot: async () => snap({}), save } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(save).not.toHaveBeenCalled();
  });

  test("saveForClose of a deck with a file saves it in place", async () => {
    const save = mock(async () => snap({ path: "/x.oku-slides.pdf" }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(save).toHaveBeenCalledWith(null, false);
  });

  test("saveForClose with history open closes it first and saves the deck found again", async () => {
    const calls: string[] = [];
    const history = { index: 0 } as unknown as Snapshot["history"];
    const api = {
      snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf", history }),
      historyClose: async () => (calls.push("historyClose"), snap({ dirty: true, path: "/x.oku-slides.pdf", history: null })),
      save: async () => (calls.push("save"), snap({ path: "/x.oku-slides.pdf" })),
    } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(calls).toEqual(["historyClose", "save"]);
  });

  test("saveForClose of a new deck saves to the default folder without a dialog", async () => {
    const save = mock(async () => snap({ path: "/Documents/Oku Slides/essai.oku-slides.pdf" }));
    const api = { snapshot: async () => snap({ dirty: true }), save } as unknown as TabApi;
    const pick = mock(async () => null);
    expect(await saveForClose(api, { pick })).toBe(true);
    expect(save).toHaveBeenCalledWith(null, false);
    expect(pick).not.toHaveBeenCalled();
  });

  test("saveForClose of a file replaced by another deck, given up, stays", async () => {
    const save = mock(async () => snap({ dirty: true, path: "/x.oku-slides.pdf", disk_replaced: true }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api, { ask: async () => "cancel" })).toBe(false);
    expect(save).toHaveBeenCalledTimes(1);
  });

  test("saveForClose of a replaced file with overwrite saves and closes", async () => {
    const calls: unknown[] = [];
    const save = async (p: string | null, o = false) =>
      (calls.push([p, o]), snap({ path: "/x.oku-slides.pdf", disk_replaced: calls.length === 1 }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api, { ask: async () => "overwrite" })).toBe(true);
    expect(calls).toEqual([[null, false], [null, true]]);
  });

  test("saveForClose of a pack tab answers its dirty flag", async () => {
    const calls: string[] = [];
    const api = {
      savePack: async () => (calls.push("savePack"), snap({})),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    const answers = [
      await saveForClose(api, { first: snap({ dirty: true, editing: editing() }) }),
      await saveForClose(api, { first: snap({ dirty: false, editing: editing() }) }),
    ];
    expect([answers, calls]).toEqual([[false, true], []]);
  });

  test("saveForClose takes the first snapshot it is given", async () => {
    const snapshot = mock(async () => snap({}));
    const save = mock(async () => snap({ path: "/x.oku-slides.pdf" }));
    const api = { snapshot, save } as unknown as TabApi;
    expect(await saveForClose(api, { first: snap({ dirty: true, path: "/x.oku-slides.pdf" }) })).toBe(true);
    expect(snapshot).not.toHaveBeenCalled();
    expect(save).toHaveBeenCalledTimes(1);
  });
});

describe("mayClose", () => {
  const tab = (dirty: boolean, document: TabDocument = { kind: "deck", path: null, deck_uid: null }): TabInfo => ({
    id: "t1",
    title: "essai",
    dirty,
    pristine: !dirty,
    theme: null,
    document,
  });
  const packDoc: TabDocument = { kind: "pack", pack: "s" };
  const deps = { confirm: yes };
  /** A pack tab's fake API: each call noted in `calls`; `saved` answers savePack and the snapshot read after it. */
  const packApi = (saved: () => Promise<Snapshot>) => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => (calls.push("snapshot"), saved()),
      savePack: async () => (calls.push("savePack"), saved()),
      saveGuarded: async () => (calls.push("saveGuarded"), snap({})),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    return { api, calls };
  };
  const packSnap = (o: Partial<PackMode> = {}, dirty = true) => snap({ dirty, editing: editing({ display_name: "Acme", ...o }) });

  test("mayClose of a clean tab keeps and saves nothing", async () => {
    const api = { snapshot: mock(async () => snap({})) } as unknown as TabApi;
    expect(await mayClose(tab(false), null, api, deps)).toBe("keep");
    expect(api.snapshot).not.toHaveBeenCalled();
  });

  test("mayClose of a dirty tab saves without a question and keeps", async () => {
    const save = mock(async () => snap({ path: "/x.oku-slides.pdf" }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await mayClose(tab(true), null, api, deps)).toBe("keep");
    expect(save).toHaveBeenCalledTimes(1);
  });

  test("mayClose of a dirty tab whose save is refused (file replaced) stays", async () => {
    const save = mock(async () => snap({ dirty: true, path: "/x.oku-slides.pdf", disk_replaced: true }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await mayClose(tab(true), null, api, { ...deps, ask: async () => "cancel" })).toBeNull();
  });

  test("pack tab not dirty keeps", async () => {
    const { api, calls } = packApi(async () => snap({}));
    expect(await mayClose(tab(false, packDoc), packSnap({}, false), api, deps)).toBe("keep");
    expect(calls).toEqual([]);
  });

  test("pack tab still dirty after savePack asks then discards", async () => {
    const { api, calls } = packApi(async () => packSnap());
    const confirm = mock(() => true);
    expect(await mayClose(tab(true, packDoc), packSnap(), api, { confirm })).toBe("discard");
    expect(calls).toEqual(["savePack", "snapshot"]);
    expect(confirm).toHaveBeenCalledWith("Quit without saving the modified pack?");
  });

  test("pack tab still dirty with confirm declined stays", async () => {
    const { api, calls } = packApi(async () => packSnap());
    expect(await mayClose(tab(true, packDoc), packSnap(), api, { confirm: () => false })).toBeNull();
    expect(calls).toEqual(["savePack", "snapshot"]);
  });

  test("pack tab saves then keeps", async () => {
    const { api, calls } = packApi(async () => packSnap({}, false));
    const confirm = mock(() => true);
    expect(await mayClose(tab(true, packDoc), packSnap(), api, { confirm })).toBe("keep");
    expect(calls).toEqual(["savePack", "snapshot"]);
    expect(confirm).not.toHaveBeenCalled();
  });

  test("packTabVerdict of an unsent clean pack tab asks ui-confirm-quit-pack", async () => {
    const { api, calls } = packApi(async () => packSnap({ deck_only: true, unsent: true }, false));
    const confirm = mock(() => false);
    expect(await mayClose(tab(true, packDoc), packSnap({ deck_only: true, unsent: true }, false), api, { confirm })).toBeNull();
    expect(calls).toEqual(["savePack", "snapshot"]);
    expect(confirm).toHaveBeenCalledWith("Quit without saving the modified pack?");
  });

  test("packTabVerdict re-reads the snapshot after savePack", async () => {
    let forwarded = false;
    const calls: string[] = [];
    const api = {
      snapshot: async () => (calls.push("snapshot"), packSnap({ deck_only: true, unsent: !forwarded }, false)),
      // The save's answer predates the forward: still unsent.
      savePack: async () => (calls.push("savePack"), (forwarded = true), packSnap({ deck_only: true, unsent: true }, false)),
    } as unknown as TabApi;
    const confirm = mock(() => true);
    expect(await mayClose(tab(true, packDoc), packSnap({ deck_only: true, unsent: true }), api, { confirm })).toBe("keep");
    expect(calls).toEqual(["savePack", "snapshot"]);
    expect(confirm).not.toHaveBeenCalled();
  });

  test("pack tab savePack rejection stays open", async () => {
    const { api } = packApi(async () => Promise.reject(new Error("disque plein")));
    const confirm = mock(() => true);
    await expect(mayClose(tab(true, packDoc), packSnap(), api, { confirm })).rejects.toThrow("disque plein");
    expect(confirm).not.toHaveBeenCalled();
  });

  test("read-only dirty pack tab asks without saving then discards", async () => {
    const { api, calls } = packApi(async () => snap({}));
    const confirm = mock(() => true);
    expect(await mayClose(tab(true, packDoc), packSnap({ read_only: true, leased: false }), api, { confirm })).toBe("discard");
    expect(calls).toEqual([]);
    expect(confirm).toHaveBeenCalledWith(
      "⁨Acme⁩ cannot be saved here. Close this tab and lose its changes? To keep them, ask the agent to save it as a new pack.",
    );
  });

  test("read-only dirty pack tab declined stays without saving", async () => {
    const { api, calls } = packApi(async () => snap({}));
    expect(await mayClose(tab(true, packDoc), packSnap({ read_only: true, leased: false }), api, { confirm: () => false })).toBeNull();
    expect(calls).toEqual([]);
  });

  test("a free unleased dirty pack tab saves", async () => {
    const { api, calls } = packApi(async () => packSnap({ leased: true }, false));
    const confirm = mock(() => true);
    expect(await mayClose(tab(true, packDoc), packSnap({ read_only: false, leased: false }), api, { confirm })).toBe("keep");
    expect(calls).toEqual(["savePack", "snapshot"]);
    expect(confirm).not.toHaveBeenCalled();
  });

  test("clean pack tab never calls savePack", async () => {
    const { api, calls } = packApi(async () => snap({}));
    expect(await mayClose(tab(true, packDoc), packSnap({}, false), api, deps)).toBe("keep");
    expect(calls).toEqual([]);
  });

  test("cached pack kind with a deck snapshot takes the deck path", async () => {
    const calls: string[] = [];
    const api = {
      savePack: async () => (calls.push("savePack"), snap({})),
      save: async () => (calls.push("save"), snap({ path: "/x.oku-slides.pdf" })),
    } as unknown as TabApi;
    expect(await mayClose(tab(true, packDoc), snap({ dirty: true, path: "/x.oku-slides.pdf" }), api, deps)).toBe("keep");
    expect(calls).toEqual(["save"]);
  });

  const themeDoc: TabDocument = { kind: "theme", theme: "t" };
  /** A theme tab's live snapshot: no editing view, the draft in `theming`. */
  const themeSnap = (o: Partial<ThemeTabView> = {}, dirty = true) =>
    snap({ dirty, editing: null, theming: { read_only: false, leased: true, display_name: "Maison", session_version: null, ...o } as ThemeTabView });
  /** A theme tab's fake API: `after` is the snapshot read once saveTheme answered. */
  const themeApi = (after: () => Snapshot) => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => (calls.push("snapshot"), after()),
      saveTheme: async () => (calls.push("saveTheme"), null),
      savePack: async () => (calls.push("savePack"), snap({})),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    return { api, calls };
  };

  test("mayClose of a clean theme tab keeps and saves nothing", async () => {
    const { api, calls } = themeApi(() => themeSnap({}, false));
    expect(await mayClose(tab(false, themeDoc), themeSnap({}, false), api, deps)).toBe("keep");
    expect(calls).toEqual([]);
  });

  test("mayClose of a dirty theme tab saves its theme then keeps", async () => {
    const { api, calls } = themeApi(() => themeSnap({}, false));
    const confirm = mock(() => true);
    expect(await mayClose(tab(true, themeDoc), themeSnap(), api, { confirm })).toBe("keep");
    expect(calls).toEqual(["saveTheme", "snapshot"]);
    expect(confirm).not.toHaveBeenCalled();
  });

  test("mayClose of a read-only dirty theme tab asks without saving then discards", async () => {
    const { api, calls } = themeApi(() => themeSnap());
    const confirm = mock(() => true);
    expect(await mayClose(tab(true, themeDoc), themeSnap({ read_only: true, leased: false }), api, { confirm })).toBe("discard");
    expect(calls).toEqual([]);
    expect(confirm).toHaveBeenCalledWith("⁨Maison⁩ cannot be saved here. Close this tab and lose its changes?");
  });

  test("mayClose of a theme tab still dirty after its save asks then discards", async () => {
    const { api, calls } = themeApi(() => themeSnap());
    const confirm = mock(() => true);
    expect(await mayClose(tab(true, themeDoc), themeSnap(), api, { confirm })).toBe("discard");
    expect(calls).toEqual(["saveTheme", "snapshot"]);
    expect(confirm).toHaveBeenCalledWith("Quit without saving the modified theme?");
  });

  test("mayClose of a theme tab still dirty with confirm declined stays", async () => {
    const { api } = themeApi(() => themeSnap());
    expect(await mayClose(tab(true, themeDoc), themeSnap(), api, { confirm: () => false })).toBeNull();
  });

  test("unknown document kind takes the deck path", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => (calls.push("snapshot"), snap({ dirty: true, path: "/x.oku-slides.pdf" })),
      savePack: async () => (calls.push("savePack"), snap({})),
      save: async () => (calls.push("save"), snap({ path: "/x.oku-slides.pdf" })),
    } as unknown as TabApi;
    const unknown = { kind: "board" } as unknown as TabDocument;
    expect(await mayClose(tab(true, unknown), null, api, deps)).toBe("keep");
    expect(calls).toEqual(["snapshot", "save"]);
  });
});
