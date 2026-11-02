// Native `change` of a field whose browser may fire it on every move.
//
// WKWebView (macOS) fires `change` on each move in NSColorPanel, not only when
// the panel closes (t-213): the value is sent once the field has been quiet for
// SETTLE_MS, when it loses focus (if it had it), or when it unmounts. Holding
// the mouse still mid-drag for longer than SETTLE_MS sends an intermediate value.

import { useLayoutEffect, useRef, type RefObject } from "react";

/** Quiet time, in ms, after the last `change` before its value is sent. */
export const SETTLE_MS = 500;

/** A settling field: its ref, and `cancel` to drop a value not yet sent. */
export interface SettledChange {
  ref: RefObject<HTMLInputElement | null>;
  cancel: () => void;
}

/** Sends a field's last `change` value once it settles. */
export function useSettledChange(onSettle: (value: string) => void): SettledChange {
  const ref = useRef<HTMLInputElement>(null);
  const handler = useRef(onSettle);
  handler.current = onSettle;
  const pending = useRef<{ value: string; timer: ReturnType<typeof setTimeout> } | null>(null);
  const cancel = useRef(() => {
    if (pending.current) clearTimeout(pending.current.timer);
    pending.current = null;
  }).current;
  // A layout effect: its unmount cleanup runs before SurfaceView disposes the surface
  // in a passive one, so the flushed action still reaches the host.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const flush = () => {
      const p = pending.current;
      cancel();
      if (p) handler.current(p.value);
    };
    const onChange = () => {
      cancel();
      pending.current = { value: el.value, timer: setTimeout(flush, SETTLE_MS) };
    };
    const onBlur = () => flush();
    el.addEventListener("change", onChange);
    el.addEventListener("blur", onBlur);
    return () => {
      el.removeEventListener("change", onChange);
      el.removeEventListener("blur", onBlur);
      flush();
    };
  }, [cancel]);
  return { ref, cancel };
}
