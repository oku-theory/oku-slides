/**
 * The panels of a pack tab: the header (brand, title, caption, preview theme), the toolbar, the finish button,
 * the pile of items in the left column, and the selected item's view in the sheet area.
 *
 * A delete in the pile asks in a `ConfirmDialog`, reported open through `onDialog` so that
 * nothing behind it reacts.
 */
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import type { Snapshot, TabApi } from "./api";
import ConfirmDialog from "./ConfirmDialog";
import { t } from "./i18n";
import { Icon } from "./icons";
import { PackBrand } from "./PackBrand";
import { pileItems } from "./packItems";
import PackPile from "./PackPile";
import PreviewThemeMenu from "./PreviewThemeMenu";
import { showFinish } from "./packGestures";
import PileColumn from "./PileColumn";
import TitleField from "./TitleField";
import { ToolsTray } from "./ToolsTray";
import type { KeptUi } from "./useKeptUi";
import type { PackItem } from "./usePackItem";
import WidgetPane from "./WidgetPane";

/** Applies a request that answers a snapshot; `null` when nothing was done. */
type Run = (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
/** A thumbnail's URL at its hash; `null` while it renders. */
type SvgOf = (id: string, hash: string) => string | null;

/** What a pack tab's panels show and the tab's gestures they need. */
export interface PackPanelsProps {
  api: TabApi;
  snap: Snapshot;
  item: PackItem;
  keptUi: KeptUi;
  /** The deck style's fingerprint: the item preview follows it. */
  look: string;
  run: Run;
  /** The host's settings of the selected item; `null`: none. */
  settings: ReactNode | null;
  svg: SvgOf;
  mainMenu: ReactNode;
  /** The agent's drawer slot, at the foot of the sheet area. */
  drawer: ReactNode;
  /** Closes this tab (the finish button); absent when the tab cannot close. */
  onClose?: () => void;
  /** The confirmation dialog opened or closed. */
  onDialog(open: boolean): void;
}

/** A question waiting for its answer. */
interface Asking {
  message: string;
  answer(yes: boolean): void;
}

/** A pack tab's header, tools, pile and selected item. */
export default function PackPanels({
  api, snap, item, keptUi, look, run, settings, svg, mainMenu, drawer, onClose, onDialog,
}: PackPanelsProps) {
  const [asking, setAsking] = useState<Asking | null>(null);
  const askingRef = useRef(asking);
  useLayoutEffect(() => {
    askingRef.current = asking;
  });
  const open = asking !== null;
  // Reported open, then closed when it closes or the panels go with it open.
  useEffect(() => {
    onDialog(open);
    return () => {
      if (open) onDialog(false);
    };
  }, [open]);
  // A question left open when the tab goes is answered no.
  useEffect(() => () => askingRef.current?.answer(false), []);
  const confirm = (message: string) =>
    new Promise<boolean>((resolve) =>
      setAsking({
        message,
        answer: (yes) => {
          setAsking(null);
          resolve(yes);
        },
      }),
    );

  const editing = snap.editing;
  if (!editing) return null;
  const kind = editing.kind;
  const items = pileItems(snap);
  const layouts = kind === "layouts";
  const folded = layouts && keptUi.folded.includes("layouts");
  const pile = (
    <PackPile
      snap={snap} kind={kind} files={item.files} selected={item.selected} svg={svg}
      onSelect={item.select} onRemoved={item.removed} confirm={confirm}
    />
  );
  const status = t(snap.dirty ? "ui-status-unsaved" : "ui-status-saved");

  return (
    <>
      <header className="label editing-pack">
        <PackBrand name={editing.name} kind={kind} deckOnly={editing.deck_only} />
        <TitleField
          value={snap.title_source ?? ""}
          display={snap.title ?? snap.name}
          onCommit={(v) => void run(api.apply(t("ui-undo-rename-pack"), [{ op: "set_deck_param", key: "title", value: v }]))}
        />
        <span className="caption" role="status">{t("ui-caption-pack", { kind, count: items.length, status })}</span>
        <PreviewThemeMenu themes={snap.themes} theme={snap.theme} onPick={(ns) => void run(api.setPreviewTheme(ns))} />
      </header>
      <ToolsTray
        api={api} snap={snap} view={null} item={item} keptUi={keptUi} mainMenu={mainMenu} themeTab={false}
        run={run} onAdd={() => {}} onPacks={() => {}} thumbs={null}
      />
      {/* Top right: finish (a lone pack tab that cannot close has none). */}
      {showFinish(snap, onClose !== undefined) && (
        <div className="tray">
          <button className="object token" aria-label={t("ui-toolbar-finish")} title={t("ui-toolbar-finish")} onClick={onClose}>{Icon.check}</button>
        </div>
      )}
      <PileColumn
        title={t("ui-pile-layouts")}
        open={!folded}
        onToggle={() => keptUi.setFolded("layouts", !folded)}
        slides={layouts ? pile : null}
        widgets={layouts ? null : pile}
      />
      <main className="sheet-area">
        <WidgetPane
          api={api} item={item} trials={snap.widget_trials} look={look} code={keptUi.code} codeWidth={keptUi.codeWidth}
          onCodeWidth={keptUi.setCodeWidth} run={run} palette={snap.appearance.chart_palette} settings={settings}
        />
        {drawer}
      </main>
      {asking && (
        <ConfirmDialog
          message={asking.message}
          confirmLabel={t("ui-confirm-delete-ok")}
          onConfirm={() => asking.answer(true)}
          onCancel={() => asking.answer(false)}
        />
      )}
    </>
  );
}
