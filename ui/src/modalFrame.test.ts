import { describe, expect, test } from "bun:test";
import arming from "../../fixtures/ui/arming.json";
import { ARMING_MS } from "./modalFrame";

describe("ARMING_MS", () => {
  // The host refuses a spoken answer begun before the modal is armed: one delay, shared with Rust.
  test("equals the host's ARMING through the shared fixture", () => {
    expect(ARMING_MS).toBe(arming.arming_ms);
  });
});
