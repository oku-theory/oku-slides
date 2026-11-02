// The target of a receipt's trace action.

import { describe, expect, test } from "bun:test";
import { traceTarget } from "./trace";

const trace = (context: Record<string, unknown>) => ({ name: "trace", context });

describe("traceTarget", () => {
  test("a valid action gives its target", () => {
    expect(traceTarget(trace({ slide: "s1", element: "e1", pane: true }))).toEqual({ slide: "s1", element: "e1", pane: true });
  });
  test("another action gives null", () => {
    expect(traceTarget({ name: "suggest", context: { slide: "s1", element: "e1", pane: false } })).toBeNull();
  });
  test("a missing element gives null", () => {
    expect(traceTarget(trace({ slide: "s1", pane: false }))).toBeNull();
  });
  test("a non-string slide or element gives null", () => {
    expect([traceTarget(trace({ slide: 1, element: "e1", pane: false })), traceTarget(trace({ slide: "s1", element: {}, pane: false }))]).toEqual([null, null]);
  });
  test("a non-boolean pane gives null", () => {
    expect(traceTarget(trace({ slide: "s1", element: "e1", pane: "yes" }))).toBeNull();
  });
  test("empty strings give null", () => {
    expect([traceTarget(trace({ slide: "", element: "e1", pane: false })), traceTarget(trace({ slide: "s1", element: "", pane: false }))]).toEqual([null, null]);
  });
});
