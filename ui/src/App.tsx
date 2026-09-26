import { Suspense, lazy, useCallback, useEffect, useRef, useState } from "react";
import type { Op, Paragraph, SetInfo, SlideView, Snapshot, TemplateView } from "./api";
import { api, inTauri, pickOpenPath, pickSavePath } from "./api";
import { applyAppearance } from "./appearance";
import { findElement, gridPlacement, instantiate, neighbourSlide, newId, placeOnGridOps, slideToFetch, textOps, textToParagraphs } from "./deck";
import TemplatePicker from "./TemplatePicker";
import SlidePile from "./SlidePile";
import Stage, { fieldLabel } from "./Stage";
import { t } from "./i18n";
import Dictation from "./Dictation";
import { ChartPanel } from "./chart/ChartPanel";
import { starterChart } from "./chart/chartDeck";
import type { Cell } from "./grid";
import { moveCell, trackSize } from "./grid";
import type { AddKind } from "./slideTools";
import { ADD_KINDS, canAdd, canRemoveSlide } from "./slideTools";
import type { A2uiClientAction, A2uiMessage } from "@a2ui/web_core/v0_9";
import { panelActionOps } from "./a2ui/panelActions";

const Icon = {
  undo: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M9 14L4 9l5-5" /><path d="M4 9h11a5 5 0 0 1 0 10h-3" /></svg>,
  redo: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M15 14l5-5-5-5" /><path d="M20 9H9a5 5 0 0 0 0 10h3" /></svg>,
  remove: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M4 7h16" /><path d="M9 7V4h6v3" /><path d="M6 7l1 13h10l1-13" /><path d="M10 11v5M14 11v5" /></svg>,
  grid: (
    <svg viewBox="0 0 24 24" fill="currentColor">
      {[5, 9.7, 14.3, 19].flatMap((y) => [5, 9.7, 14.3, 19].map((x) => <circle key={`${x}-${y}`} cx={x} cy={y} r={1.3} />))}
    </svg>
  ),
};

// @a2ui/react (et zod, lit…) ne sert qu'au panneau des modèles : chargé à la demande.
const PanelSurface = lazy(() => import("./a2ui/PanelSurface").then((m) => ({ default: m.PanelSurface })));

/** Panneau vide (hors des modèles) : même tableau, pas de nouveau rendu. */
const NO_PANEL: A2uiMessage[] = [];

/** SVG des slides, en URL d'objet, mis en cache par empreinte de rendu. */
function useSvgCache() {
  const cache = useRef(new Map<string, string>());
  const [, bump] = useState(0);
  const pending = useRef(new Set<string>());
  const get = useCallback((id: string, hash: string): string | null => {
    const hit = cache.current.get(hash);
    if (hit) return hit;
    if (hash && !pending.current.has(hash)) {
      pending.current.add(hash);
      api.svg(id).then((svg) => {
        const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
        cache.current.set(hash, url);
        pending.current.delete(hash);
        bump((n) => n + 1);
      }).catch(() => pending.current.delete(hash));
    }
    return null;
  }, []);
  return get;
}

/** Titre de la présentation, modifiable sur place (clic, Entrée pour valider, Échap pour annuler). */
function TitleField({ value, display, onCommit }: { value: string; display: string; onCommit: (v: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  if (!editing) {
    return (
      <button
        className="doc-name doc-name-button"
        title={t("ui-title-rename")}
        onClick={() => { setDraft(value); setEditing(true); }}
      >
        {display}
      </button>
    );
  }
  const commit = () => {
    setEditing(false);
    if (draft !== value) onCommit(draft);
  };
  return (
    <input
      className="doc-name doc-name-input"
      aria-label={t("ui-title-field")}
      autoFocus
      value={draft}
      size={Math.max(12, draft.length + 1)}
      onFocus={(e) => e.currentTarget.select()}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") { e.preventDefault(); commit(); }
        else if (e.key === "Escape") { e.preventDefault(); setEditing(false); }
        e.stopPropagation();
      }}
    />
  );
}

/** Dans l'app macOS, la barre de titre est intégrée : les feux se posent sur la table. */
const titlebarOverlay = inTauri && /Mac/i.test(navigator.userAgent);

export default function App() {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [current, setCurrent] = useState<string | null>(null);
  const [view, setView] = useState<SlideView | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [toast, setToast] = useState<{ text: string; error: boolean } | null>(null);
  const svg = useSvgCache();

  const say = (text: string, error = false) => {
    setToast({ text, error });
    window.setTimeout(() => setToast((cur) => (cur?.text === text ? null : cur)), error ? 6000 : 2500);
  };

  const run = useCallback(async (p: Promise<Snapshot>) => {
    try {
      const s = await p;
      setSnap(s);
      if (s.notice) say(s.notice, true);
      return s;
    } catch (e) {
      say(String(e instanceof Error ? e.message : e), true);
      return null;
    }
  }, []);

  useEffect(() => {
    run(api.snapshot());
  }, [run]);

  // L'interface suit le thème des slides, à chaque changement.
  const appearanceKey = snap ? JSON.stringify(snap.appearance) : "";
  useEffect(() => {
    if (snap) applyAppearance(snap.appearance);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [appearanceKey]);

  // Slide courante : la première si l'actuelle a disparu.
  useEffect(() => {
    if (!snap) return;
    if (!current || !snap.slides.some((s) => s.id === current)) setCurrent(snap.slides[0]?.id ?? null);
  }, [snap, current]);

  // Jamais la vue d'une slide que le snapshot n'a plus (elle vient d'être supprimée).
  const fetchId = snap ? slideToFetch(snap.slides, current) : null;
  const currentHash = snap?.slides.find((s) => s.id === fetchId)?.hash;
  useEffect(() => {
    if (!fetchId) return;
    api.slide(fetchId).then(setView).catch((e) => say(String(e), true));
  }, [fetchId, currentHash]);

  const apply = useCallback(
    async (label: string, ops: Op[]) => {
      await run(api.apply(label, ops));
    },
    [run],
  );

  const open = async () => {
    const path = await pickOpenPath();
    if (path && (await run(api.open(path)))) {
      setCurrent(null);
      setSelected(null);
    }
  };

  const save = async (as = false) => {
    if (!snap) return;
    const path = as || !snap.path ? await pickSavePath(snap.name) : null;
    if ((as || !snap.path) && !path) return;
    if (await run(api.save(path))) say(t("ui-toast-saved"));
  };


  /** Ajoute un texte, une carte, une forme ou un graphique ; sur une grille, accroché aux nœuds. */
  const addElement = (kind: AddKind, text?: Paragraph[], label?: string) => {
    if (!view) return;
    const slide = view.slide;
    const id = newId(kind[0]);
    const zone = view.zone;
    const g = zone?.grid;
    const body: Record<string, unknown> =
      kind === "card"
        ? { kind, title: t("ui-new-card-title"), text: [{ runs: [{ t: t("ui-new-card-detail") }] }] }
        : kind === "shape"
          ? { kind, style: { fill: "surface", radius: "md" } }
          : kind === "chart"
            ? starterChart()
            : { kind, text: text ?? [{ runs: [{ t: t("ui-new-text") }] }] };
    label ??= t("ui-undo-add", { kind });
    let ops: Op[];
    const hasBlocks = (slide.blocks?.length ?? 0) > 0;
    if (zone && g && !hasBlocks) {
      // Sur la grille : le nouvel élément prend la moitié gauche de la première
      // ligne libre sous les autres. (Une slide à blocs reçoit un bloc de plus.)
      const placed: Cell[] = (slide.elements ?? []).flatMap((e) => (e.cell ? [e.cell] : []));
      const below = placed.reduce((m, c) => Math.max(m, c.row + (c.rows ?? 1)), 0);
      const row = below < g.rows ? below : 0;
      const cell = { col: 0, row, cols: Math.max(1, Math.round(g.columns / 2)), rows: 1 };
      ops = [{ op: "add_element", slide: slide.id, zone: "elements", element: { id, ...body, cell } }];
    } else {
      ops = [{ op: "add_element", slide: slide.id, zone: "blocks", element: { id, ...body } }];
    }
    apply(label, ops).then(() => {
      setSelected(id);
      if (kind === "text" && !text) setEditing(id);
    });
  };

  const [picking, setPicking] = useState(false);
  const addSlide = () => setPicking(true);
  const insertTemplate = (tpl: TemplateView) => {
    if (!snap) return;
    setPicking(false);
    const index = snap.slides.findIndex((s) => s.id === current) + 1;
    const slide = instantiate(tpl.slide);
    if (snap.editing) {
      // Nouveau modèle : copie du modèle choisi, sous un nom à lui.
      slide.name = t("ui-template-variant-name", { name: tpl.name });
      slide.description = tpl.description;
    }
    apply(t(snap.editing ? "ui-undo-new-template" : "ui-undo-new-slide", { name: tpl.name }), [{ op: "add_slide", index, slide }]).then(() => {
      setCurrent(slide.id);
      setSelected(null);
    });
  };

  // --- Grille de la slide -------------------------------------------------
  const [gridAlways, setGridAlways] = useState(() => {
    try {
      return localStorage.getItem("oku-slides.grid") !== "off";
    } catch {
      return true;
    }
  });
  useEffect(() => {
    try {
      localStorage.setItem("oku-slides.grid", gridAlways ? "on" : "off");
    } catch {
      /* préférence facultative */
    }
  }, [gridAlways]);
  // Toute slide à zone libre a une grille, résolue par le rendu (celle par
  // défaut si la slide n'en précise pas) ; l'accrochage est automatique.
  const slideGrid = view?.zone?.grid ?? undefined;

  // Panneau de droite : surface A2UI de la session, en édition des modèles.
  const [panel, setPanel] = useState<A2uiMessage[]>(NO_PANEL);
  useEffect(() => {
    if (!current || !snap?.editing) return setPanel(NO_PANEL);
    api.panel(current).then(setPanel).catch((e) => say(String(e), true));
  }, [current, snap]);
  const onPanelAction = (action: A2uiClientAction) => {
    if (!view) return;
    const change = panelActionOps(action, view.slide, slideGrid);
    if (change) apply(change.label, change.ops);
  };

  // --- Édition des modèles ------------------------------------------------
  const editTemplates = async () => {
    if (!snap) return;
    if (await run(api.editTemplates(snap.templates))) {
      setCurrent(null);
      setSelected(null);
    }
  };
  const finishTemplates = async () => {
    if (!snap?.editing) return;
    if (snap.dirty && !window.confirm(t("ui-confirm-quit-templates"))) return;
    if (await run(api.closeTemplates())) {
      setCurrent(null);
      setSelected(null);
    }
  };
  const saveTemplates = async (asNew = false) => {
    const s = await run(api.saveTemplates(asNew));
    if (s?.editing) say(t("ui-toast-templates-saved", { name: s.editing.name }));
  };
  const adoptSet = (set: SetInfo) => {
    if (!snap || snap.editing || set.id === snap.templates) return;
    apply(t("ui-undo-templates", { name: set.name }), [{ op: "set_templates", templates: set.default ? null : set.id }]);
  };

  // --- Réarranger les slides par glisser-déposer -----------------------------
  const moveSlide = (id: string, slot: number) => {
    if (!snap) return;
    const from = snap.slides.findIndex((s) => s.id === id);
    // `slot` : place entre deux vignettes (0 = avant la première).
    const to = slot > from ? slot - 1 : slot;
    if (from < 0 || to === from) return;
    apply(t(snap.editing ? "ui-undo-move-template" : "ui-undo-move-slide"), [{ op: "move_slide", slide: id, index: to }]);
  };

  const removeSlide = () => {
    if (!snap || !current || !canRemoveSlide(snap.slides.length)) return;
    const next = neighbourSlide(snap.slides, current);
    run(api.apply(t("ui-undo-remove-slide"), [{ op: "remove_slide", slide: current }])).then((s) => {
      if (s) setCurrent(next);
    });
  };

  // Raccourcis clavier.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (editing || picking || (e.target as HTMLElement).closest("input, textarea, select")) return;
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "z") {
        e.preventDefault();
        run(e.shiftKey ? api.redo() : api.undo());
      } else if (mod && e.key.toLowerCase() === "s") {
        e.preventDefault();
        save(e.shiftKey);
      } else if (mod && e.key.toLowerCase() === "o") {
        e.preventDefault();
        open();
      } else if (mod && e.key === "'") {
        e.preventDefault();
        setGridAlways((v) => !v);
      } else if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown") && snap && current) {
        e.preventDefault();
        const i = snap.slides.findIndex((s) => s.id === current);
        const slot = e.key === "ArrowUp" ? i - 1 : i + 2;
        if (slot >= 0 && slot <= snap.slides.length) moveSlide(current, slot);
      } else if (!view || !selected) {
        return;
      } else if (e.key === "Escape") {
        setSelected(null);
      } else if (e.key === "Enter") {
        const tg = view.targets.find((x) => x.id === selected);
        if (tg?.text !== null && tg?.text !== undefined) {
          e.preventDefault();
          setEditing(selected);
        }
      } else if ((e.key === "Delete" || e.key === "Backspace") && !selected.startsWith("@")) {
        e.preventDefault();
        apply(t("ui-undo-remove"), [{ op: "remove_element", slide: view.slide.id, element: selected }]);
        setSelected(null);
      } else if (e.key.startsWith("Arrow")) {
        const dx = e.key === "ArrowLeft" ? -1 : e.key === "ArrowRight" ? 1 : 0;
        const dy = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
        // Sur la grille : une case à la fois. Un bloc fait passer la slide sur
        // la grille ; un élément libre s'y accroche s'il ne l'était pas.
        const place = gridPlacement(view, selected);
        if (place) {
          e.preventDefault();
          const { zone, grid } = place;
          const { cw, rh } = trackSize(zone, grid);
          const cell = moveCell(zone, grid, place.cell, dx * (cw + grid.gap), dy * (rh + grid.gap));
          apply(t(place.block ? "ui-undo-place-on-grid" : "ui-undo-move-on-grid"), placeOnGridOps(view, selected, cell));
          return;
        }
        const el = findElement(view.slide, selected);
        if (!el?.frame) return;
        e.preventDefault();
        const step = e.shiftKey ? 10 : 1;
        const f = { ...el.frame };
        if (e.key === "ArrowLeft") f.x -= step;
        if (e.key === "ArrowRight") f.x += step;
        if (e.key === "ArrowUp") f.y -= step;
        if (e.key === "ArrowDown") f.y += step;
        apply(t("ui-undo-move"), [{ op: "set_frame", slide: view.slide.id, element: selected, frame: f }]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // --- Dictée : où va le texte ----------------------------------------------
  const [dictated, setDictated] = useState<{ seq: number; text: string } | null>(null);
  const target = view && selected ? view.targets.find((x) => x.id === selected) : undefined;
  const selectedElement = view && selected ? findElement(view.slide, selected) : undefined;
  const writable = target && target.text !== null ? target : undefined;
  const destination = editing
    ? t("ui-dictation-to-cursor")
    : writable
      ? writable.target === "field"
        ? t("ui-dictation-to-field", { field: fieldLabel(writable.kind).toLowerCase() })
        : t("ui-dictation-to-block")
      : view?.slide.kind === "content"
        ? t("ui-dictation-new-block")
        : t("ui-dictation-select-zone");
  const whisperPrompt = snap
    ? [snap.title, ...snap.slides.map((s) => s.title)].filter(Boolean).join(". ").slice(0, 600)
    : "";

  const onDictated = (text: string) => {
    if (editing) {
      setDictated((d) => ({ seq: (d?.seq ?? 0) + 1, text }));
      return;
    }
    if (!view) return;
    if (writable) {
      const previous = writable.text ?? "";
      const keep = writable.target === "element" && previous.trim() !== "" && previous !== t("ui-new-text");
      apply(t("ui-undo-dictation"), textOps(view, writable, keep ? `${previous}\n${text}` : text));
      return;
    }
    if (view.slide.kind === "content") {
      addElement("text", textToParagraphs(text), t("ui-undo-dictation"));
      return;
    }
    say(t("ui-toast-select-zone", { text }), true);
  };

  if (!snap) return <div className="table"><p className="empty" style={{ padding: 40 }}>{t("ui-loading")}</p></div>;

  const cur = snap.slides.find((s) => s.id === current);
  const templateNames: Record<string, string> = Object.fromEntries(snap.slides.map((s) => [s.id, s.name ?? s.title ?? t("ui-template-default-name")]));
  const index = snap.slides.findIndex((s) => s.id === current);
  const status = t(snap.dirty ? "ui-status-unsaved" : snap.path ? "ui-status-saved" : "ui-status-never-saved");

  return (
    <div className={`table${titlebarOverlay ? " overlay-titlebar" : ""}`}>
      {/* Bord supérieur : on y saisit la fenêtre pour la déplacer (barre de titre intégrée). */}
      {titlebarOverlay && <div className="drag-strip" data-tauri-drag-region />}
      {/* Étiquette du document, posée sur la table. */}
      <header className={`label object${snap.editing ? " editing-templates" : ""}`}>
        <span className="brand">{t(snap.editing ? "ui-brand-templates" : "ui-brand")}</span>
        <TitleField
          value={snap.title_source ?? ""}
          display={snap.title ?? snap.name}
          onCommit={(v) => apply(t(snap.editing ? "ui-undo-rename-set" : "ui-undo-rename-deck"), [{ op: "set_param", key: "title", value: v.trim() || null }])}
        />
        <span className="caption" role="status">
          {snap.editing
            ? t("ui-caption-templates", {
                count: snap.slides.length,
                status: t(snap.editing.builtin ? "ui-status-builtin" : snap.dirty ? "ui-status-unsaved" : "ui-status-saved"),
              })
            : <>{snap.path ? `${snap.name}.oku-slides.pdf · ` : ""}{t("ui-caption-slides", { count: snap.slides.length, status })}</>}
        </span>
      </header>

      {/* Outils posés en haut de la table. */}
      <div className="tray">
        <div className="object theme-card add-card" role="group" aria-label={t("ui-toolbar-add-label")}>
          <span className="caption">{t("ui-toolbar-add")}</span>
          {ADD_KINDS.map((k) => (
            <button key={k} className="linkish" disabled={!canAdd(k, view?.slide.kind)} onClick={() => addElement(k)}>
              {t("ui-add-kind", { kind: k })}
            </button>
          ))}
        </div>
        <button
          className="object token"
          aria-label={t(snap.editing ? "ui-toolbar-remove-template" : "ui-toolbar-remove-slide")}
          title={t("ui-toolbar-remove-hint", { n: index + 1 })}
          disabled={!canRemoveSlide(snap.slides.length)}
          onClick={removeSlide}
        >
          {Icon.remove}
        </button>
        <button className="object token" aria-label={snap.undo_label ? t("ui-toolbar-undo-label", { label: snap.undo_label }) : t("ui-toolbar-undo")} title={snap.undo_label ?? t("ui-toolbar-undo")} disabled={!snap.can_undo} onClick={() => run(api.undo())}>{Icon.undo}</button>
        <button className="object token" aria-label={snap.redo_label ? t("ui-toolbar-redo-label", { label: snap.redo_label }) : t("ui-toolbar-redo")} title={snap.redo_label ?? t("ui-toolbar-redo")} disabled={!snap.can_redo} onClick={() => run(api.redo())}>{Icon.redo}</button>
        <button
          className={`object token grid-toggle${gridAlways ? " on" : ""}`}
          aria-pressed={gridAlways}
          aria-label={t(gridAlways ? "ui-toolbar-grid-hide" : "ui-toolbar-grid-show")}
          title={t(gridAlways ? "ui-toolbar-grid-hide-title" : "ui-toolbar-grid-show-title")}
          onClick={() => setGridAlways((v) => !v)}
        >
          {Icon.grid}
        </button>
        <div className="object theme-card">
          <label className="caption" htmlFor="theme">{t("ui-toolbar-theme")}</label>
          <select id="theme" className="select" value={snap.theme ?? ""} onChange={(e) => { e.currentTarget.blur(); apply(t("ui-undo-theme"), [{ op: "set_theme", theme: e.target.value || null }]); }}>
            <option value="">{t("ui-toolbar-theme-default")}</option>
            {snap.themes.map((name) => <option key={name} value={name}>{name}</option>)}
          </select>
        </div>
        {snap.editing ? (
          <>
            {!snap.editing.builtin && <button className="object btn" onClick={() => saveTemplates(true)}>{t("ui-toolbar-save-copy")}</button>}
            <button className="object btn primary" onClick={() => saveTemplates(false)} disabled={!snap.dirty && !snap.editing.builtin}>
              {t(snap.editing.builtin ? "ui-toolbar-save-copy" : "ui-toolbar-save")}
            </button>
            <button className="object btn" onClick={finishTemplates}>{t("ui-toolbar-finish")}</button>
          </>
        ) : (
          <>
            <button className="object btn" onClick={editTemplates} title={t("ui-toolbar-templates-title")}>{t("ui-toolbar-templates")}</button>
            <button className="object btn" onClick={open}>{t("ui-toolbar-open")}</button>
            <button className="object btn primary" onClick={() => save(false)} disabled={!snap.dirty && !!snap.path}>{t("ui-toolbar-save")}</button>
          </>
        )}
      </div>

      {/* Pile de slides, à gauche. */}
      <SlidePile
        slides={snap.slides}
        current={current}
        svg={svg}
        names={snap.editing ? templateNames : undefined}
        label={t(snap.editing ? "ui-pile-templates" : "ui-pile-slides")}
        addLabel={t(snap.editing ? "ui-pile-add-template" : "ui-pile-add-slide")}
        onSelect={(id) => { setCurrent(id); setSelected(null); setEditing(null); }}
        onMove={moveSlide}
        onAdd={addSlide}
      />

      {/* La feuille, posée au centre. */}
      <main className="sheet-area">
        {view && view.slide.id === current ? (
          <Stage
            view={view}
            svgUrl={cur ? svg(cur.id, cur.hash) : null}
            selected={selected}
            editing={editing}
            onSelect={setSelected}
            onEdit={setEditing}
            onApply={apply}
            dictated={dictated}
            showGridAlways={gridAlways}
          />
        ) : (
          <div className="stage-wrap empty">{t("ui-stage-rendering")}</div>
        )}
        {view && selectedElement?.kind === "chart" && (
          <ChartPanel
            slide={view.slide.id}
            element={selectedElement}
            palette={snap.appearance.chart_palette}
            onApply={apply}
            onKind={(kind) => run(api.chartKind(t("ui-undo-chart-kind"), view.slide.id, selectedElement.id, kind))}
          />
        )}
      </main>

      {/* Post-it à droite de la feuille. */}
      <aside className="notes" aria-label={t("ui-notes")}>
        {panel.length > 0 && (
          <Suspense fallback={null}>
            <PanelSurface messages={panel} onAction={onPanelAction} onError={(m) => say(m, true)} />
          </Suspense>
        )}
      </aside>

      {/* Appareil de dictée, posé devant la feuille. */}
      <Dictation destination={destination} prompt={whisperPrompt} onText={onDictated} onError={(m) => say(m, true)} />

      {picking && (
        <TemplatePicker
          themeKey={appearanceKey}
          initialSet={snap.editing ? snap.editing.set : snap.templates}
          title={t(snap.editing ? "ui-picker-new-template" : "ui-picker-new-slide")}
          onPick={insertTemplate}
          onSetChange={adoptSet}
          onClose={() => setPicking(false)}
        />
      )}

      {toast && <div className={`toast${toast.error ? " error" : ""}`} role={toast.error ? "alert" : "status"}>{toast.text}</div>}
    </div>
  );
}
