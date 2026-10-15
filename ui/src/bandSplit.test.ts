import { expect, test } from "bun:test";
import { fitIn } from "./bounds";
import { BAND_MIN, bandBounds, bandHeight, STAGE_MIN } from "./bandSplit";

test("bandBounds within bounds rounds the size", () => {
  expect(fitIn(bandBounds(800), 300.4)).toBe(300);
});

test("bandBounds below the band minimum returns the minimum", () => {
  expect(fitIn(bandBounds(800), 40)).toBe(BAND_MIN);
});

test("bandBounds leaving the stage less than its minimum gives it its minimum", () => {
  expect(fitIn(bandBounds(800), 790)).toBe(800 - STAGE_MIN);
});

test("bandBounds with no room for both minimums keeps the band minimum, as the CSS does", () => {
  expect(fitIn(bandBounds(300), 180)).toBe(BAND_MIN);
});

test("bandBounds with exactly both minimums keeps both", () => {
  expect(fitIn(bandBounds(BAND_MIN + STAGE_MIN), 400)).toBe(BAND_MIN);
});

test("bandBounds without anything measured returns zero", () => {
  expect(fitIn(bandBounds(0), 300)).toBe(0);
});

test("bandHeight with a size keeps the stage minimum in CSS", () => {
  expect(bandHeight(320)).toBe(`max(${BAND_MIN}px, min(320px, calc(100% - ${STAGE_MIN}px)))`);
});

test("bandHeight of a kept height below the minimum still floors it in CSS", () => {
  expect(bandHeight(0)).toBe(`max(${BAND_MIN}px, min(0px, calc(100% - ${STAGE_MIN}px)))`);
});

test("bandHeight without a size returns null", () => {
  expect(bandHeight(null)).toBeNull();
});
