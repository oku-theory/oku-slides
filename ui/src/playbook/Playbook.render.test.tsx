// The gallery mounts every example without a surface error and logs the actions sent.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import fixture from "../a2ui/fixtures/playbook.json";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { Playbook } = await import("./Playbook");
type Entry = import("./playbookEntries").PlaybookEntry;

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mountGallery(): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Playbook, { entries: fixture as unknown as Entry[] })));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  return host;
}

describe("playbook gallery", () => {
  test("shows the four groups in order", async () => {
    const host = await mountGallery();
    const titles = [...host.querySelectorAll(".playbook-group > h2")].map((h) => h.textContent);
    expect(titles).toEqual(["Layout", "Atoms", "Fields", "Host only"]);
  });

  test("renders one card per example", async () => {
    const host = await mountGallery();
    expect(host.querySelectorAll(".playbook-example")).toHaveLength(fixture.length);
  });

  test("reports no surface error", async () => {
    const host = await mountGallery();
    expect([...host.querySelectorAll(".playbook-error")].map((e) => e.textContent)).toEqual([]);
  });

  test("draws something in every surface", async () => {
    const host = await mountGallery();
    const empty = [...host.querySelectorAll<HTMLElement>(".playbook-example")]
      .filter((card) => !card.querySelector(".playbook-surface *"))
      .map((card) => card.dataset.example);
    expect(empty).toEqual([]);
  });

  test("a click on the suggest button logs its event and prompt", async () => {
    const host = await mountGallery();
    const button = host.querySelector<HTMLButtonElement>('[data-example="Button/suggest"] .playbook-surface button')!;
    await act(async () => button.click());
    const logged = host.querySelector(".playbook-log")?.textContent ?? "";
    expect(logged).toContain('"name":"suggest"');
    expect(logged).toContain('"prompt":"Make it shorter"');
  });
});
