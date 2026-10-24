import { describe, expect, test } from "bun:test";

import type { Element, SharedClip } from "./api";
import type { ClipRef } from "./useClipboard";
import { clipSync, fromShared, remoteClip, toShared } from "./useSharedClip";

const element = { id: "e-image", kind: "base/image", params: { asset: "sha256:ab" } } as unknown as Element;
const shared: SharedClip = { tab: "t-1", text: "base/image", element, origin: "w-1" };
/** Une copie de l'onglet `tab`, envoyée par la fenêtre `origin`. */
const from = (tab: string, origin: string): SharedClip => ({ ...shared, tab, origin });

describe("remoteClip", () => {
  test("remoteClip from another window returns the element without its origin", () => {
    expect(remoteClip(shared, "w-2")).toEqual({ tab: "t-1", text: "base/image", element });
  });

  test("remoteClip echoing this window returns null", () => {
    expect(remoteClip(shared, "w-1")).toBeNull();
  });

  test("remoteClip of no copy returns null", () => {
    expect(remoteClip(null, "w-2")).toBeNull();
  });
});

describe("toShared", () => {
  test("toShared of a cut element sends it without the cut, tagged with the window", () => {
    const cut = { tab: "t-1", text: "base/image", element, cut: { slide: "s-1" } };
    expect(toShared(cut, "w-1")).toEqual(shared);
  });
});

describe("fromShared", () => {
  test("fromShared of a host copy drops its origin", () => {
    expect(fromShared(shared)).toEqual({ tab: "t-1", text: "base/image", element });
  });
});

describe("clipSync", () => {
  test("clipSync read on open adopts the host copy, whatever its window", () => {
    const clip: ClipRef = { current: null };
    clipSync(clip, "w-1").read({ clip: shared, seq: 1 });
    expect(clip.current?.tab).toBe("t-1");
  });

  test("clipSync read on open keeps a copy made here meanwhile", () => {
    const clip: ClipRef = { current: { tab: "t-5", text: "x", element } };
    clipSync(clip, "w-2").read({ clip: shared, seq: 1 });
    expect(clip.current?.tab).toBe("t-5");
  });

  test("clipSync read on open older than an event already seen changes nothing", () => {
    const clip: ClipRef = { current: null };
    const sync = clipSync(clip, "w-2");
    sync.event({ clip: from("t-2", "w-3"), seq: 2 });
    clip.current = null;
    sync.read({ clip: shared, seq: 1 });
    expect(clip.current).toBeNull();
  });

  test("clipSync event from another window replaces the copy and says so", () => {
    const clip: ClipRef = { current: null };
    const changed = clipSync(clip, "w-2").event({ clip: shared, seq: 1 });
    expect([changed, clip.current?.tab]).toEqual([true, "t-1"]);
  });

  test("clipSync event echoing this window keeps the copy", () => {
    const local = { tab: "t-1", text: "base/image", element, cut: { slide: "s-1" } };
    const clip: ClipRef = { current: local };
    const changed = clipSync(clip, "w-1").event({ clip: shared, seq: 1 });
    expect([changed, clip.current]).toEqual([false, local]);
  });

  test("clipSync event older than the last one seen is ignored", () => {
    const clip: ClipRef = { current: null };
    const sync = clipSync(clip, "w-2");
    sync.event({ clip: from("t-2", "w-3"), seq: 2 });
    const changed = sync.event({ clip: from("t-1", "w-3"), seq: 1 });
    expect([changed, clip.current?.tab]).toEqual([false, "t-2"]);
  });

  test("clipSync reply to a copy made here restores it over an older event", () => {
    const local = { tab: "t-5", text: "x", element };
    const clip: ClipRef = { current: local };
    const sync = clipSync(clip, "w-2");
    sync.sent(local);
    // Copiée ailleurs avant elle, la copie d'à côté arrive pendant que la nôtre est en route.
    sync.event({ clip: from("t-1", "w-3"), seq: 1 });
    const restored = sync.shared({ clip: toShared(local, "w-2"), seq: 2 }, local);
    expect([restored, clip.current]).toEqual([true, local]);
  });

  test("clipSync reply restoring a cut copy brings it back without its cut", () => {
    const local = { tab: "t-5", text: "x", element, cut: { slide: "s-1" } };
    const clip: ClipRef = { current: local };
    const sync = clipSync(clip, "w-2");
    sync.sent(local);
    sync.event({ clip: from("t-1", "w-3"), seq: 1 });
    sync.shared({ clip: toShared(local, "w-2"), seq: 2 }, local);
    expect(clip.current?.cut).toBeUndefined();
  });

  test("clipSync reply to a copy still in place changes nothing", () => {
    const local = { tab: "t-5", text: "x", element, cut: { slide: "s-1" } };
    const clip: ClipRef = { current: local };
    const sync = clipSync(clip, "w-2");
    sync.sent(local);
    const restored = sync.shared({ clip: toShared(local, "w-2"), seq: 1 }, local);
    expect([restored, clip.current]).toEqual([false, local]);
  });

  test("clipSync reply to an older copy made here keeps the newer one", () => {
    const older = { tab: "t-5", text: "base/image", element };
    const newer = { tab: "t-6", text: "base/image", element };
    const clip: ClipRef = { current: newer };
    const sync = clipSync(clip, "w-2");
    sync.sent(older);
    sync.sent(newer);
    sync.shared({ clip: toShared(older, "w-2"), seq: 1 }, older);
    expect(clip.current).toBe(newer);
  });

  test("clipSync echo of a copy made here, before its reply, restores it over an older event", () => {
    // L'hôte diffuse l'écho avant de répondre à `set_clip` : c'est l'ordre réel.
    const local = { tab: "t-5", text: "x", element };
    const clip: ClipRef = { current: local };
    const sync = clipSync(clip, "w-2");
    sync.sent(local);
    sync.event({ clip: from("t-1", "w-3"), seq: 1 });
    const restored = sync.event({ clip: toShared(local, "w-2"), seq: 2 });
    sync.shared({ clip: toShared(local, "w-2"), seq: 2 }, local);
    expect([restored, clip.current]).toEqual([true, local]);
  });

  test("clipSync read of an empty host keeps no copy", () => {
    const clip: ClipRef = { current: null };
    clipSync(clip, "w-1").read({ clip: null, seq: 0 });
    expect(clip.current).toBeNull();
  });

  test("clipSync reply to a copy made here yields to a newer event", () => {
    const local = { tab: "t-5", text: "x", element };
    const clip: ClipRef = { current: local };
    const sync = clipSync(clip, "w-2");
    sync.sent(local);
    sync.event({ clip: from("t-1", "w-3"), seq: 3 });
    sync.shared({ clip: toShared(local, "w-2"), seq: 2 }, local);
    expect(clip.current?.tab).toBe("t-1");
  });
});
