// An agent box as a non-modal popover next to its opener.

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Surface } from "../api";
import { t } from "../i18n";
import { FOCUSABLE, modalOpen } from "../modalFrame";
import { popoverPlace } from "../popoverPlace";
import { firstText } from "./a2uiModel";
import { AgentSurfaceBox } from "./AgentSurfaceBox";

/** What a popover receives: its surface, its opener's anchor name and how to close it. */
export interface AgentPopoverProps {
  surface: Surface;
  /** The `data-popover-anchor` value of the element it opens next to. */
  anchor: string;
  /** The anchor used when `anchor` is not in the page (the box's own button). */
  fallback?: string;
  /** The surface has the quick keys (the theme picker): its 1–4 show. */
  hints?: boolean;
  /** Its clicks, raw: the panel gives the host the first look (a packs row opens a tab). */
  onAction: (surface: Surface, action: A2uiClientAction) => void;
  onError: (message: string) => void;
  /** Escape, an outside click or the anchor's second click: the panel dismisses it on the hub. */
  onClose: () => void;
  /** Whether it shows at an anchor in this window, on each change and `false` when it goes. */
  onPlaced?: (shown: boolean) => void;
}

/** The element carrying `data-popover-anchor="name"`, compared by value: any name is safe. */
function anchorEl(name: string): HTMLElement | null {
  const all = document.querySelectorAll<HTMLElement>("[data-popover-anchor]");
  return [...all].find((el) => el.getAttribute("data-popover-anchor") === name) ?? null;
}

/**
 * The box next to its anchor, inside the viewport (flipped above near the bottom).
 *
 * The anchor is `anchor`, else `fallback`; with neither in the page it renders nothing (another
 * window's box). An anchor that leaves while open is replaced by the other one, else the box
 * closes. It takes the focus on open and gives it back to the anchor on close. Escape, with the
 * focus in the box or on its anchor and no modal open, closes it and goes no further; a
 * pointerdown outside it and its anchor closes it.
 */
export function AgentPopover({ surface, anchor, fallback, hints = false, onAction, onError, onClose, onPlaced }: AgentPopoverProps) {
  const frame = useRef<HTMLDivElement>(null);
  const [opener, setOpener] = useState<HTMLElement | null>(null);
  const [place, setPlace] = useState<{ left: number; top: number } | null>(null);
  const close = useRef(onClose);
  close.current = onClose;
  const resolve = () => anchorEl(anchor) ?? (fallback === undefined ? null : anchorEl(fallback));
  const find = useRef(resolve);
  find.current = resolve;
  // The anchor, found again when it changes; the box shows only once it is there.
  useLayoutEffect(() => setOpener(find.current()), [anchor, fallback]);
  const placed = useRef(onPlaced);
  placed.current = onPlaced;
  const shown = opener !== null;
  useLayoutEffect(() => {
    placed.current?.(shown);
    return () => placed.current?.(false);
  }, [shown]);
  // An anchor that leaves the page (its panel closed): the other one, else the box closes.
  useEffect(() => {
    if (!opener) return;
    const check = () => {
      if (opener.isConnected) return;
      const next = find.current();
      if (next) setOpener(next);
      else close.current();
    };
    const watch = new MutationObserver(check);
    watch.observe(document.body, { childList: true, subtree: true });
    return () => watch.disconnect();
  }, [opener]);
  // Placed next to the anchor, again on resize, scroll and once its content has grown.
  useLayoutEffect(() => {
    if (!opener) return;
    const update = () => {
      const box = frame.current;
      if (!box) return;
      const viewport = { left: 0, top: 0, width: window.innerWidth, height: window.innerHeight } as DOMRect;
      setPlace(popoverPlace(viewport, opener.getBoundingClientRect(), box.offsetWidth, box.offsetHeight));
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    const grows = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
    if (frame.current) grows?.observe(frame.current);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
      grows?.disconnect();
    };
  }, [opener]);
  // Focus in on open (its first control, else the box), back to the anchor on close.
  useLayoutEffect(() => {
    const box = frame.current;
    if (!opener || !box) return;
    const first = box.querySelector<HTMLElement>(FOCUSABLE) ?? box.querySelector<HTMLElement>("[data-surface-id]");
    first?.focus();
    return () => {
      const at = document.activeElement;
      if (!at || at === document.body || box.contains(at)) opener.focus();
    };
  }, [opener]);
  useEffect(() => {
    if (!opener) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented || e.isComposing || modalOpen()) return;
      const at = document.activeElement;
      if (!(at instanceof Node) || !(frame.current?.contains(at) || opener.contains(at))) return;
      e.preventDefault();
      e.stopPropagation();
      close.current();
    };
    const onPointer = (e: PointerEvent) => {
      const at = e.target;
      if (at instanceof Node && (frame.current?.contains(at) || opener.contains(at))) return;
      close.current();
    };
    window.addEventListener("keydown", onKey, true);
    document.addEventListener("pointerdown", onPointer, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      document.removeEventListener("pointerdown", onPointer, true);
    };
  }, [opener]);
  if (!opener) return null;
  return (
    <div
      ref={frame}
      className="agent-popover"
      style={place ? { left: place.left, top: place.top } : { opacity: 0 }}
    >
      <AgentSurfaceBox
        surface={surface}
        hints={hints}
        onAction={onAction}
        onError={onError}
        className="agent-popover-box"
        role="dialog"
        label={firstText(surface) || t("ui-agent-thread")}
      />
    </div>
  );
}
