/**
 * Sous-onglet d'un fichier de widget : aperçu et diagnostics en haut, éditeur dessous.
 *
 * L'interrupteur de la barre d'outils affiche le code (masqué par défaut, choix
 * commun gardé par l'hôte) : masqué, l'aperçu est centré ; affiché, le code se
 * place dessous ou à droite (repli dessous faute de largeur), à la taille réglée
 * au séparateur. Masqué, l'éditeur reste monté (inerte) : frappes et
 * historique demeurent.
 * Pour Commun, les widgets en erreur à la place de l'aperçu. Caché quand il
 * n'est pas actif, il reste monté : l'éditeur garde son historique.
 * L'aperçu reste dans un `<img>` : un SVG de widget ne s'y exécute pas.
 */
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { EditorView } from "@codemirror/view";
import type { TabApi, WidgetPreview } from "./api";
import { t } from "./i18n";
import CodeDivider from "./CodeDivider";
import { type CodeAxis, codeTrack } from "./codeSplit";
import TypstEditor, { type EditorCommands, type EditorHistory } from "./TypstEditor";
import type { CodeLayout } from "./useViewOptions";
import type { WidgetFiles } from "./useWidgetFiles";
import { croppedSvg } from "./widgetPreview";
import { COMMON, diagnosticPlace, fileDiagnostics, listedDiagnostics, stateOf, widgetLabel } from "./widgetList";

export interface WidgetTabProps {
  api: TabApi;
  files: WidgetFiles;
  /** Id stable du sous-onglet (`OpenWidget.id`), pour les ids ARIA. */
  id: number;
  /** Nom courant du fichier ; un renommage le change sans remonter le composant. */
  name: string;
  revision: string;
  /** Révision des essais (commune au pack) : l'aperçu se relance quand elle change. */
  trials: number;
  /** Seul l'actif est visible, relit sa source et demande un aperçu. */
  active: boolean;
  /** Code source affiché ; masqué, l'éditeur reste monté mais inerte. */
  code: boolean;
  /** Place du code et tailles choisies, communes à tous les sous-onglets. */
  layout: CodeLayout;
  /** Retient une nouvelle taille réglée au séparateur. */
  onLayout: (layout: CodeLayout) => void;
  /** Commun : ouvre le widget cliqué. */
  onOpen: (name: string) => void;
  onCommands: (c: EditorCommands | null) => void;
  onHistory: (h: EditorHistory) => void;
}

export default function WidgetTab({ api, files, id, name, revision, trials, active, code, layout, onLayout, onOpen, onCommands, onHistory }: WidgetTabProps) {
  const [source, setSource] = useState<string | null>(null);
  const [editorKey, setEditorKey] = useState(0);
  const [preview, setPreview] = useState<WidgetPreview | null>(null);
  const host = useRef<HTMLElement>(null);
  const nameRef = useRef(name);
  const filesRef = useRef(files);
  useLayoutEffect(() => {
    nameRef.current = name;
    filesRef.current = files;
  });
  /** Revision dont la source est lue (ou due). */
  const readFor = useRef<string | null>(null);
  const shown = useRef(false);
  const previewSeq = useRef(0);

  // Source : au montage, puis à chaque revision quand actif (un caché la lit en devenant actif).
  useEffect(() => {
    if (readFor.current === revision || (!active && shown.current)) return;
    readFor.current = revision;
    let alive = true;
    let arrived = false;
    const file = nameRef.current;
    const asked = filesRef.current.edits(file);
    api
      .widgetSource(file)
      .then((s) => {
        arrived = true;
        if (!alive || (shown.current && filesRef.current.stale(file, asked, s))) return;
        shown.current = true;
        filesRef.current.loaded(file, s);
        setSource(s);
        setEditorKey((k) => k + 1);
      })
      .catch((e: unknown) => alive && filesRef.current.fail(e));
    return () => {
      alive = false;
      // Réponse perdue (onglet caché entre-temps) : la lecture reste due.
      if (!arrived) readFor.current = null;
    };
  }, [api, revision, active]);

  // Aperçu : actif seulement, après la source puis chaque écriture enregistrée.
  const written = files.writes.get(name) ?? 0;
  const ready = source !== null;
  useEffect(() => {
    if (!active || name === COMMON || !ready) return;
    const seq = ++previewSeq.current;
    api
      .previewWidget(name)
      .then((p) => {
        if (seq === previewSeq.current) setPreview(p);
      })
      .catch((e: unknown) => seq === previewSeq.current && filesRef.current.fail(e));
  }, [api, active, name, written, ready, editorKey, trials]);

  // Redevenu visible (onglet actif, code déplié) : l'éditeur caché se remesure.
  useEffect(() => {
    if (!active || !code) return;
    const el = host.current?.querySelector(".cm-content");
    if (el) EditorView.findFromDOM(el as HTMLElement)?.requestMeasure();
  }, [active, code]);

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

  /** Axe affiché : la feuille de style replie « à droite » dessous faute de largeur. */
  const axisOf = (el: HTMLElement): CodeAxis =>
    getComputedStyle(el).getPropertyValue("--code-axis").trim() === "right" ? "right" : "below";
  // Relu après chaque changement de place et quand le sous-onglet change de taille.
  const [axis, setAxis] = useState<CodeAxis>("below");
  useLayoutEffect(() => {
    const el = host.current;
    if (!el || !code) return;
    const read = () => setAxis(axisOf(el));
    read();
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(read);
    watch.observe(el);
    return () => watch.disconnect();
  }, [code, layout.code_place]);
  /** Axe affiché et tailles mesurées, au début d'un geste. */
  const measure = () => {
    const el = host.current;
    const codeEl = el?.querySelector<HTMLElement>(".widget-tab-code");
    if (!el || !codeEl) return { axis, total: 0, size: 0 };
    const shown = axisOf(el);
    return shown === "right"
      ? { axis: shown, total: el.clientWidth, size: codeEl.offsetWidth }
      : { axis: shown, total: el.clientHeight, size: codeEl.offsetHeight };
  };
  /** Taille de l'axe affiché, gardée par l'hôte sous `code_height` ou `code_width`. */
  const sizeKey = (axis: CodeAxis) => (axis === "right" ? "code_width" : "code_height");
  /** Pendant le glisser, la taille ne touche que le DOM. */
  const show = (axis: CodeAxis, size: number) =>
    host.current?.style.setProperty(axis === "right" ? "--code-w" : "--code-h", codeTrack(size));
  const tracks = { "--code-h": codeTrack(layout.code_height) ?? undefined, "--code-w": codeTrack(layout.code_width) ?? undefined };

  const inError = (files.statuses ?? []).filter((s) => s.name !== COMMON && stateOf(s) === "errors");

  return (
    <section
      ref={host}
      className="widget-tab"
      hidden={!active}
      role="tabpanel"
      id={`widget-panel-${id}`}
      aria-labelledby={`widget-tab-${id}`}
      data-code={code ? "shown" : "folded"}
      data-place={layout.code_place}
      style={tracks as React.CSSProperties}
    >
      <aside className="widget-tab-side" aria-label={t(name === COMMON ? "ui-widgets-in-error" : "ui-widgets-preview")}>
        {name === COMMON ? (
          <div className="widget-tab-broken">
            <h3>{t("ui-widgets-in-error")}</h3>
            {inError.length === 0 ? (
              <p className="caption">{t("ui-widgets-none-in-error")}</p>
            ) : (
              <ul>
                {inError.map((s) => (
                  <li key={s.name}>
                    <button className="btn" onClick={() => onOpen(s.name)}>
                      {s.name}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ) : previewUrl ? (
          <img src={previewUrl} alt={t("ui-widgets-preview")} />
        ) : (
          <p className="caption">{t("ui-widgets-no-preview")}</p>
        )}
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
      </aside>
      {code && (
        <CodeDivider
          axis={axis}
          label={t("ui-divider-code")}
          measure={measure}
          onResize={(size, done) => {
            const shown = measure().axis;
            if (done) onLayout({ ...layout, [sizeKey(shown)]: size });
            else show(shown, size);
          }}
          onReset={() => onLayout({ ...layout, [sizeKey(measure().axis)]: null })}
        />
      )}
      <div className="widget-tab-code" id={`widget-code-${id}`} inert={!code}>
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
    </section>
  );
}
