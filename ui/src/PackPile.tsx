/**
 * A pack tab's pile: the shared file first when it exists, one thumbnail per item, then the
 * add button with its name field.
 *
 * F2 or a double-click renames an item; its trash button, Delete or Backspace delete it after
 * the shared confirmation (`confirm`, a `ConfirmDialog`). Its context menu offers both (the
 * shared file: delete only). After a delete, the focus goes to the
 * neighbour thumbnail, else the shared row, else the add button. Thumbnails stay `<img>` (via
 * `SlidePile`); no id or selector is built from an item name.
 */
import { useEffect, useId, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import type { Snapshot } from "./api";
import ContextMenu from "./ContextMenu";
import { t } from "./i18n";
import { Icon } from "./icons";
import { itemSlide, neighbour, pileItems, slideItem } from "./packItems";
import type { PackKind } from "./packTypes";
import { IconButton } from "./primitives";
import SlidePile from "./SlidePile";
import type { Confirm } from "./useConfirm";
import type { WidgetFiles } from "./useWidgetFiles";
import { COMMON, nameAvailable, nameIssue, widgetLabel, widgetSlug } from "./widgetList";

/** The pack tab's items, its selection and the gestures on them. */
export interface PackPileProps {
  snap: Snapshot;
  kind: PackKind;
  files: WidgetFiles;
  selected: string | null;
  /** A thumbnail's URL at its hash; `null` while it renders. */
  svg: (id: string, hash: string) => string | null;
  onSelect(name: string): void;
  /** After a delete the session accepted. */
  onRemoved(name: string): void;
  /** Asks a question in a `ConfirmDialog`; true when confirmed. */
  confirm: Confirm;
}

/** Where the focus goes once the pile shows a change. */
type Focus = { item: string } | "common" | "add";

/** The name being typed: a new item (`from` null) or a rename. */
interface Naming {
  from: string | null;
  value: string;
}

/** An item's context menu, at the pointer. */
interface ItemMenu {
  name: string;
  x: number;
  y: number;
}

/** The pile of a pack tab: shared file, item thumbnails, add, rename, delete. */
export default function PackPile({ snap, kind, files, selected, svg, onSelect, onRemoved, confirm }: PackPileProps) {
  const [naming, setNaming] = useState<Naming | null>(null);
  const [menu, setMenu] = useState<ItemMenu | null>(null);
  const [refocus, setRefocus] = useState<Focus | null>(null);
  const issueId = useId();
  const box = useRef<HTMLDivElement>(null);
  const commonRow = useRef<HTMLButtonElement>(null);
  const addButton = useRef<HTMLButtonElement>(null);
  const items = pileItems(snap);
  const order = items.map((i) => i.name);
  const hasCommon = !!files.statuses?.some((s) => s.name === COMMON);

  /** The thumbnail of `name`, found by its place in the pile. */
  const thumbOf = (name: string) => {
    const at = order.indexOf(name);
    return at < 0 ? null : (box.current?.querySelectorAll<HTMLButtonElement>(".thumb")[at] ?? null);
  };
  // A renamed item has no thumbnail yet: the focus waits for the render that shows it.
  useEffect(() => {
    if (naming || !refocus) return;
    const el = refocus === "common" ? commonRow.current : refocus === "add" ? addButton.current : thumbOf(refocus.item);
    if (!el) return;
    el.focus();
    setRefocus(null);
  });

  const ok = (n: Naming) => nameAvailable(files.statuses, n.value, n.from);
  const close = (focus: Focus | null) => {
    setRefocus(focus);
    setNaming(null);
  };
  const commit = () => {
    if (!naming || !ok(naming)) return;
    const { from, value } = naming;
    if (from === null) {
      close(null);
      void files.create(value).then((done) => done && onSelect(value));
      return;
    }
    close({ item: value });
    void files.rename(from, value).then((done) => {
      if (done) return;
      // Refused: the thumbnail keeps its name and gets the focus back.
      setRefocus({ item: from });
    });
  };
  const remove = async (name: string) => {
    const question = kind === "layouts"
      ? t("ui-layouts-confirm-delete", { name: widgetLabel(name) })
      : t("ui-widgets-confirm-delete", { name: widgetLabel(name) });
    if (!(await confirm({ message: question, ok: t("ui-confirm-delete-ok"), danger: true }))) return;
    const next = neighbour(order, name, () => true);
    if (!(await files.remove(name))) return;
    setRefocus(next !== null ? { item: next } : name !== COMMON && hasCommon ? "common" : "add");
    onRemoved(name);
  };
  const startRename = (name: string) => {
    if (name !== COMMON) setNaming({ from: name, value: name });
  };
  /** Opens `name`'s menu at the pointer, or at the element's corner from the keyboard. */
  const openMenu = (name: string, e: MouseEvent<HTMLElement>) => {
    e.preventDefault();
    const box = e.currentTarget.getBoundingClientRect();
    const keyboard = e.clientX === 0 && e.clientY === 0;
    setMenu({ name, x: keyboard ? box.left : e.clientX, y: keyboard ? box.bottom : e.clientY });
  };
  /** F2 renames; Delete or Backspace deletes, the window's keys never seeing them. */
  const onItemKey = (name: string, e: KeyboardEvent<HTMLButtonElement>) => {
    if (e.key === "F2") startRename(name);
    else if (e.key === "Delete" || e.key === "Backspace") void remove(name);
    else return;
    e.preventDefault();
    e.stopPropagation();
  };

  const issue = naming && nameIssue(files.statuses, naming.value, naming.from);
  const empty = items.length === 0 && !naming;
  const commonLabel = widgetLabel(COMMON);
  const deleteLabel = (name: string) => t("ui-widgets-delete-named", { name: widgetLabel(name) });
  const unsaved = new Set([...files.unsaved].map((n) => itemSlide(kind, n)).filter((id): id is string => id !== null));

  return (
    <div className="pack-pile" ref={box}>
      {hasCommon && (
        <div className="pack-pile-common">
          <button
            ref={commonRow}
            className="pack-pile-item"
            aria-current={selected === COMMON}
            onClick={() => onSelect(COMMON)}
            onKeyDown={(e) => onItemKey(COMMON, e)}
            onContextMenu={(e) => openMenu(COMMON, e)}
          >
            {commonLabel}
            {files.unsaved.has(COMMON) && <span className="unsaved-dot" role="img" aria-label={t("ui-widgets-unsaved")} title={t("ui-widgets-unsaved")} />}
          </button>
          <IconButton className="token ghost pack-pile-remove" label={deleteLabel(COMMON)} icon={Icon.trash} onClick={() => void remove(COMMON)} />
        </div>
      )}
      {naming && (
        <form className="pack-pile-name" onSubmit={(e) => { e.preventDefault(); commit(); }}>
          <input
            className="select"
            autoFocus
            aria-label={t(kind === "layouts" ? "ui-layouts-name" : "ui-widgets-name")}
            aria-invalid={naming.value !== "" && !ok(naming)}
            aria-describedby={issue ? issueId : undefined}
            value={naming.value}
            // Brought to a name's form as typed: « Carte » becomes `carte`.
            onChange={(e) => setNaming({ ...naming, value: widgetSlug(e.target.value) })}
            onKeyDown={(e) => {
              if (e.key !== "Escape") return;
              e.stopPropagation();
              close(naming.from === null ? "add" : { item: naming.from });
            }}
          />
          {issue && <p id={issueId} className="pack-pile-issue" role="status">{t(issue)}</p>}
        </form>
      )}
      <SlidePile
        slides={items.map((i) => i.slide)}
        current={selected === null ? null : itemSlide(kind, selected)}
        svg={svg}
        names={Object.fromEntries(items.map((i) => [i.slide.id, i.name]))}
        label={t(kind === "layouts" ? "ui-pile-layouts" : "ui-widget-picker-list")}
        addLabel={t(kind === "layouts" ? "ui-layouts-new" : "ui-widgets-new")}
        hint={t("ui-widgets-rename-hint")}
        unsaved={unsaved}
        onSelect={(id) => {
          const name = slideItem(id);
          if (name !== null) onSelect(name);
        }}
        onThumbKey={(id, e) => {
          const name = slideItem(id);
          if (name !== null) onItemKey(name, e);
        }}
        onThumbDoubleClick={(id) => {
          const name = slideItem(id);
          if (name !== null) startRename(name);
        }}
        onThumbContextMenu={(id, e) => {
          const name = slideItem(id);
          if (name !== null) openMenu(name, e);
        }}
        removeLabel={(id) => deleteLabel(slideItem(id) ?? id)}
        canRemove
        onRemove={(id) => {
          const name = slideItem(id);
          if (name !== null) void remove(name);
        }}
      />
      {empty && <p className="caption pack-pile-empty">{t(kind === "layouts" ? "ui-layouts-empty" : "ui-widgets-empty")}</p>}
      <IconButton
        ref={addButton}
        className="object token pack-pile-add"
        label={t(kind === "layouts" ? "ui-layouts-new" : "ui-widgets-new")}
        icon={Icon.add}
        disabled={files.statuses === null}
        onClick={() => setNaming({ from: null, value: "" })}
      />
      {menu && (
        <ContextMenu x={menu.x} y={menu.y} above={false} label={t("ui-pile-menu", { name: widgetLabel(menu.name) })} onClose={() => setMenu(null)}>
          {menu.name !== COMMON && (
            <button role="menuitem" onClick={() => { setMenu(null); startRename(menu.name); }}>{t("ui-pile-menu-rename")}</button>
          )}
          <button role="menuitem" onClick={() => { setMenu(null); void remove(menu.name); }}>{t("ui-pile-menu-delete")}</button>
        </ContextMenu>
      )}
    </div>
  );
}
