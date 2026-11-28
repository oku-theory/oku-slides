// A deck's knowledge base, read-only: the tree on the side, one sub-tab per open file.
//
// It reads through the deck tab's session (`api`). A new `revision` re-reads the tree and the
// shown file; the other open files go stale and are read again when shown. Re-reads coalesce
// (one in flight, at most one pending) and each reply is checked against the latest request of
// its kind (one `Latest` for the tree, one per file). A file gone from the tree closes its
// sub-tab with one toast; a reply for it after that is ignored. While the deck browses its
// history (`deckInHistory`) the session refuses the reads: the tab shows a notice instead of the
// pane, and leaving history re-reads, as the knowledge revision does not change then. The open
// and shown files are kept in `subTabs` under the tab's id: a tab switch remounts the view.
//
// An approved file shows its header and its Markdown body, images as `blob:` URLs only; one whose
// frontmatter does not parse shows its raw text under a notice. An awaiting one shows its raw
// text, hidden characters revealed, and Review opens the knowledge dialog; nothing in it is
// rendered or loaded.

import { type KeyboardEvent, useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { errorText, type TabApi } from "../api";
import type { Run } from "../GuidesSection";
import { locale, t } from "../i18n";
import { Icon } from "../icons";
import KnowledgeDialog from "../KnowledgeDialog";
import { revealHidden } from "../knowledgeText";
import { Latest } from "../latest";
import { Button, IconButton } from "../primitives";
import { stripKeyNav } from "../stripKeyNav";
import { KnowledgeHeader } from "./KnowledgeHeaderView";
import { KnowledgeMarkdown } from "./KnowledgeMarkdown";
import { buildTree, nodeLabel, pathLabel, plainPath } from "./knowledgeTree";
import { KnowledgeTreeView } from "./KnowledgeTreeView";
import { keepSubTabs, subTabsOf } from "./subTabs";
import type { KnowledgeNode, KnowledgePage, KnowledgeRevision } from "./types";
import { useKnowledgeAssets } from "./useKnowledgeAssets";

interface Props {
  /** This knowledge tab's id: its sub-tabs are kept under it. */
  tab: string;
  /** The deck tab's api: every read goes to its session. */
  api: TabApi;
  /** The deck's knowledge revision; `null` when unknown (nothing is re-read). */
  revision: KnowledgeRevision | null;
  /** Whether the deck tab browses its history: its session refuses every read meanwhile. */
  deckInHistory: boolean;
  /** Shows a toast. */
  say: (message: string) => void;
  /** Runs an approval of the knowledge dialog. */
  run: Run;
}

/** The root's own page, opened first. */
const ROOT_INDEX = "/index.md";

/** An awaiting file's raw text, hidden characters revealed, and its Review button. */
function AwaitingPage({ page, onReview }: { page: KnowledgePage; onReview: () => void }) {
  const revealed = useMemo(() => revealHidden(page.body), [page.body]);
  return (
    <>
      <header className="knowledge-header">
        <h1><bdi>{plainPath(page.path)}</bdi></h1>
        <span className="knowledge-awaiting-badge">{t("ui-knowledge-awaiting-badge")}</span>
        <Button onClick={onReview}>{t("ui-knowledge-review")}</Button>
      </header>
      <pre dir="auto" className="guide-text knowledge-raw">{revealed.text}</pre>
      {revealed.hidden > 0 && (
        <p className="caption knowledge-hidden">{t("ui-knowledge-hidden-chars", { count: revealed.hidden })}</p>
      )}
    </>
  );
}

/** Whether two path sets hold the same paths. */
const samePaths = (a: ReadonlySet<string>, b: ReadonlySet<string>) => a.size === b.size && [...a].every((p) => b.has(p));

/** The knowledge tab `tab` of a deck tab. */
export function KnowledgeTab({ tab, api, revision, deckInHistory, say, run }: Props) {
  const baseId = useId();
  // The sub-tabs this tab showed before a remount, if any.
  const [kept] = useState(() => subTabsOf(tab));
  const [files, setFiles] = useState<KnowledgeNode[] | null>(null);
  // The bundle's paths, kept while they do not change: the Markdown is not rendered again.
  const [paths, setPaths] = useState<ReadonlySet<string>>(new Set());
  // The open files, as the last tree has them (a moved or retitled file keeps its sub-tab).
  const [open, setOpen] = useState<KnowledgeNode[]>(kept?.open ?? []);
  const [active, setActive] = useState<string | null>(kept?.active ?? null);
  const [pages, setPages] = useState<ReadonlyMap<string, KnowledgePage>>(new Map());
  const [reviewing, setReviewing] = useState(false);

  // What the async reads check: the state they act on, mirrored as it is set.
  const filesRef = useRef<KnowledgeNode[]>([]);
  const openRef = useRef<KnowledgeNode[]>(kept?.open ?? []);
  const activeRef = useRef<string | null>(kept?.active ?? null);
  const inHistory = useRef(deckInHistory);
  inHistory.current = deckInHistory;
  const stale = useRef(new Set<string>());
  const treeLatest = useRef(new Latest());
  const pageLatest = useRef(new Map<string, Latest>());
  const alive = useRef(false);
  const busy = useRef(false);
  const pending = useRef(false);
  // Set once the root page was opened first, or when sub-tabs were kept: a remount opens none.
  const started = useRef(kept !== undefined);
  // The revision last read; `undefined` before the first read.
  const lastRevision = useRef<string | null | undefined>(undefined);
  // Set while the deck browses its history; leaving it re-reads.
  const wasInHistory = useRef(false);
  const strip = useRef<HTMLDivElement>(null);
  // Set by a keyboard close: the shown sub-tab takes the focus once rendered.
  const focusShown = useRef(false);

  const isOpen = (id: string) => openRef.current.some((n) => n.id === id);

  /** Shows sub-tabs `nodes` with `id` in front. */
  const show = (nodes: KnowledgeNode[], id: string | null) => {
    openRef.current = nodes;
    activeRef.current = id;
    keepSubTabs(tab, { open: nodes, active: id });
    setOpen(nodes);
    setActive(id);
  };

  /**
   * Says a refused read, unless the deck browses its history: the notice says it then, and
   * leaving it re-reads. A refusal that lands before the view says so is told once.
   */
  const fail = (e: unknown) => {
    if (alive.current && !inHistory.current) say(errorText(e));
  };

  /** Reads file `id`'s page; a reply for a sub-tab closed since, or not the latest, is dropped. */
  const readPage = async (id: string) => {
    stale.current.delete(id);
    let latest = pageLatest.current.get(id);
    if (!latest) pageLatest.current.set(id, (latest = new Latest()));
    const seq = latest.begin();
    // Closed and opened again since, the file has a new `Latest`: this reply is not its latest.
    const current = () => alive.current && isOpen(id) && pageLatest.current.get(id) === latest && latest.accept(seq);
    try {
      const page = await api.knowledgePage(id);
      if (!current()) return;
      setPages((m) => new Map(m).set(id, page));
    } catch (e) {
      if (!current()) return;
      // Shown again, it is read again.
      stale.current.add(id);
      fail(e);
    }
  };

  /** Closes sub-tabs `ids`; the next one (else the previous) is shown if the shown one closed. */
  const close = (ids: ReadonlySet<string>) => {
    const was = openRef.current;
    const next = was.filter((n) => !ids.has(n.id));
    let shown = activeRef.current;
    if (shown !== null && ids.has(shown)) {
      const at = was.findIndex((n) => n.id === shown);
      const after = was.slice(at + 1).find((n) => !ids.has(n.id));
      const before = was.slice(0, at).reverse().find((n) => !ids.has(n.id));
      shown = (after ?? before)?.id ?? null;
    }
    for (const id of ids) {
      // A reply still in flight for it finds it closed (`isOpen`) and is dropped.
      pageLatest.current.delete(id);
      stale.current.delete(id);
    }
    setPages((m) => new Map([...m].filter(([id]) => !ids.has(id))));
    show(next, shown);
    return shown;
  };

  /** Opens `node` in a sub-tab, or shows the one it has; reads it unless current. */
  const openFile = (node: KnowledgeNode) => {
    const known = isOpen(node.id);
    show(known ? openRef.current : [...openRef.current, node], node.id);
    if (!known || stale.current.has(node.id)) void readPage(node.id);
  };

  /** The user closes a sub-tab. */
  const closeFile = (id: string) => {
    const shown = close(new Set([id]));
    if (shown !== null && stale.current.has(shown)) void readPage(shown);
  };

  /** Takes a new tree: closes the sub-tabs of files gone (one toast each), opens the root page first. */
  const applyTree = (next: KnowledgeNode[]) => {
    filesRef.current = next;
    setFiles(next);
    const nextPaths = new Set(next.map((f) => f.path));
    setPaths((was) => (samePaths(was, nextPaths) ? was : nextPaths));
    const byId = new Map(next.map((f) => [f.id, f]));
    // A closed sub-tab leaves `openRef` at once: each removal toasts once, a later one again.
    const gone = openRef.current.filter((n) => !byId.has(n.id));
    for (const n of gone) say(t("ui-knowledge-file-gone", { path: plainPath(n.path) }));
    if (gone.length > 0) close(new Set(gone.map((n) => n.id)));
    // Every file left open is in the tree: take its new path and title.
    show(openRef.current.flatMap((n) => byId.get(n.id) ?? []), activeRef.current);
    if (!started.current) {
      started.current = true;
      const index = next.find((f) => f.path === ROOT_INDEX && !f.awaiting);
      if (index) show([index], index.id);
    }
  };

  /** One re-read: the tree, then the shown file; the other open files go stale. */
  const cycle = async () => {
    const seq = treeLatest.current.begin();
    let next: KnowledgeNode[];
    try {
      next = (await api.knowledgeTree()).files;
    } catch (e) {
      if (alive.current && treeLatest.current.accept(seq)) fail(e);
      return;
    }
    if (!alive.current || !treeLatest.current.accept(seq)) return;
    applyTree(next);
    const shown = activeRef.current;
    for (const n of openRef.current) if (n.id !== shown) stale.current.add(n.id);
    if (shown !== null) await readPage(shown);
  };

  /** Re-reads now, or once more after the read in flight. */
  const refresh = () => {
    if (busy.current) {
      pending.current = true;
      return;
    }
    busy.current = true;
    void (async () => {
      try {
        do {
          pending.current = false;
          await cycle();
        } while (pending.current && alive.current);
      } finally {
        busy.current = false;
      }
    })();
  };

  // Declared first: the read below checks it. A StrictMode remount sets it again.
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const revisionKey = revision === null ? null : `${revision.generation}:${revision.approvals}`;
  // The first read, then one per new revision and one on leaving history (the revision stays the
  // same then); `null` re-reads nothing, and nothing is read in history. A StrictMode remount
  // finds the revision already read, and the read in flight still lands (`alive` is set again).
  useEffect(() => {
    if (deckInHistory) {
      wasInHistory.current = true;
      return;
    }
    const left = wasInHistory.current;
    wasInHistory.current = false;
    const last = lastRevision.current;
    if (!left && last !== undefined && (revisionKey === null || revisionKey === last)) return;
    if (revisionKey !== null || last === undefined) lastRevision.current = revisionKey;
    refresh();
  }, [revisionKey, deckInHistory]);

  // Stable for the Markdown's memo: it reads the latest `openFile` through a ref.
  const openFileRef = useRef(openFile);
  openFileRef.current = openFile;
  const onOpenPath = useCallback((path: string) => {
    const file = filesRef.current.find((f) => f.path === path);
    if (file) openFileRef.current(file);
  }, []);
  const lang = locale();
  const tree = useMemo(() => files && buildTree(files, lang), [files, lang]);

  const page = active === null ? null : pages.get(active) ?? null;
  // No page in history: the refused images load again once it is left.
  const image = useKnowledgeAssets(api, deckInHistory ? null : page);
  const activeAt = open.findIndex((n) => n.id === active);

  useEffect(() => {
    if (!focusShown.current) return;
    focusShown.current = false;
    strip.current?.querySelector<HTMLButtonElement>('[role="tab"][aria-selected="true"]')?.focus();
  }, [active, open]);

  const onStripKey = (e: KeyboardEvent<HTMLDivElement>) => {
    // Delete (Backspace on a Mac keyboard) closes the shown sub-tab, as its close button does.
    // The shown sub-tab is the focused one by construction: only it is in the tab order
    // (roving tabindex), its close button is not, and a click or an arrow key shows the sub-tab
    // it focuses.
    if ((e.key === "Delete" || e.key === "Backspace") && activeAt >= 0) {
      e.preventDefault();
      focusShown.current = true;
      closeFile(open[activeAt].id);
      return;
    }
    const rtl = getComputedStyle(e.currentTarget).direction === "rtl";
    const to = stripKeyNav(e, activeAt, open.length, rtl);
    if (to === null) return;
    e.preventDefault();
    openFile(open[to]);
    strip.current?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[to]?.focus();
  };

  const tabId = (i: number) => `${baseId}-tab-${i}`;
  const panelId = `${baseId}-panel`;

  return (
    <section className="knowledge-tab">
      {tree && <KnowledgeTreeView root={tree} active={active} onOpen={openFile} />}
      <div className="knowledge-main">
        {open.length > 0 && (
          <div
            ref={strip} className="widget-tab-tabs knowledge-subtabs" role="tablist"
            aria-label={t("ui-knowledge-open-files")} onKeyDown={onStripKey}
          >
            {open.map((node, i) => (
              <span key={node.id} className="knowledge-subtab">
                <button
                  type="button" role="tab" className="widget-tab-tab" id={tabId(i)}
                  aria-selected={node.id === active} aria-controls={node.id === active ? panelId : undefined}
                  tabIndex={node.id === active ? 0 : -1} onClick={() => openFile(node)}
                >
                  <bdi className="knowledge-subtab-name">{nodeLabel(node)}</bdi>
                </button>
                <IconButton
                  className="knowledge-subtab-close" label={t("ui-knowledge-close-file")} icon={Icon.close}
                  tabIndex={-1} onClick={() => closeFile(node.id)}
                />
              </span>
            ))}
          </div>
        )}
        {deckInHistory ? (
          <p className="caption knowledge-in-history" role="status">{t("ui-knowledge-in-history")}</p>
        ) : files?.length === 0 ? (
          <p className="caption knowledge-empty">{t("ui-knowledge-empty")}</p>
        ) : active !== null && (
          <div className="knowledge-pane" role="tabpanel" id={panelId} aria-labelledby={tabId(activeAt)}>
            {page === null ? (
              <p className="caption">{t("ui-loading")}</p>
            ) : page.awaiting ? (
              <AwaitingPage page={page} onReview={() => setReviewing(true)} />
            ) : page.header_invalid ? (
              <>
                <KnowledgeHeader header={null} fallback={pathLabel(page.path)} />
                <p className="caption knowledge-header-invalid" role="note">{t("ui-knowledge-header-invalid")}</p>
                <pre dir="auto" className="guide-text knowledge-raw">{page.body}</pre>
              </>
            ) : (
              <>
                <KnowledgeHeader header={page.header} fallback={pathLabel(page.path)} />
                <KnowledgeMarkdown text={page.body} from={page.path} paths={paths} onOpen={onOpenPath} image={image} />
              </>
            )}
            {page !== null && page.truncated_bytes > 0 && (
              <p className="caption knowledge-truncated">
                {t("ui-knowledge-truncated", { kb: Math.ceil(page.truncated_bytes / 1024) })}
              </p>
            )}
          </div>
        )}
      </div>
      {reviewing && <KnowledgeDialog api={api} run={run} onError={say} onClose={() => setReviewing(false)} />}
    </section>
  );
}
