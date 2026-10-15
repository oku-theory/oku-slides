import { expect, test } from "bun:test";
import { fitIn } from "./bounds";

test("fitIn within bounds rounds the size", () => {
  expect(fitIn({ min: 100, max: 300 }, 200.6)).toBe(201);
});

test("fitIn outside bounds returns the nearest bound", () => {
  expect([fitIn({ min: 100, max: 300 }, 40), fitIn({ min: 100, max: 300 }, 900)]).toEqual([100, 300]);
});
