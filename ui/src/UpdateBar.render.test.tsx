// Bandeau de mise à jour : actions selon le mode, avancée du téléchargement.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { UpdateOffer } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const UpdateBar = (await import("./UpdateBar")).default;

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const offer: UpdateOffer = { version: "2.2.0", mode: "install" };

async function mount(calls: string[], o: { offer?: UpdateOffer; installing?: boolean; percent?: number | null } = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(UpdateBar, {
      offer: o.offer ?? offer,
      installing: o.installing ?? false,
      percent: o.percent ?? null,
      onInstall: () => calls.push("install"),
      onLater: () => calls.push("later"),
      onOpenRelease: () => calls.push("open"),
    }));
  });
  return host;
}

const buttons = (host: HTMLElement) => [...host.querySelectorAll("button")];
const labels = (host: HTMLElement) => buttons(host).map((b) => b.textContent);
const click = (host: HTMLElement, i: number) => act(async () => buttons(host)[i]!.click());

describe("UpdateBar", () => {
  test("UpdateBar install mode offers install and later", async () => {
    expect(labels(await mount([]))).toEqual(["ui-update-install", "ui-update-later"]);
  });

  test("UpdateBar notify-only mode offers the release page", async () => {
    expect(labels(await mount([], { offer: { ...offer, mode: "notify_only" } }))).toEqual(["ui-update-see-release", "ui-update-later"]);
  });

  test("UpdateBar install button calls onInstall", async () => {
    const calls: string[] = [];
    await click(await mount(calls), 0);
    expect(calls).toEqual(["install"]);
  });

  test("UpdateBar release button calls onOpenRelease", async () => {
    const calls: string[] = [];
    await click(await mount(calls, { offer: { ...offer, mode: "notify_only" } }), 0);
    expect(calls).toEqual(["open"]);
  });

  test("UpdateBar later button calls onLater", async () => {
    const calls: string[] = [];
    await click(await mount(calls), 1);
    expect(calls).toEqual(["later"]);
  });

  test("UpdateBar installing disables its buttons", async () => {
    const host = await mount([], { installing: true, percent: 40 });
    expect(buttons(host).every((b) => b.disabled)).toBe(true);
  });

  test("UpdateBar installing shows the percent", async () => {
    const host = await mount([], { installing: true, percent: 40 });
    expect([host.querySelector(".update-bar-text")?.textContent, host.querySelector<HTMLProgressElement>("progress")?.value]).toEqual(["ui-update-installing", 40]);
  });

  test("UpdateBar unknown size shows an indeterminate bar", async () => {
    const host = await mount([], { installing: true, percent: null });
    expect([host.querySelector(".update-bar-text")?.textContent, host.querySelector("progress")?.hasAttribute("value")]).toEqual(["ui-update-downloading", false]);
  });
});
