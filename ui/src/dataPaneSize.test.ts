// The data pane's height at its divider: bounds against the stage, the sheet grid's rows.

import { describe, expect, test } from "bun:test";
import { DATA_MIN, dataPaneBounds, sheetRows } from "./dataPaneSize";

describe("dataPaneBounds", () => {
  test("dataPaneBounds lets the pane take the stage's room down to its minimum", () => {
    expect(dataPaneBounds({ stage: 400, pane: 200, stageMin: 160 })).toEqual({ min: DATA_MIN, max: 440 });
  });

  test("dataPaneBounds in a short sheet never goes under the pane's minimum", () => {
    expect(dataPaneBounds({ stage: 160, pane: 0, stageMin: 160 })).toEqual({ min: DATA_MIN, max: DATA_MIN });
  });
});

describe("sheetRows", () => {
  test("sheetRows reads the stage and pane rows of the sheet grid", () => {
    expect(sheetRows("412px 0px 236.5px", "160px")).toEqual({ stage: 412, pane: 236.5, stageMin: 160 });
  });

  test("sheetRows without rows or stage minimum reads zeros", () => {
    expect(sheetRows("none", "")).toEqual({ stage: 0, pane: 0, stageMin: 0 });
  });
});
