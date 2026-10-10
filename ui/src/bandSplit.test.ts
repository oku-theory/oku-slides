import { expect, test } from "bun:test";
import { BAND_MIN, bandHeight, fitBand, STAGE_MIN } from "./bandSplit";

test("fitBand within bounds rounds the size", () => {
  expect(fitBand(800, 300.4)).toBe(300);
});

test("fitBand below the band minimum returns the minimum", () => {
  expect(fitBand(800, 40)).toBe(BAND_MIN);
});

test("fitBand leaving the stage less than its minimum gives it its minimum", () => {
  expect(fitBand(800, 790)).toBe(800 - STAGE_MIN);
});

test("fitBand with no room for both minimums keeps the band minimum, as the CSS does", () => {
  expect(fitBand(300, 180)).toBe(BAND_MIN);
});

test("fitBand with exactly both minimums keeps both", () => {
  expect(fitBand(BAND_MIN + STAGE_MIN, 400)).toBe(BAND_MIN);
});

test("fitBand without anything measured returns zero", () => {
  expect(fitBand(0, 300)).toBe(0);
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
