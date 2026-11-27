// The files attached in a view: each goes to the host, and an image's thumbnail stays local to
// this view (a blob URL of the user's file), revoked when the image leaves the draft.

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { errorText } from "../hostCall";
import { t } from "../i18n";
import type { AttachLimits, Attachment } from "./attachments";
import { MIB, assetId, boundedName, isImageFile, toBase64 } from "./attachments";

/** What the view shows of its attachments, and how it receives them. */
export interface Attachments {
  /** Thumbnails of the images attached in this view, by id. */
  thumbs: ReadonlyMap<string, string>;
  /** Sends the attached files to the host, one at a time. */
  onAttach: (files: File[]) => void;
}

/** An attachment held in the draft or sent by this batch, as the count checks see it. */
interface Held {
  id: string;
  image: boolean;
  /** Its size in bytes; `null` for an image of the draft, whose size the draft does not carry. */
  bytes: number | null;
}

const heldOf = (a: Attachment): Held =>
  a.type === "image" ? { id: a.id, image: true, bytes: null } : { id: a.id, image: false, bytes: a.bytes };

/** Why `f` is refused before it is read, by its size; `null`: it may be read. */
function sizeRefusal(f: File, limits: AttachLimits): string | null {
  const name = boundedName(f.name);
  if (f.size === 0) return t("ui-attach-empty", { name });
  const max = isImageFile(f) ? limits.image_bytes : limits.document_bytes;
  return f.size > max ? t("ui-attach-too-large", { name, mb: Math.round((max / MIB) * 10) / 10 }) : null;
}

/**
 * Sends attached files to the host (with their name) and keeps local image thumbnails.
 *
 * `limits`: the host's bounds (`Snapshot.attach_limits`); `null` while unknown, when the files
 * are refused with one message. More files at once than the draft can hold are refused together,
 * with one message and none read. A file past its size bound, empty, or past the draft's count is
 * refused before reaching the host, which still checks each one.
 */
export function useAttachments(
  attachments: Attachment[],
  attach: (name: string, data: string) => Promise<void>,
  say: (m: string) => void,
  limits: AttachLimits | null,
): Attachments {
  const [thumbs, setThumbs] = useState<ReadonlyMap<string, string>>(new Map());
  const owned = useRef(new Map<string, string>());
  const send = useRef(attach);
  send.current = attach;
  const report = useRef(say);
  report.current = say;
  const bounds = useRef(limits);
  bounds.current = limits;
  const live = useRef(attachments);
  live.current = attachments;
  // Every id the draft has shown, so a reply can tell an image already gone from one not shown yet.
  // Filled at commit, before paint: a reply landing after commit sees the shown ids.
  const seen = useRef(new Set<string>());
  useLayoutEffect(() => {
    for (const a of attachments) seen.current.add(a.id);
  }, [attachments]);
  const mounted = useRef(false);
  const queue = useRef(Promise.resolve());
  // An attachment removed from the draft (here or elsewhere): its thumbnail is released.
  useEffect(() => {
    const ids = new Set(attachments.map((i) => i.id));
    let changed = false;
    for (const [id, url] of owned.current) {
      if (!ids.has(id)) {
        URL.revokeObjectURL(url);
        owned.current.delete(id);
        changed = true;
      }
    }
    if (changed) setThumbs(new Map(owned.current));
  }, [attachments]);
  // StrictMode unmounts then remounts: everything is released, the view starts without thumbnails.
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      for (const url of owned.current.values()) URL.revokeObjectURL(url);
      owned.current.clear();
    };
  }, []);
  const onAttach = useCallback((files: File[]) => {
    // Fixed at attach time: switching tabs meanwhile does not send the file there.
    const attach = send.current;
    const limits = bounds.current;
    if (!limits) return report.current(t("ui-attach-not-ready"));
    // A batch the draft cannot hold is refused whole: one message rather than one per file.
    const most = limits.draft_images + limits.draft_documents;
    if (files.length > most) return report.current(t("ui-attach-too-many-files", { max: most }));
    // What this batch already sent: the draft's event may not have shown it yet.
    const sent: Held[] = [];
    for (const f of files) {
      // A failing file is reported and the chain goes on: the next file is still sent.
      queue.current = queue.current
        .then(async () => {
          const refusal = sizeRefusal(f, limits);
          if (refusal) throw new Error(refusal);
          const image = isImageFile(f);
          const max = image ? limits.draft_images : limits.draft_documents;
          const full = () => new Error(t(image ? "ui-attach-too-many-images" : "ui-attach-too-many-documents", { max }));
          const held = [...live.current.map(heldOf), ...sent].filter((h) => h.image === image);
          // Attaching again an item already in the draft does not grow it (the host allows it),
          // and it has the same size: with none of that size held, the draft is full for it.
          // An image's size is not in the draft: any held image may be it.
          const unique = new Set(held.map((h) => h.id));
          if (unique.size >= max && !held.some((h) => h.bytes === null || h.bytes === f.size)) throw full();
          const buf = await f.arrayBuffer();
          const id = await assetId(buf);
          // Without `crypto.subtle` there is no id to compare: the host alone enforces the count.
          if (id && unique.size >= max && !unique.has(id)) throw full();
          // Seen from here on means shown in reply to this attach.
          if (id) seen.current.delete(id);
          await attach(boundedName(f.name), toBase64(buf));
          if (id) sent.push({ id, image, bytes: f.size });
          // View unmounted meanwhile: a thumbnail made here would never be released. An image the
          // draft showed and dropped before the reply: no draft change would release it.
          const gone = id !== null && seen.current.has(id) && !live.current.some((a) => a.id === id);
          if (image && id && !gone && mounted.current && !owned.current.has(id)) {
            owned.current.set(id, URL.createObjectURL(f));
            setThumbs(new Map(owned.current));
          }
        })
        .catch((e) => report.current(errorText(e)));
    }
  }, []);
  return { thumbs, onAttach };
}
