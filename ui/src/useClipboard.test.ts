import { describe, expect, test } from "bun:test";

import type { Element, Op, SlideView } from "./api";
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
  const view = { slide: { id: "s-1", kind: "essentiel/content", params: { blocks: [] } } } as unknown as SlideView;
  type Sent = [string, string, string, Op[]];

  test("pasteCopy in the same tab sends paste_element with the paste ops", async () => {
    const sent: Sent[] = [];
    const id = await pasteCopy(clip(), "t-1", view, async (...a) => (sent.push(a), true));
    const op = sent[0]?.[3][0] as { op: string; element: Element };
    expect([sent.length, sent[0]?.[0], sent[0]?.[1], op.op, op.element.id]).toEqual([1, "t-1", "t-1", "add_element", id]);
  });

  test("pasteCopy from another tab sends paste_element from that tab", async () => {
    const sent: Sent[] = [];
    await pasteCopy(clip(), "t-2", view, async (...a) => (sent.push(a), true));
    expect(sent.map(([from, to]) => [from, to])).toEqual([["t-1", "t-2"]]);
  });

  test("pasteCopy refused by the host selects nothing", async () => {
    expect(await pasteCopy(clip(), "t-2", view, async () => false)).toBeNull();
  });
});
