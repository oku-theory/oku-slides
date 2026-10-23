import { describe, expect, test } from "bun:test";
import { asChartKind } from "./chartDeck";

describe("asChartKind", () => {
  test("asChartKind of an unknown type is null", () => {
    expect(asChartKind("donut")).toBeNull();
  });
});
