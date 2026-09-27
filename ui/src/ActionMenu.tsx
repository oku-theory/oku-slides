// Un menu d'actions derrière un bouton « ⋯ » : ce qui sert rarement se fait
// oublier. Fermé par Échap, par un clic ailleurs ou après un choix.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

/** Une ligne du menu : une action, un séparateur, ou un contrôle (sélecteur de thème…). */
export type MenuEntry =
  | { kind: "item"; label: string; shortcut?: string; disabled?: boolean; onSelect: () => void }
  | { kind: "separator" }
  | { kind: "custom"; node: ReactNode };

const DOTS = (
  <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden>
    <circle cx="5" cy="12" r="1.6" /><circle cx="12" cy="12" r="1.6" /><circle cx="19" cy="12" r="1.6" />
  </svg>
);

/** Le bouton « ⋯ » et son menu. */
export function ActionMenu({ label, entries }: { label: string; entries: MenuEntry[] }) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);

  const items = () => [...(root.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? [])];
  const close = (refocus: boolean) => {
    setOpen(false);
    if (refocus) trigger.current?.focus();
  };

  useEffect(() => {
    if (!open) return;
    items()[0]?.focus();
    const away = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", away);
    return () => document.removeEventListener("pointerdown", away);
  }, [open]);

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close(true);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const list = items();
      if (list.length === 0) return;
      const at = list.indexOf(document.activeElement as HTMLButtonElement);
      const step = e.key === "ArrowDown" ? 1 : -1;
      list[(at + step + list.length) % list.length].focus();
    }
  };

  return (
    <div className="action-menu" ref={root}>
      <button
        ref={trigger}
        type="button"
        className="object token action-menu-trigger"
        aria-label={label}
        title={label}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        {DOTS}
      </button>
      {open && (
        <div className="action-menu-list object" role="menu" aria-label={label} onKeyDown={onKey}>
          {entries.map((e, i) =>
            e.kind === "separator" ? (
              <hr key={i} className="action-menu-sep" />
            ) : e.kind === "custom" ? (
              <div key={i} className="action-menu-custom">{e.node}</div>
            ) : (
              <button
                key={i}
                type="button"
                role="menuitem"
                disabled={e.disabled}
                onClick={() => {
                  close(false);
                  e.onSelect();
                }}
              >
                <span>{e.label}</span>
                {e.shortcut && <kbd>{e.shortcut}</kbd>}
              </button>
            ),
          )}
        </div>
      )}
    </div>
  );
}
