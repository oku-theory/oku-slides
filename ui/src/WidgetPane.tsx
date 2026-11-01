/** The sheet area of a pack tab: the item strip, then the selected item's view, if any. */
import type { ReactNode } from "react";
import type { GuidesView, PreviewSpan, Snapshot, TabApi, WidgetPreview } from "./api";
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
  /** The preview slot of an element widget, shared by every item. */
  span: PreviewSpan;
  /** The item strip (`PackPile`), above the view. */
  strip: ReactNode;
  /** The Code tab is open, shared by every item. */
  code: boolean;
  onCode: (code: boolean) => void;
  /** The host's settings of the selected item, in its Settings tab. */
  settings?: ReactNode;
  /** Example value writes: refusals toasted, snapshot updated. */
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  /** Swatch colours in a field's editor (the theme's chart palette). */
  palette: string[];
  /** The pack's guides for agents, in Commun's tabs; `null`: none. */
  guides: GuidesView | null;
  /** Each preview of the selected item, with its name. */
  onPreview?: (name: string, preview: WidgetPreview) => void;
}

/** The strip, then one `WidgetTab`, mounted again for each item selected (its pending write settled first). */
export default function WidgetPane({ api, item, trials, look, span, strip, code, onCode, settings = null, run, palette, guides, onPreview }: Props) {
  return (
    <div className="widget-pane">
      <div className="slide-strip pack-strip">{strip}</div>
      {item.selected !== null && (
        <WidgetTab
          key={item.selected}
          api={api}
          files={item.files}
          name={item.selected}
          revision={item.revision}
          trials={trials}
          look={look}
          span={span}
          code={code}
          onCode={onCode}
          settings={settings}
          run={run}
          palette={palette}
          guides={guides}
          onPreview={onPreview}
          onOpen={item.select}
          onCommands={item.onCommands}
          onHistory={item.onHistory}
        />
      )}
    </div>
  );
}
