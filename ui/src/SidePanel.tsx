/**
 * A deck tab's left side panel: a rail of tabs (Packs, Themes, Inspector) and the open tab's body.
 *
 * The packs tab shows the hub's `host-packs` box, the themes tab its `host-theme-picker`, which
 * `AgentPanel` puts in its `side` and `sideThemes` slots; one box at a time wins over the inspector
 * while it is open, and the inspector shows again once it goes. A click on the pressed tab folds
 * the panel to its rail; so does the fold button at the rail's foot, which reopens the last tab.
 */
import type { ReactNode } from "react";
import { useEffect, useRef, useSyncExternalStore } from "react";
import type { AgentApi } from "./agent/AgentPanel";
import { type Slot, useSlot } from "./agent/slot";
import { HOST_PACKS, HOST_THEME_PICKER } from "./agent/surfaces";
import { errorText } from "./hostCall";
import { t } from "./i18n";
import { Icon } from "./icons";
import { IconButton, IconToggle } from "./primitives";

/** What the side panel shows and what its rail triggers. */
export interface SidePanelProps {
  /** The `side` slot of the agent: the deck's packs box while it is open. */
  packs: Slot;
  /** The `sideThemes` slot of the agent: the theme picker while it is open. */
  themes: Slot;
  /** The inspector's body while it is kept open; `null` otherwise. */
  inspector: ReactNode;
  /** A pin is older than the library's version: a dot on the packs tab. */
  updates: boolean;
  /** Opens the packs tab. */
  onPacks: () => void;
  /** Opens the themes tab (the theme picker on the slide shown). */
  onThemes: () => void;
  /** Opens the inspector tab. */
  onInspector: () => void;
  /** Folds the panel to its rail. */
  onFold: () => void;
  /** Closes the open box only, packs or themes (Escape, or its pressed tab over a kept inspector). */
  onCloseBox: () => void;
}

/** A tab of the side panel; its rail toggle has the class `<tab>-toggle`. */
type SideTab = "packs" | "themes" | "inspector";

/**
 * The rail and, when a tab is open, its body beside it. A pressed tab folds the panel, except a box
 * (packs, themes) over a kept inspector, which closes that box only. Focus left in a body that goes
 * returns to the rail.
 */
export function SidePanel({ packs, themes, inspector, updates, onPacks, onThemes, onInspector, onFold, onCloseBox }: SidePanelProps) {
  const packsBody = useSlot(packs);
  const themesBody = useSlot(themes);
  const tab: SideTab | null = packsBody ? "packs" : themesBody ? "themes" : inspector ? "inspector" : null;
  const box = tab === "packs" || tab === "themes";
  // With an update, the accessible name and the tooltip say it too (one Fluent sentence).
  const packsLabel = t(updates ? "ui-side-packs-update" : "ui-side-packs");
  const rail = useRef<HTMLElement>(null);
  const body = useRef<HTMLDivElement>(null);
  /** The body held the focus at the last render: a body that goes gives it to the rail. */
  const focusInBody = useRef(false);
  /** The tab shown last: a body that goes gives the focus to its toggle. */
  const lastTab = useRef<SideTab | null>(tab);
  /** The tab open last: the fold button reopens it once the panel is folded (packs at first). */
  const shown = useRef<SideTab>(tab ?? "packs");
  useEffect(() => {
    if (tab) shown.current = tab;
  }, [tab]);
  const opens: Record<SideTab, () => void> = { packs: onPacks, themes: onThemes, inspector: onInspector };
  useEffect(() => {
    const inside = () => (focusInBody.current = !!body.current?.contains(document.activeElement));
    document.addEventListener("focusin", inside);
    return () => document.removeEventListener("focusin", inside);
  }, []);
  useEffect(() => {
    const was = lastTab.current;
    lastTab.current = tab;
    if (!focusInBody.current || body.current?.contains(document.activeElement)) return;
    focusInBody.current = false;
    if (document.activeElement && document.activeElement !== document.body) return;
    const target = tab ?? was;
    if (target) rail.current?.querySelector<HTMLButtonElement>(`.${target}-toggle`)?.focus();
  }, [tab, packsBody, themesBody]);
  /** A tab's toggle: opens it, or, pressed, closes its box over a kept inspector or folds the panel. */
  const toggle = (own: SideTab, open: () => void) =>
    tab !== own ? open : own !== "inspector" && inspector ? onCloseBox : onFold;
  return (
    <div className={`side-panel${tab ? " open" : ""}`}>
      <nav ref={rail} className="side-rail" aria-label={t("ui-side-panel")}>
        <IconToggle
          className={`token ghost packs-toggle${updates ? " has-update" : ""}`}
          label={packsLabel}
          pressed={tab === "packs"}
          onToggle={toggle("packs", onPacks)}
          icon={Icon.packs}
        />
        <IconToggle
          className="token ghost themes-toggle"
          label={t("ui-themes")}
          pressed={tab === "themes"}
          onToggle={toggle("themes", onThemes)}
          icon={Icon.palette}
        />
        <IconToggle
          className="token ghost inspector-toggle"
          label={t("ui-inspector")}
          pressed={tab === "inspector"}
          onToggle={toggle("inspector", onInspector)}
          icon={Icon.inspector}
        />
        <IconButton
          className="token ghost fold-toggle"
          aria-expanded={!!tab}
          label={t(tab ? "ui-side-fold" : "ui-side-unfold")}
          onClick={tab ? onFold : () => opens[shown.current]()}
          icon={tab ? Icon.panelFold : Icon.panelUnfold}
        />
      </nav>
      {tab && (
        <div
          ref={body}
          className="side-body"
          onKeyDown={(e) => {
            if (!box || e.key !== "Escape" || e.defaultPrevented) return;
            e.preventDefault();
            onCloseBox();
          }}
        >
          {tab === "packs" ? packsBody : tab === "themes" ? themesBody : inspector}
        </div>
      )}
    </div>
  );
}

/** What `useSidePanel` drives: the agent's packs and themes boxes and the kept inspector state. */
export interface SidePanelWiring {
  agent: Pick<AgentApi, "showPacks" | "showThemes" | "surfaceAction">;
  /** The agent's `side` slot, filled while the packs box is open. */
  packs: Slot;
  /** The agent's `sideThemes` slot, filled while the theme picker is open. */
  themes: Slot;
  /** The inspector is kept open (`keptUi.inspector`). */
  inspector: boolean;
  setInspector: (open: boolean) => void;
  say: (message: string, error?: boolean) => void;
}

/**
 * Keeps one side panel tab open at a time: opening a box (packs, themes) folds the inspector once
 * the box shows, a box that opens (from the rail, a prompt or the agent) dismisses the other one on
 * the hub, and opening the inspector (its tab, a trace, ⌘⇧E) dismisses both, once per open box.
 */
export function useSidePanel({ agent, packs, themes, inspector, setInspector, say }: SidePanelWiring) {
  // Booleans only: a box's node is rebuilt on any agent surface change, which must not re-render App.
  const packsOpen = useSyncExternalStore(packs.subscribe, () => packs.get() !== null);
  const themesOpen = useSyncExternalStore(themes.subscribe, () => themes.get() !== null);
  /** The boxes whose dismissal is sent and not answered yet: a second one would find no box. */
  const dismissing = useRef(new Set<string>());
  const dismiss = (id: string, open: boolean) => {
    if (!open || dismissing.current.has(id)) return;
    dismissing.current.add(id);
    void agent
      .surfaceAction(id, "dismiss", {})
      .catch((e) => say(errorText(e), true))
      .finally(() => dismissing.current.delete(id));
  };
  /** The box last asked from the rail, until it shows or fails; folds the inspector on showing. */
  const asked = useRef<"packs" | "themes" | null>(null);
  const closeBoxes = () => {
    // Every caller means "not a box": a box asked and not shown yet must not fold the inspector.
    asked.current = null;
    dismiss(HOST_PACKS, packsOpen);
    dismiss(HOST_THEME_PICKER, themesOpen);
  };
  /** The asked box shows: the inspector it replaces folds then, not before (no fold-and-open flash). */
  const foldFor = (own: "packs" | "themes") => {
    if (asked.current !== own) return;
    asked.current = null;
    setInspector(false);
  };
  /** Asks a box from the rail; the rail asks only for a box that is not open. */
  const ask = (own: "packs" | "themes", show: () => Promise<unknown>) => {
    asked.current = own;
    void show().catch((e) => {
      if (asked.current === own) asked.current = null;
      say(errorText(e), true);
    });
  };
  const openPacks = () => ask("packs", () => agent.showPacks());
  /** Opens the theme picker on `slide`, the slide shown (`null`: none). */
  const openThemes = (slide: string | null) => ask("themes", () => agent.showThemes(slide));
  // Read by the effects below, which run on a change of state, not of these functions.
  const latest = useRef({ dismiss, closeBoxes, foldFor });
  latest.current = { dismiss, closeBoxes, foldFor };
  useEffect(() => {
    if (packsOpen) latest.current.foldFor("packs");
  }, [packsOpen]);
  useEffect(() => {
    if (themesOpen) latest.current.foldFor("themes");
  }, [themesOpen]);
  // The inspector opened by a path that does not close the boxes itself (the ⌘⇧E shortcut).
  const wasOpen = useRef(inspector);
  useEffect(() => {
    if (inspector && !wasOpen.current) latest.current.closeBoxes();
    wasOpen.current = inspector;
  }, [inspector]);
  // Both boxes open: the one that did not just open goes (the packs when both came together), so
  // the rail shows one tab at a time.
  const was = useRef({ packs: packsOpen, themes: themesOpen });
  useEffect(() => {
    const before = was.current;
    was.current = { packs: packsOpen, themes: themesOpen };
    if (!packsOpen || !themesOpen) return;
    const packsJustOver = !before.packs && before.themes;
    latest.current.dismiss(packsJustOver ? HOST_THEME_PICKER : HOST_PACKS, true);
  }, [packsOpen, themesOpen]);
  return { openPacks, openThemes, closeBoxes };
}
