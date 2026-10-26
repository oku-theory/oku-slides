/** Sous-onglets de widgets du mode pack, pour `App`. */
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { isMac, type Snapshot, type TabApi } from "./api";
import { deckPinsKey } from "./packOps";
import { t } from "./i18n";
import { tabShortcut } from "./tabs";
import type { EditorCommands, EditorHistory } from "./TypstEditor";
import { useWidgetFiles, type WidgetFiles } from "./useWidgetFiles";
import { NO_TABS, THEME_TAB, activated, activeName, activeWidget, closed, kept, opened, placeTouched, pruned, renamed, withRestored, type ActiveTab, type KeptTabs, type WidgetTabs } from "./widgetTabs";

export interface PackWidgetEditor {
  files: WidgetFiles;
  tabs: WidgetTabs;
  /** La revision des packs épinglés et du pack édité (statuts et sources s'y relisent). */
  revision: string;
  /** Nom du sous-onglet widget actif ; `null` : la scène, « Thème » ou hors mode pack. */
  activeName: string | null;
  /** Pour `useKeptUi` : vide hors mode pack, dès le même rendu. */
  kept: KeptTabs;
  open(name: string): void;
  activate(id: ActiveTab): void;
  close(id: number): void;
  removed(name: string): void;
  /** L'agent a écrit (ou supprimé) `name` : l'ouvrir, sans écraser les frappes. */
  touched(name: string, removed: boolean): void;
  undo(): void;
  redo(): void;
  /** Profondeurs d'annulation de l'éditeur actif. */
  history: EditorHistory;
  commandsOf(id: number): (c: EditorCommands | null) => void;
  historyOf(id: number): (h: EditorHistory) => void;
  /** Frappes écrites et file vide, puis `close` (✓). */
  finish(close: () => Promise<unknown>): Promise<void>;
}

interface Callbacks {
  changed(): void;
  error(message: string): void;
  /** Information (pas une erreur) pour l'utilisateur. */
  notice(message: string): void;
  /** Vrai quand une fenêtre, l'historique ou une saisie prend le clavier. */
  blocked(): boolean;
}

const NONE: KeptTabs = { widgets: [], widget: null };
const keptOf = (snap: Snapshot): KeptTabs => ({ widgets: snap.ui.widgets, widget: snap.ui.widget });
const ZERO: EditorHistory = { undo: 0, redo: 0 };

export function usePackWidgetEditor(api: TabApi, snap: Snapshot | null, on: Callbacks): PackWidgetEditor {
  const pack = snap?.editing && !snap.history ? snap.editing.pack : null;
  // Un pack de layouts a l'onglet de sa scène (`PackTabs`).
  const stageTab = snap?.editing?.kind === "layouts";
  const revision = `${deckPinsKey(snap)}|${snap?.editing?.base ?? ""}`;
  const [tabs, setTabs] = useState<WidgetTabs>(NO_TABS);
  const onRef = useRef(on);
  useLayoutEffect(() => {
    onRef.current = on;
  });
  // Hors mode pack, aucun fichier ne se crée : la sorte n'y sert pas.
  const files = useWidgetFiles(api, pack, snap?.editing?.kind ?? "widgets", revision, {
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
  /** Écrits par l'agent, pas encore dans la liste des fichiers : ouverts quand elle les contient. */
  const [awaiting, setAwaiting] = useState<ReadonlySet<string>>(new Set());
  if (pack !== seenPack) {
    setSeenPack(pack);
    setAwaiting(new Set());
    // `next` continue : un id ne resservira pas à un autre fichier.
    setTabs((s) => ({ ...NO_TABS, next: s.next }));
    setRestoredFor(null);
    setHostTabs(pack === null ? null : keptOf(snap!));
  } else if (pack !== null && files.statuses && files.statusesFor === revision) {
    const names = new Set(files.statuses.map((s) => s.name));
    const arrived = [...awaiting].filter((n) => names.has(n));
    if (arrived.length > 0 && restoredFor === pack) {
      setTabs((s) => arrived.reduce((acc, n) => placeTouched(acc, n, files.pending), s));
      setAwaiting((a) => new Set([...a].filter((n) => !names.has(n))));
    }
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
  // Le sous-onglet widget actif (son éditeur) ; « Thème » n'en a pas.
  const active = pack === null ? null : activeWidget(tabs);
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
  const activate = useCallback((id: ActiveTab) => setTabs((s) => activated(s, id)), []);
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
    // Plus d'onglet actif : « Layouts » (la scène d'un pack de layouts), sinon « Thème ».
    document.getElementById(`widget-tab-${tabs.active ?? (stageTab ? 0 : THEME_TAB)}`)?.focus();
  }, [tabs]);
  const removed = useCallback(
    (name: string) =>
      setTabs((s) => {
        const tab = s.tabs.find((t) => t.name === name);
        return tab ? closed(s, tab.id) : s;
      }),
    [],
  );
  const touched = useCallback(
    (name: string, gone: boolean) => {
      if (packRef.current === null) return;
      if (gone) {
        files.forget(name);
        removed(name);
        setAwaiting((a) => (a.has(name) ? new Set([...a].filter((n) => n !== name)) : a));
        onRef.current.notice(t("ui-widgets-removed-by-agent", { name }));
        return;
      }
      if (files.pending(name)) files.keep(name);
      if (files.statuses?.some((s) => s.name === name)) setTabs((s) => placeTouched(s, name, files.pending));
      else setAwaiting((a) => (a.has(name) ? a : new Set(a).add(name)));
    },
    [files.forget, files.keep, files.pending, files.statuses, removed],
  );
  const undo = useCallback(() => {
    if (activeRef.current !== null) commands.current.get(activeRef.current)?.undo();
  }, []);
  const redo = useCallback(() => {
    if (activeRef.current !== null) commands.current.get(activeRef.current)?.redo();
  }, []);

  const finish = useCallback(
    async (closePack: () => Promise<unknown>) => {
      // Avant de quitter : un changement de pack abandonnerait la file.
      await settle();
      await closePack();
    },
    [settle],
  );

  // ⌘W et ⌘Z d'un sous-onglet widget, avant le `Workspace` et l'`App` (capture).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const id = activeWidget(tabsRef.current);
      if (e.repeat || packRef.current === null || id === null || onRef.current.blocked()) return;
      const target = e.target as HTMLElement | null;
      const inEditor = !!target?.closest?.(".typst-editor");
      const inField = !inEditor && !!target?.closest?.('input, textarea, select, [contenteditable="true"]');
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
    touched,
    undo,
    redo,
    history: shown.id === active ? shown.h : ZERO,
    commandsOf,
    historyOf,
    finish,
  };
}
