import { describe, expect, test } from "bun:test";

import { markers, pileMarks, Seeker, step } from "./history";

const v = (peer: string, merge = false, start = false) =>
  ({ author: peer, timestamp: 0, label: peer, session_start: start, merge }) as never;

const versions = [v("A", false, true), v("A"), v("B", false, true), v("B"), v("A", true, false), v("C", false, true)];

describe("markers", () => {
  test("markers flags each session start and each merge", () => {
    expect(markers(versions)).toEqual([
      { index: 0, kind: "session" },
      { index: 2, kind: "session" },
      { index: 4, kind: "merge" },
      { index: 5, kind: "session" },
    ]);
  });
});

describe("step", () => {
  test("step with an arrow moves one version and stays in bounds", () => {
    expect(step(0, versions, "ArrowLeft", false)).toBe(0);
    expect(step(1, versions, "ArrowRight", false)).toBe(2);
    expect(step(5, versions, "ArrowRight", false)).toBe(5);
  });

  test("step with shift jumps to the previous or next session start", () => {
    expect(step(3, versions, "ArrowLeft", true)).toBe(2);
    expect(step(2, versions, "ArrowLeft", true)).toBe(0);
    expect(step(1, versions, "ArrowRight", true)).toBe(2);
    expect(step(5, versions, "ArrowRight", true)).toBe(5);
  });
});

describe("pileMarks", () => {
  test("pileMarks marks removed and changed slides, not added ones", () => {
    expect(pileMarks({ added_since: ["n"], removed_since: ["r"], changed_since: ["c"] })).toEqual({
      r: "removed",
      c: "changed",
    });
  });
});

describe("Seeker", () => {
  test("Seeker keeps only the answer to the last requested version", async () => {
    const shown: number[] = [];
    const resolvers: ((n: number) => void)[] = [];
    const seeker = new Seeker(
      (i: number) => new Promise<number>((r) => resolvers.push(() => r(i))),
      (s: number) => shown.push(s),
    );
    const first = seeker.go(1);
    const second = seeker.go(2);
    resolvers[1](2);
    await second;
    resolvers[0](1);
    await first;
    expect(shown).toEqual([2]);
  });

  test("Seeker cancel drops an answer still in flight, so a later close wins", async () => {
    const shown: number[] = [];
    let answer: (n: number) => void = () => {};
    const seeker = new Seeker(
      (i: number) => new Promise<number>((r) => (answer = () => r(i))),
      (s: number) => shown.push(s),
    );
    const seek = seeker.go(3);
    // Close or Restore goes first: the step's reply, arriving later, does not show.
    seeker.cancel();
    answer(3);
    await seek;
    expect(shown).toEqual([]);
  });
});
