// Grille d'ajout : les widgets d'élément de chaque pack épinglé, une section par pack, filtrés par
// nom ou description.

import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent } from "react";
import type { WidgetPreview } from "./api";
import { t } from "./i18n";
import { gridStep, rowLength, sectionSlots, type GridKey } from "./gridNav";
import { Icon } from "./icons";
import { useModalFrame } from "./modalFrame";
import { filterChoices, type WidgetChoice, type WidgetSection } from "./widgetChoices";
import { croppedSvg } from "./widgetPreview";

interface Props {
  /** Les sections de la grille, une par pack de widgets. */
  sections: WidgetSection[];
  /** Faux : la carte se voit grisée (le slot libre de la slide ne l'accepte pas). */
  canPick: (c: WidgetChoice) => boolean;
  /**
   * Change avec le thème : les aperçus sont rechargés. Chaque ouverture les recharge aussi ; un
   * widget réécrit pendant l'ouverture garde son ancien aperçu (t-962).
   */
  themeKey: string;
  /** Aperçu d'un widget, rendu seul : son fichier `name` dans le pack épinglé `pack` (requête `preview_widget`). */
  preview: (name: string, pack?: string) => Promise<WidgetPreview>;
  onPick: (c: WidgetChoice) => void;
  onClose: () => void;
}

/**
 * Fenêtre modale du bouton d'ajout : un champ de filtre, puis la grille des cartes.
 *
 * Depuis le filtre, les flèches parcourent les cartes ajoutables (← → au bord du texte seulement,
 * pour ne pas voler le curseur), Entrée ajoute celle au focus ; Échap ferme.
 */
export default function WidgetPicker({ sections, canPick, themeKey, preview, onPick, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const grid = useRef<HTMLDivElement>(null);
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: onClose });
  const [query, setQuery] = useState("");
  const choices = useMemo(() => sections.flatMap((s) => s.choices), [sections]);
  const urls = usePreviewUrls(choices, themeKey, preview);
  // Chaque section garde ses cartes filtrées ; une section vidée par le filtre disparaît.
  const shownSections = useMemo(
    () => sections.map((s) => ({ ...s, choices: filterChoices(s.choices, query) })).filter((s) => s.choices.length > 0),
    [sections, query],
  );
  const shown = useMemo(() => shownSections.flatMap((s) => s.choices), [shownSections]);
  // La carte au focus, par clé : `null` vaut la première ajoutable. Une clé disparue ou devenue
  // refusée (liste changée pendant l'ouverture) retombe sur elle aussi ; sans carte ajoutable,
  // aucune n'a le focus (`at` vaut -1).
  const [focusKey, setFocusKey] = useState<string | null>(null);
  const found = shown.findIndex((c) => c.key === focusKey && canPick(c));
  const at = found >= 0 ? found : shown.findIndex(canPick);
  const focused = at >= 0 ? shown[at] : undefined;

  // Le filtre prend le focus à l'ouverture (le cadre l'a d'abord posé sur la fenêtre).
  useEffect(() => input.current?.focus(), []);
  // Dernière position du pointeur : après un défilement, WebKit renvoie un mousemove sans
  // mouvement, qui ne doit pas voler le focus donné au clavier.
  const pointer = useRef<{ x: number; y: number } | null>(null);

  // La carte au focus reste visible dans la grille (sans animation si l'utilisateur la refuse).
  useEffect(() => {
    const smooth = !window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    cardsOf(grid.current)[at]?.scrollIntoView({ behavior: smooth ? "smooth" : "auto", block: "nearest" });
  }, [at, focused?.key]);

  /**
   * Passe à la carte ajoutable voisine dans le sens `key`. ↑ ↓ lisent les colonnes à l'écran :
   * chaque section commence une rangée, sa dernière rangée finit par des trous.
   */
  const move = (key: GridKey) => {
    const vertical = key === "up" || key === "down";
    const columns = vertical ? Math.max(1, ...shownSections.map((_, i) => rowLength(cardTops(grid.current, i)))) : 1;
    const slots = sectionSlots(shownSections.map((s) => s.choices.length), columns);
    const from = slots.indexOf(at);
    const next = gridStep(from, key, columns, slots.length, (j) => {
      const i = slots[j];
      return i !== null && canPick(shown[i]);
    });
    const i = next >= 0 ? slots[next] : null;
    if (next !== from && i !== null) setFocusKey(shown[i].key);
  };

  /** Le pointeur a vraiment bougé sur la carte `c` : elle prend le focus. */
  const hover = (c: WidgetChoice, e: MouseEvent) => {
    const last = pointer.current;
    if (last && last.x === e.clientX && last.y === e.clientY) return;
    pointer.current = { x: e.clientX, y: e.clientY };
    if (canPick(c) && c.key !== focused?.key) setFocusKey(c.key);
  };

  const pick = (c: WidgetChoice | undefined) => {
    if (c && canPick(c)) onPick(c);
  };

  // Les touches restent dans la fenêtre : rien n'atteint les raccourcis de l'app. Seul le filtre
  // pilote les cartes ; ailleurs (fermer), le bouton fait son travail.
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    e.stopPropagation();
    const field = input.current;
    if (e.target !== field || !field || e.nativeEvent.isComposing) return;
    // Avec une touche de modification, les flèches gardent leur rôle dans le champ (sélection…).
    const plain = !(e.shiftKey || e.altKey || e.metaKey || e.ctrlKey);
    const caret = field.selectionStart === field.selectionEnd ? field.selectionStart : null;
    if (e.key === "ArrowRight" && plain && caret === field.value.length) move("right");
    else if (e.key === "ArrowLeft" && plain && caret === 0) move("left");
    else if (e.key === "ArrowDown" && plain) move("down");
    else if (e.key === "ArrowUp" && plain) move("up");
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
          <span className="caption">{t("ui-widget-picker-keys")}</span>
          <button className="object token" aria-label={t("ui-picker-close")} onClick={onClose}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"><path d="M6 6l12 12M18 6L6 18" /></svg>
          </button>
        </div>
        <div className="widget-picker-list" ref={grid} id="widget-picker-list" role="listbox" aria-label={t("ui-widget-picker-list")}>
          {shownSections.map((section) => {
            const first = shown.indexOf(section.choices[0]);
            const heading = `widget-picker-pack-${section.namespace}`;
            return (
              <section key={section.namespace} className="widget-picker-pack" role="group" aria-labelledby={heading}>
                <h3 id={heading}>{section.name}</h3>
                <div className="widget-picker-grid">
                  {section.choices.map((c, k) => {
                    const i = first + k;
                    return (
                      <button
                        key={c.key}
                        id={optionId(i)}
                        role="option"
                        aria-selected={i === at}
                        className={`widget-choice${i === at ? " focused" : ""}`}
                        disabled={!canPick(c)}
                        tabIndex={-1}
                        onMouseDown={keepFilterFocus}
                        onMouseMove={(e) => hover(c, e)}
                        onClick={() => pick(c)}
                      >
                        <span className="widget-choice-face">
                          {urls.has(c.name) ? <img src={urls.get(c.name)} alt="" draggable={false} /> : Icon.widget}
                        </span>
                        <span className="card-name">{c.label}</span>
                        <span className="caption">{c.description}</span>
                      </button>
                    );
                  })}
                </div>
              </section>
            );
          })}
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

/** Les cartes de la liste, toutes sections confondues, dans l'ordre. */
function cardsOf(list: HTMLElement | null): HTMLElement[] {
  return list ? [...list.querySelectorAll<HTMLElement>('[role="option"]')] : [];
}

/** Hauteur de chaque carte de la section `section` dans la mise en page, dans l'ordre. */
function cardTops(list: HTMLElement | null, section: number): number[] {
  const grid = list?.querySelectorAll(".widget-picker-grid")[section];
  return grid ? [...grid.children].map((el) => (el instanceof HTMLElement ? el.offsetTop : 0)) : [];
}

/** Identifiant de la carte `i`, que le filtre désigne à un lecteur d'écran. */
function optionId(i: number): string {
  return `widget-picker-option-${i}`;
}

/**
 * Adresses des aperçus SVG des widgets de la grille, par nom, chargées une à une.
 *
 * L'aperçu est la slide d'essai entière : il est recadré sur ce que dessine le widget (`bbox`),
 * qui remplit alors sa vignette.
 *
 * Un aperçu raté (widget cassé, rendu refusé) ou un kind non qualifié laisse l'icône générique.
 */
function usePreviewUrls(
  choices: WidgetChoice[],
  themeKey: string,
  preview: (name: string, pack?: string) => Promise<WidgetPreview>,
): Map<string, string> {
  // Une Map, pas un objet : un widget nommé `constructor` ne lit pas une propriété héritée.
  const [urls, setUrls] = useState<Map<string, string>>(() => new Map());
  const names = choices.map((c) => c.name).join("\n");

  useEffect(() => {
    let alive = true;
    const made: string[] = [];
    setUrls(new Map());
    (async () => {
      for (const name of names ? names.split("\n") : []) {
        // La session nomme un widget par son fichier dans son pack : `base/card` → `card` de `base`.
        const slash = name.indexOf("/");
        if (slash <= 0) continue;
        const asked = preview(name.slice(slash + 1), name.slice(0, slash));
        const svg = await asked.then((p) => (p.svg ? croppedSvg(p.svg, p.bbox ?? null) : null), () => null);
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
