/**
 * Planche des cas d'exemple d'un widget : tous ses cas côte à côte, en
 * vignettes (l'aperçu de chaque cas, cadré comme la scène), avec leurs
 * alertes (« Champ vide », « Déborde », sinon « OK ») et un bilan.
 *
 * Deux bascules propres à la vue, non gardées : clair / sombre (départ : le
 * schéma du deck) et 16:9 / 4:3 ; elles refont toutes les vignettes. Les
 * aperçus sont demandés ensemble, au plus `EXAMPLES_MAX`, à chaque révision
 * des cas, style du deck ou bascule ; une réponse périmée est ignorée.
 * Un clic sur une vignette ouvre son cas dans la scène ; « Nouveau cas »
 * ajoute une copie du cas courant, comme « + Cas » des pastilles, et l'ouvre.
 */
import { useEffect, useRef, useState } from "react";
import type { ExampleAlert, PreviewLook, Snapshot, TabApi, WidgetPreview } from "./api";
import { PREVIEW_DELAY_MS } from "./debounce";
import { EXAMPLES_MAX, addCase } from "./ExamplePills";
import { t } from "./i18n";
import { Icon } from "./icons";
import { croppedSvg } from "./widgetPreview";

type Scheme = NonNullable<PreviewLook["scheme"]>;
type Ratio = NonNullable<PreviewLook["ratio"]>;

export interface ExampleBoardProps {
  api: TabApi;
  /** Fichier du widget. */
  widget: string;
  /** Noms des cas, dans l'ordre. */
  names: string[];
  /** Cas courant, parmi `names` : copié par « Nouveau cas ». */
  current: string;
  /** Révision des cas (commune au pack) : les vignettes se refont quand elle change. */
  trials: number;
  /** Empreinte du style du deck : les vignettes se refont quand elle change. */
  look: string;
  /** Schéma du deck : celui des vignettes au départ. */
  scheme?: Scheme;
  /** Ouvre le cas dans la scène. */
  onOpen: (name: string) => void;
  /** Envoie une écriture : refus dit en toast (`null`), snapshot mis à jour. */
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
}

const RATIOS: Ratio[] = ["16:9", "4:3"];

/** Les genres d'alertes d'un cas, une fois chacun, dans l'ordre de leur première apparition. */
function kinds(alerts: ExampleAlert[]): ExampleAlert["kind"][] {
  return [...new Set(alerts.map((a) => a.kind))];
}

export default function ExampleBoard({ api, widget, names, current, trials, look, scheme: deckScheme = "light", onOpen, run }: ExampleBoardProps) {
  const [scheme, setScheme] = useState<Scheme>(deckScheme);
  const [ratio, setRatio] = useState<Ratio>("16:9");
  const shown = names.slice(0, EXAMPLES_MAX);
  const key = shown.join("\n");
  /** Aperçu de chaque cas ; `null` : refusé. Absent : pas encore arrivé. */
  const [previews, setPreviews] = useState<Record<string, WidgetPreview | null>>({});
  const seq = useRef(0);

  const load = () => {
    const asked = ++seq.current;
    const list = key === "" ? [] : key.split("\n");
    void Promise.all(
      list.map((name) => api.previewWidget(widget, name, {}, { scheme, ratio }).catch(() => null)),
    ).then((got) => {
      if (asked === seq.current) setPreviews(Object.fromEntries(list.map((name, i) => [name, got[i]])));
    });
  };
  const loadRef = useRef(load);
  loadRef.current = load;
  // Cas, schéma, format : tout de suite (geste de l'utilisateur sur la planche).
  useEffect(() => loadRef.current(), [api, widget, key, scheme, ratio]);
  // Pack ou style du deck changés (une frappe dans le code enregistre le pack) : une seule
  // relance après la rafale, pas douze rendus par enregistrement.
  const settled = useRef(false);
  useEffect(() => {
    if (!settled.current) {
      settled.current = true;
      return;
    }
    const timer = setTimeout(() => loadRef.current(), PREVIEW_DELAY_MS);
    return () => clearTimeout(timer);
  }, [trials, look]);

  // Les images des vignettes, cadrées comme la scène ; libérées au rendu suivant.
  const [urls, setUrls] = useState<Record<string, string>>({});
  useEffect(() => {
    const made: Record<string, string> = {};
    for (const [name, p] of Object.entries(previews)) {
      if (!p?.svg) continue;
      made[name] = URL.createObjectURL(new Blob([croppedSvg(p.svg, p.bbox ?? null)], { type: "image/svg+xml" }));
    }
    setUrls(made);
    return () => Object.values(made).forEach((u) => URL.revokeObjectURL(u));
  }, [previews]);

  const add = () => void addCase(api, widget, names, current, run).then((name) => name && onOpen(name));

  const toggle = <T extends string>(value: T, on: T, set: (v: T) => void, label: string) => (
    <button key={value} type="button" className="seg-item" aria-pressed={value === on} onClick={() => set(value)}>
      {label}
    </button>
  );

  return (
    <section className="example-board" aria-label={t("ui-board")}>
      <div className="board-bar">
        <div className="board-toggles">
          <div className="seg" role="group" aria-label={t("ui-board-scheme")}>
            {toggle<Scheme>("light", scheme, setScheme, t("ui-board-light"))}
            {toggle<Scheme>("dark", scheme, setScheme, t("ui-board-dark"))}
          </div>
          <div className="seg" role="group" aria-label={t("ui-board-ratio")}>
            {RATIOS.map((r) => toggle<Ratio>(r, ratio, setRatio, r))}
          </div>
        </div>
      </div>
      <div className="board-grid">
        {shown.map((name) => {
          const p = previews[name];
          return (
            <button
              key={name}
              type="button"
              className="board-card"
              aria-current={name === current ? true : undefined}
              title={t("ui-board-open", { name })}
              onClick={() => onOpen(name)}
            >
              <span className="board-frame" data-ratio={ratio}>
                {urls[name] ? <img className="board-thumb" src={urls[name]} alt="" /> : null}
              </span>
              <span className="board-name">{name}</span>
              <span className="board-badges">
                {/* Sans image (rendu raté) : pas de verdict. */}
                {/* Seules les alertes se disent : un cas sans alerte n'a pas de badge. */}
                {p?.svg &&
                  kinds(p.alerts ?? []).map((k) => (
                      <span
                        key={k}
                        className={`board-badge ${k}`}
                        title={(p.alerts ?? []).filter((a) => a.kind === k).map((a) => a.field).join(", ")}
                      >
                        {t(k === "empty" ? "ui-board-empty" : "ui-board-overflow")}
                      </span>
                    ))}
              </span>
            </button>
          );
        })}
        <button type="button" className="board-card board-new" disabled={names.length >= EXAMPLES_MAX} onClick={add}>
          <span className="board-frame" data-ratio={ratio} aria-hidden>{Icon.add}</span>
          <span className="board-name">{t("ui-board-new")}</span>
        </button>
      </div>
    </section>
  );
}
