import { expect, test } from "bun:test";
import { fitIn } from "./bounds";
import { CODE_MIN, codeBounds, codeTrack, SIDE_MIN, SPLIT } from "./codeSplit";

test("codeBounds within bounds rounds the size", () => {
  expect(fitIn(codeBounds(1000), 400.4)).toBe(400);
});

test("codeBounds below the code minimum returns the minimum", () => {
  expect(fitIn(codeBounds(1000), 40)).toBe(CODE_MIN);
});

test("codeBounds leaving the preview less than its minimum gives it its minimum", () => {
  expect(fitIn(codeBounds(1000), 990)).toBe(1000 - SIDE_MIN - SPLIT);
});

test("codeBounds with no room for both minimums splits in half", () => {
  expect(fitIn(codeBounds(212), 180)).toBe(100);
});

test("codeTrack with a size keeps the preview minimum in CSS", () => {
  expect(codeTrack(480)).toBe(`min(480px, calc(100% - ${SIDE_MIN + SPLIT}px))`);
});

test("codeTrack without a size returns null", () => {
  expect(codeTrack(null)).toBeNull();
});

test("codeBounds in a narrow tab keeps 240 px for the preview", () => {
  expect(fitIn(codeBounds(600), 500)).toBe(600 - 240 - SPLIT);
});

test("codeBounds with no room at all never returns a negative width", () => {
  expect([fitIn(codeBounds(0), 0), fitIn(codeBounds(5), 300)]).toEqual([0, 0]);
});
