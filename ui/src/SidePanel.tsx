/**
 * A deck tab's left side panel: a rail of tabs (Packs, Inspector) and the open tab's body.
 *
 * The packs tab shows the hub's `host-packs` box, which `AgentPanel` puts in its `side` slot; it
 * wins over the inspector while it is open, and the inspector shows again once it goes. A click on
 * the pressed tab folds the panel to its rail.
 */
import type { ReactNode } from "react";
import { useEffect, useRef, useSyncExternalStore } from "react";
import type { AgentApi } from "./agent/AgentPanel";
import { type Slot, useSlot } from "./agent/slot";
import { HOST_PACKS } from "./agent/surfaces";
import { errorText } from "./hostCall";
import { t } from "./i18n";
import { Icon } from "./icons";
import { IconToggle } from "./primitives";

/** What the side panel shows and what its rail triggers. */
export interface SidePanelProps {
  /** The `side` slot of the agent: the deck's packs box while it is open. */
  packs: Slot;
  /** The inspector's body while it is kept open; `null` otherwise. */
  inspector: ReactNode;
  /** A pin is older than the library's version: a dot on the packs tab. */
  updates: boolean;
  /** Opens the packs tab. */
  onPacks: () => void;
  /** Opens the inspector tab. */
  onInspector: () => void;
  /** Folds the panel to its rail. */
  onFold: () => void;
  /** Closes the packs box only (Escape, or its pressed tab over a kept inspector). */
  onClosePacks: () => void;
}

/**
 * The rail and, when a tab is open, its body beside it. A pressed tab folds the panel, except the
 * packs one over a kept inspector, which closes the packs only. Focus left in a body that goes
 * returns to the rail.
 */
export function SidePanel({ packs, inspector, updates, onPacks, onInspector, onFold, onClosePacks }: SidePanelProps) {
  const packsBody = useSlot(packs);
  const tab = packsBody ? "packs" : inspector ? "inspector" : null;
  // With an update, the accessible name and the tooltip say it too (one Fluent sentence).
  const packsLabel = t(updates ? "ui-side-packs-update" : "ui-side-packs");
  const rail = useRef<HTMLElement>(null);
  const body = useRef<HTMLDivElement>(null);
  /** The body held the focus at the last render: a body that goes gives it to the rail. */
  const focusInBody = useRef(false);
  useEffect(() => {
    const inside = () => (focusInBody.current = !!body.current?.contains(document.activeElement));
    document.addEventListener("focusin", inside);
    return () => document.removeEventListener("focusin", inside);
  }, []);
  useEffect(() => {
    if (!focusInBody.current || body.current?.contains(document.activeElement)) return;
    focusInBody.current = false;
    if (document.activeElement && document.activeElement !== document.body) return;
    rail.current?.querySelector<HTMLButtonElement>(tab === "inspector" ? ".inspector-toggle" : ".packs-toggle")?.focus();
  }, [tab, packsBody]);
  return (
    <div className={`side-panel${tab ? " open" : ""}`}>
      <nav ref={rail} className="side-rail" aria-label={t("ui-side-panel")}>
        <IconToggle
          className={`token ghost packs-toggle${updates ? " has-update" : ""}`}
          label={packsLabel}
          pressed={tab === "packs"}
          onToggle={tab !== "packs" ? onPacks : inspector ? onClosePacks : onFold}
          icon={Icon.packs}
        />
        <IconToggle
          className="token ghost inspector-toggle"
          label={t("ui-inspector")}
          pressed={tab === "inspector"}
          onToggle={tab === "inspector" ? onFold : onInspector}
          icon={Icon.inspector}
        />
      </nav>
      {tab && (
        <div
          ref={body}
          className="side-body"
          onKeyDown={(e) => {
            if (tab !== "packs" || e.key !== "Escape" || e.defaultPrevented) return;
            e.preventDefault();
            onClosePacks();
          }}
        >
          {tab === "packs" ? packsBody : inspector}
        </div>
      )}
    </div>
  );
}

/** What `useSidePanel` drives: the agent's packs box and the kept inspector state. */
export interface SidePanelWiring {
  agent: Pick<AgentApi, "showPacks" | "surfaceAction">;
  /** The agent's `side` slot, filled while the packs box is open. */
  packs: Slot;
  /** The inspector is kept open (`keptUi.inspector`). */
  inspector: boolean;
  setInspector: (open: boolean) => void;
  say: (message: string, error?: boolean) => void;
}

/**
 * Keeps one side panel tab open at a time: opening the packs folds the inspector, and opening the
 * inspector (its tab, a trace, ⌘⇧E) dismisses the packs box on the hub, once per open box.
 */
export function useSidePanel({ agent, packs, inspector, setInspector, say }: SidePanelWiring) {
  // A boolean only: the box's node is rebuilt on any agent surface change, which must not re-render App.
  const packsOpen = useSyncExternalStore(packs.subscribe, () => packs.get() !== null);
  /** A dismissal sent and not answered yet: a second one would find no box. */
  const dismissing = useRef(false);
  const closePacks = () => {
    if (!packsOpen || dismissing.current) return;
    dismissing.current = true;
    void agent
      .surfaceAction(HOST_PACKS, "dismiss", {})
      .catch((e) => say(errorText(e), true))
      .finally(() => (dismissing.current = false));
  };
  const openPacks = () => {
    setInspector(false);
    void agent.showPacks().catch((e) => say(errorText(e), true));
  };
  // The inspector opened by a path that does not close the packs itself (the ⌘⇧E shortcut).
  const latestClose = useRef(closePacks);
  latestClose.current = closePacks;
  const wasOpen = useRef(inspector);
  useEffect(() => {
    if (inspector && !wasOpen.current) latestClose.current();
    wasOpen.current = inspector;
  }, [inspector]);
  return { openPacks, closePacks };
}
