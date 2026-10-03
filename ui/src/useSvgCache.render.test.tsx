// SVG des slides en URL d'objet : cache par empreinte, image précédente gardée pendant un rendu.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { TabApi } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useSvgCache } = await import("./useSvgCache");
const { deferred } = await import("./test/widgetApi");

type Get = ReturnType<typeof useSvgCache>;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Une api dont chaque `svg(id)` attend qu'on le libère (`release`). */
function heldApi() {
  const calls: { id: string; done: ReturnType<typeof deferred<string>> }[] = [];
  const api = {
    svg: (id: string) => {
      const done = deferred<string>();
      calls.push({ id, done });
      return done.promise;
    },
  } as unknown as TabApi;
  const release = (i: number, svg: string) => act(async () => calls[i].done.resolve(svg));
  return { api, calls, release };
}

/** Monte le hook ; `out.get` est la fonction rendue au dernier rendu. */
async function mount(api: TabApi) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: { get: Get | null } = { get: null };
  function Probe() {
    out.get = useSvgCache(api);
    return null;
  }
  await act(async () => root.render(createElement(Probe)));
  return out;
}

describe("useSvgCache", () => {
  test("useSvgCache sans image encore chargée rend null, puis l'URL une fois le SVG reçu", async () => {
    const { api, release } = heldApi();
    const out = await mount(api);
    expect(out.get!("s-1", "h1")).toBeNull();
    await release(0, "<svg/>");
    expect(out.get!("s-1", "h1")).toStartWith("blob:");
  });

  test("useSvgCache pendant le rendu d'une nouvelle empreinte garde l'image précédente de la slide", async () => {
    const { api, release } = heldApi();
    const out = await mount(api);
    out.get!("s-1", "h1");
    await release(0, "<svg id='a'/>");
    const before = out.get!("s-1", "h1");
    // La slide a changé (édition) : son nouveau SVG n'est pas encore là.
    expect(out.get!("s-1", "h2")).toBe(before);
    await release(1, "<svg id='b'/>");
    const after = out.get!("s-1", "h2");
    expect(after).toStartWith("blob:");
    expect(after).not.toBe(before);
  });

  test("useSvgCache ne prête pas l'image d'une autre slide", async () => {
    const { api, release } = heldApi();
    const out = await mount(api);
    out.get!("s-1", "h1");
    await release(0, "<svg/>");
    expect(out.get!("s-2", "h9")).toBeNull();
  });

  test("useSvgCache demande une empreinte une seule fois tant qu'elle est en cours", async () => {
    const { api, calls } = heldApi();
    const out = await mount(api);
    out.get!("s-1", "h1");
    out.get!("s-1", "h1");
    expect(calls.length).toBe(1);
  });
});
