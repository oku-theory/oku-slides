// The data pane's height under the stage: its bounds at the divider, read from the sheet grid's rows.

import type { Bounds } from "./bounds";

/** Smallest data pane height set at its divider, in px. */
export const DATA_MIN = 120;

/** The sheet grid's live stage and data pane rows, and the stage's kept minimum, in px. */
export interface SheetRows {
  stage: number;
  pane: number;
  /** `--stage-min` of `.sheet-area` (inspector.css), the stage's room the pane never takes. */
  stageMin: number;
}

/** Bounds of the pane's height: at least `DATA_MIN`, at most what the stage can give up. */
export function dataPaneBounds({ stage, pane, stageMin }: SheetRows): Bounds {
  return { min: DATA_MIN, max: Math.max(DATA_MIN, stage + pane - stageMin) };
}

/**
 * The rows of a computed `grid-template-rows` (stage, variants drawer, data pane) and `--stage-min`;
 * 0 for what is missing. Tied to the sheet's three-row template in inspector.css: the pane is row 3.
 */
export function sheetRows(template: string, stageMin: string): SheetRows {
  const rows = template.split(" ").map(parseFloat);
  return { stage: rows[0] || 0, pane: rows[2] || 0, stageMin: parseFloat(stageMin) || 0 };
}
