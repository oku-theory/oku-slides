// Le bandeau du pack : la surface émise par la session, rendue sous le titre.

import { afterEach, describe, expect, spyOn, test } from "bun:test";
import "./test/dom";
import type { Surface } from "./api";
import agentSurfaces from "./a2ui/fixtures/agent-surfaces.json";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const PackNotice = (await import("./PackNotice")).default;
// Chargée d'avance : la surface paresseuse se résout dans le même `act`.
await import("./a2ui/SurfaceView");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Le bandeau tel que la session Rust l'émet (fixture de contrat), avec ses deux boutons. */
const banner = () => structuredClone((agentSurfaces as unknown as Surface[]).find((s) => s.id === "host-pack-notice")!);

const view = (surface: Surface, sent: string[], errors: string[]) =>
  createElement(PackNotice, { surface, onCommand: (c: string) => sent.push(c), onError: (m: string) => errors.push(m) });

async function mount(surface: Surface = banner(), sent: string[] = [], errors: string[] = []) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(view(surface, sent, errors)));
  await act(async () => {});
  return { host, root, sent, errors };
}

describe("PackNotice", () => {
  test("PackNotice renders the session banner inside the pack-notice row", async () => {
    const { host } = await mount();
    expect(host.querySelector(".pack-notice .notice-bar[role=status] .notice-bar-text")?.textContent).toContain("Charte");
  });

  test("PackNotice renders the fixture without errors", async () => {
    const { errors } = await mount();
    expect(errors).toEqual([]);
  });

  test("PackNotice sends each button's request, then close", async () => {
    const { host, sent } = await mount();
    const buttons = [...host.querySelectorAll(".notice-bar-actions button"), host.querySelector(".notice-bar-close")];
    for (const b of buttons) await act(async () => (b as HTMLElement).click());
    expect(sent).toEqual(["pack_use_mine", "pack_make_current", "pack_notice_close"]);
  });

  test("PackNotice with an equal new snapshot keeps the same banner node", async () => {
    const { host, root } = await mount();
    const before = host.querySelector(".notice-bar");
    await act(async () => root.render(view(banner(), [], [])));
    await act(async () => {});
    expect(host.querySelector(".notice-bar")).toBe(before);
  });

  test("PackNotice with an unknown event sends no request", async () => {
    const odd = banner();
    const json = JSON.stringify(odd.messages).replace('"pack_use_mine"', '"delete_deck"');
    odd.messages = JSON.parse(json);
    const logged = spyOn(console, "error").mockImplementation(() => {});
    const { host, sent } = await mount(odd);
    await act(async () => (host.querySelector(".notice-bar-actions button") as HTMLElement).click());
    logged.mockRestore();
    expect(sent).toEqual([]);
  });
});
