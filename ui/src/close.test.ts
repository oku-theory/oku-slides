// Closing a tab: saveForClose and mayClose, deck tabs, the in-deck workshop and pack tabs.
import { afterAll, beforeAll, describe, expect, mock, test } from "bun:test";
import type { Snapshot, TabApi, TabDocument, TabInfo } from "./api";
import { mayClose, saveForClose } from "./close";
import { setMessages } from "./i18n";
import type { PackMode } from "./packTypes";
import { resetMessages } from "./test/messages";

const ftl = (lang: string) => Bun.file(new URL(`../../crates/oku-slides-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();
beforeAll(async () => {
  setMessages("en", [await ftl("en")]);
});
// The other files of the run expect raw ids.
afterAll(resetMessages);

const snap = (s: Partial<Snapshot>) => ({ dirty: false, path: null, name: "essai", editing: null, ...s }) as Snapshot;
/** An editing view: an in-deck workshop by default. */
const editing = (o: Partial<PackMode> = {}) =>
  ({ pack: "s", name: "S", kind: "layouts", builtin: false, host: "deck", read_only: false, leased: true, display_name: "S", ...o }) as PackMode;
const yes = () => true;

describe("saveForClose", () => {
  test("saveForClose of a clean deck saves nothing", async () => {
    const save = mock(async () => snap({}));
    const api = { snapshot: async () => snap({}), save } as unknown as TabApi;
    expect(await saveForClose(api, { confirm: yes })).toBe(true);
    expect(save).not.toHaveBeenCalled();
  });

  test("saveForClose of a deck with a file saves it in place", async () => {
    const save = mock(async () => snap({ path: "/x.oku-slides.pdf" }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api, { confirm: yes })).toBe(true);
    expect(save).toHaveBeenCalledWith(null, false);
  });

  test("saveForClose of a dirty workshop saves the pack then closes it", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => snap({ dirty: true, editing: editing() }),
      savePack: async () => (calls.push("savePack"), snap({ editing: editing() })),
      closePack: async () => (calls.push("closePack"), snap({})),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    expect(await saveForClose(api, { confirm: yes })).toBe(true);
    expect(calls).toEqual(["savePack", "closePack"]);
  });

  test("saveForClose with history open closes it first and saves the deck found again", async () => {
    const calls: string[] = [];
    const history = { index: 0 } as unknown as Snapshot["history"];
    const api = {
      snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf", history }),
      historyClose: async () => (calls.push("historyClose"), snap({ dirty: true, path: "/x.oku-slides.pdf", history: null })),
      save: async () => (calls.push("save"), snap({ path: "/x.oku-slides.pdf" })),
    } as unknown as TabApi;
    expect(await saveForClose(api, { confirm: yes })).toBe(true);
    expect(calls).toEqual(["historyClose", "save"]);
  });

  test("saveForClose saves the deck still dirty after the workshop closed", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => snap({ dirty: true, editing: editing() }),
      savePack: async () => (calls.push("savePack"), snap({ editing: editing() })),
      closePack: async () => (calls.push("closePack"), snap({ dirty: true, path: "/x.oku-slides.pdf" })),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    expect(await saveForClose(api, { confirm: yes })).toBe(true);
    expect(calls).toEqual(["savePack", "closePack", "save"]);
  });

  test("saveForClose of a new deck saves to the default folder without a dialog", async () => {
    const save = mock(async () => snap({ path: "/Documents/Oku Slides/essai.oku-slides.pdf" }));
    const api = { snapshot: async () => snap({ dirty: true }), save } as unknown as TabApi;
    const pick = mock(async () => null);
    expect(await saveForClose(api, { confirm: yes, pick })).toBe(true);
    expect(save).toHaveBeenCalledWith(null, false);
    expect(pick).not.toHaveBeenCalled();
  });

  test("saveForClose of a file replaced by another deck, given up, stays", async () => {
    const save = mock(async () => snap({ dirty: true, path: "/x.oku-slides.pdf", disk_replaced: true }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api, { confirm: yes, ask: async () => "cancel" })).toBe(false);
    expect(save).toHaveBeenCalledTimes(1);
  });

  test("saveForClose of a replaced file with overwrite saves and closes", async () => {
    const calls: unknown[] = [];
    const save = async (p: string | null, o = false) =>
      (calls.push([p, o]), snap({ path: "/x.oku-slides.pdf", disk_replaced: calls.length === 1 }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api, { confirm: yes, ask: async () => "overwrite" })).toBe(true);
    expect(calls).toEqual([[null, false], [null, true]]);
  });

  test("saveForClose never closes a pack tab's pack", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => snap({ editing: editing({ host: "tab" }) }),
      savePack: async () => (calls.push("savePack"), snap({})),
      closePack: async () => (calls.push("closePack"), snap({})),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    await saveForClose(api, { confirm: yes });
    expect(calls).toEqual([]);
  });

  test("saveForClose takes the first snapshot it is given", async () => {
    const snapshot = mock(async () => snap({}));
    const save = mock(async () => snap({ path: "/x.oku-slides.pdf" }));
    const api = { snapshot, save } as unknown as TabApi;
    expect(await saveForClose(api, { confirm: yes, first: snap({ dirty: true, path: "/x.oku-slides.pdf" }) })).toBe(true);
    expect(snapshot).not.toHaveBeenCalled();
    expect(save).toHaveBeenCalledTimes(1);
  });
});

describe("mayClose", () => {
  const tab = (dirty: boolean, document: TabDocument = { kind: "deck", path: null, deck_uid: null }): TabInfo => ({
    id: "t1",
    title: "essai",
    dirty,
    editing_pack: document.kind === "pack",
    pristine: !dirty,
    theme: null,
    document,
  });
  const packDoc: TabDocument = { kind: "pack", pack: "s" };
  const deps = { confirm: yes };
  /** A pack tab's fake API: each call noted in `calls`; `saved` answers savePack. */
  const packApi = (saved: () => Promise<Snapshot>) => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => (calls.push("snapshot"), snap({})),
      savePack: async () => (calls.push("savePack"), saved()),
      saveGuarded: async () => (calls.push("saveGuarded"), snap({})),
      save: async () => (calls.push("save"), snap({})),
      closePack: async () => (calls.push("closePack"), snap({})),
    } as unknown as TabApi;
    return { api, calls };
  };
  const packSnap = (o: Partial<PackMode> = {}, dirty = true) => snap({ dirty, editing: editing({ host: "tab", display_name: "Acme", ...o }) });

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
    expect(calls).toEqual(["savePack"]);
    expect(confirm).toHaveBeenCalledWith("Quit without saving the modified pack?");
  });

  test("pack tab still dirty with confirm declined stays", async () => {
    const { api, calls } = packApi(async () => packSnap());
    expect(await mayClose(tab(true, packDoc), packSnap(), api, { confirm: () => false })).toBeNull();
    expect(calls).toEqual(["savePack"]);
  });

  test("pack tab saves then keeps without closePack", async () => {
    const { api, calls } = packApi(async () => packSnap({}, false));
    const confirm = mock(() => true);
    expect(await mayClose(tab(true, packDoc), packSnap(), api, { confirm })).toBe("keep");
    expect(calls).toEqual(["savePack"]);
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
    expect(calls).toEqual(["savePack"]);
    expect(confirm).not.toHaveBeenCalled();
  });

  test("read-only dirty deck workshop asks, then closes the pack and saves the deck", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => (calls.push("snapshot"), snap({})),
      savePack: async () => (calls.push("savePack"), snap({})),
      closePack: async () => (calls.push("closePack"), snap({ dirty: true, path: "/x.oku-slides.pdf" })),
      save: async () => (calls.push("save"), snap({ path: "/x.oku-slides.pdf" })),
    } as unknown as TabApi;
    const confirm = mock(() => true);
    const workshop = snap({ dirty: true, editing: editing({ read_only: true, leased: false, display_name: "Acme" }) });
    expect(await mayClose(tab(true), workshop, api, { confirm })).toBe("keep");
    expect(calls).toEqual(["closePack", "save"]);
    expect(confirm).toHaveBeenCalledWith(
      "⁨Acme⁩ cannot be saved here. Close this tab and lose its changes? To keep them, ask the agent to save it as a new pack.",
    );
  });

  test("read-only dirty deck workshop declined stays and closes nothing", async () => {
    const calls: string[] = [];
    const api = {
      savePack: async () => (calls.push("savePack"), snap({})),
      closePack: async () => (calls.push("closePack"), snap({})),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    const workshop = snap({ dirty: true, editing: editing({ read_only: true, leased: false }) });
    expect(await mayClose(tab(true), workshop, api, { confirm: () => false })).toBeNull();
    expect(calls).toEqual([]);
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
