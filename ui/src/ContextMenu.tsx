// A context menu at the pointer: rendered in `body`, measured before paint, kept in the window.
//
// Its first enabled entry takes the focus (the menu itself when none is, so Escape still reaches
// it); arrows move it, Escape closes the menu and gives the focus back, and Tab or a pointer down
// elsewhere closes it. No key pressed inside reaches the app behind.

import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { menuAbove, menuBelow } from "./tabs";

/** Where the menu opens and what it holds. */
export interface ContextMenuProps {
  /** The pointer, in window coordinates. */
  x: number;
  y: number;
  /** Opens upward (the tab bar, at the bottom), else downward. */
  above: boolean;
  /** The menu's accessible name; absent when its entries say enough. */
  label?: string;
  onClose(): void;
  /** `role="menuitem"` buttons; each closes the menu itself when picked. */
  children: ReactNode;
}

/** The menu's entries that can take the focus. */
const entries = (root: HTMLElement | null) => [...(root?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? [])];

/** A menu at (`x`, `y`) holding `children`. */
export default function ContextMenu({ x, y, above, label, onClose, children }: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const close = useRef(onClose);
  close.current = onClose;
  // Measured once shown, before it is painted, rather than estimated in advance.
  useLayoutEffect(() => {
    const rect = ref.current?.getBoundingClientRect();
    if (!rect) return;
    const place = above ? menuAbove : menuBelow;
    setAt(place(x, y, rect.width, rect.height, window.innerWidth, window.innerHeight));
  }, [x, y, above]);
  // The focus goes in, and back to where it was when the menu goes, unless a pick moved it on.
  useEffect(() => {
    const node = ref.current;
    const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    (entries(node)[0] ?? node)?.focus();
    const away = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) close.current();
    };
    window.addEventListener("pointerdown", away);
    return () => {
      window.removeEventListener("pointerdown", away);
      const now = document.activeElement;
      if (before?.isConnected && (!now || now === document.body || node?.contains(now))) before.focus();
    };
  }, []);

  const onKeyDown = (e: KeyboardEvent) => {
    e.stopPropagation();
    if (e.key === "Escape") {
      e.preventDefault();
      close.current();
    } else if (e.key === "Tab") {
      // The focus leaves for the next control: the menu goes with it.
      close.current();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const list = entries(ref.current);
      if (list.length === 0) return;
      const from = list.indexOf(document.activeElement as HTMLButtonElement);
      list[(from + (e.key === "ArrowDown" ? 1 : -1) + list.length) % list.length].focus();
    }
  };

  return createPortal(
    <div
      ref={ref}
      className="context-menu object"
      role="menu"
      tabIndex={-1}
      aria-label={label}
      style={{ left: at?.x ?? x, top: at?.y ?? y }}
      onKeyDown={onKeyDown}
    >
      {children}
    </div>,
    document.body,
  );
}
