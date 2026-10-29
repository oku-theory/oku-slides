/**
 * The one selected item (a widget or layout file, `_commun` included) of a pack tab.
 *
 * The host keeps the selection (`ui.widget`); it is restored once per pack, when the statuses of
 * the current revision arrive. With none named, the first pile item is selected, else `_commun`,
 * else nothing. It falls back (neighbour, `_commun`, else nothing) when those statuses no longer
 * list it, unless it was just created or written by the agent. The selected item's slide is the
 * tab's current slide (`ui.slide`, the agent's context); `_commun` has none. Every value
 * reported through `kept` is an expected echo: a host value found among them changes nothing, any
 * other host value is an open the host made (an agent `open`), which selects it.
 */
import { useCallback, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Snapshot, TabApi, WidgetStatus } from "./api";
import { t } from "./i18n";
import { itemSlide, neighbour, pileItems } from "./packItems";
import { deckPinsKey } from "./packOps";
import type { EditorCommands, EditorHistory } from "./TypstEditor";
import { useWidgetFiles, type WidgetFiles } from "./useWidgetFiles";
import { COMMON } from "./widgetList";

/** A pack tab's selected item, its files and its editor. */
export interface PackItem {
  /** The pack's files: statuses, typing, the one write queue. */
  files: WidgetFiles;
  /** The revision of the pinned packs and the edited pack; statuses and sources follow it. */
  revision: string;
  /** The selected item; `null` outside a pack tab, before the restore, or when none is left. */
  selected: string | null;
  /** The selected item's slide; `null` for `_commun`, for none, or outside a pack tab. */
  slide: string | null;
  /** Selects `name`, after sending the previous item's pending typing. */
  select(name: string): void;
  /** The agent wrote (or removed) `name`: select it without overwriting typing. */
  touched(name: string, removed: boolean): void;
  /** `name` was deleted from the pile: its neighbour is selected. */
  removed(name: string): void;
  /** The selected item's editor commands; `null` while none is mounted. */
  commands: EditorCommands | null;
  /** The selected item's editor undo depths. */
  history: EditorHistory;
  onCommands(c: EditorCommands | null): void;
  onHistory(h: EditorHistory): void;
  /** Undoes in the selected item's editor. */
  undo(): void;
  /** Redoes in the selected item's editor. */
  redo(): void;
  /** What `useKeptUi` has the host keep: the host's own values until the restore. */
  kept: { widget: string | null; slide: string | null };
}

/** What the pack item tells its tab. */
interface Callbacks {
  changed(): void;
  error(message: string): void;
  /** Information (not an error) for the user. */
  notice(message: string): void;
  /** True while a dialog, history or field editing holds the tab: agent writes move nothing. */
  blocked(): boolean;
}

const ZERO: EditorHistory = { undo: 0, redo: 0 };
const NONE: ReadonlySet<string> = new Set();
/** No statuses read yet: no order to find a neighbour in. */
const UNKNOWN: { of: WidgetStatus[] | null; names: string[] } = { of: null, names: [] };

/** The shared file when listed, else nothing. */
const commonOr = (names: ReadonlySet<string>) => (names.has(COMMON) ? COMMON : null);
/** The first pile item the statuses list, else the shared file, else nothing. */
const firstOr = (pile: readonly string[], names: ReadonlySet<string>) => pile.find((n) => names.has(n)) ?? commonOr(names);

/** The selected item of the pack `snap` edits; nothing outside a pack tab or in its history. */
export function usePackItem(api: TabApi, snap: Snapshot | null, on: Callbacks): PackItem {
  const pack = snap?.editing && !snap.history ? snap.editing.pack : null;
  const host = pack === null ? null : (snap?.ui.widget ?? null);
  const kind = snap?.editing?.kind ?? "widgets";
  const revision = `${deckPinsKey(snap)}|${snap?.editing?.base ?? ""}`;
  const onRef = useRef(on);
  useLayoutEffect(() => {
    onRef.current = on;
  });
  const [sel, setSel] = useState<string | null>(host);
  /** Selected before the statuses list it (just created, agent-written, opened by the host). */
  const [awaiting, setAwaiting] = useState<ReadonlySet<string>>(NONE);
  const files = useWidgetFiles(api, pack, kind, revision, {
    changed: () => onRef.current.changed(),
    error: (m) => onRef.current.error(m),
    renamed: (from, to) => {
      setSel((s) => (s === from ? to : s));
      setAwaiting((a) => (a.has(from) ? new Set([to]) : a));
    },
  });

  // Adjusted while rendering: no render shows another pack's item, or one gone for too long.
  const [seenPack, setSeenPack] = useState(pack);
  /** The pack whose restore is done. */
  const [restoredFor, setRestoredFor] = useState<string | null>(null);
  /** The last statuses read and their names, in order: where a gone item's neighbour is. */
  const [known, setKnown] = useState(UNKNOWN);
  if (pack !== seenPack) {
    setSeenPack(pack);
    setSel(host);
    setRestoredFor(null);
    setAwaiting(NONE);
    setKnown(UNKNOWN);
  } else if (pack !== null && files.statuses) {
    const listed = files.statuses.map((s) => s.name);
    if (files.statusesFor === revision) {
      const names = new Set(listed);
      if ([...awaiting].some((n) => names.has(n))) setAwaiting(new Set([...awaiting].filter((n) => !names.has(n))));
      const kept = sel === null || names.has(sel) || awaiting.has(sel);
      if (restoredFor !== pack) {
        // Opened on no item: the first one shows. No earlier list: a restored item gone has no neighbour.
        if (sel === null) setSel(firstOr(snap ? pileItems(snap).map((i) => i.name) : [], names));
        else if (!kept) setSel(commonOr(names));
        setRestoredFor(pack);
      } else if (!kept) {
        setSel(neighbour(known.names, sel, (n) => names.has(n)) ?? commonOr(names));
      }
    }
    if (files.statuses !== known.of) setKnown({ of: files.statuses, names: listed });
  }

  const selRef = useRef(sel);
  const packRef = useRef(pack);
  const kindRef = useRef(kind);
  const listedRef = useRef<string[]>([]);
  useLayoutEffect(() => {
    selRef.current = sel;
    packRef.current = pack;
    kindRef.current = kind;
    listedRef.current = files.statuses?.map((s) => s.name) ?? [];
  });

  const { flush } = files;
  const select = useCallback(
    (name: string) => {
      const previous = selRef.current;
      if (previous !== null && previous !== name) flush(previous);
      selRef.current = name;
      setSel(name);
      // Protected until the statuses list it; at once for an item already listed.
      setAwaiting(new Set([name]));
    },
    [flush],
  );
  /** The item selected instead of `gone`: its neighbour in the statuses, else `_commun`, else none. */
  const fallbackOf = useCallback((gone: string) => {
    const listed = listedRef.current;
    const names = new Set(listed.filter((n) => n !== gone));
    return neighbour(listed, gone, (n) => names.has(n)) ?? commonOr(names);
  }, []);
  const removed = useCallback(
    (name: string) => {
      if (selRef.current !== name) return;
      const next = fallbackOf(name);
      selRef.current = next;
      setSel(next);
    },
    [fallbackOf],
  );
  const { forget, keep, pending } = files;
  const touched = useCallback(
    (name: string, gone: boolean) => {
      if (packRef.current === null) return;
      if (gone) {
        forget(name);
        setAwaiting((a) => (a.has(name) ? new Set([...a].filter((n) => n !== name)) : a));
        removed(name);
        const key = kindRef.current === "layouts" ? "ui-layouts-removed-by-agent" : "ui-widgets-removed-by-agent";
        onRef.current.notice(t(key, { name }));
        return;
      }
      if (pending(name)) keep(name);
      const current = selRef.current;
      if (current === name || (current !== null && pending(current)) || onRef.current.blocked()) return;
      select(name);
    },
    [forget, keep, pending, removed, select],
  );

  // Echoes: each value kept is expected back from the host; anything else the host shows, it opened.
  const echoes = useRef<(string | null)[]>([]);
  const lastHost = useRef(host);
  useLayoutEffect(() => {
    echoes.current = [];
    lastHost.current = host;
    // Only a new pack starts over; `host` is read as it is then.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pack]);
  useLayoutEffect(() => {
    if (pack === null || host === lastHost.current) return;
    lastHost.current = host;
    const at = echoes.current.indexOf(host);
    if (at >= 0) echoes.current.splice(0, at + 1);
    else if (host !== null) select(host);
  }, [pack, host, select]);
  const keptWidget = pack === null ? null : sel;
  const selected = pack !== null && restoredFor === pack ? sel : null;
  const slide = selected === null ? null : itemSlide(kind, selected);
  // Until the restore, the host's own slide: never overwritten with nothing.
  const keptSlide = pack === null ? null : restoredFor === pack ? slide : (snap?.ui.slide ?? null);
  // A value the host already holds (the mount value, an item it opened) comes back as no echo.
  useLayoutEffect(() => {
    if (pack !== null && keptWidget !== lastHost.current) echoes.current.push(keptWidget);
  }, [pack, keptWidget]);

  // The mounted editor's commands and depths; one editor at a time.
  const [commands, setCommands] = useState<EditorCommands | null>(null);
  const [history, setHistory] = useState<EditorHistory>(ZERO);
  const commandsRef = useRef(commands);
  useLayoutEffect(() => {
    commandsRef.current = commands;
  });
  const onCommands = useCallback((c: EditorCommands | null) => {
    setCommands(() => c);
    if (c === null) setHistory(ZERO);
  }, []);
  const onHistory = useCallback(
    (h: EditorHistory) => setHistory((prev) => (prev.undo === h.undo && prev.redo === h.redo ? prev : h)),
    [],
  );
  const undo = useCallback(() => commandsRef.current?.undo(), []);
  const redo = useCallback(() => commandsRef.current?.redo(), []);
  const kept = useMemo(() => ({ widget: keptWidget, slide: keptSlide }), [keptWidget, keptSlide]);

  return {
    files,
    revision,
    selected,
    slide,
    select,
    touched,
    removed,
    commands,
    history,
    onCommands,
    onHistory,
    undo,
    redo,
    kept,
  };
}
