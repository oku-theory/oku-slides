import { expect, test } from "bun:test";
import { CODE_MIN, codeTrack, fitCode, PREVIEW_MIN, SPLIT } from "./codeSplit";

test("fitCode within bounds rounds the size", () => {
  expect(fitCode(1000, 400.4)).toBe(400);
});

test("fitCode below the code minimum returns the minimum", () => {
  expect(fitCode(1000, 40)).toBe(CODE_MIN);
});

test("fitCode leaving the preview less than its minimum gives it its minimum", () => {
  expect(fitCode(1000, 990)).toBe(1000 - PREVIEW_MIN - SPLIT);
});

test("fitCode with no room for both minimums splits in half", () => {
  expect(fitCode(212, 180)).toBe(100);
});

test("codeTrack with a size keeps the preview minimum in CSS", () => {
  expect(codeTrack(480)).toBe(`min(480px, calc(100% - ${PREVIEW_MIN + SPLIT}px))`);
});

test("codeTrack without a size returns null", () => {
  expect(codeTrack(null)).toBeNull();
});
