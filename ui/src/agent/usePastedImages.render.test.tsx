// Les images collées d'une vue : un collage qui échoue n'arrête pas les suivants, les miniatures sont libérées.

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import "../test/dom";
import type { PastedImage } from "../api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { usePastedImages } = await import("./usePastedImages");

const realCreate = URL.createObjectURL;
const realRevoke = URL.revokeObjectURL;
let created: string[] = [];
let revoked: string[] = [];
beforeEach(() => {
  created = [];
  revoked = [];
  URL.createObjectURL = () => (created.push(`blob:${created.length}`), `blob:${created.length - 1}`);
  URL.revokeObjectURL = (u: string) => void revoked.push(u);
});
let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  URL.createObjectURL = realCreate;
  URL.revokeObjectURL = realRevoke;
  await cleanup?.();
  cleanup = null;
});

const png = () => new File([new Uint8Array([1, 2, 3])], "x.png", { type: "image/png" });
const image = (id: string): PastedImage => ({ id, mime: "image/png", width: 1, height: 1 });

async function mount(paste: (data: string) => Promise<void>) {
  const said: string[] = [];
  let images: PastedImage[] = [];
  let hook: ReturnType<typeof usePastedImages> | null = null;
  let current = paste;
  const Probe = ({ list }: { list: PastedImage[] }) => {
    hook = usePastedImages(list, current, (m) => said.push(m));
    return null;
  };
  const root = createRoot(document.createElement("div"));
  const render = (list: PastedImage[]) => act(async () => (images = list, root.render(createElement(Probe, { list }))));
  await render(images);
  cleanup = async () => act(async () => root.unmount());
  const setPaste = (p: (data: string) => Promise<void>) => act(async () => (current = p, root.render(createElement(Probe, { list: images }))));
  return { said, hook: () => hook!, render, setPaste, unmount: () => act(async () => root.unmount()) };
}

const settle = () => act(async () => new Promise((r) => setTimeout(r, 20)));

describe("usePastedImages", () => {
  test("usePastedImages a rejected paste is reported and the next paste is still sent", async () => {
    const sent: string[] = [];
    let first = true;
    const m = await mount(async (d) => {
      if (first) {
        first = false;
        throw new Error("refusée");
      }
      sent.push(d);
    });
    await act(async () => m.hook().onPaste([png()]));
    await settle();
    await act(async () => m.hook().onPaste([png()]));
    await settle();
    expect([m.said.length, sent.length]).toEqual([1, 1]);
  });

  test("usePastedImages a file that cannot be read does not stop the next paste", async () => {
    const sent: string[] = [];
    const m = await mount(async (d) => void sent.push(d));
    const broken = { arrayBuffer: () => Promise.reject(new Error("illisible")) } as unknown as File;
    await act(async () => m.hook().onPaste([broken]));
    await settle();
    await act(async () => m.hook().onPaste([png()]));
    await settle();
    expect([m.said.length, sent.length]).toEqual([1, 1]);
  });

  test("usePastedImages revokes the thumbnail when the image leaves the draft", async () => {
    const m = await mount(async () => {});
    await act(async () => m.hook().onPaste([png()]));
    await settle();
    const id = [...m.hook().thumbs.keys()][0];
    if (!id) throw new Error("no thumbnail: crypto.subtle missing");
    await m.render([image(id)]);
    await m.render([]);
    expect(revoked).toEqual(created.map((_, i) => `blob:${i}`));
    expect(revoked.length).toBe(1);
  });

  test("usePastedImages sends a queued file to the paste it was pasted with, not the tab switched to", async () => {
    const old: string[] = [];
    const next: string[] = [];
    let release: () => void = () => {};
    let first = true;
    const m = await mount(async (d) => {
      if (first) {
        first = false;
        await new Promise<void>((r) => (release = r));
      }
      old.push(d);
    });
    await act(async () => m.hook().onPaste([png(), png()]));
    await settle();
    await m.setPaste(async (d) => void next.push(d));
    release();
    await settle();
    expect([old.length, next.length]).toEqual([2, 0]);
  });

  test("usePastedImages makes no thumbnail after unmount", async () => {
    let release: () => void = () => {};
    const m = await mount(() => new Promise<void>((r) => (release = r)));
    await act(async () => m.hook().onPaste([png()]));
    await settle();
    await m.unmount();
    release();
    await settle();
    expect(created.length).toBe(0);
  });
});
