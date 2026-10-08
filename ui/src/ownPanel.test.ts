import { describe, expect, test } from "bun:test";

import { hasOwnPanel } from "./ownPanel";

describe("hasOwnPanel", () => {
  test("hasOwnPanel chart or table returns true", () => {
    expect([hasOwnPanel("chart"), hasOwnPanel("table")]).toEqual([true, true]);
  });

  test("hasOwnPanel other or missing kind returns false", () => {
    expect([hasOwnPanel("text"), hasOwnPanel(undefined)]).toEqual([false, false]);
  });
});
