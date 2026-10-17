// Dialogue d'import d'une présentation PowerPoint : copie fidèle (hi-fi) ou sur mes modèles
// (retrofit, avec son pack et son thème), et peaufinage par l'agent une fois l'onglet prêt.

import { useEffect, useRef, useState } from "react";
import { errorText, type ImportTarget, type PackInfo, type TabApi, type ThemeInfo } from "./api";
import { t } from "./i18n";
import { fileName } from "./importFile";
import { useModalFrame } from "./modalFrame";
import { packOptionLabel } from "./packOps";

interface Props {
  /** Le `.pptx` choisi par « Ouvrir ». */
  path: string;
  /** Requêtes de l'onglet d'où l'on importe : les packs de la bibliothèque. */
  api: Pick<TabApi, "templateSets">;
  /** Thèmes de l'onglet (snapshot) : ceux de la bibliothèque sont proposés. */
  themes: ThemeInfo[];
  /** L'import tourne (tenu par l'espace de travail : il continue si le dialogue se ferme). */
  busy: boolean;
  /** Le dernier refus de l'hôte, localisé. */
  error: string | null;
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
export default function ImportDialog({ path, api, themes, busy, error, onImport, onClose }: Props) {
  const [mode, setMode] = useState<ImportTarget["mode"]>("hifi");
  const [sets, setSets] = useState<PackInfo[] | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [pack, setPack] = useState("");
  // `""` : le thème par défaut de la bibliothèque.
  const [theme, setTheme] = useState("");
  const [tune, setTune] = useState(true);
  const ref = useRef<HTMLDivElement>(null);
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: onClose });

  useEffect(() => {
    let live = true;
    api
      .templateSets()
      .then((list) => {
        if (!live) return;
        setSets(list);
        setPack((p) => p || ((list.find((s) => s.default) ?? list[0])?.id ?? ""));
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

  const ready = mode === "hifi" || pack !== "";
  const submit = () => {
    if (busy || !ready) return;
    onImport(mode === "hifi" ? { mode: "hifi" } : { mode: "retrofit", pack, theme: theme || null }, tune);
  };
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
            <label className="settings-row">
              <span className="caption">{t("ui-picker-set")}</span>
              <select className="select" value={pack} disabled={busy || !sets?.length} onChange={(e) => setPack(e.target.value)}>
                {(sets ?? []).map((s) => <option key={s.id} value={s.id}>{packOptionLabel(s)}</option>)}
              </select>
            </label>
            {sets?.length === 0 && <p className="caption import-empty">{t("ui-import-no-packs")}</p>}
            {listError && <p className="import-error" role="alert">{listError}</p>}
            <label className="settings-row">
              <span className="caption">{t("ui-toolbar-theme")}</span>
              <select className="select" value={theme} disabled={busy} onChange={(e) => setTheme(e.target.value)}>
                <option value="">{t("ui-import-theme-default")}</option>
                {choices.map((th) => <option key={th.id} value={th.id}>{th.name}</option>)}
              </select>
            </label>
          </div>
        )}
        <label className="check import-tune">
          <input type="checkbox" checked={tune} disabled={busy} aria-describedby="import-tune-note" onChange={(e) => setTune(e.target.checked)} />
          <span>{t("ui-import-tune")}</span>
        </label>
        <p id="import-tune-note" className="caption">{t(mode === "hifi" ? "ui-import-tune-note" : "ui-import-tune-note-retrofit")}</p>
        {error && <p className="import-error" role="alert">{error}</p>}
        <div className="import-actions">
          {busy && <span className="caption" role="status">{t("ui-import-busy")}</span>}
          <button type="button" className="btn ghost" onClick={onClose}>{t("ui-close-cancel")}</button>
          <button type="button" className="btn primary" disabled={busy || !ready} onClick={submit}>{t("ui-import-submit")}</button>
        </div>
      </div>
    </div>
  );
}
