// Rendu du bandeau du pack dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { PackNotice } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const PackNoticeBar = (await import("./PackNoticeBar")).default;
const { expectIconOnly } = await import("./test/iconOnly");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const notice: PackNotice = {
  pack: "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10",
  name: "Charte",
  relation: "descendant",
  message: "Ce deck utilise une version plus récente de « Charte » que la vôtre.",
  use_mine: "Passer à ma version",
  make_current: "Faire de la sienne ma version courante",
};

async function mount(calls: string[], shown: PackNotice = notice) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(PackNoticeBar, {
      notice: shown,
      onUseMine: () => calls.push("mine"),
      onMakeCurrent: () => calls.push("current"),
      onClose: () => calls.push("close"),
    }));
  });
  return host;
}

describe("PackNoticeBar", () => {
  test("PackNoticeBar shows the message and both actions from the session", async () => {
    const host = await mount([]);
    expect(host.querySelector(".notice-bar-text")?.textContent).toBe(notice.message);
    const [mine, current] = host.querySelectorAll(".notice-bar-actions button");
    expect(mine.textContent).toBe(notice.use_mine);
    expect(current.textContent).toBe(notice.make_current);
  });

  test("PackNoticeBar buttons call their action", async () => {
    const calls: string[] = [];
    const host = await mount(calls);
    const [mine, current, close] = host.querySelectorAll("button");
    await act(async () => (mine as HTMLButtonElement).click());
    await act(async () => (current as HTMLButtonElement).click());
    await act(async () => (close as HTMLButtonElement).click());
    expect(calls).toEqual(["mine", "current", "close"]);
  });

  test("PackNoticeBar close button shows an icon only", async () => {
    const host = await mount([]);
    expectIconOnly(host.querySelector(".pack-notice-close"));
  });

  test("PackNoticeBar is a bar: text, then text buttons side by side, then close", async () => {
    const host = await mount([]);
    const bar = host.querySelector(".pack-notice");
    // Une barre posée, pas un objet qui se soulève au survol.
    expect(bar?.classList.contains("object")).toBe(false);
    expect([...(bar?.children ?? [])].map((c) => c.className)).toEqual([
      "notice-bar-text",
      "notice-bar-actions",
      "token ghost pack-notice-close",
    ]);
    // Des boutons de texte, pas des pastilles rondes d'icône (`token`).
    const [mine, current] = [...host.querySelectorAll(".notice-bar-actions button")];
    expect(mine?.className).toBe("btn primary");
    expect(current?.className).toBe("btn ghost");
  });

  test("PackNoticeBar hides make current when the session does not offer it", async () => {
    const host = await mount([], { ...notice, make_current: null });
    const actions = host.querySelectorAll(".notice-bar-actions button");
    expect(actions.length).toBe(1);
    expect(actions[0].textContent).toBe(notice.use_mine);
  });
});
