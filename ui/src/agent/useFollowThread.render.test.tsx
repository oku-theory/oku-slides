import { afterEach, expect, test } from "bun:test";
import "../test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useFollowThread } = await import("./useFollowThread");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mountFollow(follow: () => Promise<void>) {
  let state: ReturnType<typeof useFollowThread> | null = null;
  const Probe = () => {
    state = useFollowThread({ follow });
    return null;
  };
  const root = createRoot(document.createElement("div"));
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => act(async () => root.unmount());
  return { state: () => state! };
}

test("useFollowThread is ready after follow succeeds", async () => {
  const h = await mountFollow(async () => {});
  expect(h.state().ready).toBe(false);
  await act(async () => h.state().onReady());
  expect(h.state().ready).toBe(true);
});

test("useFollowThread asks follow again on reconnection", async () => {
  let n = 0;
  const h = await mountFollow(async () => void n++);
  await act(async () => h.state().onReady());
  await act(async () => h.state().onReady());
  expect(n).toBe(2);
});

test("useFollowThread follow failure is not ready", async () => {
  const h = await mountFollow(async () => {
    throw new Error("no tools");
  });
  await act(async () => h.state().onReady());
  expect(h.state().ready).toBe(false);
});

test("useFollowThread a later follow failure makes it not ready", async () => {
  const replies: { resolve: () => void; reject: (e: Error) => void }[] = [];
  const h = await mountFollow(() => new Promise<void>((resolve, reject) => void replies.push({ resolve, reject })));
  await act(async () => h.state().onReady());
  await act(async () => replies[0].resolve());
  expect(h.state().ready).toBe(true);
  await act(async () => h.state().onReady());
  await act(async () => replies[1].reject(new Error("coupé")));
  expect(h.state().ready).toBe(false);
});

test("useFollowThread a late failure of an older follow does not undo the latest", async () => {
  const replies: { resolve: () => void; reject: (e: Error) => void }[] = [];
  const h = await mountFollow(() => new Promise<void>((resolve, reject) => void replies.push({ resolve, reject })));
  await act(async () => h.state().onReady());
  await act(async () => h.state().onReady());
  await act(async () => replies[1].resolve());
  await act(async () => replies[0].reject(new Error("tard")));
  expect(h.state().ready).toBe(true);
});

test("useFollowThread status follows status events", async () => {
  const h = await mountFollow(async () => {});
  await act(async () => h.state().onAgentEvent({ kind: "status", state: "starting" }));
  expect(h.state().state).toBe("starting");
});
