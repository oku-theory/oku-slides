import { describe, expect, test } from "bun:test";
import { presentKey, presentShortcut } from "./present";

const key = (k: string, mods: Partial<Pick<KeyboardEvent, "metaKey" | "ctrlKey" | "altKey" | "shiftKey">> = {}) => ({
  key: k,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe("presentShortcut", () => {
  test("presentShortcut cmd enter returns screen", () => {
    expect(presentShortcut(key("Enter", { metaKey: true }))).toBe("screen");
  });

  test("presentShortcut ctrl enter returns screen", () => {
    expect(presentShortcut(key("Enter", { ctrlKey: true }))).toBe("screen");
  });

  test("presentShortcut cmd shift enter returns window", () => {
    expect(presentShortcut(key("Enter", { metaKey: true, shiftKey: true }))).toBe("window");
  });

  test("presentShortcut plain enter returns null", () => {
    expect(presentShortcut(key("Enter"))).toBeNull();
  });

  test("presentShortcut cmd alt enter returns null", () => {
    expect(presentShortcut(key("Enter", { metaKey: true, altKey: true }))).toBeNull();
  });
});

describe("presentKey", () => {
  test("presentKey escape returns exit", () => {
    expect(presentKey(key("Escape"), 1, 3)).toEqual({ kind: "exit" });
  });

  test("presentKey space returns next slide", () => {
    expect(presentKey(key(" "), 0, 3)).toEqual({ kind: "show", index: 1 });
  });

  test("presentKey arrow left returns previous slide", () => {
    expect(presentKey(key("ArrowLeft"), 2, 3)).toEqual({ kind: "show", index: 1 });
  });

  test("presentKey end returns last slide", () => {
    expect(presentKey(key("End"), 0, 3)).toEqual({ kind: "show", index: 2 });
  });

  test("presentKey next on last slide returns stay", () => {
    expect(presentKey(key("ArrowRight"), 2, 3)).toEqual({ kind: "stay" });
  });

  test("presentKey unrelated key returns null", () => {
    expect(presentKey(key("a"), 0, 3)).toBeNull();
  });

  test("presentKey cmd key returns null", () => {
    expect(presentKey(key("ArrowRight", { metaKey: true }), 0, 3)).toBeNull();
  });
});
