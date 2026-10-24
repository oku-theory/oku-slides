// Les boîtes de l'hôte suivent la langue de l'interface : un changement les redemande au hub.

import { afterAll, afterEach, expect, test } from "bun:test";
import "../test/dom";
import type { Surface } from "../api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useRelocalizedBoxes } = await import("./useRelocalizedBoxes");
const { HOST_PACKS, HOST_VOICE } = await import("./surfaces");
const { applyMessages } = await import("../i18n");
const { resetMessages } = await import("../test/messages");

afterAll(resetMessages);
let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

const surface = (id: string) => ({ id, place: "thread", live: true, question: false, messages: [] }) as Surface;

async function mount(
  surfaces: Surface[],
  reshow: ReadonlyMap<string, () => Promise<void>>,
  say: (m: string, error?: boolean) => void = () => {},
  enabled = true,
) {
  function Probe() {
    useRelocalizedBoxes(surfaces, reshow, say, enabled);
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

/** Une table de réaffichage avec une seule boîte. */
const only = (id: string, show: () => Promise<void>) => new Map([[id, show]]);

test("useRelocalizedBoxes with the packs panel shown asks for it again on a locale change", async () => {
  let asked = 0;
  await mount([surface("ask-1"), surface(HOST_PACKS)], only(HOST_PACKS, async () => void asked++));
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  expect(asked).toBe(1);
});

test("useRelocalizedBoxes reshows the voice box on a locale change", async () => {
  let asked = 0;
  await mount([surface(HOST_VOICE)], only(HOST_VOICE, async () => void asked++));
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  expect(asked).toBe(1);
});

test("useRelocalizedBoxes ignores a box absent from reshow", async () => {
  let asked = 0;
  await mount([surface(HOST_VOICE)], only(HOST_PACKS, async () => void asked++));
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  expect(asked).toBe(0);
});

test("useRelocalizedBoxes without a box shown sends nothing on a locale change", async () => {
  let asked = 0;
  await mount([surface("host-widget")], only(HOST_PACKS, async () => void asked++));
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  expect(asked).toBe(0);
});

test("useRelocalizedBoxes once unmounted sends nothing", async () => {
  let asked = 0;
  await mount([surface(HOST_PACKS)], only(HOST_PACKS, async () => void asked++));
  await unmount!();
  unmount = null;
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  expect(asked).toBe(0);
});

test("useRelocalizedBoxes with a refused request toasts the host's error text", async () => {
  const said: string[] = [];
  await mount([surface(HOST_PACKS)], only(HOST_PACKS, () => Promise.reject(new Error("pas d'outils"))), (m) => void said.push(m));
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  await new Promise((r) => setTimeout(r, 0));
  expect(said).toEqual(["pas d'outils"]);
});

// Agent détaché : la fenêtre principale garde un panneau monté sans fil ; seule celle de l'agent redemande.
test("useRelocalizedBoxes disabled sends nothing on a locale change", async () => {
  let asked = 0;
  await mount([surface(HOST_PACKS)], only(HOST_PACKS, async () => void asked++), () => {}, false);
  applyMessages({ locale: "de", resources: [], chosen: "de" });
  expect(asked).toBe(0);
});
