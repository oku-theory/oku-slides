/** Section « Widgets » de la pile : ouvrir, créer, renommer, supprimer. */
import { useEffect, useRef, useState } from "react";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { WidgetFiles } from "./useWidgetFiles";
import { COMMON, nameAvailable, stateOf, widgetLabel } from "./widgetList";

export interface PileWidgetsProps {
  files: WidgetFiles;
  /** Fichier du sous-onglet actif ; `null` : « Modèles ». */
  current: string | null;
  onOpen: (name: string) => void;
  /** Après une suppression réussie (le sous-onglet se ferme). */
  onRemoved: (name: string) => void;
  /** Change : le focus va au titre de la section (☰ « Éditer les widgets »). */
  focus: number;
  /** Confirmation (injectable pour les tests) ; défaut : `window.confirm`. */
  confirm?: (message: string) => boolean;
}

export default function PileWidgets({ files, current, onOpen, onRemoved, focus, confirm = (m) => window.confirm(m) }: PileWidgetsProps) {
  const [naming, setNaming] = useState<{ from: string | null; value: string } | null>(null);
  const title = useRef<HTMLHeadingElement>(null);
  const rows = useRef(new Map<string, HTMLButtonElement>());
  /** Ligne à refocaliser après le champ du nom. */
  const refocus = useRef<string | null>(null);
  useEffect(() => {
    if (focus > 0) title.current?.focus();
  }, [focus]);
  useEffect(() => {
    if (naming || !refocus.current) return;
    // Un renommage n'a pas encore sa ligne : on attend le rendu qui la porte.
    const target = rows.current.get(refocus.current);
    if (!target) return;
    target.focus();
    refocus.current = null;
  });
  const ok = (n: { from: string | null; value: string }) => nameAvailable(files.statuses, n.value, n.from);

  const close = (focusOn: string | null) => {
    refocus.current = focusOn;
    setNaming(null);
  };
  const commit = () => {
    if (!naming || !ok(naming)) return;
    const { from, value } = naming;
    close(from ? value : null);
    if (from) {
      void files.rename(from, value).then((done) => {
        if (done) return;
        // Refusé : la ligne garde son nom, le focus lui revient.
        refocus.current = null;
        rows.current.get(from)?.focus();
      });
    }
    else void files.create(value).then((done) => done && onOpen(value));
  };
  const remove = (name: string) => {
    if (!confirm(t("ui-widgets-confirm-delete", { name: widgetLabel(name) }))) return;
    void files.remove(name).then((done) => done && onRemoved(name));
  };
  const startRename = (name: string) => {
    if (name !== COMMON) setNaming({ from: name, value: name });
  };

  const field = naming && (
    <form className="pile-widgets-name" onSubmit={(e) => { e.preventDefault(); commit(); }}>
      <input
        className="select"
        autoFocus
        aria-label={t("ui-widgets-name")}
        aria-invalid={naming.value !== "" && !ok(naming)}
        value={naming.value}
        onChange={(e) => setNaming({ ...naming, value: e.target.value })}
        onKeyDown={(e) => {
          if (e.key !== "Escape") return;
          e.stopPropagation();
          close(naming.from);
        }}
      />
    </form>
  );

  return (
    <section className="pile-widgets" aria-label={t("ui-widgets-files")}>
      <div className="pile-widgets-head">
        <h3 ref={title} tabIndex={-1}>{t("ui-widgets-section")}</h3>
        <button
          className="token ghost"
          aria-label={t("ui-widgets-new")}
          title={t("ui-widgets-new")}
          disabled={files.statuses === null}
          onClick={() => setNaming({ from: null, value: "" })}
        >
          {Icon.add}
        </button>
      </div>
      {naming?.from === null && field}
      <ul>
        {(files.statuses ?? []).map((s) => {
          if (naming?.from === s.name) return <li key={s.name}>{field}</li>;
          const state = stateOf(s);
          const label = widgetLabel(s.name);
          return (
            <li key={s.name} className="pile-widgets-row">
              <button
                ref={(el) => { if (el) rows.current.set(s.name, el); else rows.current.delete(s.name); }}
                className="pile-widgets-file"
                aria-current={s.name === current}
                title={s.name === COMMON ? undefined : t("ui-widgets-rename-hint")}
                onClick={() => onOpen(s.name)}
                onDoubleClick={() => startRename(s.name)}
                onKeyDown={(e) => {
                  if (e.key !== "F2") return;
                  e.preventDefault();
                  startRename(s.name);
                }}
              >
                <span className={`pill ${state}`} role="img" aria-label={t(state === "valid" ? "ui-widgets-valid" : "ui-widgets-errors")} />
                {label}
                {files.unsaved.has(s.name) && <span className="unsaved-dot" role="img" aria-label={t("ui-widgets-unsaved")} title={t("ui-widgets-unsaved")} />}
              </button>
              <button className="token ghost pile-widgets-remove" aria-label={t("ui-widgets-delete-named", { name: label })} title={t("ui-widgets-delete-named", { name: label })} onClick={() => remove(s.name)}>
                {Icon.trash}
              </button>
            </li>
          );
        })}
      </ul>
      {files.statuses?.length === 0 && <p className="caption">{t("ui-widgets-empty")}</p>}
    </section>
  );
}
