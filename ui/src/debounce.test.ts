import { describe, expect, test } from "bun:test";
import { PREVIEW_DELAY_MS, type Timers, debounced } from "./debounce";

/** Minuteries à la main : on déclenche ce qui attend. */
function manual() {
  let pending: { fn: () => void; ms: number } | null = null;
  const timers: Timers = {
    set: (fn, ms) => {
      pending = { fn, ms };
      return pending;
    },
    clear: (h) => {
      if (h === pending) pending = null;
    },
  };
  return { timers, fire: () => pending?.fn(), delay: () => pending?.ms ?? null };
}

describe("debounced", () => {
  test("seule la dernière frappe part, après le délai de l'aperçu", () => {
    const m = manual();
    const seen: string[] = [];
    const push = debounced((s: string) => seen.push(s), PREVIEW_DELAY_MS, m.timers);
    push("a");
    push("ab");
    push("abc");
    expect(seen).toEqual([]);
    expect(m.delay()).toBe(400);
    m.fire();
    expect(seen).toEqual(["abc"]);
  });

  test("cancel oublie ce qui attend", () => {
    const m = manual();
    const seen: string[] = [];
    const push = debounced((s: string) => seen.push(s), 400, m.timers);
    push("a");
    push.cancel();
    m.fire();
    expect(seen).toEqual([]);
  });

  test("flush envoie tout de suite ce qui attend, une seule fois", () => {
    const m = manual();
    const seen: string[] = [];
    const push = debounced((s: string) => seen.push(s), 400, m.timers);
    push.flush();
    expect(seen).toEqual([]);
    push("a");
    push.flush();
    expect(seen).toEqual(["a"]);
    m.fire();
    push.flush();
    expect(seen).toEqual(["a"]);
  });
});
