/** The sheet area of a pack tab: the selected item's view, alone, or nothing when none is. */
import type { ReactNode } from "react";
import type { Snapshot, TabApi } from "./api";
import type { PackItem } from "./usePackItem";
import WidgetTab from "./WidgetTab";

/** What the pane shows of the selected item, and its shared settings. */
interface Props {
  api: TabApi;
  item: PackItem;
  /** The pack's trial revision: the preview is asked again when it changes. */
  trials: number;
  /** The deck style's fingerprint: the preview is asked again when it changes. */
  look: string;
  /** Source code shown, shared by every item. */
  code: boolean;
  /** The code width chosen at the divider, shared by every item; `null`: the default share. */
  codeWidth: number | null;
  onCodeWidth: (width: number | null) => void;
  /** The host's settings of the selected item, under its preview. */
  settings?: ReactNode;
  /** Example value writes: refusals toasted, snapshot updated. */
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  /** Swatch colours in a field's editor (the theme's chart palette). */
  palette: string[];
}

/** One `WidgetTab`, mounted again for each item selected (its pending write settled first). */
export default function WidgetPane({ api, item, trials, look, code, codeWidth, onCodeWidth, settings = null, run, palette }: Props) {
  return (
    <div className="widget-pane">
      {item.selected !== null && (
        <WidgetTab
          key={item.selected}
          api={api}
          files={item.files}
          name={item.selected}
          revision={item.revision}
          trials={trials}
          look={look}
          code={code}
          codeWidth={codeWidth}
          onCodeWidth={onCodeWidth}
          settings={settings}
          run={run}
          palette={palette}
          onOpen={item.select}
          onCommands={item.onCommands}
          onHistory={item.onHistory}
        />
      )}
    </div>
  );
}
