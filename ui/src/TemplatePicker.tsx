// Carrousel des modèles de slides, posé sur la table au-dessus de la feuille.

import { useEffect, useMemo, useRef, useState } from "react";
import { errorText, type PackInfo, type TabApi, type TemplateView } from "./api";
import { t } from "./i18n";
import { packOptionLabel } from "./packOps";
import { Select } from "./primitives";

interface Props {
  /** Change quand le thème ou les jetons changent : les aperçus sont rechargés. */
  themeKey: string;
  /** Pack montré à l'ouverture ; `null` : celui du deck. */
  initialSet: string | null;
  title?: string;
  onPick: (t: TemplateView) => void;
  /** Autre ensemble choisi dans la liste (le deck l'adopte). */
  onSetChange?: (set: PackInfo) => void;
  onClose: () => void;
  /** Requêtes de l'onglet du picker. */
  api: TabApi;
}

export default function TemplatePicker({ themeKey, initialSet, title, onPick, onSetChange, onClose, api }: Props) {
  const [items, setItems] = useState<TemplateView[] | null>(null);
  const [shown, setShown] = useState<string>("");
  const [sets, setSets] = useState<PackInfo[]>([]);
  const [set, setSet] = useState<string | null>(initialSet);
  const [error, setError] = useState<string | null>(null);
  const [focus, setFocus] = useState(0);
  const strip = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;
    setItems(null);
    api
      .templates(set)
      .then((v) => {
        if (!alive) return;
        setItems(v.templates);
        setSets(v.sets);
        setShown(v.set.id);
        setFocus(0);
      })
      .catch((e) => alive && setError(errorText(e)));
    return () => {
      alive = false;
    };
  }, [themeKey, set, api]);

  const chooseSet = (id: string) => {
    setSet(id);
    const info = sets.find((x) => x.id === id);
    if (info) onSetChange?.(info);
  };

  const urls = useMemo(() => {
    const list = (items ?? []).map((tpl) => URL.createObjectURL(new Blob([tpl.svg], { type: "image/svg+xml" })));
    return list;
  }, [items]);
  useEffect(() => () => urls.forEach((u) => URL.revokeObjectURL(u)), [urls]);

  // La carte au focus reste visible dans la bande.
  useEffect(() => {
    strip.current?.children[focus]?.scrollIntoView({ behavior: "smooth", inline: "center", block: "nearest" });
  }, [focus, items]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const n = items?.length ?? 0;
      if (e.key === "Escape") onClose();
      else if (e.key === "ArrowRight") setFocus((f) => Math.min(n - 1, f + 1));
      else if (e.key === "ArrowLeft") setFocus((f) => Math.max(0, f - 1));
      else if (e.key === "Enter" && items?.[focus]) onPick(items[focus]);
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [items, focus, onPick, onClose]);

  return (
    <div className="picker-scrim" onPointerDown={onClose}>
      <div className="picker" role="dialog" aria-modal="true" aria-labelledby="picker-title" onPointerDown={(e) => e.stopPropagation()}>
        <div className="picker-head">
          <h2 id="picker-title">{title ?? t("ui-picker-new-slide")}</h2>
          <label className="picker-set">
            <span className="caption">{t("ui-picker-set")}</span>
            <Select value={set ?? shown} onChange={(v, el) => { el.blur(); chooseSet(v); }}
              options={sets.map((x) => ({ value: x.id, label: packOptionLabel(x) }))} />
          </label>
          <span className="caption">{t("ui-picker-keys")}</span>
          <button className="object token" aria-label={t("ui-picker-close")} onClick={onClose}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"><path d="M6 6l12 12M18 6L6 18" /></svg>
          </button>
        </div>
        {error && <p className="caption">{error}</p>}
        <div className="picker-row">
          <button className="object token picker-nav" aria-label={t("ui-picker-previous")} disabled={focus === 0} onClick={() => setFocus((f) => Math.max(0, f - 1))}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M15 6l-6 6 6 6" /></svg>
          </button>
          <div className="picker-strip" ref={strip} role="listbox" aria-label={t("ui-picker-list")}>
            {(items ?? Array.from({ length: 6 }, () => null)).map((tpl, i) => (
              <button
                key={tpl?.id ?? i}
                role="option"
                aria-selected={i === focus}
                className={`template${i === focus ? " focused" : ""}`}
                disabled={!tpl}
                onMouseEnter={() => setFocus(i)}
                onFocus={() => setFocus(i)}
                onClick={() => tpl && onPick(tpl)}
              >
                {tpl ? <img src={urls[i]} alt="" draggable={false} /> : <span className="template-empty" />}
                <span className="template-name">{tpl?.name ?? "…"}</span>
                <span className="caption">{tpl?.description ?? ""}</span>
              </button>
            ))}
          </div>
          <button className="object token picker-nav" aria-label={t("ui-picker-next")} disabled={!items || focus >= items.length - 1} onClick={() => setFocus((f) => Math.min((items?.length ?? 1) - 1, f + 1))}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M9 6l6 6-6 6" /></svg>
          </button>
        </div>
      </div>
    </div>
  );
}
