// Carrousel d'ajout : les éléments natifs et les widgets du pack, filtrés par nom ou description.

import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent } from "react";
import type { WidgetPreview } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";
import { useModalFrame } from "./modalFrame";
import { filterChoices, type WidgetChoice } from "./widgetChoices";

interface Props {
  choices: WidgetChoice[];
  /** Faux : la carte se voit grisée (le texte sur une slide vide). */
  canPick: (c: WidgetChoice) => boolean;
  /**
   * Change avec le thème : les aperçus sont rechargés. Chaque ouverture les recharge aussi ; un
   * widget réécrit pendant l'ouverture garde son ancien aperçu (t-962).
   */
  themeKey: string;
  /** Aperçu d'un widget du pack, rendu seul. */
  preview: (name: string) => Promise<WidgetPreview>;
  onPick: (c: WidgetChoice) => void;
  onClose: () => void;
}

/**
 * Fenêtre modale du bouton d'ajout : un champ de filtre, puis la bande des cartes.
 *
 * Depuis le filtre, ← → parcourent les cartes ajoutables (au bord du texte seulement, pour ne
 * pas voler le curseur), Entrée ajoute celle au focus ; Échap ferme.
 */
export default function WidgetPicker({ choices, canPick, themeKey, preview, onPick, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const strip = useRef<HTMLDivElement>(null);
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: onClose });
  const [query, setQuery] = useState("");
  const urls = usePreviewUrls(choices, themeKey, preview);
  const shown = useMemo(() => filterChoices(choices, query), [choices, query]);
  // La carte au focus, par clé : `null` vaut la première ajoutable. Une clé disparue ou devenue
  // refusée (liste changée pendant l'ouverture) retombe sur elle aussi ; sans carte ajoutable,
  // aucune n'a le focus (`at` vaut -1).
  const [focusKey, setFocusKey] = useState<string | null>(null);
  const found = shown.findIndex((c) => c.key === focusKey && canPick(c));
  const at = found >= 0 ? found : shown.findIndex(canPick);
  const focused = at >= 0 ? shown[at] : undefined;

  // Le filtre prend le focus à l'ouverture (le cadre l'a d'abord posé sur la fenêtre).
  useEffect(() => input.current?.focus(), []);
  // La carte au focus reste visible dans la bande.
  useEffect(() => {
    strip.current?.children[at]?.scrollIntoView({ behavior: "smooth", inline: "center", block: "nearest" });
  }, [at, focused?.key]);

  /** La carte ajoutable suivante dans le sens `step`, ou celle au focus s'il n'y en a pas. */
  const move = (step: 1 | -1) => {
    if (at < 0) return;
    for (let i = at + step; i >= 0 && i < shown.length; i += step) {
      if (canPick(shown[i])) return setFocusKey(shown[i].key);
    }
  };

  const pick = (c: WidgetChoice | undefined) => {
    if (c && canPick(c)) onPick(c);
  };

  // Les touches restent dans la fenêtre : rien n'atteint les raccourcis de l'app. Seul le filtre
  // pilote les cartes ; ailleurs (fermer, précédent, suivant), le bouton fait son travail.
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    e.stopPropagation();
    const field = input.current;
    if (e.target !== field || !field || e.nativeEvent.isComposing) return;
    const caret = field.selectionStart === field.selectionEnd ? field.selectionStart : null;
    if (e.key === "ArrowRight" && caret === field.value.length) move(1);
    else if (e.key === "ArrowLeft" && caret === 0) move(-1);
    else if (e.key === "Enter" && armed) pick(focused);
    else return;
    e.preventDefault();
  };

  return (
    <div className="picker-scrim" onPointerDown={onClose}>
      <div
        ref={ref}
        className="picker widget-picker"
        role="dialog"
        aria-modal="true"
        aria-labelledby="widget-picker-title"
        tabIndex={-1}
        data-armed={armed || undefined}
        onClickCapture={onClickCapture}
        onPointerDown={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <div className="picker-head">
          <h2 id="widget-picker-title">{t("ui-toolbar-add-label")}</h2>
          <input
            ref={input}
            className="widget-picker-filter"
            type="search"
            role="combobox"
            aria-controls="widget-picker-list"
            aria-expanded="true"
            aria-activedescendant={focused ? optionId(at) : undefined}
            value={query}
            placeholder={t("ui-widget-picker-filter")}
            aria-label={t("ui-widget-picker-filter")}
            onChange={(e) => {
              setQuery(e.target.value);
              setFocusKey(null);
            }}
          />
          <span className="caption">{t("ui-picker-keys")}</span>
          <button className="object token" aria-label={t("ui-picker-close")} onClick={onClose}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"><path d="M6 6l12 12M18 6L6 18" /></svg>
          </button>
        </div>
        <div className="picker-row">
          <button className="object token picker-nav" aria-label={t("ui-widget-picker-previous")} disabled={at <= 0 || !shown.slice(0, at).some(canPick)} onMouseDown={keepFilterFocus} onClick={() => move(-1)}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M15 6l-6 6 6 6" /></svg>
          </button>
          <div className="picker-strip" ref={strip} id="widget-picker-list" role="listbox" aria-label={t("ui-widget-picker-list")}>
            {shown.map((c, i) => (
              <button
                key={c.key}
                id={optionId(i)}
                role="option"
                aria-selected={i === at}
                className={`template widget-choice${i === at ? " focused" : ""}`}
                disabled={!canPick(c)}
                tabIndex={-1}
                onMouseDown={keepFilterFocus}
                onMouseMove={() => canPick(c) && c.key !== focused?.key && setFocusKey(c.key)}
                onClick={() => pick(c)}
              >
                <span className="widget-choice-face">
                  {"native" in c.pick ? Icon[c.pick.native] : urls.has(c.name) ? <img src={urls.get(c.name)} alt="" draggable={false} /> : Icon.widget}
                </span>
                <span className="template-name">{c.label}</span>
                <span className="caption">{c.description}</span>
              </button>
            ))}
          </div>
          <button className="object token picker-nav" aria-label={t("ui-widget-picker-next")} disabled={at < 0 || !shown.slice(at + 1).some(canPick)} onMouseDown={keepFilterFocus} onClick={() => move(1)}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M9 6l6 6-6 6" /></svg>
          </button>
        </div>
        {/* Hors de la liste (qui n'a que des options) : annoncé quand le filtre ne garde rien. */}
        <p className="caption widget-picker-empty" role="status">
          {shown.length === 0 ? t("ui-widget-picker-empty", { query: query.trim() }) : null}
        </p>
      </div>
    </div>
  );
}

/** Un clic de souris ne prend pas le focus : le filtre le garde, et le clavier pilote encore les cartes. */
function keepFilterFocus(e: MouseEvent) {
  e.preventDefault();
}

/** Identifiant de la carte `i`, que le filtre désigne à un lecteur d'écran. */
function optionId(i: number): string {
  return `widget-picker-option-${i}`;
}

/**
 * Adresses des aperçus SVG des widgets du pack, par nom, chargées une à une.
 *
 * Un aperçu raté (widget cassé, rendu refusé) laisse l'icône générique.
 */
function usePreviewUrls(choices: WidgetChoice[], themeKey: string, preview: (name: string) => Promise<WidgetPreview>): Map<string, string> {
  // Une Map, pas un objet : un widget nommé `constructor` ne lit pas une propriété héritée.
  const [urls, setUrls] = useState<Map<string, string>>(() => new Map());
  const names = choices.flatMap((c) => ("widget" in c.pick ? [c.name] : [])).join("\n");

  useEffect(() => {
    let alive = true;
    const made: string[] = [];
    setUrls(new Map());
    (async () => {
      for (const name of names ? names.split("\n") : []) {
        const svg = await preview(name).then((p) => p.svg, () => null);
        if (!alive) return;
        if (!svg) continue;
        const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
        made.push(url);
        setUrls((u) => new Map(u).set(name, url));
      }
    })();
    return () => {
      alive = false;
      made.forEach((u) => URL.revokeObjectURL(u));
    };
  }, [names, themeKey, preview]);

  return urls;
}
