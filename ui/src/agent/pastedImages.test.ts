import { afterAll, beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { applyMessages } from "../i18n";
import { resetMessages } from "../test/messages";
import { assetId, clipboardImages, imageLabel, toBase64 } from "./pastedImages";

const ftl = (l: string) => readFileSync(`${import.meta.dir}/../../../crates/oku-slides-i18n/src/locales/${l}/ui.ftl`, "utf8");
beforeAll(() => applyMessages({ locale: "en", resources: [ftl("en")], chosen: null }));
afterAll(resetMessages);

test("imageLabel gives the short type and the size", () => {
  expect(imageLabel("image/png", 1200, 800)).toBe("PNG 1200×800");
});

test("imageLabel of jpeg says JPEG", () => {
  expect(imageLabel("image/jpeg", 2, 1)).toBe("JPEG 2×1");
});

const file = (type: string) => new File([new Uint8Array([1, 2, 3])], "x", { type });
const clip = (text: string, files: File[]) =>
  ({ getData: (k: string) => (k === "text/plain" ? text : ""), files }) as unknown as DataTransfer;

test("clipboardImages takes the image files of a clipboard without text", () => {
  expect(clipboardImages(clip("", [file("image/png"), file("application/pdf")])).map((f) => f.type)).toEqual(["image/png"]);
});

test("clipboardImages with text keeps the text and takes no image", () => {
  expect(clipboardImages(clip("un paragraphe", [file("image/png")]))).toEqual([]);
});

test("clipboardImages falls back to the clipboard items when files holds no image", () => {
  const item = (f: File | null, kind = "file") => ({ kind, type: f?.type ?? "", getAsFile: () => f });
  const dt = {
    getData: () => "",
    files: [],
    items: [item(file("image/png")), item(null), { kind: "string", type: "image/png", getAsFile: () => file("image/png") }],
  } as unknown as DataTransfer;
  expect(clipboardImages(dt).map((f) => f.type)).toEqual(["image/png"]);
});

test("toBase64 encodes the bytes", () => {
  expect(toBase64(new Uint8Array([104, 105]).buffer)).toBe("aGk=");
});

test("assetId is the sha256 asset id the host computes", async () => {
  expect(await assetId(new TextEncoder().encode("abc").buffer as ArrayBuffer)).toBe(
    "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  );
});
