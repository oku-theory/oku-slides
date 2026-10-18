import { expect, test } from "bun:test";
import { packNoticeCommand } from "./packNoticeCommand";

test("packNoticeCommand maps each banner event to its request", () => {
  expect(["pack_use_mine", "pack_make_current", "pack_notice_close"].map(packNoticeCommand)).toEqual([
    "pack_use_mine",
    "pack_make_current",
    "pack_notice_close",
  ]);
});

test("packNoticeCommand with an unknown event returns null", () => {
  expect(packNoticeCommand("delete_deck")).toBeNull();
});
