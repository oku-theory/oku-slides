// Dialogue d'import d'une présentation PowerPoint : copie fidèle (hi-fi) ou sur mes layouts
// (retrofit, avec ses packs de layouts et son thème), et peaufinage par l'agent une fois l'onglet prêt.

import { useEffect, useRef, useState } from "react";
import { errorText, type ImportTarget, type PackInfo, type TabApi, type ThemeInfo } from "./api";
import { ESSENTIEL_NAMESPACE } from "./builtinKinds";
import { t } from "./i18n";
import { fileName } from "./importFile";
import { useModalFrame } from "./modalFrame";
import { packOptionLabel } from "./packOps";
import { Button, Field, Select } from "./primitives";
import type { ImportChoice } from "./useImportJob";

interface Props {
  /** Le `.pptx` choisi par « Ouvrir ». */
  path: string;
  /** Requêtes de l'onglet d'où l'on importe : les packs de la bibliothèque. */
  api: Pick<TabApi, "packs">;
  /** Thèmes de l'onglet (snapshot) : ceux de la bibliothèque sont proposés. */
  themes: ThemeInfo[];
  /** L'import tourne (tenu par l'espace de travail : il continue si le dialogue se ferme). */
  busy: boolean;
  /** Les choix de l'import qui tourne (dialogue rouvert) ; absent : les choix par défaut. */
  initial?: ImportChoice;
  /** Le dernier refus de l'hôte, localisé. */
  error: string | null;
  /** Pourquoi aucun agent n'est utilisable, localisé ; `null` : utilisable (ou pas encore su). */
  agentUnusable: string | null;
  /** Lance l'import ; l'espace de travail suit la suite. */
  onImport: (target: ImportTarget, tune: boolean) => void;
  onClose: () => void;
}

/**
 * La fenêtre d'import, sur un voile qui couvre l'app.
 *
 * Dans le cadre de `useModalFrame` : focus pris et gardé, Échap ferme, aucune touche ne lance
 * l'import, clics des premières 500 ms ignorés (le clic qui a choisi le fichier). Pendant
 * l'import, les choix sont figés ; Annuler et Échap ferment le dialogue sans arrêter l'import,
 * dont l'espace de travail montre l'issue.
 */
export default function ImportDialog({
  path,
  api,
  themes,
  busy,
  initial,
  error,
  agentUnusable,
  onImport,
  onClose,
}: Props) {
  const [mode, setMode] = useState<ImportTarget["mode"]>(initial?.target.mode ?? "hifi");
  // Les packs de layouts qui ont des profils : les seuls que le retrofit sait viser.
  const [packs, setPacks] = useState<PackInfo[] | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  // Ids des packs cochés, envoyés dans l'ordre de la liste : l'hôte lit un nom avant un espace de
  // noms, seul l'id désigne sûrement le pack. `null` : le défaut (`essentiel`), posé à l'arrivée de la liste.
  const [checked, setChecked] = useState<string[] | null>(
    initial?.target.mode === "retrofit" ? initial.target.layouts : null,
  );
  // `""` : le thème par défaut de la bibliothèque.
  const [theme, setTheme] = useState(initial?.target.mode === "retrofit" ? (initial.target.theme ?? "") : "");
  const [tune, setTune] = useState(initial?.tune ?? true);
  const ref = useRef<HTMLDivElement>(null);
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: onClose });

  useEffect(() => {
    let live = true;
    api
      .packs("layouts")
      .then((list) => {
        if (!live) return;
        const usable = list.filter((p) => p.profiles > 0);
        setPacks(usable);
        setChecked((c) => c ?? usable.filter((p) => p.namespace === ESSENTIEL_NAMESPACE).map((p) => p.id));
      })
      .catch((e) => live && setListError(errorText(e)));
    return () => {
      live = false;
    };
  }, [api]);

  // Le bouton qui avait le focus se désactive : la fenêtre le garde, Tab repart de ses bords.
  useEffect(() => {
    if (busy) ref.current?.focus();
  }, [busy]);

  const layouts = (packs ?? []).map((p) => p.id).filter((id) => checked?.includes(id));
  const toggle = (id: string, on: boolean) => setChecked((c) => (on ? [...(c ?? []), id] : (c ?? []).filter((x) => x !== id)));
  const ready = mode === "hifi" || layouts.length > 0;
  // Sans agent utilisable, le peaufinage est décoché sans perdre le choix, rendu s'il revient.
  // Pendant l'import, la case figée montre ce qui a été envoyé (le choix de l'import qui tourne).
  const tuning = busy ? (initial?.tune ?? tune) : tune && !agentUnusable;
  const submit = () => {
    if (busy || !ready) return;
    onImport(mode === "hifi" ? { mode: "hifi" } : { mode: "retrofit", layouts, theme: theme || null }, tuning);
  };
  const tuneNote = agentUnusable
    ? t("ui-import-tune-unavailable", { reason: agentUnusable })
    : t(mode === "hifi" ? "ui-import-tune-note" : "ui-import-tune-note-retrofit");
  // « Thème par défaut » tient lieu du thème par défaut ; un thème du deck seul n'est pas dans la bibliothèque.
  const choices = themes.filter((th) => !th.default && !th.deck_only);

  const modeChoice = (value: ImportTarget["mode"], name: string, note: string) => (
    <label className={`model${mode === value ? " chosen" : ""}`}>
      <input type="radio" name="import-mode" value={value} checked={mode === value} onChange={() => setMode(value)} />
      <span className="model-name">{name}</span>
      <span className="caption">{note}</span>
    </label>
  );

  return (
    <div className="import-veil">
      <div
        ref={ref}
        tabIndex={-1}
        className="import-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="import-title"
        aria-describedby="import-file"
        aria-busy={busy}
        data-armed={armed || undefined}
        onPointerDown={(e) => e.stopPropagation()}
        onClickCapture={onClickCapture}
      >
        <h2 id="import-title" className="settings-title">{t("ui-import-title")}</h2>
        <p id="import-file" className="caption import-file">{fileName(path)}</p>
        <fieldset className="import-modes" disabled={busy}>
          <legend className="caption">{t("ui-import-mode")}</legend>
          <div className="model-list">
            {modeChoice("hifi", t("ui-import-hifi"), t("ui-import-hifi-note"))}
            {modeChoice("retrofit", t("ui-import-retrofit"), t("ui-import-retrofit-note"))}
          </div>
        </fieldset>
        {mode === "retrofit" && (
          <div className="import-target">
            <fieldset className="import-layouts" disabled={busy}>
              <legend className="caption">{t("ui-import-layout-packs")}</legend>
              {(packs ?? []).map((p) => (
                <label key={p.id} className="check">
                  <input type="checkbox" checked={!!checked?.includes(p.id)} onChange={(e) => toggle(p.id, e.target.checked)} />
                  <span>{packOptionLabel(p)}</span>
                </label>
              ))}
            </fieldset>
            {packs?.length === 0 && <p className="caption import-empty">{t("ui-import-no-packs")}</p>}
            {listError && <p className="import-error" role="alert">{listError}</p>}
            <Field variant="row" label={t("ui-toolbar-theme")}>
              <Select value={theme} disabled={busy} onChange={setTheme}
                options={[{ value: "", label: t("ui-import-theme-default") }, ...choices.map((th) => ({ value: th.id, label: th.name }))]} />
            </Field>
          </div>
        )}
        <label className="check import-tune">
          <input
            type="checkbox"
            checked={tuning}
            disabled={busy || !!agentUnusable}
            aria-describedby="import-tune-note"
            onChange={(e) => setTune(e.target.checked)}
          />
          <span>{t("ui-import-tune")}</span>
        </label>
        <p id="import-tune-note" className="caption">{tuneNote}</p>
        {error && <p className="import-error" role="alert">{error}</p>}
        <div className="import-actions">
          {busy && <span className="caption" role="status">{t("ui-import-busy")}</span>}
          <Button ghost onClick={onClose}>{t("ui-close-cancel")}</Button>
          <Button primary disabled={busy || !ready} onClick={submit}>{t("ui-import-submit")}</Button>
        </div>
      </div>
    </div>
  );
}
