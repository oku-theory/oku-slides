/** Sous-onglets de widgets du mode modèles, pour `App`. */
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { isMac, type Snapshot, type TabApi } from "./api";
import { tabShortcut } from "./tabs";
import type { EditorCommands, EditorHistory } from "./TypstEditor";
import { useWidgetFiles, type WidgetFiles } from "./useWidgetFiles";
import { NO_TABS, activated, activeName, closed, kept, opened, pruned, renamed, withRestored, type KeptTabs, type WidgetTabs } from "./widgetTabs";

export interface TemplateWidgets {
  files: WidgetFiles;
  tabs: WidgetTabs;
  /** La revision des modèles (statuts et sources s'y relisent). */
  revision: string;
  /** Nom du sous-onglet actif ; `null` : « Modèles » ou hors mode modèles. */
  activeName: string | null;
  /** Pour `useViewOptions` : vide hors mode modèles, dès le même rendu. */
  kept: KeptTabs;
  open(name: string): void;
  activate(id: number | null): void;
  close(id: number): void;
  removed(name: string): void;
  undo(): void;
  redo(): void;
  /** Profondeurs d'annulation de l'éditeur actif. */
  history: EditorHistory;
  commandsOf(id: number): (c: EditorCommands | null) => void;
  historyOf(id: number): (h: EditorHistory) => void;
  /** Demande de focus sur la section Widgets : augmente à chaque demande, remis à 0 hors mode modèles (une seule fois). */
  pileFocus: number;
  /** Entre en mode modèles par `enter`, puis ouvre `name` ou met le focus sur la pile. */
  enterAndOpen(enter: () => Promise<boolean>, name: string | null): Promise<void>;
  /** Frappes écrites et file vide, puis `close` (✓). */
  finish(close: () => Promise<unknown>): Promise<void>;
}

interface Callbacks {
  changed(): void;
  error(message: string): void;
  /** Vrai quand une fenêtre, l'historique ou une saisie prend le clavier. */
  blocked(): boolean;
}

const NONE: KeptTabs = { widgets: [], widget: null };
const keptOf = (snap: Snapshot): KeptTabs => ({ widgets: snap.ui.widgets, widget: snap.ui.widget });
const ZERO: EditorHistory = { undo: 0, redo: 0 };

export function useTemplateWidgets(api: TabApi, snap: Snapshot | null, on: Callbacks): TemplateWidgets {
  const pack = snap?.editing && !snap.history ? snap.editing.set : null;
  const revision = `${snap?.templates?.version ?? ""}|${snap?.editing?.base ?? ""}`;
  const [tabs, setTabs] = useState<WidgetTabs>(NO_TABS);
  const onRef = useRef(on);
  useLayoutEffect(() => {
    onRef.current = on;
  });
  const files = useWidgetFiles(api, pack, revision, {
    changed: () => onRef.current.changed(),
    error: (m) => onRef.current.error(m),
    renamed: (from, to) => setTabs((s) => renamed(s, from, to)),
  });

  // Ajustés pendant le rendu : aucun rendu ne montre des sous-onglets d'un autre pack ou élagués trop tard.
  const [seenPack, setSeenPack] = useState(pack);
  /** Pack dont la reprise est faite. */
  const [restoredFor, setRestoredFor] = useState<string | null>(null);
  /**
   * Sous-onglets gardés par l'hôte, lus au premier snapshot du pack : un snapshot
   * plus récent peut déjà porter ce que l'interface a sauvé avant la reprise.
   */
  const [hostTabs, setHostTabs] = useState<KeptTabs | null>(() => (pack === null ? null : keptOf(snap!)));
  /** Ouvert dès l'entrée en mode modèles (`enterAndOpen`). */
  const [pending, setPending] = useState<string | null>(null);
  const [pileFocus, setPileFocus] = useState(0);
  if (pack !== seenPack) {
    setSeenPack(pack);
    // La demande de focus ne vaut que pour l'entrée qui l'a faite : la pile, remontée à la prochaine, ne la reprend pas.
    if (pack === null) setPileFocus(0);
    // `next` continue : un id ne resservira pas à un autre fichier.
    setTabs((s) => ({ ...NO_TABS, next: s.next }));
    setRestoredFor(null);
    setHostTabs(pack === null ? null : keptOf(snap!));
  } else if (pack === null && pending !== null) {
    // `enter` a rendu la main sans mode modèles : le nom n'attend pas une entrée future.
    setPending(null);
  } else if (pack !== null && pending !== null) {
    setTabs((s) => opened(s, pending));
    setPending(null);
  } else if (pack !== null && files.statuses && files.statusesFor === revision) {
    const names = new Set(files.statuses.map((s) => s.name));
    if (restoredFor !== pack) {
      const host = hostTabs ?? NONE;
      setTabs((s) => pruned(withRestored(s, host.widgets, host.widget), names));
      setRestoredFor(pack);
    } else if (pruned(tabs, names) !== tabs) {
      setTabs((s) => pruned(s, names));
    }
  }

  const tabsRef = useRef(tabs);
  const packRef = useRef(pack);
  useLayoutEffect(() => {
    tabsRef.current = tabs;
    packRef.current = pack;
  });

  // Commandes et profondeurs d'annulation par sous-onglet ; rappels stables par id.
  const commands = useRef(new Map<number, EditorCommands>());
  const depths = useRef(new Map<number, EditorHistory>());
  const commandsCbs = useRef(new Map<number, (c: EditorCommands | null) => void>());
  const historyCbs = useRef(new Map<number, (h: EditorHistory) => void>());
  const active = pack === null ? null : tabs.active;
  const activeRef = useRef(active);
  const [shown, setShown] = useState<{ id: number | null; h: EditorHistory }>({ id: null, h: ZERO });
  useLayoutEffect(() => {
    activeRef.current = active;
    const h = active === null ? ZERO : (depths.current.get(active) ?? ZERO);
    setShown((prev) => (prev.id === active && prev.h === h ? prev : { id: active, h }));
  }, [active]);
  useEffect(() => {
    const ids = new Set(tabs.tabs.map((t) => t.id));
    for (const m of [commands, depths, commandsCbs, historyCbs]) {
      for (const id of [...m.current.keys()]) if (!ids.has(id)) m.current.delete(id);
    }
  }, [tabs]);

  const commandsOf = useCallback((id: number) => {
    let cb = commandsCbs.current.get(id);
    if (!cb) {
      cb = (c) => {
        if (c) commands.current.set(id, c);
        else commands.current.delete(id);
      };
      commandsCbs.current.set(id, cb);
    }
    return cb;
  }, []);
  const historyOf = useCallback((id: number) => {
    let cb = historyCbs.current.get(id);
    if (!cb) {
      cb = (h) => {
        depths.current.set(id, h);
        if (activeRef.current !== id) return;
        setShown((prev) => (prev.id === id && prev.h.undo === h.undo && prev.h.redo === h.redo ? prev : { id, h }));
      };
      historyCbs.current.set(id, cb);
    }
    return cb;
  }, []);

  const open = useCallback((name: string) => setTabs((s) => opened(s, name)), []);
  const activate = useCallback((id: number | null) => setTabs((s) => activated(s, id)), []);
  const { flush, settle } = files;
  /** Un sous-onglet vient d'être fermé : le focus perdu va au sous-onglet devenu actif. */
  const refocus = useRef(false);
  const close = useCallback(
    (id: number) => {
      const tab = tabsRef.current.tabs.find((t) => t.id === id);
      if (!tab) return;
      flush(tab.name);
      refocus.current = true;
      setTabs((s) => closed(s, id));
    },
    [flush],
  );
  useEffect(() => {
    if (!refocus.current) return;
    refocus.current = false;
    const now = document.activeElement;
    if (now && now !== document.body && now.isConnected) return;
    document.getElementById(`widget-tab-${tabs.active ?? 0}`)?.focus();
  }, [tabs]);
  const removed = useCallback(
    (name: string) =>
      setTabs((s) => {
        const tab = s.tabs.find((t) => t.name === name);
        return tab ? closed(s, tab.id) : s;
      }),
    [],
  );
  const undo = useCallback(() => {
    if (activeRef.current !== null) commands.current.get(activeRef.current)?.undo();
  }, []);
  const redo = useCallback(() => {
    if (activeRef.current !== null) commands.current.get(activeRef.current)?.redo();
  }, []);

  const enterAndOpen = useCallback(
    async (enter: () => Promise<boolean>, name: string | null) => {
      if (packRef.current === null && !(await enter())) return;
      if (name === null) setPileFocus((n) => n + 1);
      else if (packRef.current !== null) open(name);
      // Le mode modèles n'est pas encore rendu : ouvert au rendu qui l'apporte.
      else setPending(name);
    },
    [open],
  );
  const finish = useCallback(
    async (closeTemplates: () => Promise<unknown>) => {
      // Avant de quitter : un changement de pack abandonnerait la file.
      await settle();
      await closeTemplates();
    },
    [settle],
  );

  // ⌘W et ⌘Z d'un sous-onglet widget, avant le `Workspace` et l'`App` (capture).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const id = tabsRef.current.active;
      if (e.repeat || packRef.current === null || id === null || onRef.current.blocked()) return;
      const target = e.target as HTMLElement | null;
      const inEditor = !!target?.closest?.(".typst-editor");
      const inField = !inEditor && !!target?.closest?.("input, textarea, select");
      if (inField) return;
      const mod = e.metaKey || e.ctrlKey;
      if (tabShortcut(e, isMac)?.kind === "close-tab") close(id);
      else if (mod && !e.altKey && e.key.toLowerCase() === "z" && !inEditor) (e.shiftKey ? redo : undo)();
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [close, undo, redo]);

  return {
    files,
    tabs: pack === null ? NO_TABS : tabs,
    revision,
    activeName: pack === null ? null : activeName(tabs),
    // Avant la reprise, l'hôte garde ses sous-onglets (plus ceux déjà ouverts) : jamais écrasés par du vide.
    kept:
      pack === null
        ? NONE
        : restoredFor === pack
          ? kept(tabs)
          : kept(withRestored(tabs, hostTabs?.widgets ?? [], hostTabs?.widget ?? null)),
    open,
    activate,
    close,
    removed,
    undo,
    redo,
    history: shown.id === active ? shown.h : ZERO,
    commandsOf,
    historyOf,
    pileFocus,
    enterAndOpen,
    finish,
  };
}
