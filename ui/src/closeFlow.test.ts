// Closing one tab: guards, stalls, saves, the opener offer, and the deps builder.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { closeFlowDeps, closeTabFlow } from "./closeFlow";
import { isolate, setMessages } from "./i18n";
import { OFFER, T0, deferred, fake, linkedOf, packEditing, snap, tabOf, themeTabOf, themeTabView } from "./test/closeFlowFake";
import { resetMessages } from "./test/messages";

const ftl = (lang: string) => Bun.file(new URL(`../../crates/oku-slides-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();
beforeAll(async () => {
  setMessages("en", [await ftl("en")]);
});
afterAll(resetMessages);

const PENDING = "report This tab is still saving: close it again in a moment.";
const ASK_STUCK = "confirm This tab is still saving. Close it anyway and lose what has not been saved?";
const ASK_QUIT = "confirm Quit without saving the modified pack?";
const ASK_UPDATE = "confirm Update “⁨Roadmap⁩” to this version of the pack “⁨Acme⁩”?";
const timeout = () => Promise.resolve("timeout" as const);

describe("closeTabFlow, guards", () => {
  test("runs the guard before freshView, mayClose and close", async () => {
    const f = fake([tabOf("a", true)]);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    const at = (s: string) => f.log.indexOf(s);
    expect(at("guard a")).toBe(0);
    expect(at("guard a")).toBeLessThan(at("fresh"));
    expect(at("fresh")).toBeLessThan(at("snapshot a"));
    expect(at("save a")).toBeLessThan(at("close a false"));
  });

  test("runs the guard again right before close", async () => {
    const f = fake([tabOf("a")]);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", "guard a", "fresh", "close a false"]);
  });

  test("a second close of the same id while one runs returns false and asks nothing", async () => {
    const f = fake([tabOf("a")]);
    const hold = deferred<"done">();
    f.state.guard = (_id, n) => (n === 1 ? hold.promise : Promise.resolve("done"));
    const first = closeTabFlow("a", f.deps());
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["guard a"]);
    hold.resolve("done");
    expect(await first).toBe(true);
  });

  test("a timed-out guard refuses the close and asks nothing", async () => {
    const f = fake([tabOf("a", true)]);
    f.state.guard = timeout;
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["guard a", PENDING]);
  });

  test("a rejected guard counts like a timeout", async () => {
    const f = fake([tabOf("a")]);
    f.state.guard = () => Promise.reject(new Error("garde"));
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["guard a", PENDING]);
    expect(f.input.stalls.get("a")).toBe(T0);
  });

  test.each([
    ["a pack tab", true],
    ["a deck tab", false],
  ])("a second timeout within 30 s asks one generic confirm (%s)", async (_name, pack) => {
    const f = fake([tabOf("a", true, pack)]);
    f.state.guard = timeout;
    f.input.stalls.set("a", T0 - 10_000);
    f.state.answers = [false];
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["guard a", "fresh", ASK_STUCK]);
  });

  test("the stalled close confirms with Close without saving, a danger", async () => {
    const f = fake([tabOf("a", true)]);
    f.state.guard = timeout;
    f.input.stalls.set("a", T0 - 10_000);
    f.state.answers = [false];
    await closeTabFlow("a", f.deps());
    expect(f.questions.map(({ ok, cancel, danger }) => ({ ok, cancel, danger }))).toEqual([{ ok: "Close without saving", cancel: undefined, danger: true }]);
  });

  test("yes on a clean tab closes without discard", async () => {
    const f = fake([tabOf("a")]);
    f.state.guard = timeout;
    f.input.stalls.set("a", T0 - 10_000);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", ASK_STUCK, "close a false"]);
  });

  test("yes on a dirty tab closes with discard", async () => {
    const f = fake([tabOf("a", true)]);
    f.state.guard = timeout;
    f.input.stalls.set("a", T0 - 10_000);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", ASK_STUCK, "close a true"]);
  });

  test("a second stall on a clean tab whose plain close is refused falls back to discard", async () => {
    const f = fake([tabOf("a")]);
    f.state.guard = timeout;
    f.state.refused.add("a false");
    f.input.stalls.set("a", T0 - 10_000);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", ASK_STUCK, "close a false", "run error refusé", "close a true"]);
  });

  test("the tab gone after the second timeout closes nothing", async () => {
    const f = fake([]);
    f.state.guard = timeout;
    f.input.stalls.set("a", T0 - 10_000);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh"]);
  });

  test("stall entries older than 30 s are pruned", async () => {
    const f = fake([tabOf("a")]);
    f.state.guard = timeout;
    f.input.stalls.set("old", T0 - 31_000);
    f.input.stalls.set("a", T0 - 31_000);
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["guard a", PENDING]);
    expect([...f.input.stalls]).toEqual([["a", T0]]);
  });

  test("a flush at the last guard that dirties the tab runs mayClose once more", async () => {
    const f = fake([tabOf("a")]);
    f.state.guard = (_id, n) => {
      if (n === 2) f.state.tabs = [tabOf("a", true)];
      return Promise.resolve("done");
    };
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", "guard a", "fresh", "snapshot a", "save a", "guard a", "fresh", "close a false"]);
  });

  test("a write flushed at the last guard is the version offered", async () => {
    const f = fake([tabOf("a", false, true)]);
    f.state.guard = (_id, n) => {
      if (n === 2) f.state.offer = OFFER;
      return Promise.resolve("done");
    };
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.slice(0, 5)).toEqual(["guard a", "fresh", "guard a", "fresh", "opener a"]);
    expect(f.log).toContain("take t-1 p v2 d1");
  });

  test("a tab list read failure is reported and the tab stays", async () => {
    const f = fake([tabOf("a")]);
    f.input.freshView = () => Promise.reject(new Error("vue"));
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["guard a", "report vue"]);
    expect(f.input.closing.size).toBe(0);
  });

  test("the tab gone after the guard closes nothing", async () => {
    const f = fake([]);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh"]);
  });
});

describe("closeTabFlow, live state", () => {
  test("a clean deck tab reads no snapshot", async () => {
    const f = fake([tabOf("a")]);
    await closeTabFlow("a", f.deps());
    expect(f.log.filter((l) => l.startsWith("snapshot"))).toEqual([]);
  });

  test("closeTabFlow clean pack tab skips the snapshot", async () => {
    const f = fake([tabOf("a", false, true)]);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", "guard a", "fresh", "opener a", "close a false"]);
  });

  test("a pack tab snapshot rejection tries savePack before offering to discard", async () => {
    const f = fake([tabOf("a", true, true)]);
    f.state.snapshot = () => Promise.reject(new Error("rendu"));
    f.state.savePack = () => Promise.reject(new Error("rendu"));
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", "snapshot a", "report rendu", "fresh", "savePack a", ASK_QUIT, "close a true"]);
  });

  test("a pack tab snapshot rejection whose savePack resolves closes with keep", async () => {
    const f = fake([tabOf("a", true, true)]);
    f.state.snapshot = () => Promise.reject(new Error("rendu"));
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", "snapshot a", "report rendu", "fresh", "savePack a", "close a false"]);
  });

  test("a snapshot rejection of a tab refreshed to a deck stays open", async () => {
    const f = fake([tabOf("a", true, true)]);
    f.state.snapshot = () => {
      f.state.tabs = [tabOf("a", true)];
      return Promise.reject(new Error("rendu"));
    };
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["guard a", "fresh", "snapshot a", "report rendu", "fresh"]);
  });

  test("a deck tab snapshot rejection reports and stays open", async () => {
    const f = fake([tabOf("a", true)]);
    f.state.snapshot = () => Promise.reject(new Error("rendu"));
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["guard a", "fresh", "snapshot a", "report rendu", "fresh"]);
  });

  test("the read-only confirm isolates the display name", async () => {
    const f = fake([tabOf("a", true, true)]);
    f.state.snapshot = async () => snap({ dirty: true, editing: packEditing({ read_only: true, leased: false, display_name: "Acme" }) });
    f.state.answers = [false];
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toContain(
      "confirm ⁨Acme⁩ cannot be saved here. Close this tab and lose its changes? To keep them, ask the agent to save it as a new pack.",
    );
  });

  test("a discarded pack tab sends discard and offers nothing", async () => {
    const f = fake([tabOf("a", true, true)]);
    f.state.snapshot = async () => snap({ dirty: true, editing: packEditing({ read_only: true, leased: false }) });
    f.state.offer = OFFER;
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.at(-1)).toBe("close a true");
    expect(f.log.filter((l) => l.startsWith("opener") || l.startsWith("guard"))).toEqual(["guard a"]);
  });
});

describe("closeTabFlow, theme tabs", () => {
  const ASK_QUIT_THEME = "confirm Quit without saving the modified theme?";

  test("a kept theme tab saves its theme and closes without an opener offer", async () => {
    const f = fake([themeTabOf("a", true)]);
    let saved = false;
    f.state.snapshot = async () => snap({ dirty: !saved, theming: themeTabView() });
    f.state.saveTheme = async () => {
      saved = true;
      f.state.tabs = [themeTabOf("a", false)];
      return null;
    };
    f.state.offer = OFFER;
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", "snapshot a", "saveTheme a", "snapshot a", "guard a", "fresh", "close a false"]);
  });

  test("a theme tab snapshot rejection tries saveTheme before offering to discard", async () => {
    const f = fake([themeTabOf("a", true)]);
    f.state.snapshot = () => Promise.reject(new Error("rendu"));
    f.state.saveTheme = () => Promise.reject(new Error("rendu"));
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", "snapshot a", "report rendu", "fresh", "saveTheme a", ASK_QUIT_THEME, "close a true"]);
  });

  test("a theme tab snapshot rejection whose saveTheme resolves closes with keep", async () => {
    const f = fake([themeTabOf("a", true)]);
    f.state.snapshot = () => Promise.reject(new Error("rendu"));
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard a", "fresh", "snapshot a", "report rendu", "fresh", "saveTheme a", "close a false"]);
  });
});

describe("closeTabFlow, the opener offer", () => {
  test("a kept pack tab offers, closes, then updates the opener tab", async () => {
    const f = fake([tabOf("a", false, true)]);
    f.state.offer = OFFER;
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.slice(4)).toEqual(["opener a", ASK_UPDATE, "close a false", "take t-1 p v2 d1", "refresh"]);
  });

  test("the offer confirm carries the title and the pack name, each isolated", async () => {
    const f = fake([tabOf("a", false, true)]);
    f.state.offer = { ...OFFER, title: "Feuille de route", pack_name: "Maison" };
    await closeTabFlow("a", f.deps());
    expect(f.log).toContain("confirm Update “⁨Feuille de route⁩” to this version of the pack “⁨Maison⁩”?");
  });

  test("the offer confirm updates with Update, its cancel is Not now, not a danger", async () => {
    const f = fake([tabOf("a", false, true)]);
    f.state.offer = OFFER;
    await closeTabFlow("a", f.deps());
    expect(f.questions.map(({ ok, cancel, danger }) => ({ ok, cancel, danger }))).toEqual([{ ok: "Update", cancel: "Not now", danger: false }]);
  });

  test("a declined offer closes without updating", async () => {
    const f = fake([tabOf("a", false, true)]);
    f.state.offer = OFFER;
    f.state.answers = [false];
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.slice(4)).toEqual(["opener a", ASK_UPDATE, "close a false"]);
  });

  test("no opener asks nothing", async () => {
    const f = fake([tabOf("a", false, true)]);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.filter((l) => l.startsWith("confirm"))).toEqual([]);
  });

  test("a packOpener rejection is reported and the tab closes", async () => {
    const f = fake([tabOf("a", false, true)]);
    f.state.offer = new Error("opener");
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.slice(4)).toEqual(["opener a", "report opener", "close a false"]);
  });

  test("a refused close updates nothing", async () => {
    const f = fake([tabOf("a", false, true)]);
    f.state.offer = OFFER;
    f.state.refused.add("a false");
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log.filter((l) => l.startsWith("take") || l === "refresh")).toEqual([]);
  });

  test("an update rejection is reported and still refreshes", async () => {
    const f = fake([tabOf("a", false, true)]);
    f.state.offer = OFFER;
    f.state.take = () => Promise.reject(new Error("expirée"));
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.slice(-3)).toEqual(["take t-1 p v2 d1", "report expirée", "refresh"]);
  });

  test("closeTabFlow of a deck-pack tab offers no opener", async () => {
    const f = fake([{ ...tabOf("a", true), document: { kind: "deck_pack", deck_tab: "t-1", pack: "p" } }]);
    f.state.offer = OFFER;
    f.state.snapshot = () => Promise.resolve(snap({ dirty: !!f.state.tabs[0]?.dirty, editing: packEditing({ deck_only: true }) }));
    f.state.savePack = () => ((f.state.tabs = f.state.tabs.map((t) => ({ ...t, dirty: false }))), Promise.resolve(snap()));
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.filter((l) => l.startsWith("opener") || l.startsWith("close"))).toEqual(["close a false"]);
  });

  test("a deck tab never asks packOpener", async () => {
    const f = fake([tabOf("a", true)]);
    f.state.offer = OFFER;
    await closeTabFlow("a", f.deps());
    expect(f.log.filter((l) => l.startsWith("opener"))).toEqual([]);
  });
});

describe("closeTabFlow, linked deck-pack tabs", () => {
  /** Owner `a` clean, its deck-pack tab `p`; `p`'s snapshot is unsent until a save delivers when `delivers`. */
  const owned = (delivers: boolean) => {
    const f = fake([tabOf("a"), linkedOf("p", "a", true)]);
    let unsent = true;
    const deckSnapshot = f.state.snapshot;
    const packSave = f.state.savePack;
    f.state.snapshot = (id) => (id === "p" ? Promise.resolve(snap({ editing: packEditing({ deck_only: true, unsent }) })) : deckSnapshot(id));
    f.state.savePack = (id) => {
      if (delivers) unsent = false;
      return packSave(id);
    };
    return f;
  };

  test("closeTabFlow of a clean owner saves its unsent linked tab first", async () => {
    const f = owned(true);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log).toEqual([
      "guard a", "fresh", "guard p", "snapshot p", "savePack p", "snapshot p", "fresh",
      "snapshot a", "save a", "guard a", "fresh", "close a false",
    ]);
  });

  test("closeTabFlow of an owner whose linked tab stays unsent asks ui-confirm-quit-pack", async () => {
    const f = owned(false);
    f.state.answers = [false];
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log.slice(-2)).toEqual(["snapshot p", ASK_QUIT]);
  });

  test("closeTabFlow of an owner whose linked tab stays unsent closes with discard on yes", async () => {
    const f = owned(false);
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.filter((l) => l.startsWith("close") || l.startsWith("confirm"))).toEqual([ASK_QUIT, "close a true"]);
  });

  test("closeTabFlow of an owner marks its linked tabs closing", async () => {
    const f = owned(true);
    let closing = false;
    const save = f.state.savePack;
    f.state.savePack = (id) => ((closing = f.input.closing.has("p")), save(id));
    await closeTabFlow("a", f.deps());
    expect(closing).toBe(true);
  });

  test("closeTabFlow of an owner that stops clears its linked tabs from closing", async () => {
    const f = owned(false);
    f.state.answers = [false];
    await closeTabFlow("a", f.deps());
    expect(f.input.closing.size).toBe(0);
  });

  test("closeTabFlow of an owner leaves a linked tab already closing alone", async () => {
    const f = owned(false);
    f.input.closing.add("p");
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.filter((l) => l.endsWith(" p"))).toEqual([]);
    expect(f.input.closing.has("p")).toBe(true);
  });

  test("closeTabFlow of an owner saves a linked tab in another window without its guard", async () => {
    const f = owned(true);
    f.state.tabs = [tabOf("a")];
    f.state.otherWindows = [{ label: "w-2", active: "p", tabs: [linkedOf("p", "a", true)] }];
    expect(await closeTabFlow("a", f.deps())).toBe(true);
    expect(f.log.slice(0, 5)).toEqual(["guard a", "fresh", "snapshot p", "savePack p", "snapshot p"]);
  });

  test("closeTabFlow stalled twice on an owner names its dirty linked tabs", async () => {
    const f = fake([tabOf("a"), linkedOf("p", "a", true), linkedOf("q", "a")]);
    f.state.guard = timeout;
    f.input.stalls.set("a", T0 - 10_000);
    f.state.answers = [false];
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual([
      "guard a", "fresh",
      "confirm This tab is still saving, and these pack tabs close with it, unsaved: ⁨p⁩. Close them anyway and lose what has not been saved?",
    ]);
  });

  test("closeTabFlow of a deck-pack tab whose snapshot fails tries savePack", async () => {
    const f = fake([tabOf("a"), linkedOf("p", "a", true)]);
    f.state.snapshot = () => Promise.reject(new Error("rendu"));
    expect(await closeTabFlow("p", f.deps())).toBe(true);
    expect(f.log).toEqual(["guard p", "fresh", "snapshot p", "report rendu", "fresh", "savePack p", "close p false"]);
  });
});

describe("closeFlowDeps", () => {
  test("closeFlowDeps builds share stalls", () => {
    const f = fake([]);
    expect(f.deps().stalls).toBe(f.deps().stalls);
  });

  test("closeFlowDeps builds share closing", () => {
    const f = fake([]);
    expect(f.deps().closing).toBe(f.deps().closing);
  });

  test("close reports a refused close as false", async () => {
    const f = fake([tabOf("a")]);
    f.state.refused.add("a true");
    expect(await f.deps().close("a", true)).toBe(false);
    expect(f.log).toEqual(["close a true", "run error refusé"]);
  });

  test("report ignores errors after unmount", () => {
    const f = fake([]);
    const deps = f.deps();
    f.input.alive.current = false;
    deps.report(new Error("tard"));
    expect(f.log).toEqual([]);
  });

  test("packOpener and activate come from the injected workspace", async () => {
    const f = fake([]);
    f.state.offer = OFFER;
    const deps = f.deps();
    expect(await deps.packOpener("a")).toEqual(OFFER);
    await deps.activate("b");
    expect(f.log).toEqual(["opener a", "activate b"]);
  });

  test("isolate wraps a value in one FSI and one PDI", () => {
    expect(isolate("Roadmap")).toBe("⁨Roadmap⁩");
  });

  test("closeFlowDeps takes the clock from its input", () => {
    const f = fake([]);
    f.state.clock = 42;
    expect(closeFlowDeps(f.input).now()).toBe(42);
  });
});
