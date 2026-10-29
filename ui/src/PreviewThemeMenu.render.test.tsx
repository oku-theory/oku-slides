// The pack tab header's preview-theme select.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { ThemeInfo } from "./packTypes";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: PreviewThemeMenu } = await import("./PreviewThemeMenu");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

const theme = (name: string, namespace: string) => ({ id: namespace, version: "v", name, namespace, extends: [], builtin: false }) as ThemeInfo;
const THEMES = [theme("Papier", "papier"), theme("Nuit", "nuit")];

async function mount(current: string | null, picks: string[]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(PreviewThemeMenu, { themes: THEMES, theme: current, onPick: (ns: string) => picks.push(ns) })));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  return host.querySelector("select") as HTMLSelectElement;
}

describe("PreviewThemeMenu", () => {
  test("PreviewThemeMenu lists the themes by name with the current one selected", async () => {
    const select = await mount("nuit", []);
    expect([...select.options].map((o) => o.textContent)).toEqual(["Papier", "Nuit"]);
    expect(select.value).toBe("nuit");
  });

  test("PreviewThemeMenu is labelled by the preview theme key", async () => {
    const select = await mount("papier", []);
    expect(select.getAttribute("aria-label")).toBe("ui-preview-theme");
  });

  test("PreviewThemeMenu picking a theme reports its namespace", async () => {
    const picks: string[] = [];
    const select = await mount("papier", picks);
    await act(async () => {
      select.value = "nuit";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(picks).toEqual(["nuit"]);
  });
});
