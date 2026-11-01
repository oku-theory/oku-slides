/**
 * The view of a pack tab's selected item: its preview, then its settings and its code as two tabs,
 * then the diagnostics.
 *
 * The Settings tab opens by default; the open tab is a choice the host keeps for every item. With
 * the Settings tab open, the editor stays mounted (inert, hidden): typing and history remain while
 * the item stays selected. Commun has no settings: its code shows, with no tabs.
 * The preview shows the widget at its example values, kept in the pack. With its view, the
 * preview is a stage (`Stage` in preview mode): its text fields are edited in place, as on a
 * slide, and written to those values (a field bound to the deck stays read-only); its lists
 * change in those values (Cmd/Ctrl+Alt+arrow on an item), never in the deck. Without a view
 * (failure), the image alone. For Commun, the widgets in error instead of the preview.
 * The preview stays in an `<img>`: a widget SVG never runs there.
 *
 * Mounted once per selected item (keyed by name): a mount whose file still has a write pending
 * waits for the queue, then starts from the text last refused, if any, else the saved source.
 */
import { type ReactNode, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { EditorView } from "@codemirror/view";
import type { FieldView, PreviewSpan, Snapshot, TabApi, WidgetPreview } from "./api";
import { fieldValue } from "./deck";
import { t } from "./i18n";
import { Button } from "./primitives";
import TypstEditor, { type EditorCommands, type EditorHistory } from "./TypstEditor";
import type { WidgetFiles } from "./useWidgetFiles";
import Stage from "./Stage";
import { useFieldEditing } from "./useFieldEditing";
import type { OnListOp } from "./stageItems";
import { type FieldKey, fieldKey } from "./stageFields";
import { croppedSvg, croppedView } from "./widgetPreview";
import { COMMON, diagnosticPlace, fileDiagnostics, listedDiagnostics, stateOf, widgetLabel } from "./widgetList";

/** What the selected item's view shows and writes. */
export interface WidgetTabProps {
  api: TabApi;
  files: WidgetFiles;
  /** The item's file name; a rename mounts the view again under the new name. */
  name: string;
  revision: string;
  /** The pack's trial revision: the preview is asked again when it changes. */
  trials: number;
  /** The deck style's fingerprint (theme, colours): the preview, rendered in it, follows it. */
  look: string;
  /** The preview slot of an element widget. */
  span: PreviewSpan;
  /** The Code tab is open; with the Settings tab open, the editor stays mounted but inert. */
  code: boolean;
  /** Opens the Code tab (`true`) or the Settings tab. */
  onCode: (code: boolean) => void;
  /** The host's settings (the widget's example values), in the Settings tab. */
  settings?: ReactNode;
  /** Sends an example value write (refusal toasted); default: the refusal goes through the files. */
  run?: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  /** Swatch colours in a field's editor (the theme's chart palette). */
  palette?: string[];
  /** Each preview received, with the item's name (the strip's thumbnail can follow it). */
  onPreview?: (name: string, preview: WidgetPreview) => void;
  /** Commun: selects the widget clicked in its list of widgets in error. */
  onOpen: (name: string) => void;
  onCommands: (c: EditorCommands | null) => void;
  onHistory: (h: EditorHistory) => void;
}

const NO_PALETTE: string[] = [];

/**
 * Valeur validée d'un champ que l'aperçu ne montre pas encore. `acked` : dernier aperçu demandé avant
 * l'accord de l'écriture (`null` tant qu'elle est en route) ; un aperçu demandé après la remplace.
 */
interface Committed {
  text: string;
  acked: number | null;
}

/** The selected item's preview, settings, diagnostics and editor. */
export default function WidgetTab({
  api, files, name, revision, trials, look, span, code, onCode, settings = null,
  run, palette = NO_PALETTE, onPreview, onOpen, onCommands, onHistory,
}: WidgetTabProps) {
  const [source, setSource] = useState<string | null>(null);
  const [editorKey, setEditorKey] = useState(0);
  const [preview, setPreview] = useState<WidgetPreview | null>(null);
  const host = useRef<HTMLElement>(null);
  const tabs = useRef<HTMLDivElement>(null);
  const common = name === COMMON;
  /** The code shows: its tab is open, or the item is Commun, which has no settings. */
  const coding = code || common;
  const nameRef = useRef(name);
  const filesRef = useRef(files);
  const onPreviewRef = useRef(onPreview);
  useLayoutEffect(() => {
    nameRef.current = name;
    filesRef.current = files;
    onPreviewRef.current = onPreview;
  });
  /** Revision dont la source est lue (ou due). */
  const readFor = useRef<string | null>(null);
  const shown = useRef(false);
  const previewSeq = useRef(0);

  // Source: on mount, then at each revision. A mount whose file still has a write pending (unsent,
  // in flight or refused) waits for the queue, then shows the text last refused, else the saved source.
  useEffect(() => {
    if (readFor.current === revision) return;
    readFor.current = revision;
    let alive = true;
    let arrived = false;
    const file = nameRef.current;
    const first = !shown.current;
    const settled = first && filesRef.current.pending(file) ? filesRef.current.settle() : Promise.resolve();
    settled
      .then(async () => {
        if (!alive) return null;
        const tried = first ? filesRef.current.attempted(file) : undefined;
        if (tried !== undefined) return { text: tried, asked: null };
        const asked = filesRef.current.edits(file);
        return { text: await api.widgetSource(file), asked };
      })
      .then((got) => {
        arrived = true;
        if (!alive || !got) return;
        const { text, asked } = got;
        if (asked !== null && shown.current && filesRef.current.stale(file, asked, text)) return;
        shown.current = true;
        if (asked !== null) filesRef.current.loaded(file, text);
        setSource(text);
        setEditorKey((k) => k + 1);
      })
      .catch((e: unknown) => alive && filesRef.current.fail(e));
    return () => {
      alive = false;
      // Answer lost (unmounted meanwhile): the read stays due.
      if (!arrived) readFor.current = null;
    };
  }, [api, revision]);

  // Preview: after the source, then after each saved write. Without a pack, as the statuses and
  // sources: the view exists only in a pack tab, where the host takes the edited pack.
  const written = files.writes.get(name) ?? 0;
  const ready = source !== null;
  useEffect(() => {
    if (name === COMMON || !ready) return;
    const seq = ++previewSeq.current;
    api
      .previewWidget(name, undefined, span)
      .then((p) => {
        if (seq !== previewSeq.current) return;
        setPreview(p);
        onPreviewRef.current?.(nameRef.current, p);
        // Demandé après l'accord des écritures : l'aperçu montre leurs valeurs (ou ce que l'hôte en a fait).
        setCommitted((m) => {
          const left = new Map([...m].filter(([, c]) => c.acked === null || c.acked >= seq));
          return left.size === m.size ? m : left;
        });
      })
      .catch((e: unknown) => seq === previewSeq.current && filesRef.current.fail(e));
    // `trials` et `look` ne sont pas lus : ils relancent l'aperçu quand les essais ou le style du deck changent.
  }, [api, name, written, ready, editorKey, trials, look, span]);

  // The code shown again: the editor, measured while hidden, measures again.
  useEffect(() => {
    if (!coding) return;
    const el = host.current?.querySelector(".cm-content");
    if (el) EditorView.findFromDOM(el as HTMLElement)?.requestMeasure();
  }, [coding]);

  const status = files.statuses?.find((s) => s.name === name);
  const diagnostics = useMemo(() => fileDiagnostics(status, name), [status, name]);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!preview?.svg) {
      setPreviewUrl(null);
      return;
    }
    // Un widget d'élément est cadré sur ce qu'il dessine, pas sur la slide entière.
    const svg = croppedSvg(preview.svg, preview.bbox ?? null);
    const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [preview]);

  // La scène : les champs du widget dans le repère de l'image recadrée.
  // Les vues portent le genre qualifié du widget (`espace/nom`), celui de son schéma ;
  // sans schéma, aucun champ sur l'aperçu : l'image seule.
  const kind = status?.schema?.name ?? null;
  const croppedScene = useMemo(
    () => (preview?.view && kind !== null ? croppedView(preview.view, preview.bbox ?? null, kind) : null),
    [preview, kind],
  );
  // Un champ rouvert avant le nouvel aperçu part de la valeur validée, pas de l'ancienne :
  // la refermer telle quelle n'annule pas l'écriture (t-524).
  const [committed, setCommitted] = useState<ReadonlyMap<FieldKey, Committed>>(() => new Map());
  const sceneView = useMemo(() => {
    if (!croppedScene || committed.size === 0) return croppedScene;
    const fields = croppedScene.fields.map((f) => {
      const c = committed.get(fieldKey(f));
      return c ? { ...f, text: c.text } : f;
    });
    return { ...croppedScene, fields };
  }, [croppedScene, committed]);
  const { editing, setEditing } = useFieldEditing(sceneView);
  const write = run ?? ((p: Promise<Snapshot>) => p.catch((e: unknown) => (filesRef.current.fail(e), null)));
  /** Écrit le brouillon d'un champ dans les valeurs d'exemple : texte, ou paragraphes d'un `rich`. */
  const onField = (f: FieldView, text: string) => {
    if (!sceneView) return;
    const value = fieldValue(sceneView, f, text);
    const key = fieldKey(f);
    const entry: Committed = { text, acked: null };
    setCommitted((m) => new Map(m).set(key, entry));
    /** Remplace l'entrée de cette écriture, si une plus récente ne l'a pas déjà remplacée. */
    const settle = (next: Committed | null) =>
      setCommitted((m) => {
        if (m.get(key) !== entry) return m;
        const out = new Map(m);
        if (next) out.set(key, next);
        else out.delete(key);
        return out;
      });
    // Un champ d'item de liste : sa colonne, dans la liste des valeurs d'exemple.
    void write(
      f.list && f.item
        ? api.editExampleList(name, f.list, { op: "set_field", item: f.item, field: f.column, value })
        : api.setExampleValue(name, f.field, value),
    ).then((snap) => settle(snap ? { text, acked: previewSeq.current } : null));
  };

  /** Un changement de liste de l'aperçu : le changement, dans la liste des valeurs d'exemple. */
  const onListOp: OnListOp = (_owner, list, change) => void write(api.editExampleList(name, list, change));

  /** Arrows, Home and End move to the other tab and open it (two tabs: any of them does). */
  const onTabKey = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) return;
    e.preventDefault();
    const next = e.key === "Home" ? false : e.key === "End" ? true : !code;
    onCode(next);
    tabs.current?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next ? 1 : 0]?.focus();
  };

  const inError = (files.statuses ?? []).filter((s) => s.name !== COMMON && stateOf(s) === "errors");

  return (
    <section
      ref={host}
      className="widget-tab"
      role="region"
      id="widget-panel"
      aria-label={widgetLabel(name)}
      data-code={coding ? "shown" : "folded"}
      data-item={common ? "common" : undefined}
    >
      <section className="widget-tab-preview" aria-label={t(common ? "ui-widgets-in-error" : "ui-widgets-preview")}>
        {common ? (
          <div className="widget-tab-broken">
            <h3>{t("ui-widgets-in-error")}</h3>
            {inError.length === 0 ? (
              <p className="caption">{t("ui-widgets-none-in-error")}</p>
            ) : (
              <ul>
                {inError.map((s) => (
                  <li key={s.name}>
                    <Button onClick={() => onOpen(s.name)}>
                      {s.name}
                    </Button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ) : sceneView ? (
          <Stage
            mode="preview"
            view={sceneView}
            svgUrl={previewUrl}
            editing={editing}
            onEdit={setEditing}
            onField={onField}
            onListOp={onListOp}
            palette={palette}
          />
        ) : previewUrl ? (
          <img src={previewUrl} alt={t("ui-widgets-preview")} />
        ) : (
          <p className="caption">{t("ui-widgets-no-preview")}</p>
        )}
      </section>
      <div className="widget-tab-panel">
        {!common && (
          <div ref={tabs} className="widget-tab-tabs" role="tablist" aria-label={widgetLabel(name)} onKeyDown={onTabKey}>
            {[false, true].map((c) => (
              <button
                key={String(c)}
                type="button"
                role="tab"
                className="widget-tab-tab"
                id={c ? "widget-code-tab" : "widget-settings-tab"}
                aria-selected={code === c}
                aria-controls={c ? "widget-code" : "widget-settings"}
                tabIndex={code === c ? 0 : -1}
                onClick={() => onCode(c)}
              >
                {t(c ? "ui-widgets-source" : "ui-widgets-settings-tab")}
              </button>
            ))}
          </div>
        )}
        {!common && (
          <div className="widget-tab-side" id="widget-settings" role="tabpanel" aria-labelledby="widget-settings-tab" tabIndex={0} hidden={code}>
            {settings}
          </div>
        )}
        <div
          className="widget-tab-code"
          id="widget-code"
          role={common ? undefined : "tabpanel"}
          aria-labelledby={common ? undefined : "widget-code-tab"}
          inert={!coding}
        >
          {source !== null && (
            <TypstEditor
              key={editorKey}
              text={source}
              rewrite={files.rewrites.get(name) ?? null}
              diagnostics={diagnostics}
              diagnosticsFor={files.checked.get(name) ?? null}
              label={widgetLabel(name)}
              onChange={(text) => files.edit(name, text)}
              onCommands={onCommands}
              onHistory={onHistory}
            />
          )}
        </div>
        <ul className="widget-tab-diagnostics">
          {listedDiagnostics(status, preview?.diagnostics ?? []).map((d, i) => {
            const place = diagnosticPlace(d);
            return (
              <li key={i} className={d.error ? "error" : undefined}>
                {place ? `${place} ` : ""}
                {d.message}
              </li>
            );
          })}
        </ul>
      </div>
    </section>
  );
}
