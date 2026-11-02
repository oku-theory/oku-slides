// The receipt's traces: the element a click shows.

import type { QuickAction } from "./quick";

/** The action name of a receipt's trace button (Rust `TRACE_EVENT`). */
export const TRACE_EVENT = "trace";

/** The element a trace names, and whether its click opens the data pane. */
export interface TraceTarget {
  slide: string;
  element: string;
  pane: boolean;
}

const id = (v: unknown): v is string => typeof v === "string" && v !== "";

/** The target of a `trace` action, or `null` for any other action or a malformed context. */
export function traceTarget(a: QuickAction): TraceTarget | null {
  if (a.name !== TRACE_EVENT) return null;
  const { slide, element, pane } = a.context;
  if (!id(slide) || !id(element) || typeof pane !== "boolean") return null;
  return { slide, element, pane };
}
