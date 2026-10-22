/** Section « Widgets » de la pile : ouvrir, créer, renommer, supprimer. */
import { useEffect, useId, useRef, useState } from "react";
import { t } from "./i18n";
import { Icon } from "./icons";
import PileSection from "./PileSection";
import type { WidgetFiles } from "./useWidgetFiles";
import { COMMON, nameAvailable, nameIssue, stateOf, widgetLabel, widgetSlug } from "./widgetList";

export interface PileWidgetsProps {
  files: WidgetFiles;
  /** Section ouverte (gardé par l'hôte) ; `onOpenChange` la replie ou la rouvre. */
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Fichier du sous-onglet actif ; `null` : la scène. */
  current: string | null;
  onOpen: (name: string) => void;
  /** Après une suppression réussie (le sous-onglet se ferme). */
  onRemoved: (name: string) => void;
  /** Confirmation (injectable pour les tests) ; défaut : `window.confirm`. */
  confirm?: (message: string) => boolean;
}

export default function PileWidgets({ files, open, onOpenChange, current, onOpen, onRemoved, confirm = (m) => window.confirm(m) }: PileWidgetsProps) {
  const [naming, setNaming] = useState<{ from: string | null; value: string } | null>(null);
  const issueId = useId();
  const rows = useRef(new Map<string, HTMLButtonElement>());
  /** Ligne à refocaliser après le champ du nom. */
  const refocus = useRef<string | null>(null);
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

  const issue = naming && nameIssue(files.statuses, naming.value, naming.from);
  const field = naming && (
    <form className="pile-widgets-name" onSubmit={(e) => { e.preventDefault(); commit(); }}>
      <input
        className="select"
        autoFocus
        aria-label={t("ui-widgets-name")}
        aria-invalid={naming.value !== "" && !ok(naming)}
        aria-describedby={issue ? issueId : undefined}
        value={naming.value}
        // Ramené à la forme d'un nom à la frappe : « Carte » devient `carte`.
        onChange={(e) => setNaming({ ...naming, value: widgetSlug(e.target.value) })}
        onKeyDown={(e) => {
          if (e.key !== "Escape") return;
          e.stopPropagation();
          close(naming.from);
        }}
      />
      {issue && <p id={issueId} className="pile-widgets-issue" role="status">{t(issue)}</p>}
    </form>
  );

  return (
    <PileSection
      className="pile-widgets"
      title={t("ui-widgets-section")}
      label={t("ui-widgets-files")}
      open={open}
      onToggle={() => onOpenChange(!open)}
      actions={
        <button
          className="token ghost"
          aria-label={t("ui-widgets-new")}
          title={t("ui-widgets-new")}
          disabled={files.statuses === null}
          // Nommer un widget dans une section repliée : on la rouvre.
          onClick={() => { onOpenChange(true); setNaming({ from: null, value: "" }); }}
        >
          {Icon.add}
        </button>
      }
    >
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
      {files.statuses?.length === 0 && !naming && <p className="caption">{t("ui-widgets-empty")}</p>}
    </PileSection>
  );
}
