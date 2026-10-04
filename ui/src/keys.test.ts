import { describe, expect, test } from "bun:test";
import { keyLabel } from "./keys";

describe("keyLabel", () => {
  test("keyLabel on macOS keeps the symbols", () => {
    expect(keyLabel("⌘⇧S", true)).toBe("⌘⇧S");
  });

  test("keyLabel elsewhere writes Ctrl, Alt and Shift in the PC order", () => {
    expect(keyLabel("⌘⇧S", false)).toBe("Ctrl+Shift+S");
    expect(keyLabel("⌥↑", false)).toBe("Alt+↑");
    expect(keyLabel("⌘'", false)).toBe("Ctrl+'");
    expect(keyLabel("⌘?", false)).toBe("Ctrl+?");
  });

  test("keyLabel elsewhere joins a modifier to a word after a space", () => {
    expect(keyLabel("⌘ Entrée", false)).toBe("Ctrl+Entrée");
  });

  test("keyLabel elsewhere leaves a lone modifier as its name", () => {
    expect(keyLabel("⇧", false)).toBe("Shift");
  });
});
