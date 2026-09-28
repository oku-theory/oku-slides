// Bandeau de l'historique : la version montrée, le curseur des versions (repères
// de session et de fusion), Restaurer et Revenir. Le curseur est un crochet à
// part : la barre et les raccourcis clavier de l'onglet le partagent.

import type { CSSProperties } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { HistoryView, Snapshot, TabApi, Version } from "./api";
import { markers, RENDER_WHILE_DRAGGING, Seeker, step } from "./history";
import { locale, t } from "./i18n";
import { Icon } from "./icons";

export interface HistoryCursor {
  /** Toutes les versions, lues une fois à l'ouverture (vide avant). */
  versions: Version[];
  /** Cran du curseur (peut précéder la version montrée pendant un glissement). */
  value: number;
  /** Place le curseur ; `render` : demander ce cran à la session. */
  go: (index: number, render?: boolean) => void;
  /** ← / → : une version ; avec ⇧ : un début de session. */
  arrow: (key: "ArrowLeft" | "ArrowRight", shift: boolean) => void;
  /**
   * Oublie les crans en vol : à appeler avant de quitter l'historique, sinon
   * leur réponse, arrivée après, ramènerait le bandeau (et l'onglet) dedans.
   */
  cancel: () => void;
}

/** Le curseur de l'historique d'un onglet ; `show` reçoit le snapshot du dernier cran demandé. */
export function useHistoryCursor(
  api: TabApi,
  history: HistoryView | null,
  show: (s: Snapshot) => void,
  onError: (e: unknown) => void,
): HistoryCursor {
  const [versions, setVersions] = useState<Version[]>([]);
  const [value, setValue] = useState(0);
  // Refs : un appui répété part du dernier cran, avant le prochain rendu.
  const valueRef = useRef(0);
  const sent = useRef(-1);
  const versionsRef = useRef<Version[]>([]);
  versionsRef.current = versions;
  const handlers = useRef({ show, onError });
  handlers.current = { show, onError };
  const [seeker] = useState(
    () =>
      new Seeker<Snapshot | null>(
        (i) =>
          api.historySeek(i).catch((e) => {
            handlers.current.onError(e);
            return null;
          }),
        (s) => s && handlers.current.show(s),
      ),
  );

  const open = history !== null;
  const opened = history?.index ?? 0;
  useEffect(() => {
    setVersions([]);
    if (!open) return;
    valueRef.current = sent.current = opened;
    setValue(opened);
    let live = true;
    api
      .historyVersions()
      .then((v) => live && setVersions(v))
      .catch((e) => handlers.current.onError(e));
    return () => {
      live = false;
    };
    // Seulement à l'ouverture : les crans suivants viennent de ce curseur.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, api]);

  const go = useCallback(
    (index: number, render = true) => {
      valueRef.current = index;
      setValue(index);
      // Le relâchement suit le dernier glissement : pas deux fois le même cran.
      if (!render || index === sent.current) return;
      sent.current = index;
      void seeker.go(index);
    },
    [seeker],
  );

  const arrow = useCallback(
    (key: "ArrowLeft" | "ArrowRight", shift: boolean) => {
      if (versionsRef.current.length) go(step(valueRef.current, versionsRef.current, key, shift));
    },
    [go],
  );

  const cancel = useCallback(() => {
    seeker.cancel();
    sent.current = -1;
  }, [seeker]);

  return { versions, value, go, arrow, cancel };
}

/** Date et heure d'une version, dans la langue de l'interface. */
function versionDate(timestamp: number): string {
  return new Intl.DateTimeFormat(locale(), { dateStyle: "medium", timeStyle: "short" }).format(new Date(timestamp * 1000));
}

export default function HistoryBar({ history, cursor, onRestore, onBack }: {
  history: HistoryView;
  cursor: HistoryCursor;
  onRestore: () => void;
  onBack: () => void;
}) {
  const range = useRef<HTMLInputElement>(null);
  const max = Math.max(0, history.count - 1);
  // Pendant un glissement sans rendu, l'étiquette suit le curseur.
  const version = cursor.versions[cursor.value] ?? history.version;
  const date = versionDate(version.timestamp);
  const added = history.changes.added_since.length;

  // `onChange` de React suit chaque mouvement ; le relâchement est l'événement
  // `change` natif.
  const { go } = cursor;
  useEffect(() => {
    const el = range.current;
    if (!el) return;
    const release = () => go(Number(el.value));
    el.addEventListener("change", release);
    return () => el.removeEventListener("change", release);
  }, [go]);

  return (
    <section className="history-bar" aria-label={t("ui-history")}>
      <div className="history-version" role="status">
        <span className="brand">{t("ui-history")}</span>
        <span className="history-title">{t("ui-history-version", { date })}</span>
        <span className="caption">
          {[version.author ?? t("ui-history-anonymous"), version.label, added > 0 ? t("ui-history-added-since", { count: added }) : null]
            .filter(Boolean)
            .join(" · ")}
        </span>
      </div>
      <div className="history-track" dir="ltr">
        <div className="history-marks" aria-hidden>
          {markers(cursor.versions).map((m) => (
            <span key={m.index} className={`history-mark ${m.kind}`} style={{ "--at": max ? m.index / max : 0 } as CSSProperties} />
          ))}
        </div>
        <input
          ref={range}
          type="range"
          className="history-range"
          min={0}
          max={max}
          step={1}
          value={Math.min(cursor.value, max)}
          aria-label={t("ui-history-slider")}
          aria-valuetext={date}
          onChange={(e) => go(Number(e.currentTarget.value), RENDER_WHILE_DRAGGING)}
          onKeyDown={(e) => {
            if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
            e.preventDefault();
            cursor.arrow(e.key, e.shiftKey);
          }}
        />
      </div>
      <div className="history-actions">
        <button className="object token primary" aria-label={t("ui-history-restore")} title={t("ui-history-restore")} onClick={onRestore}>{Icon.restore}</button>
        <button className="object token history-back" aria-label={t("ui-history-back")} title={t("ui-history-back")} onClick={onBack}>{Icon.back}</button>
      </div>
    </section>
  );
}
