// A tab's snapshot pipeline: `run` applies the newest reply, a `nonce` change refetches,
// a notice is said once, and a stale reply never brings the deck back.

import { afterEach, describe, expect, mock, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useTabSnapshot } = await import("./useTabSnapshot");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** A snapshot told apart by its title. */
const snapOf = (title: string, extra: object = {}) => ({ title, ...extra }) as unknown as Snapshot;

type Pipeline = ReturnType<typeof useTabSnapshot>;

/** Mounts the hook over a fake API whose `snapshot` replies from `replies`; returns the latest result and spies. */
async function mount(replies: Snapshot[], notice: string | null = null) {
  const queue = [...replies];
  const snapshot = mock(async () => queue.shift() ?? snapOf("empty"));
  const api = { snapshot } as unknown as TabApi;
  const say = mock((_m: string, _sticky?: boolean) => {});
  const onChange = mock(() => {});
  const out: { current: Pipeline | null } = { current: null };
  function Probe({ nonce }: { nonce: number }) {
    out.current = useTabSnapshot(api, { nonce, notice, onChange, say });
    return null;
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const render = (nonce: number) => act(async () => root.render(createElement(Probe, { nonce })));
  await render(0);
  return { out, snapshot, say, onChange, render };
}

const titleOf = (s: Snapshot | null) => (s as unknown as { title: string } | null)?.title;

describe("useTabSnapshot", () => {
  test("mount_fetches_snapshot_and_exposes_it", async () => {
    const { out, snapshot } = await mount([snapOf("first")]);
    expect(snapshot).toHaveBeenCalledTimes(1);
    expect(titleOf(out.current?.snap ?? null)).toBe("first");
    expect(titleOf(out.current?.snapRef.current ?? null)).toBe("first");
  });

  test("run_with_a_newer_snapshot_updates_snap_and_notifies", async () => {
    const { out, onChange } = await mount([snapOf("first")]);
    await act(async () => void (await out.current?.run(Promise.resolve(snapOf("second")))));
    expect(titleOf(out.current?.snap ?? null)).toBe("second");
    expect(onChange).toHaveBeenCalledTimes(2);
  });

  test("run_with_null_keeps_the_state", async () => {
    const { out } = await mount([snapOf("first")]);
    let r: unknown = "unset";
    await act(async () => void (r = await out.current?.run(Promise.resolve(null))));
    expect(r).toBeNull();
    expect(titleOf(out.current?.snap ?? null)).toBe("first");
  });

  test("run_with_a_rejection_says_the_error_and_returns_null", async () => {
    const { out, say } = await mount([snapOf("first")]);
    await act(async () => void (await out.current?.run(Promise.reject(new Error("boom")))));
    expect(say.mock.calls.at(-1)?.[1]).toBe(true);
    expect(titleOf(out.current?.snap ?? null)).toBe("first");
  });

  test("run_with_a_stale_reply_arriving_last_keeps_the_newer_snapshot_but_says_its_notice", async () => {
    const { out, say } = await mount([snapOf("first")]);
    let release: (s: Snapshot) => void = () => {};
    const slow = new Promise<Snapshot>((r) => (release = r));
    await act(async () => {
      const old = out.current?.run(slow);
      await out.current?.run(Promise.resolve(snapOf("newer")));
      release(snapOf("older", { notice: "once" }));
      await old;
    });
    expect(titleOf(out.current?.snap ?? null)).toBe("newer");
    expect(say).toHaveBeenCalledWith("once", true);
  });

  test("nonce_change_refetches_the_snapshot", async () => {
    const { out, snapshot, render } = await mount([snapOf("first"), snapOf("second")]);
    await render(1);
    expect(snapshot).toHaveBeenCalledTimes(2);
    expect(titleOf(out.current?.snap ?? null)).toBe("second");
  });

  test("notice_prop_is_said_sticky", async () => {
    const { say } = await mount([snapOf("first")], "from elsewhere");
    expect(say).toHaveBeenCalledWith("from elsewhere", true);
  });
});
