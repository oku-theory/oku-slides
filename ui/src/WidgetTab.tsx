/**
 * Sous-onglet d'un fichier de widget : l'aperçu en haut, sur toute la largeur ;
 * dessous, réglages et diagnostics à gauche, éditeur à droite.
 *
 * L'interrupteur de la barre d'outils affiche le code (masqué par défaut, choix
 * commun gardé par l'hôte) : masqué, les réglages prennent toute la largeur
 * sous l'aperçu ; affiché, le code se place à leur droite, à la largeur réglée
 * au séparateur. Masqué, l'éditeur reste monté (inerte) : frappes et
 * historique demeurent.
 * L'aperçu montre le widget à ses valeurs d'exemple, gardées dans le pack.
 * Avec sa vue, l'aperçu est une scène (`Stage` en aperçu) : ses champs texte
 * s'éditent sur place, comme sur la slide, et s'écrivent dans ces valeurs
 * (un champ lié au deck reste en lecture) ; ses listes se changent
 * dans ces valeurs (Cmd/Ctrl+Alt+flèche sur un item), jamais dans le deck.
 * Sans vue (échec), l'image seule.
 * Pour Commun, les widgets en erreur à la place de l'aperçu. Caché quand il
 * n'est pas actif, il reste monté : l'éditeur garde son historique.
 * L'aperçu reste dans un `<img>` : un SVG de widget ne s'y exécute pas.
 */
import { type ReactNode, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { EditorView } from "@codemirror/view";
import type { FieldView, Snapshot, TabApi, WidgetPreview } from "./api";
import { fieldValue } from "./deck";
import { t } from "./i18n";
import { Button } from "./primitives";
import CodeDivider from "./CodeDivider";
import { codeTrack } from "./codeSplit";
import TypstEditor, { type EditorCommands, type EditorHistory } from "./TypstEditor";
import type { WidgetFiles } from "./useWidgetFiles";
import Stage from "./Stage";
import { useFieldEditing } from "./useFieldEditing";
import type { OnListOp } from "./stageItems";
import { type FieldKey, fieldKey } from "./stageFields";
import { croppedSvg, croppedView } from "./widgetPreview";
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
  /** Empreinte du style du deck (thème, couleurs) : l'aperçu, rendu à ce style, se relance quand elle change. */
  look: string;
  /** Seul l'actif est visible, relit sa source et demande un aperçu. */
  active: boolean;
  /** Code source affiché ; masqué, l'éditeur reste monté mais inerte. */
  code: boolean;
  /** Largeur du code choisie, commune à tous les sous-onglets ; `null` : proportion par défaut. */
  codeWidth: number | null;
  /** Retient une nouvelle largeur réglée au séparateur. */
  onCodeWidth: (width: number | null) => void;
  /** Réglages de l'hôte (valeurs d'exemple du widget), sous l'aperçu à gauche du code ; `null` : aucun. */
  settings?: ReactNode;
  /** Envoie une écriture des valeurs d'exemple (refus en toast) ; défaut : le refus passe par les fichiers. */
  run?: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  /** Couleurs des pastilles dans l'éditeur d'un champ (palette des graphiques du thème). */
  palette?: string[];
  /** Commun : ouvre le widget cliqué. */
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

export default function WidgetTab({
  api, files, id, name, revision, trials, look, active, code, codeWidth, onCodeWidth, settings = null,
  run, palette = NO_PALETTE, onOpen, onCommands, onHistory,
}: WidgetTabProps) {
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
        if (seq !== previewSeq.current) return;
        setPreview(p);
        // Demandé après l'accord des écritures : l'aperçu montre leurs valeurs (ou ce que l'hôte en a fait).
        setCommitted((m) => {
          const left = new Map([...m].filter(([, c]) => c.acked === null || c.acked >= seq));
          return left.size === m.size ? m : left;
        });
      })
      .catch((e: unknown) => seq === previewSeq.current && filesRef.current.fail(e));
    // `trials` et `look` ne sont pas lus : ils relancent l'aperçu quand les essais ou le style du deck changent.
  }, [api, active, name, written, ready, editorKey, trials, look]);

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

  /** Largeurs du sous-onglet (la rangée du bas en prend toute la largeur) et du code, au début d'un geste. */
  const measure = () => {
    const el = host.current;
    const codeEl = el?.querySelector<HTMLElement>(".widget-tab-code");
    return el && codeEl ? { total: el.clientWidth, size: codeEl.offsetWidth } : { total: 0, size: 0 };
  };
  /** Pendant le glisser, la largeur ne touche que le DOM. */
  const show = (size: number) => host.current?.style.setProperty("--code-w", codeTrack(size));
  const tracks = { "--code-w": codeTrack(codeWidth) ?? undefined };

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
      style={tracks as React.CSSProperties}
    >
      <section className="widget-tab-preview" aria-label={t(name === COMMON ? "ui-widgets-in-error" : "ui-widgets-preview")}>
        {name === COMMON ? (
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
      <div className="widget-tab-side">
        {settings}
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
      {code && (
        <CodeDivider
          label={t("ui-divider-code")}
          measure={measure}
          onResize={(size, done) => (done ? onCodeWidth(size) : show(size))}
          onReset={() => onCodeWidth(null)}
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
