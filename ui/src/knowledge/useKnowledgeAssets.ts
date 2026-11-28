// A knowledge page's cited images, fetched from the host and shown as `blob:` URLs only.
//
// The bytes are untrusted: they reach the DOM only as a `blob:` `<img>`, typed with one of the
// four image types the host sniffed, never another sink. Each effect run builds its own map and
// revokes the URLs it made on cleanup, so a StrictMode remount or a page switch starts afresh; a
// reply after cleanup is dropped before any URL is made. At most `MAX_IN_FLIGHT` fetches run at
// once, and the results land together: a page load renders the body again once, not per image.

import { useCallback, useEffect, useMemo, useState } from "react";
import type { TabApi } from "../api";
import type { KnowledgePage } from "./types";

/** The image types the host serves; anything else is shown as alt text. */
const IMAGE_TYPES = new Set(["image/png", "image/jpeg", "image/gif", "image/webp"]);

/** The asset fetches running at once: each one holds the deck session while it encodes. */
const MAX_IN_FLIGHT = 4;

/** The URL of each asset of one page (`null` once refused or undecodable), tagged with its effect run's key. */
interface Assets {
  key: string;
  urls: ReadonlyMap<string, string | null>;
}

/** The base64 `data` as bytes; throws on anything that is not base64. */
function decode(data: string): Uint8Array<ArrayBuffer> {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

/**
 * The `blob:` URL of the image at each resolved bundle path `page` cites, `null` while they load,
 * when refused or for an uncited path. An awaiting page, or `null` (the deck browses its history),
 * loads nothing. The URLs are kept across re-reads of the same page with the same images: asset
 * ids are content hashes and approvals only grow, so their bytes never change.
 */
export function useKnowledgeAssets(api: TabApi, page: KnowledgePage | null): (path: string) => string | null {
  const shown = page === null || page.awaiting ? null : page;
  const id = shown?.id ?? null;
  // By value: a re-read of the page with the same images keeps the lookup (and its identity).
  const pairs = shown === null ? "[]" : JSON.stringify(shown.images.map((i) => [i.path, i.asset]));
  const byPath = useMemo(() => new Map<string, string>(JSON.parse(pairs) as [string, string][]), [pairs]);
  const assets = useMemo(() => [...new Set(byPath.values())].sort(), [byPath]);
  const key = id === null ? "" : JSON.stringify([id, assets]);
  const [loaded, setLoaded] = useState<Assets>({ key: "", urls: new Map() });

  useEffect(() => {
    if (id === null || assets.length === 0) return;
    let live = true;
    const made: string[] = [];
    const urls = new Map<string, string | null>();
    const fetchOne = async (asset: string) => {
      let url: string | null = null;
      try {
        const reply = await api.knowledgeAsset(id, asset);
        if (!live) return;
        if (IMAGE_TYPES.has(reply.mime)) {
          url = URL.createObjectURL(new Blob([decode(reply.data)], { type: reply.mime }));
          made.push(url);
        }
      } catch {
        // A refused asset (SVG, too large, no longer cited) or bytes that are not base64 show the
        // alt text.
      }
      urls.set(asset, url);
    };
    const queue = [...assets];
    const worker = async () => {
      for (let asset = queue.shift(); asset !== undefined && live; asset = queue.shift()) await fetchOne(asset);
    };
    void Promise.all(Array.from({ length: Math.min(MAX_IN_FLIGHT, assets.length) }, worker)).then(() => {
      if (live) setLoaded({ key, urls });
    });
    return () => {
      live = false;
      for (const url of made) URL.revokeObjectURL(url);
    };
    // `key` covers `id` and `assets`.
  }, [api, key]);

  return useCallback(
    (path: string) => {
      if (loaded.key !== key) return null;
      const asset = byPath.get(path);
      return (asset === undefined ? null : loaded.urls.get(asset)) ?? null;
    },
    [byPath, key, loaded],
  );
}
