import { describe, expect, test } from "bun:test";

import type { Element } from "./api";
import { type Clip, movesHere, pasteCopy } from "./useClipboard";

const element = { id: "e-image", kind: "base/image", params: { asset: "sha256:ab" } } as unknown as Element;
const clip = (over: Partial<Clip> = {}): Clip => ({ element, text: "base/image", tab: "t-1", ...over });

describe("movesHere", () => {
  test("movesHere with a cut from this tab returns true", () => {
    expect(movesHere(clip({ cut: { slide: "s" } }), "t-1")).toBe(true);
  });

  test("movesHere with a cut from another tab returns false", () => {
    expect(movesHere(clip({ cut: { slide: "s" } }), "t-2")).toBe(false);
  });

  test("movesHere without a cut returns false", () => {
    expect(movesHere(clip(), "t-1")).toBe(false);
  });
});

describe("pasteCopy", () => {
  test("pasteCopy in the same tab pastes without carrying", async () => {
    const calls: unknown[] = [];
    await pasteCopy(clip(), "t-1", async (...a) => void calls.push(["carry", ...a]), async () => {
      calls.push("paste");
      return true;
    });
    expect(calls).toEqual(["paste"]);
  });

  test("pasteCopy from another tab carries the assets before pasting", async () => {
    const calls: unknown[] = [];
    await pasteCopy(clip(), "t-2", async (...a) => void calls.push(["carry", ...a]), async () => {
      calls.push("paste");
      return true;
    });
    expect(calls).toEqual([["carry", "t-1", "t-2", element], "paste"]);
  });

  test("pasteCopy whose carry fails pastes nothing", async () => {
    const calls: unknown[] = [];
    const failed = pasteCopy(clip(), "t-2", async () => Promise.reject(new Error("non")), async () => {
      calls.push("paste");
      return true;
    });
    await expect(failed).rejects.toThrow("non");
    expect(calls).toEqual([]);
  });
});
