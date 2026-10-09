import { describe, expect, test } from "bun:test";
import { PREVIEW_DELAY_MS, debounced } from "./debounce";
import { manualTimers } from "./test/timers";

describe("debounced", () => {
  test("seule la dernière frappe part, après le délai de l'aperçu", () => {
    const timers = manualTimers();
    const seen: string[] = [];
    const push = debounced((s: string) => seen.push(s), PREVIEW_DELAY_MS, timers);
    push("a");
    push("ab");
    push("abc");
    expect(seen).toEqual([]);
    expect((timers.delays.at(-1) ?? null)).toBe(400);
    timers.run();
    expect(seen).toEqual(["abc"]);
  });

  test("cancel oublie ce qui attend", () => {
    const timers = manualTimers();
    const seen: string[] = [];
    const push = debounced((s: string) => seen.push(s), 400, timers);
    push("a");
    push.cancel();
    timers.run();
    expect(seen).toEqual([]);
  });

  test("flush envoie tout de suite ce qui attend, une seule fois", () => {
    const timers = manualTimers();
    const seen: string[] = [];
    const push = debounced((s: string) => seen.push(s), 400, timers);
    push.flush();
    expect(seen).toEqual([]);
    push("a");
    push.flush();
    expect(seen).toEqual(["a"]);
    timers.run();
    push.flush();
    expect(seen).toEqual(["a"]);
  });
});
