import { afterEach, expect, test } from "bun:test";
import "../test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useFollowScope } = await import("./useFollowScope");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mountFollow(fail = false) {
  const follows: number[] = [];
  const agent = {
    follow: async () => {
      follows.push(1);
      if (fail) throw new Error("no tools");
    },
  };
  let state: ReturnType<typeof useFollowScope> | null = null;
  const Probe = () => {
    state = useFollowScope(agent);
    return null;
  };
  const root = createRoot(document.createElement("div"));
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => act(async () => root.unmount());
  return { follows, state: () => state! };
}

test("useFollowScope à l'abonnement prêt demande follow", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onReady());
  expect(h.follows).toHaveLength(1);
});

test("useFollowScope à la reconnexion redemande follow", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onReady());
  await act(async () => h.state().onReady());
  expect(h.follows).toHaveLength(2);
});

test("useFollowScope un évènement viewing fixe la portée", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onMcp({ kind: "viewing", scope: { kind: "atelier", pack: "p-1" } }));
  expect(h.state().scope).toEqual({ kind: "atelier", pack: "p-1" });
});

test("useFollowScope n'est pas prêt avant le premier viewing", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onReady());
  expect(h.state().ready).toBe(false);
});

test("useFollowScope est prêt après follow réussi et un viewing", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onReady());
  await act(async () => h.state().onMcp({ kind: "viewing", scope: { kind: "slides" } }));
  expect(h.state().ready).toBe(true);
});

test("useFollowScope follow en échec : pas prêt", async () => {
  const h = await mountFollow(true);
  await act(async () => h.state().onReady());
  await act(async () => h.state().onMcp({ kind: "viewing", scope: { kind: "slides" } }));
  expect(h.state().ready).toBe(false);
});

test("useFollowScope un échec tardif d'un follow plus ancien ne défait pas le plus récent", async () => {
  const replies: { resolve: () => void; reject: (e: Error) => void }[] = [];
  const agent = { follow: () => new Promise<void>((resolve, reject) => void replies.push({ resolve, reject })) };
  let state: ReturnType<typeof useFollowScope> | null = null;
  const Probe = () => {
    state = useFollowScope(agent);
    return null;
  };
  const root = createRoot(document.createElement("div"));
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => act(async () => root.unmount());
  await act(async () => state!.onReady());
  await act(async () => state!.onReady());
  await act(async () => state!.onMcp({ kind: "viewing", scope: { kind: "slides" } }));
  await act(async () => replies[1].resolve());
  await act(async () => replies[0].reject(new Error("tard")));
  expect(state!.ready).toBe(true);
});
