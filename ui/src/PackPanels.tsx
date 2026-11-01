/**
 * The panels of a pack tab: the header (brand, title, caption, preview theme), the toolbar, the finish button,
 * and in the sheet area the strip of items, then the selected item's view (Commun's holds the pack's guides for agents).
 *
 * A delete in the pile asks in a `ConfirmDialog` (useConfirm), reported open through `onDialog` so that
 * nothing behind it reacts.
 */
import { useEffect, type ReactNode } from "react";
import type { Snapshot, TabApi } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";
import { PackBrand } from "./PackBrand";
import PackPile from "./PackPile";
import PreviewSpanMenu from "./PreviewSpanMenu";
import PreviewThemeMenu from "./PreviewThemeMenu";
import { COMMON } from "./widgetList";
import { showFinish } from "./packGestures";
import { pileItems, slideItem } from "./packItems";
import TitleField from "./TitleField";
import { ToolsTray } from "./ToolsTray";
import { useConfirm } from "./useConfirm";
import type { KeptUi } from "./useKeptUi";
import type { PackItem } from "./usePackItem";
import WidgetPane from "./WidgetPane";
import { useWidgetThumbs } from "./useWidgetThumbs";

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

export default function PackPanels({
  api, snap, item, keptUi, look, run, settings, svg, mainMenu, drawer, onClose, onDialog,
}: PackPanelsProps) {
  // A question left open when the tab goes is answered no (useConfirm).
  const { confirm, asking: open, dialog } = useConfirm();
  // A widget pack's thumbnails show each widget alone; a layout pack's, its slides. Example values
  // change the trials of the whole pack, but are edited on the selected widget: only its key follows them.
  const widgets = snap.editing?.kind === "widgets";
  const thumbs = useWidgetThumbs(
    api,
    widgets
      ? pileItems(snap).map((i) => ({
        name: i.name,
        key: `${i.slide.hash}|${look}${i.name === item.selected ? `|${snap.widget_trials}` : ""}`,
      }))
      : null,
  );
  // Reported open, then closed when it closes or the panels go with it open.
  useEffect(() => {
    onDialog(open);
    return () => {
      if (open) onDialog(false);
    };
  }, [open]);

  const editing = snap.editing;
  // A delete question asked as the tab leaves pack mode still shows.
  if (!editing) return <>{dialog}</>;
  const kind = editing.kind;
  const items = pileItems(snap);
  const pile = (
    <PackPile
      snap={snap} kind={kind} files={item.files} selected={item.selected}
      svg={widgets ? (id) => { const name = slideItem(id); return name === null ? null : thumbs.url(name); } : svg}
      onSelect={item.select} onRemoved={item.removed} confirm={confirm} keepCommon={!!snap.guides}
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
        {/* One group, so the preview controls wrap together under the title on a narrow sheet. */}
        <span className="pack-preview-controls">
          <span className="preview-label">{t("ui-preview-label")}</span>
          <PreviewThemeMenu themes={snap.themes} theme={snap.theme} onPick={(ns) => void run(api.setPreviewTheme(ns))} />
          {kind === "widgets" && item.selected !== null && item.selected !== COMMON && (
            <PreviewSpanMenu span={keptUi.previewSpan} onPick={keptUi.setPreviewSpan} />
          )}
        </span>
      </header>
      <ToolsTray
        api={api} snap={snap} item={item} keptUi={keptUi} mainMenu={mainMenu} themeTab={false}
        run={run} onAdd={null} onPacks={() => {}} onHistory={() => {}} thumbs={null}
      />
      {/* Top right: finish (a lone pack tab that cannot close has none). */}
      {showFinish(snap, onClose !== undefined) && (
        <div className="tray">
          <button className="object token" aria-label={t("ui-toolbar-finish")} title={t("ui-toolbar-finish")} onClick={onClose}>{Icon.check}</button>
        </div>
      )}
      <main className="sheet-area">
        <WidgetPane
          api={api} item={item} trials={snap.widget_trials} look={look} span={keptUi.previewSpan} strip={pile} code={keptUi.code}
          onCode={keptUi.setCode} run={run}
          onPreview={widgets && keptUi.previewSpan === "full" ? thumbs.put : undefined} palette={snap.appearance.chart_palette} settings={settings}
          guides={snap.guides}
        />
        {drawer}
      </main>
      {dialog}
    </>
  );
}
