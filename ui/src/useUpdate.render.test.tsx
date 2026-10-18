// Mise à jour : état relu à l'abonnement et à chaque signal, « Plus tard », installation.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { UpdateApi, UpdateEvents, UpdateOffer, UpdateStatus } from "./api";

const { act, createElement, StrictMode } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useUpdate } = await import("./useUpdate");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const offer: UpdateOffer = { version: "2.2.0", mode: "install" };
const status = (o: Partial<UpdateStatus> = {}): UpdateStatus => ({ offer: null, installing: false, ...o });

/** Un hôte factice : `state` est ce que `status()` rend, `fire` déclenche ses événements. */
function fakeApi(o: { check?: () => Promise<UpdateStatus>; install?: () => Promise<void>; openRelease?: () => Promise<void> } = {}) {
  const calls: string[] = [];
  const live = new Set<UpdateEvents>();
  const fake = {
    state: status(),
    calls,
    live,
    api: {
      status: async () => (calls.push("status"), fake.state),
      check: o.check ?? (async () => fake.state),
      install: o.install ?? (async () => void calls.push("install")),
      openRelease: o.openRelease ?? (async () => void calls.push("open")),
      subscribe: async (h: UpdateEvents) => {
        calls.push("subscribe");
        live.add(h);
        return () => live.delete(h);
      },
    } satisfies UpdateApi,
    changed: () => live.forEach((h) => h.onChanged()),
    progress: (received: number, total: number | null) => live.forEach((h) => h.onProgress({ received, total })),
    requested: () => live.forEach((h) => h.onCheckRequested()),
  };
  return fake;
}

const errors: string[] = [];
const onError = (text: string) => void errors.push(text);

async function mount(api: UpdateApi | null, strict = false) {
  errors.length = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: { hook: ReturnType<typeof useUpdate> | null } = { hook: null };
  function Probe() {
    out.hook = useUpdate(api, onError);
    return null;
  }
  const tree = strict ? createElement(StrictMode, null, createElement(Probe)) : createElement(Probe);
  await act(async () => root.render(tree));
  return { hook: () => out.hook! };
}

describe("useUpdate", () => {
  test("useUpdate without host does nothing", async () => {
    const m = await mount(null);
    await act(async () => {
      expect(await m.hook().check()).toBeNull();
      m.hook().install();
      m.hook().later();
      m.hook().openRelease();
    });
    expect([m.hook().offer, errors]).toEqual([null, []]);
  });

  test("useUpdate reads the status once subscribed", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    expect([m.hook().offer, f.calls]).toEqual([offer, ["subscribe", "status"]]);
  });

  test("useUpdate rerender keeps a single subscription", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => f.progress(10, 100));
    expect([f.calls.filter((c) => c === "subscribe").length, f.live.size, m.hook().percent]).toEqual([1, 1, 10]);
  });

  test("useUpdate strict mode ends with one live handler", async () => {
    const f = fakeApi();
    await mount(f.api, true);
    expect(f.live.size).toBe(1);
  });

  test("useUpdate unmount unsubscribes", async () => {
    const f = fakeApi();
    await mount(f.api);
    await act(async () => mounted!.root.unmount());
    mounted = null;
    expect(f.live.size).toBe(0);
  });

  test("useUpdate changed signal rereads the status", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    f.state = status({ offer });
    await act(async () => f.changed());
    expect(m.hook().offer).toEqual(offer);
  });

  test("useUpdate keeps only the latest status reply", async () => {
    const f = fakeApi();
    let first: (s: UpdateStatus) => void = () => {};
    let calls = 0;
    f.api.status = () => (++calls === 1 ? new Promise((r) => (first = r)) : Promise.resolve(status({ offer })));
    const m = await mount(f.api);
    await act(async () => f.changed());
    await act(async () => first(status()));
    expect(m.hook().offer).toEqual(offer);
  });

  test("useUpdate later hides this version", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().later());
    await act(async () => f.changed());
    expect(m.hook().offer).toBeNull();
  });

  test("useUpdate later still shows a newer version", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().later());
    f.state = status({ offer: { ...offer, version: "2.3.0" } });
    await act(async () => f.changed());
    expect(m.hook().offer?.version).toBe("2.3.0");
  });

  test("useUpdate installing shows the offer even after later", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().later());
    f.state = status({ offer, installing: true });
    await act(async () => f.changed());
    expect([m.hook().offer, m.hook().installing]).toEqual([offer, true]);
  });

  test("useUpdate install ending rereads installing false", async () => {
    const f = fakeApi();
    f.state = status({ offer, installing: true });
    const m = await mount(f.api);
    f.state = status({ offer });
    await act(async () => f.changed());
    expect(m.hook().installing).toBe(false);
  });

  test("useUpdate progress gives a percent", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => f.progress(250, 1000));
    expect(m.hook().percent).toBe(25);
  });

  test("useUpdate progress of unknown size has no percent", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => f.progress(250, null));
    expect(m.hook().percent).toBeNull();
  });

  test("useUpdate install calls the host", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().install());
    expect(f.calls.at(-1)).toBe("install");
  });

  test("useUpdate install failure reports it and stops installing", async () => {
    const f = fakeApi({ install: async () => { throw new Error("réseau"); } });
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().install());
    expect([m.hook().installing, errors]).toEqual([false, ["réseau"]]);
  });

  test("useUpdate install resets a previous percent", async () => {
    let attempt = 0;
    const f = fakeApi({
      install: () => (attempt++ === 0 ? Promise.reject(new Error("réseau")) : new Promise<void>(() => {})),
    });
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => f.progress(400, 1000));
    await act(async () => m.hook().install());
    expect(m.hook().percent).toBeNull();
    await act(async () => f.progress(400, 1000));
    expect(m.hook().percent).toBe(40);
    await act(async () => m.hook().install());
    expect(m.hook().percent).toBeNull();
  });

  test("useUpdate open release calls the host", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => m.hook().openRelease());
    expect(f.calls.at(-1)).toBe("open");
  });

  test("useUpdate open release failure reports it", async () => {
    const f = fakeApi({ openRelease: async () => { throw new Error("refusé"); } });
    const m = await mount(f.api);
    await act(async () => m.hook().openRelease());
    expect(errors).toEqual(["refusé"]);
  });

  test("useUpdate check after later shows the offer again", async () => {
    const f = fakeApi({ check: async () => status({ offer }) });
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().later());
    await act(async () => void (await m.hook().check()));
    expect(m.hook().offer).toEqual(offer);
  });

  test("useUpdate check-requested event counts a request", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => f.requested());
    expect(m.hook().requested).toBe(1);
  });
});
