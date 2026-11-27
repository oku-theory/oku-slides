// The agent draft's attachments (images and documents): what the UI reads and shows of them.

import { z } from "zod";
import { t } from "../i18n";

/** The longest file name sent to the host, in characters (the host keeps at most as many). */
const NAME_MAX = 255;

/** An attachment id's longest form: `sha256:` and 64 hex digits, with room to spare. */
const ID_MAX = 128;

/** An attachment id, as the host gives it. */
const attachmentId = z.string().min(1).max(ID_MAX);

/**
 * A draft attachment as the host holds it (`AttachmentRef`, tagged by `type`).
 *
 * A document's `kind` is only shown, so any short string passes: a kind the host adds does not
 * drop the draft event. Its name is the host's, at most {@link NAME_MAX} characters, so at most
 * twice as many UTF-16 units; the bound leaves twice that again.
 */
export const attachment = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("image"),
    id: attachmentId,
    mime: z.string().min(1),
    width: z.number().int().nonnegative(),
    height: z.number().int().nonnegative(),
  }),
  z.object({
    type: z.literal("document"),
    id: attachmentId,
    name: z.string().max(NAME_MAX * 4),
    kind: z.string().min(1).max(64),
    bytes: z.number().int().nonnegative(),
  }),
]);

/** A draft attachment: an image or, in a deck tab, a document. */
export type Attachment = z.infer<typeof attachment>;

/** A draft image attachment. */
type ImageAttachment = Extract<Attachment, { type: "image" }>;

/** True for an image attachment. */
export const isImage = (a: Attachment): a is ImageAttachment => a.type === "image";

/** One mebibyte, the unit of the size bounds. */
export const MIB = 1024 * 1024;

/** A document's size: kilobytes rounded up, from 1024 of them megabytes to one decimal. */
export function documentSize(bytes: number): string {
  const kb = Math.ceil(bytes / 1024);
  return kb < 1024
    ? t("ui-attachment-size", { unit: "kb", size: kb })
    : t("ui-attachment-size", { unit: "mb", size: Math.round((bytes / MIB) * 10) / 10 });
}

/**
 * An image's label: its short type and its size, "PNG 1200×800".
 *
 * The size is one argument, so one isolate: two would swap width and height in a right-to-left
 * interface (t-1380).
 */
export function imageLabel(mime: string, width: number, height: number): string {
  const type = mime.replace(/^image\//, "").toUpperCase();
  return t("ui-pasted-image", { type, size: `${width}×${height}` });
}

/**
 * The attachment bounds the host checks (`Snapshot.attach_limits`, from `[paste]`): the view
 * checks them before reading a file, the host still checks each attach.
 */
export interface AttachLimits {
  image_bytes: number;
  document_bytes: number;
  draft_images: number;
  draft_documents: number;
}

/** The extensions of the images the host attaches (`PASTE_TYPES`: PNG, JPEG, GIF, WebP). */
const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp"];

/**
 * True for a file counted as an image: by its `image/` type, else by its extension.
 *
 * Pre-check only; the host decides by the bytes.
 */
export function isImageFile(f: { type: string; name: string }): boolean {
  if (f.type.startsWith("image/")) return true;
  const dot = f.name.lastIndexOf(".");
  return dot >= 0 && IMAGE_EXTENSIONS.includes(f.name.slice(dot + 1).toLowerCase());
}

/**
 * The files of a paste. Text wins (a copy from Word adds an image of the rendering), except for
 * a Finder file copy, whose text is only the copied files' names or `file:` URLs.
 */
export function clipboardFiles(dt: DataTransfer | null): File[] {
  if (!dt) return [];
  let files = [...dt.files];
  // Some browsers fill only `items` for a copied image.
  if (files.length === 0) {
    for (const item of dt.items ?? []) {
      const file = item.kind === "file" ? item.getAsFile() : null;
      if (file) files.push(file);
    }
  }
  if (files.length === 0) return [];
  const lines = dt.getData("text/plain").split(/\r?\n/).map((l) => l.trim()).filter((l) => l !== "");
  return lines.every((l) => isFileName(l, files)) ? files : [];
}

/** `line` names one of `files`, as such or as the last segment of a `file:` URL. */
function isFileName(line: string, files: File[]): boolean {
  let name = line;
  if (line.startsWith("file:")) {
    try {
      name = decodeURIComponent(new URL(line).pathname.split("/").pop() ?? "");
    } catch {
      return false;
    }
  }
  return files.some((f) => f.name === name);
}

/** `name` cut to {@link NAME_MAX} characters, its extension kept: the host reads a document's kind from it. */
export function boundedName(name: string): string {
  const chars = [...name];
  if (chars.length <= NAME_MAX) return name;
  const dot = name.lastIndexOf(".");
  const ext = dot > 0 ? [...name.slice(dot)] : [];
  // An extension as long as the bound is no extension: the name is cut at the end.
  const keep = ext.length < NAME_MAX ? ext : [];
  return [...chars.slice(0, NAME_MAX - keep.length), ...keep].join("");
}

/**
 * The bytes in base64, in slices: `String.fromCharCode` refuses very long lists, and the slices
 * are joined once rather than concatenated one by one.
 */
export function toBase64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf);
  const slices: string[] = [];
  for (let i = 0; i < bytes.length; i += 0x8000) slices.push(String.fromCharCode(...bytes.subarray(i, i + 0x8000)));
  return btoa(slices.join(""));
}

/** The host's asset id (`sha256:<hex>`); `null` outside a secure context (no `crypto.subtle`). */
export async function assetId(buf: ArrayBuffer): Promise<string | null> {
  if (!globalThis.crypto?.subtle) return null;
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", buf));
  return `sha256:${[...digest].map((b) => b.toString(16).padStart(2, "0")).join("")}`;
}
