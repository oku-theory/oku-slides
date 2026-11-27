import { afterAll, beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { applyMessages } from "../i18n";
import { resetMessages } from "../test/messages";
import { assetId, attachment, boundedName, clipboardFiles, documentSize, imageLabel, isImageFile, toBase64 } from "./attachments";

const ftl = (l: string) => readFileSync(`${import.meta.dir}/../../../crates/oku-slides-i18n/src/locales/${l}/ui.ftl`, "utf8");
beforeAll(() => applyMessages({ locale: "en", resources: [ftl("en")], chosen: null }));
afterAll(resetMessages);

test("imageLabel gives the short type and the size", () => {
  expect(imageLabel("image/png", 1200, 800)).toBe("\u2068PNG\u2069 \u20681200×800\u2069");
});

test("imageLabel of jpeg says JPEG", () => {
  expect(imageLabel("image/jpeg", 2, 1)).toBe("\u2068JPEG\u2069 \u20682×1\u2069");
});

const file = (type: string) => new File([new Uint8Array([1, 2, 3])], "x", { type });
const clip = (text: string, files: File[]) =>
  ({ getData: (k: string) => (k === "text/plain" ? text : ""), files }) as unknown as DataTransfer;

test("clipboardFiles takes every file of a clipboard without text", () => {
  expect(clipboardFiles(clip("", [file("image/png"), file("application/pdf")])).map((f) => f.type)).toEqual(["image/png", "application/pdf"]);
});

test("clipboardFiles with text keeps the text and takes no file", () => {
  expect(clipboardFiles(clip("un paragraphe", [file("image/png")]))).toEqual([]);
});

test("clipboardFiles finder copy of two files takes them", () => {
  const a = new File(["a"], "a b.pdf");
  const b = new File(["b"], "notes.md");
  expect(clipboardFiles(clip("a b.pdf\nfile:///Users/x/notes.md", [a, b])).map((f) => f.name)).toEqual(["a b.pdf", "notes.md"]);
});

test("clipboardFiles text naming another file keeps the text", () => {
  expect(clipboardFiles(clip("autre.pdf", [new File(["a"], "a.pdf")]))).toEqual([]);
});

test("clipboardFiles falls back to the clipboard items when files is empty", () => {
  const item = (f: File | null, kind = "file") => ({ kind, type: f?.type ?? "", getAsFile: () => f });
  const dt = {
    getData: () => "",
    files: [],
    items: [item(file("image/png")), item(null), { kind: "string", type: "image/png", getAsFile: () => file("image/png") }],
  } as unknown as DataTransfer;
  expect(clipboardFiles(dt).map((f) => f.type)).toEqual(["image/png"]);
});

test("boundedName short name unchanged", () => {
  expect(boundedName("plan.pdf")).toBe("plan.pdf");
});

test("boundedName long name cut to 255 characters keeps its extension", () => {
  const cut = boundedName(`${"😀".repeat(300)}.pptx`);
  expect([[...cut].length, cut.endsWith("😀.pptx")]).toEqual([255, true]);
});

test("toBase64 encodes the bytes", () => {
  expect(toBase64(new Uint8Array([104, 105]).buffer)).toBe("aGk=");
});

test("toBase64 several slices encodes the bytes", () => {
  const bytes = Uint8Array.from({ length: 0x8000 * 2 + 5 }, (_, i) => (i * 7) % 256);
  expect(toBase64(bytes.buffer)).toBe(Buffer.from(bytes).toString("base64"));
});

test("documentSize under a megabyte in kilobytes, rounded up", () => {
  expect([documentSize(1), documentSize(2048)]).toEqual(["1 KB", "2 KB"]);
});

test("documentSize from a megabyte in megabytes, one decimal", () => {
  expect([documentSize(2 * 1024 * 1024), documentSize(1.25 * 1024 * 1024)]).toEqual(["2 MB", "1.3 MB"]);
});

test("assetId is the sha256 asset id the host computes", async () => {
  expect(await assetId(new TextEncoder().encode("abc").buffer as ArrayBuffer)).toBe(
    "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  );
});

test("attachments parse document variant", () => {
  const doc = { type: "document", id: "sha256:dd", name: "notes.md", kind: "markdown", bytes: 2048 };
  expect(attachment.parse(doc)).toEqual(doc);
});

test("attachments document of a kind the UI does not know parses", () => {
  const doc = { type: "document", id: "sha256:dd", name: "notes.epub", kind: "epub", bytes: 2048 };
  expect(attachment.parse(doc)).toEqual(doc);
});

test("attachments document with an overlong name, id or kind refused", () => {
  const doc = { type: "document", id: "sha256:dd", name: "notes.md", kind: "markdown", bytes: 2048 };
  expect([
    attachment.safeParse({ ...doc, name: "x".repeat(1021) }).success,
    attachment.safeParse({ ...doc, id: `sha256:${"a".repeat(200)}` }).success,
    attachment.safeParse({ ...doc, kind: "k".repeat(65) }).success,
    attachment.safeParse({ ...doc, kind: "" }).success,
  ]).toEqual([false, false, false, false]);
});

test("attachments image with an overlong id refused", () => {
  expect(attachment.safeParse({ type: "image", id: `sha256:${"a".repeat(200)}`, mime: "image/png", width: 1, height: 1 }).success).toBe(false);
});

test("attachments reject unknown type", () => {
  expect(attachment.safeParse({ type: "video", id: "sha256:aa", mime: "video/mp4", width: 1, height: 1 }).success).toBe(false);
});

test("isImageFile empty type with a png name counts as an image", () => {
  expect(isImageFile({ type: "", name: "capture.PNG" })).toBe(true);
});

test("isImageFile image type counts as an image", () => {
  expect(isImageFile({ type: "image/webp", name: "x" })).toBe(true);
});

test("isImageFile extension the host does not attach as an image is a document", () => {
  expect([isImageFile({ type: "", name: "x.heic" }), isImageFile({ type: "application/pdf", name: "x.pdf" })]).toEqual([false, false]);
});
