// Trace clicks: show the slide, select the element, open its place.

import { useRef } from "react";
import type { SlideView, Snapshot, Surface } from "./api";
import { findElement } from "./deck";
import { t } from "./i18n";
import type { QuickAction } from "./agent/quick";
import { isReceipt } from "./agent/surfaces";
import { traceTarget, type TraceTarget } from "./agent/trace";
import { useAgentFocus } from "./agent/useAgentFocus";

/** What a trace click reads and changes in its tab. */
export interface TraceInput {
  snap: Snapshot | null;
  view: SlideView | null;
  current: string | null;
  editing: string | null;
  show: (slide: string) => void;
  select: (element: string | null) => void;
  /** Opens the data pane once a data trace's element is selected. */
  onPane?: () => void;
  /** Opens the inspector once any other trace's element is selected. */
  onInspector?: () => void;
  say: (text: string, error?: boolean) => void;
}

/** A receipt action handler (`true`: a trace, handled); `reveal` shows a target sent by the agent window. */
export type TraceHandler = ((s: Surface, a: QuickAction) => boolean) & { reveal: (target: TraceTarget) => void };

/**
 * Handles the `trace` actions of receipts only. A trace to a slide or element gone from the deck
 * says `ui-trace-gone` and changes nothing; otherwise its slide shows (deferred while a text is
 * edited, as an agent focus), its element is selected, then `onPane` or `onInspector` is called.
 */
export function useTrace({ snap, view, current, editing, show, select, onPane, onInspector, say }: TraceInput): TraceHandler {
  /** The trace whose element is awaited. */
  const pending = useRef<TraceTarget | null>(null);
  const latest = useRef({ onPane, onInspector, say });
  latest.current = { onPane, onInspector, say };
  const gone = () => {
    pending.current = null;
    latest.current.say(t("ui-trace-gone"), true);
  };
  const focus = useAgentFocus({
    slides: snap?.slides,
    view,
    current,
    editing,
    show,
    select: (element) => {
      select(element);
      const p = pending.current;
      if (!p || element !== p.element) return;
      pending.current = null;
      (p.pane ? latest.current.onPane : latest.current.onInspector)?.();
    },
    onMissing: gone,
  });
  const reveal = (target: TraceTarget) => {
    // The shown slide's view is current: an element it lacks is gone, no newer view is awaited.
    if (current === target.slide && view?.slide.id === target.slide && !findElement(view.slide, target.element)) {
      return gone();
    }
    pending.current = target;
    focus(target.slide, target.element);
  };
  const onAction = (s: Surface, a: QuickAction) => {
    if (!isReceipt(s)) return false;
    const target = traceTarget(a);
    if (!target) return false;
    reveal(target);
    return true;
  };
  return Object.assign(onAction, { reveal });
}
