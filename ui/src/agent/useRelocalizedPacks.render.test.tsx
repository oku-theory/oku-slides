// Le panneau des packs suit la langue de l'interface : un changement le redemande au hub.

import { afterAll, afterEach, expect, test } from "bun:test";
import "../test/dom";
import type { Surface } from "../api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useRelocalizedPacks } = await import("./useRelocalizedPacks");
const { HOST_PACKS } = await import("./surfaces");
const { applyMessages } = await import("../i18n");
const { resetMessages } = await import("../test/messages");

afterAll(resetMessages);
let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

const surface = (id: string) => ({ id, place: "thread", live: true, question: false, messages: [] }) as Surface;

async function mount(surfaces: Surface[], showPacks: () => Promise<void>, say: (m: string, error?: boolean) => void = () => {}) {
  function Probe() {
    useRelocalizedPacks(surfaces, showPacks, say);
    return null;
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  unmount = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  await act(async () => root.render(createElement(Probe)));
}

test("useRelocalizedPacks with the packs panel shown asks for it again on a locale change", async () => {
  let asked = 0;
  await mount([surface("ask-1"), surface(HOST_PACKS)], async () => void asked++);
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  expect(asked).toBe(1);
});

test("useRelocalizedPacks without the packs panel sends nothing on a locale change", async () => {
  let asked = 0;
  await mount([surface("host-widget")], async () => void asked++);
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  expect(asked).toBe(0);
});

test("useRelocalizedPacks once unmounted sends nothing", async () => {
  let asked = 0;
  await mount([surface(HOST_PACKS)], async () => void asked++);
  await unmount!();
  unmount = null;
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  expect(asked).toBe(0);
});

test("useRelocalizedPacks with a refused request toasts the host's error text", async () => {
  const said: string[] = [];
  await mount([surface(HOST_PACKS)], () => Promise.reject(new Error("pas d'outils")), (m) => void said.push(m));
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  await new Promise((r) => setTimeout(r, 0));
  expect(said).toEqual(["pas d'outils"]);
});
