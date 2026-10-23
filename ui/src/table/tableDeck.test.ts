import { describe, expect, test } from "bun:test";
import { asColumnAlign } from "./tableDeck";

describe("tableDeck", () => {
  test("asColumnAlign of an unknown value is auto", () => {
    expect(asColumnAlign("diagonal")).toBe("auto");
  });
});
