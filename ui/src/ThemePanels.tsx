/**
 * The panels of a theme tab: the header, the toolbar, the fixed specimens pile and the theme
 * panel.
 *
 * The theme panel goes into the notes column (`notes`, App's `aside`), where the tokens replace
 * the agent thread while the tab shows.
 */
import type { ReactNode } from "react";
import { createPortal } from "react-dom";
import type { Snapshot, TabApi } from "./api";
import DeckPile from "./DeckPile";
import { t } from "./i18n";
import PileColumn from "./PileColumn";
import ThemePanel from "./ThemePanel";
import ThemeTabHeader from "./ThemeTabHeader";
import { ToolsTray } from "./ToolsTray";
import type { KeptUi } from "./useKeptUi";

/** What a theme tab's panels show and the gestures they need. */
export interface ThemePanelsProps {
  api: TabApi;
  snap: Snapshot;
  keptUi: KeptUi;
  /** The specimen shown on the stage. */
  current: string | null;
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  say: (text: string, error?: boolean) => void;
  /** A thumbnail's URL at its hash; `null` while it renders. */
  svg: (id: string, hash: string) => string | null;
  mainMenu: ReactNode;
  /** The notes column the theme panel goes into; `null` before it is mounted. */
  notes: HTMLElement | null;
  /** Shows a specimen. */
  onSelect: (id: string) => void;
  /** Closes this tab (the finish button); absent when the tab cannot close. */
  onClose?: () => void;
}

/** A theme tab's header, tools, specimens and theme panel. */
export default function ThemePanels({ api, snap, keptUi, current, run, say, svg, mainMenu, notes, onSelect, onClose }: ThemePanelsProps) {
  if (!snap.theming) return null;
  const open = !keptUi.folded.includes("layouts");
  const noop = () => {};
  return (
    <>
      <ThemeTabHeader theming={snap.theming} dirty={snap.dirty} onFinish={onClose} />
      <ToolsTray
        api={api} snap={snap} view={null} item={null} keptUi={keptUi} mainMenu={mainMenu} themeTab
        run={run} onAdd={noop} onPacks={noop} onHistory={noop} thumbs={null}
      />
      <PileColumn
        title={t("ui-pile-layouts")}
        open={open}
        onToggle={() => keptUi.setFolded("layouts", open)}
        slides={<DeckPile snap={snap} themeTab current={current} svg={svg} onSelect={onSelect} onMove={noop} onAdd={noop} onRemove={noop} axis="y" />}
        widgets={null}
      />
      {notes && createPortal(<ThemePanel api={api} snap={snap} run={run} say={say} />, notes)}
    </>
  );
}
