// A view's attachments: a failing attach does not stop the next ones, thumbnails are released.

import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import "../test/dom";
import { applyMessages } from "../i18n";
import { resetMessages } from "../test/messages";
import type { AttachLimits, Attachment } from "./attachments";
import { assetId } from "./attachments";

// The UI's own refusals are read in French: the test checks the key and a non-English locale.
const ftl = (l: string, f = "ui") => readFileSync(`${import.meta.dir}/../../../crates/oku-slides-i18n/src/locales/${l}/${f}.ftl`, "utf8");
beforeAll(() => applyMessages({ locale: "fr", resources: [ftl("en"), ftl("fr")], chosen: null }));
afterAll(resetMessages);

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useAttachments } = await import("./useAttachments");

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
const image = (id: string): Attachment => ({ type: "image", id, mime: "image/png", width: 1, height: 1 });

const LIMITS: AttachLimits = { image_bytes: 1 << 20, document_bytes: 2 << 20, draft_images: 8, draft_documents: 2 };

async function mount(paste: (name: string, data: string) => Promise<void>, limits: AttachLimits | null = LIMITS) {
  const said: string[] = [];
  let images: Attachment[] = [];
  let hook: ReturnType<typeof useAttachments> | null = null;
  let current = paste;
  const Probe = ({ list }: { list: Attachment[] }) => {
    hook = useAttachments(list, current, (m) => said.push(m), limits);
    return null;
  };
  const root = createRoot(document.createElement("div"));
  const render = (list: Attachment[]) => act(async () => (images = list, root.render(createElement(Probe, { list }))));
  await render(images);
  cleanup = async () => act(async () => root.unmount());
  const setPaste = (p: (name: string, data: string) => Promise<void>) => act(async () => (current = p, root.render(createElement(Probe, { list: images }))));
  return { said, hook: () => hook!, render, setPaste, unmount: () => act(async () => root.unmount()) };
}

const settle = () => act(async () => new Promise((r) => setTimeout(r, 20)));

describe("useAttachments", () => {
  test("useAttachments a rejected paste is reported and the next paste is still sent", async () => {
    const sent: string[] = [];
    let first = true;
    const m = await mount(async (_name, d) => {
      if (first) {
        first = false;
        throw new Error("refusée");
      }
      sent.push(d);
    });
    await act(async () => m.hook().onAttach([png()]));
    await settle();
    await act(async () => m.hook().onAttach([png()]));
    await settle();
    expect([m.said.length, sent.length]).toEqual([1, 1]);
  });

  test("useAttachments a file that cannot be read does not stop the next paste", async () => {
    const sent: string[] = [];
    const m = await mount(async (_name, d) => void sent.push(d));
    const broken = { name: "x.png", size: 3, type: "image/png", arrayBuffer: () => Promise.reject(new Error("illisible")) } as unknown as File;
    await act(async () => m.hook().onAttach([broken]));
    await settle();
    await act(async () => m.hook().onAttach([png()]));
    await settle();
    expect([m.said.length, sent.length]).toEqual([1, 1]);
  });

  test("useAttachments revokes the thumbnail when the image leaves the draft", async () => {
    const m = await mount(async () => {});
    await act(async () => m.hook().onAttach([png()]));
    await settle();
    const id = [...m.hook().thumbs.keys()][0];
    if (!id) throw new Error("no thumbnail: crypto.subtle missing");
    await m.render([image(id)]);
    await m.render([]);
    expect(revoked).toEqual(created.map((_, i) => `blob:${i}`));
    expect(revoked.length).toBe(1);
  });

  test("useAttachments image removed before its reply makes no thumbnail", async () => {
    let release: () => void = () => {};
    const m = await mount(() => new Promise<void>((r) => (release = r)));
    const id = await assetId(new Uint8Array([1, 2, 3]).buffer as ArrayBuffer);
    if (!id) throw new Error("crypto.subtle missing");
    await act(async () => m.hook().onAttach([png()]));
    await settle();
    // The host shows the image, then another view removes it, before the attach reply.
    await m.render([image(id)]);
    await m.render([]);
    release();
    await settle();
    expect([created.length, revoked.length, m.hook().thumbs.size]).toEqual([0, 0, 0]);
  });

  test("useAttachments thumbnail made before the draft shows the image is kept", async () => {
    const m = await mount(async () => {});
    await act(async () => m.hook().onAttach([png()]));
    await settle();
    const id = [...m.hook().thumbs.keys()][0];
    if (!id) throw new Error("no thumbnail: crypto.subtle missing");
    await m.render([image(id)]);
    expect([created.length, revoked.length, m.hook().thumbs.get(id)]).toEqual([1, 0, "blob:0"]);
  });

  test("useAttachments more files than the draft can hold refused at once, none read", async () => {
    const sent: string[] = [];
    const m = await mount(async (name) => void sent.push(name), { ...LIMITS, draft_images: 1, draft_documents: 1 });
    const files = [counted("a.pdf", 4), counted("b.pdf", 4), counted("c.pdf", 4)];
    await act(async () => m.hook().onAttach(files.map((x) => x.f)));
    await settle();
    expect([files.map((x) => x.reads.n), sent, m.said]).toEqual([
      [0, 0, 0],
      [],
      ["Trop de fichiers à la fois : la limite est de 2."],
    ]);
  });

  test("useAttachments without host limits reports once for several files", async () => {
    const m = await mount(async () => {}, null);
    await act(async () => m.hook().onAttach([counted("a.pdf", 4).f, counted("b.pdf", 4).f]));
    await settle();
    expect(m.said.length).toBe(1);
  });

  test("useAttachments size limit shown with one decimal", async () => {
    const m = await mount(async () => {}, { ...LIMITS, document_bytes: 1_234_567 });
    await act(async () => m.hook().onAttach([counted("gros.pdf", 2_000_000).f]));
    await settle();
    expect(m.said).toEqual(["\u2068gros.pdf\u2069 dépasse la limite de 1,2 Mo."]);
  });

  test("useAttachments sends a queued file to the paste it was pasted with, not the tab switched to", async () => {
    const old: string[] = [];
    const next: string[] = [];
    let release: () => void = () => {};
    let first = true;
    const m = await mount(async (_name, d) => {
      if (first) {
        first = false;
        await new Promise<void>((r) => (release = r));
      }
      old.push(d);
    });
    await act(async () => m.hook().onAttach([png(), png()]));
    await settle();
    await m.setPaste(async (_name, d) => void next.push(d));
    release();
    await settle();
    expect([old.length, next.length]).toEqual([2, 0]);
  });

  test("useAttachments makes no thumbnail after unmount", async () => {
    let release: () => void = () => {};
    const m = await mount(() => new Promise<void>((r) => (release = r)));
    await act(async () => m.hook().onAttach([png()]));
    await settle();
    await m.unmount();
    release();
    await settle();
    expect(created.length).toBe(0);
  });

  test("useAttachments sends the file name and makes no thumbnail for a document", async () => {
    const names: string[] = [];
    const m = await mount(async (name) => void names.push(name));
    const doc = new File([new Uint8Array([35, 32, 120])], "notes.md", { type: "text/markdown" });
    await act(async () => m.hook().onAttach([doc]));
    await settle();
    expect([names, created.length]).toEqual([["notes.md"], 0]);
  });
  /** A file whose reads are counted. */
  const counted = (name: string, size: number, type = "application/pdf") => {
    const reads = { n: 0 };
    const f = { name, size, type, arrayBuffer: () => (reads.n++, Promise.resolve(new Uint8Array(size).buffer)) } as unknown as File;
    return { f, reads };
  };

  test("useAttachments oversize file refused before reading", async () => {
    const sent: string[] = [];
    const m = await mount(async (name) => void sent.push(name));
    const { f, reads } = counted("rapport.pdf", (2 << 20) + 1);
    await act(async () => m.hook().onAttach([f]));
    await settle();
    expect([reads.n, sent, m.said]).toEqual([0, [], ["\u2068rapport.pdf\u2069 dépasse la limite de 2 Mo."]]);
  });

  test("useAttachments empty file refused", async () => {
    const sent: string[] = [];
    const m = await mount(async (name) => void sent.push(name));
    const { f, reads } = counted("vide.md", 0, "text/markdown");
    await act(async () => m.hook().onAttach([f]));
    await settle();
    expect([reads.n, sent, m.said]).toEqual([0, [], ["\u2068vide.md\u2069 est vide."]]);
  });

  test("useAttachments host refusal shown in fr", async () => {
    const refusal = /^session-attach-document-deck-only = (.*)$/m.exec(ftl("fr", "session"))?.[1] ?? "";
    expect(refusal).not.toBe("");
    const m = await mount(async () => {
      throw new Error(refusal);
    });
    await act(async () => m.hook().onAttach([counted("plan.pdf", 4).f]));
    await settle();
    expect(m.said).toEqual(["Les documents ne se joignent que dans un onglet de deck."]);
  });

  test("useAttachments document past the draft count refused", async () => {
    const sent: string[] = [];
    const m = await mount(async (name) => void sent.push(name));
    const doc = (id: string): Attachment => ({ type: "document", id, name: `${id}.pdf`, kind: "pdf", bytes: 4 });
    await m.render([doc("sha256:a1")]);
    // Two files of different bytes: the second would be the third document.
    const a = { name: "b.pdf", size: 1, type: "application/pdf", arrayBuffer: () => Promise.resolve(new Uint8Array([1]).buffer) } as unknown as File;
    const b = { name: "c.pdf", size: 1, type: "application/pdf", arrayBuffer: () => Promise.resolve(new Uint8Array([2]).buffer) } as unknown as File;
    await act(async () => m.hook().onAttach([a, b]));
    await settle();
    expect([sent, m.said]).toEqual([["b.pdf"], ["Le brouillon a déjà autant de documents qu'il peut en contenir (2)."]]);
  });

  const doc = (id: string, bytes = 4): Attachment => ({ type: "document", id, name: `${id}.pdf`, kind: "pdf", bytes });

  test("useAttachments document past the draft count of other sizes refused before reading", async () => {
    const sent: string[] = [];
    const m = await mount(async (name) => void sent.push(name));
    await m.render([doc("sha256:a1"), doc("sha256:a2")]);
    // No held document has its size: it cannot be one of them attached again.
    const { f, reads } = counted("trois.pdf", 9);
    await act(async () => m.hook().onAttach([f]));
    await settle();
    expect([reads.n, sent, m.said]).toEqual([0, [], ["Le brouillon a déjà autant de documents qu'il peut en contenir (2)."]]);
  });

  test("useAttachments without crypto.subtle leaves the draft count to the host", async () => {
    const sent: string[] = [];
    const m = await mount(async (name) => void sent.push(name));
    await m.render([doc("sha256:a1", 4), doc("sha256:a2", 4)]);
    const real = Object.getOwnPropertyDescriptor(globalThis, "crypto");
    Object.defineProperty(globalThis, "crypto", { value: {}, configurable: true });
    try {
      // Same size as a held one: only its digest could tell, and there is none.
      await act(async () => m.hook().onAttach([counted("autre.pdf", 4).f]));
      await settle();
    } finally {
      if (real) Object.defineProperty(globalThis, "crypto", real);
    }
    expect([sent, m.said]).toEqual([["autre.pdf"], []]);
  });

  test("useAttachments without host limits refuses and reads nothing", async () => {
    const sent: string[] = [];
    const m = await mount(async (name) => void sent.push(name), null);
    const { f, reads } = counted("plan.pdf", 4);
    await act(async () => m.hook().onAttach([f]));
    await settle();
    expect([reads.n, sent, m.said.length]).toEqual([0, [], 1]);
  });

  test("useAttachments long name bounded to 255 characters keeping its extension", async () => {
    const names: string[] = [];
    const m = await mount(async (name) => void names.push(name));
    const long = `${"é".repeat(300)}.docx`;
    await act(async () => m.hook().onAttach([counted(long, 4).f]));
    await settle();
    expect([[...names[0]!].length, names[0]!.endsWith("é.docx")]).toEqual([255, true]);
  });
});
