// Un menu d'actions derrière un bouton burger : ce qui sert rarement se fait
// oublier. Fermé par Échap, par un clic ailleurs ou après un choix.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { keyLabel } from "./keys";

/** Une ligne du menu : une action, un séparateur, ou un contrôle (sélecteur de thème…). */
export type MenuEntry =
  /** `shortcut` : écrit à la macOS (`⌘⇧S`), montré dans la notation de la plateforme. */
  | { kind: "item"; label: string; shortcut?: string; disabled?: boolean; onSelect: () => void }
  | { kind: "separator" }
  | { kind: "custom"; node: ReactNode };

const BURGER = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden>
    <line x1="5" y1="7" x2="19" y2="7" /><line x1="5" y1="12" x2="19" y2="12" /><line x1="5" y1="17" x2="19" y2="17" />
  </svg>
);

export interface ActionMenuProps {
  label: string;
  entries: MenuEntry[];
  /** Pictogramme du bouton ; défaut : le burger. */
  icon?: ReactNode;
  /** Classes du bouton ; défaut : `object token`. */
  triggerClass?: string;
  disabled?: boolean;
}

/** Un bouton (burger par défaut) et son menu. */
export function ActionMenu({ label, entries, icon = BURGER, triggerClass = "object token", disabled }: ActionMenuProps) {
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
        className={`${triggerClass} action-menu-trigger`}
        aria-label={label}
        title={label}
        disabled={disabled}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        {icon}
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
                {e.shortcut && <kbd>{keyLabel(e.shortcut)}</kbd>}
              </button>
            ),
          )}
        </div>
      )}
    </div>
  );
}
