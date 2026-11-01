/**
 * A widget pack's thumbnails: each widget alone, as its preview draws it, not its whole slide.
 *
 * Each item's preview (`previewWidget`, full slot) is cropped to what the widget draws
 * (`croppedSvg`) and kept as an object URL under the item's key; a new key asks again, one
 * widget at a time, the last image showing meanwhile. The selected item's own preview can be
 * handed over (`put`) instead of asked twice. The SVG stays in an `<img>`: it never runs.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { TabApi, WidgetPreview } from "./api";
import { croppedSvg } from "./widgetPreview";

/** A widget of the strip and what its thumbnail follows (slide hash, trials, look). */
export interface ThumbItem {
  name: string;
  key: string;
}

/** A thumbnail shown and the key it was made for. */
interface Shown {
  key: string;
  url: string | null;
}

/** The thumbnails of `items` (`null`: not a widget pack, nothing asked). */
export function useWidgetThumbs(api: TabApi, items: ThumbItem[] | null) {
  const shown = useRef(new Map<string, Shown>());
  /** The request in flight; only its own answer is taken. Kept across a StrictMode remount. */
  const busy = useRef<object | null>(null);
  const alive = useRef(true);
  const latest = useRef(items);
  latest.current = items;
  const [, bump] = useState(0);

  useEffect(() => {
    alive.current = true;
    const map = shown.current;
    return () => {
      alive.current = false;
      map.forEach((s) => s.url && URL.revokeObjectURL(s.url));
      map.clear();
    };
  }, []);

  /** Keeps `preview` as `name`'s thumbnail for `key`; without an image, the last one stays. */
  const store = useCallback((name: string, key: string, preview: WidgetPreview | null) => {
    const old = shown.current.get(name);
    let url = old?.url ?? null;
    if (preview?.svg) {
      if (url) URL.revokeObjectURL(url);
      url = URL.createObjectURL(new Blob([croppedSvg(preview.svg, preview.bbox ?? null)], { type: "image/svg+xml" }));
    }
    shown.current.set(name, { key, url });
    bump((n) => n + 1);
  }, []);

  // One widget at a time: the first whose thumbnail is missing or older than its key. No deps on
  // purpose: it runs after every render, and `busy` keeps it to one request.
  useEffect(() => {
    if (!items) return;
    const names = new Set(items.map((i) => i.name));
    for (const [name, s] of shown.current) {
      if (names.has(name)) continue;
      if (s.url) URL.revokeObjectURL(s.url);
      shown.current.delete(name);
    }
    if (busy.current) return;
    const next = items.find((i) => shown.current.get(i.name)?.key !== i.key);
    if (!next) return;
    const ticket = {};
    busy.current = ticket;
    const done = (p: WidgetPreview | null) => {
      if (busy.current !== ticket) return;
      busy.current = null;
      if (!alive.current) return;
      // A fresher image came meanwhile (`put`, under the item's current key): the answer is older.
      const now = latest.current?.find((i) => i.name === next.name)?.key;
      if (now !== next.key && shown.current.get(next.name)?.key === now) return void bump((n) => n + 1);
      store(next.name, next.key, p);
    };
    // A refused preview keeps the last image under the new key: no loop of retries.
    api.previewWidget(next.name, undefined, "full").then(done, () => done(null));
  });

  /** The thumbnail URL of `name`; `null` before its first image. */
  const url = useCallback((name: string) => shown.current.get(name)?.url ?? null, []);
  /** Takes `name`'s preview, rendered elsewhere at the full slot, as its current thumbnail. */
  const put = useCallback(
    (name: string, preview: WidgetPreview) => {
      const key = latest.current?.find((i) => i.name === name)?.key;
      if (key !== undefined && preview.svg) store(name, key, preview);
    },
    [store],
  );
  return { url, put };
}
