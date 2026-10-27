/**
 * A theme tab's panel, right of the stage: its title, the draft's name, the
 * unsaved mark, then its tokens (surface `host-theme`, filtered by its own
 * field). A theme tab never changes which theme it edits: there is no base picker.
 */
import { useEffect, useRef, useState } from "react";
import type { Snapshot, TabApi } from "./api";
import HostSettings from "./HostSettings";
import { t } from "./i18n";
import { useThemeSettings } from "./useThemeSettings";

interface Props {
  api: TabApi;
  snap: Snapshot;
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  say: (m: string, error?: boolean) => void;
}

export default function ThemePanel({ api, snap, run, say }: Props) {
  // Le filtre saisi dans la surface : gardé ici, renvoyé à la session à chaque relecture.
  const [filter, setFilter] = useState("");
  const settings = useThemeSettings(api, snap, filter, run, say, setFilter);
  const theme = snap.theming?.theme ?? null;
  const name = theme?.name ?? "";
  const [draft, setDraft] = useState(name);
  // Le nom déjà envoyé : Entrée puis la sortie du champ ne le renvoient pas.
  const sent = useRef<string | null>(null);
  useEffect(() => {
    setDraft(name);
    sent.current = null;
  }, [name]);
  const commit = () => {
    const next = draft.trim();
    if (!next || next === name) return setDraft(name);
    if (next === sent.current) return;
    sent.current = next;
    void run(api.renameEditedTheme(next)).then((done) => {
      // Refusé : le champ reprend le nom de la session.
      if (!done) {
        sent.current = null;
        setDraft(name);
      }
    });
  };

  return (
    <section id="theme-panel" role="region" aria-labelledby="theme-panel-title" className="theme-panel">
      <div className="theme-panel-head">
        <h2 id="theme-panel-title" className="theme-panel-title">{t("ui-tab-theme")}</h2>
        {theme && (
          <label className="note-field">
            <span className="caption">
              {t("ui-theme-name")}
              {theme.dirty && <span className="unsaved-dot" role="img" aria-label={t("ui-theme-unsaved")} />}
            </span>
            <input
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onBlur={commit}
              onKeyDown={(e) => {
                if (e.key === "Enter") commit();
                else if (e.key === "Escape") setDraft(name);
                // Les raccourcis de l'app ne voient pas la saisie.
                e.stopPropagation();
              }}
            />
          </label>
        )}
      </div>
      {settings.surface && <HostSettings surface={settings.surface} onAction={settings.onAction} outcome={settings.outcome} onError={(m) => say(m, true)} />}
    </section>
  );
}
