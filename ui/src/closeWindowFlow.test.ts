// Closing a window: every tab's guard, the saves of its dirty tabs, stuck tabs discarded on request.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { closeTabFlow, closeWindowFlow } from "./closeFlow";
import { setMessages } from "./i18n";
import { T0, deferred, fake, linkedOf, packEditing, snap, tabOf } from "./test/closeFlowFake";
import { resetMessages } from "./test/messages";

const ftl = (lang: string) => Bun.file(new URL(`../../crates/oku-slides-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();
beforeAll(async () => {
  setMessages("en", [await ftl("en")]);
});
afterAll(resetMessages);

const PENDING = "report This tab is still saving: close it again in a moment.";
const ASK_STUCK = "confirm This tab is still saving. Close it anyway and lose what has not been saved?";
const timeout = () => Promise.resolve("timeout" as const);
const onlyFor = (id: string) => (g: string) => Promise.resolve(g === id ? ("timeout" as const) : ("done" as const));
const readOnlyPack = async () => snap({ dirty: true, editing: packEditing({ read_only: true, leased: false }) });

describe("closeWindowFlow", () => {
  test("runs every tab's guard before reading dirty", async () => {
    const f = fake([tabOf("a", true), tabOf("b")]);
    expect(await closeWindowFlow("a", f.deps())).toBe(true);
    expect(f.log.slice(0, 4)).toEqual(["fresh", "guard a", "guard b", "fresh"]);
    expect(f.log.at(-1)).toBe("closeWindow ");
  });

  test("a timed-out guard refuses the window close", async () => {
    const f = fake([tabOf("a"), tabOf("b")]);
    f.state.guard = onlyFor("b");
    expect(await closeWindowFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["fresh", "guard a", "guard b", PENDING, "activate a"]);
    expect(f.input.stalls.get("b")).toBe(T0);
  });

  test("a second window-close timeout within 30 s asks for a pack tab", async () => {
    const f = fake([tabOf("a"), tabOf("p", true, true)]);
    f.state.guard = onlyFor("p");
    f.input.stalls.set("p", T0 - 5_000);
    expect(await closeWindowFlow("a", f.deps())).toBe(true);
    expect(f.log).toContain(ASK_STUCK);
    expect(f.log.at(-1)).toBe("closeWindow p");
  });

  test("a window close with a first and a second stall refuses after recording both", async () => {
    const f = fake([tabOf("a"), tabOf("b")]);
    f.state.guard = timeout;
    f.input.stalls.set("b", T0 - 5_000);
    expect(await closeWindowFlow("a", f.deps())).toBe(false);
    expect(f.log).toEqual(["fresh", "guard a", "guard b", PENDING, "activate a"]);
    expect([...f.input.stalls].sort()).toEqual([["a", T0], ["b", T0 - 5_000]]);
  });

  test("a snapshot rejection in a window close offers discard for a pack tab only", async () => {
    const f = fake([tabOf("p", true, true), tabOf("d", true)]);
    f.state.snapshot = () => Promise.reject(new Error("rendu"));
    f.state.savePack = () => Promise.reject(new Error("rendu"));
    expect(await closeWindowFlow("x", f.deps())).toBe(false);
    expect(f.log.filter((l) => l.startsWith("confirm"))).toEqual(["confirm Quit without saving the modified pack?"]);
    expect(f.log.at(-1)).toBe("activate x");
    expect(f.log.filter((l) => l.startsWith("closeWindow"))).toEqual([]);
  });

  test("collects discarded pack tabs and offers nothing", async () => {
    const f = fake([tabOf("p", true, true)]);
    f.state.snapshot = readOnlyPack;
    expect(await closeWindowFlow("p", f.deps())).toBe(true);
    expect(f.log.at(-1)).toBe("closeWindow p");
    expect(f.log.filter((l) => l.startsWith("opener"))).toEqual([]);
  });

  test("a collected stalled tab is not asked again", async () => {
    const f = fake([tabOf("p", true, true)]);
    f.state.guard = timeout;
    f.input.stalls.set("p", T0 - 5_000);
    expect(await closeWindowFlow("p", f.deps())).toBe(true);
    expect(f.log).toEqual(["fresh", "guard p", ASK_STUCK, "fresh", "closeWindow p"]);
  });

  test("runs the guards again before closeWindow", async () => {
    const f = fake([tabOf("a", true), tabOf("b")]);
    await closeWindowFlow("a", f.deps());
    const tail = f.log.slice(f.log.indexOf("save a") + 1);
    expect(tail).toEqual(["guard a", "guard b", "closeWindow "]);
  });

  test("skips tabs already closing", async () => {
    const f = fake([tabOf("a", true), tabOf("b", true)]);
    f.input.closing.add("b");
    expect(await closeWindowFlow("a", f.deps())).toBe(true);
    expect(f.log.filter((l) => l.endsWith(" b") && !l.startsWith("guard"))).toEqual([]);
    expect(f.input.closing.has("b")).toBe(true);
  });

  test("a tab × between the window flow and closeWindow asks nothing", async () => {
    const f = fake([tabOf("a", true)]);
    let second: boolean | null = null;
    f.state.duringCloseWindow = async () => {
      second = await closeTabFlow("a", f.deps());
    };
    expect(await closeWindowFlow("a", f.deps())).toBe(true);
    expect(second).toBe(false);
    expect(f.log.slice(f.log.indexOf("closeWindow "))).toEqual(["closeWindow "]);
    expect(f.input.closing.size).toBe(0);
  });

  test("a tab close during a window close asks once", async () => {
    const f = fake([tabOf("p", true, true)]);
    const shown = deferred<void>();
    let second: boolean | null = null;
    f.state.snapshot = async () => {
      second = await closeTabFlow("p", f.deps());
      shown.resolve();
      return readOnlyPack();
    };
    expect(await closeWindowFlow("p", f.deps())).toBe(true);
    await shown.promise;
    expect(second).toBe(false);
    expect(f.log.filter((l) => l.startsWith("confirm"))).toHaveLength(1);
  });

  test("a refused save re-activates the previous tab and returns false", async () => {
    const f = fake([tabOf("p", true, true)]);
    f.state.snapshot = async () => snap({ dirty: true, editing: packEditing() });
    f.state.savePack = async () => snap({ dirty: true, editing: packEditing() });
    f.state.answers = [false];
    expect(await closeWindowFlow("x", f.deps())).toBe(false);
    expect(f.log.slice(-4)).toEqual(["savePack p", "snapshot p", "confirm Quit without saving the modified pack?", "activate x"]);
  });

  test("a last-round stall re-activates the previous tab and returns false", async () => {
    const f = fake([tabOf("a", true)]);
    f.state.guard = (_id, n) => Promise.resolve(n === 2 ? ("timeout" as const) : ("done" as const));
    expect(await closeWindowFlow("x", f.deps())).toBe(false);
    expect(f.log.slice(-3)).toEqual(["guard a", PENDING, "activate x"]);
  });

  test("a failing save re-activates the previous tab and returns false", async () => {
    const f = fake([tabOf("a", true)]);
    f.state.save = () => Promise.reject(new Error("disque"));
    expect(await closeWindowFlow("x", f.deps())).toBe(false);
    expect(f.log.slice(-3)).toEqual(["save a", "report disque", "activate x"]);
  });

  test("the final window guard join skips collected tabs", async () => {
    const f = fake([tabOf("a"), tabOf("b")]);
    f.state.guard = (id) => Promise.resolve(id === "a" ? ("timeout" as const) : ("done" as const));
    expect(await closeTabFlow("a", f.deps())).toBe(false);
    expect(await closeWindowFlow("b", f.deps())).toBe(true);
    expect([f.guardCalls.get("a"), f.guardCalls.get("b")]).toEqual([2, 2]);
    expect(f.log.at(-1)).toBe("closeWindow a");
  });

  test("closeWindowFlow skips a linked tab the user discarded", async () => {
    const f = fake([tabOf("a"), linkedOf("p", "a", true)]);
    f.state.guard = onlyFor("p");
    f.input.stalls.set("p", T0 - 5_000);
    expect(await closeWindowFlow("a", f.deps())).toBe(true);
    expect(f.log.filter((l) => l === "snapshot p" || l === "savePack p")).toEqual([]);
    expect(f.log.at(-1)).toBe("closeWindow p");
  });

  test("closeWindowFlow saves an owner's dirty linked tab before the owner", async () => {
    const f = fake([tabOf("a"), linkedOf("p", "a", true)]);
    let saved = false;
    const deckSnapshot = f.state.snapshot;
    const packSave = f.state.savePack;
    f.state.snapshot = (id) => (id === "p" ? Promise.resolve(snap({ dirty: !saved, editing: packEditing({ deck_only: true }) })) : deckSnapshot(id));
    f.state.savePack = (id) => ((saved = true), packSave(id));
    expect(await closeWindowFlow("a", f.deps())).toBe(true);
    expect(f.log.slice(4, 13)).toEqual(["guard p", "snapshot p", "savePack p", "snapshot p", "fresh", "activate a", "snapshot a", "save a", "guard a"]);
    expect(f.log.at(-1)).toBe("closeWindow ");
  });
});
