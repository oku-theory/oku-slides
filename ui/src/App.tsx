import { useCallback, useEffect, useRef, useState } from "react";
import type { Op, SlideView, Snapshot } from "./api";
import type { SetInfo, TemplateView } from "./api";
import { api, applyAppearance, findElement, inTauri, instantiate, newId, pickOpenPath, pickSavePath } from "./api";
import TemplatePicker from "./TemplatePicker";
import Stage, { FIELD_LABELS, textOps } from "./Stage";
import Dictation from "./Dictation";
import { textToParagraphs } from "./api";
import type { Cell } from "./grid";
import { cellRect, rescaleCell, snapRect } from "./grid";

const Icon = {
  undo: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M9 14L4 9l5-5" /><path d="M4 9h11a5 5 0 0 1 0 10h-3" /></svg>,
  redo: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M15 14l5-5-5-5" /><path d="M20 9H9a5 5 0 0 0 0 10h3" /></svg>,
};

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
        title="Renommer la présentation"
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
      aria-label="Titre de la présentation"
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
    window.setTimeout(() => setToast((t) => (t?.text === text ? null : t)), error ? 6000 : 2500);
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

  const currentHash = snap?.slides.find((s) => s.id === current)?.hash;
  useEffect(() => {
    if (!current) return;
    api.slide(current).then(setView).catch((e) => say(String(e), true));
  }, [current, currentHash]);

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
    if (await run(api.save(path))) say("Votre présentation est enregistrée");
  };


  /** Ajoute un texte, une carte ou une forme ; sur une grille, accroché aux nœuds. */
  const addElement = (kind: "text" | "card" | "shape") => {
    if (!view) return;
    const slide = view.slide;
    const id = newId(kind[0]);
    const free = (slide.elements?.length ?? 0) > 0 || !!slide.grid;
    const zoneGrid = view.zone?.grid;
    const frame = { x: 40, y: 40, w: 640, h: 120 };
    const body: Record<string, unknown> =
      kind === "card"
        ? { kind, title: "Titre de la carte", text: [{ runs: [{ t: "Détail" }] }] }
        : kind === "shape"
          ? { kind, style: { fill: "surface", radius: "md" } }
          : { kind, text: [{ runs: [{ t: "Nouveau texte" }] }] };
    const element = view.zone && zoneGrid
      ? { id, ...body, cell: snapRect(view.zone, zoneGrid, frame) }
      : free
        ? { id, ...body, frame }
        : { id, ...body };
    const label = { text: "Ajouter un texte", card: "Ajouter une carte", shape: "Ajouter une forme" }[kind];
    apply(label, [{ op: "add_element", slide: slide.id, zone: free ? "elements" : "blocks", element }]).then(() => {
      setSelected(id);
      if (kind === "text") setEditing(id);
    });
  };

  const [picking, setPicking] = useState(false);
  const addSlide = () => setPicking(true);
  const insertTemplate = (t: TemplateView) => {
    if (!snap) return;
    setPicking(false);
    const index = snap.slides.findIndex((s) => s.id === current) + 1;
    const slide = instantiate(t.slide);
    if (snap.editing) {
      // Nouveau modèle : copie du modèle choisi, sous un nom à lui.
      slide.name = `${t.name} (variante)`;
      slide.description = t.description;
    }
    apply(snap.editing ? `Nouveau modèle : ${t.name}` : `Nouvelle slide : ${t.name}`, [{ op: "add_slide", index, slide }]).then(() => {
      setCurrent(slide.id);
      setSelected(null);
    });
  };

  // --- Grille de la slide -------------------------------------------------
  const [gridAlways, setGridAlways] = useState(false);
  const slideGrid = view?.slide.grid;
  const gridCapable = !!view && (view.slide.kind === "content" || view.slide.kind === "blank");

  const setGrid = (columns: number, rows: number) => {
    if (!view) return;
    const slide = view.slide;
    const next = { ...(slide.grid ?? {}), columns, rows };
    const ops: Op[] = [{ op: "set_grid", slide: slide.id, grid: next }];
    // Les éléments accrochés gardent leurs proportions sur la nouvelle grille.
    if (slide.grid) {
      for (const e of slide.elements ?? []) {
        if (e.cell) ops.push({ op: "set_cell", slide: slide.id, element: e.id, cell: rescaleCell(e.cell, slide.grid, next) });
      }
    }
    apply(slide.grid ? `Grille ${columns} × ${rows}` : "Ajouter une grille", ops);
  };

  const removeGrid = () => {
    if (!view?.slide.grid || !view.zone?.grid) return;
    const { zone } = view;
    const g = zone.grid!;
    // Les éléments accrochés restent à leur place, en cadres libres.
    const ops: Op[] = (view.slide.elements ?? [])
      .filter((e) => e.cell)
      .flatMap((e) => [
        { op: "set_frame", slide: view.slide.id, element: e.id, frame: cellRect(zone, g, e.cell as Cell) },
        { op: "set_cell", slide: view.slide.id, element: e.id, cell: null },
      ]);
    ops.push({ op: "set_grid", slide: view.slide.id, grid: null });
    apply("Retirer la grille", ops);
  };

  const selectedFree = view && selected ? (view.slide.elements ?? []).find((e) => e.id === selected) : undefined;
  const toggleAnchor = () => {
    if (!view?.zone?.grid || !selectedFree) return;
    const { zone } = view;
    const g = zone.grid!;
    const slide = view.slide.id;
    if (selectedFree.cell) {
      apply("Détacher de la grille", [
        { op: "set_frame", slide, element: selectedFree.id, frame: cellRect(zone, g, selectedFree.cell) },
        { op: "set_cell", slide, element: selectedFree.id, cell: null },
      ]);
    } else if (selectedFree.frame) {
      apply("Accrocher à la grille", [{ op: "set_cell", slide, element: selectedFree.id, cell: snapRect(zone, g, selectedFree.frame) }]);
    }
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
    if (snap.dirty && !window.confirm("Quitter sans enregistrer les modèles modifiés ?")) return;
    if (await run(api.closeTemplates())) {
      setCurrent(null);
      setSelected(null);
    }
  };
  const saveTemplates = async (asNew = false) => {
    const s = await run(api.saveTemplates(asNew));
    if (s?.editing) say(`Modèles enregistrés : ${s.editing.name}`);
  };
  const adoptSet = (set: SetInfo) => {
    if (!snap || snap.editing || set.id === snap.templates) return;
    apply(`Modèles : ${set.name}`, [{ op: "set_templates", templates: set.id === "essentiel" ? null : set.id }]);
  };

  // --- Réarranger les slides par glisser-déposer -----------------------------
  const [dragSlide, setDragSlide] = useState<string | null>(null);
  const [dropAt, setDropAt] = useState<number | null>(null);
  const moveSlide = (id: string, slot: number) => {
    if (!snap) return;
    const from = snap.slides.findIndex((s) => s.id === id);
    // `slot` : place entre deux vignettes (0 = avant la première).
    const to = slot > from ? slot - 1 : slot;
    if (from < 0 || to === from) return;
    apply(snap.editing ? "Déplacer le modèle" : "Déplacer la slide", [{ op: "move_slide", slide: id, index: to }]);
  };

  const removeSlide = () => {
    if (!snap || !current || snap.slides.length < 2) return;
    apply("Supprimer la slide", [{ op: "remove_slide", slide: current }]);
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
        const t = view.targets.find((x) => x.id === selected);
        if (t?.text !== null && t?.text !== undefined) {
          e.preventDefault();
          setEditing(selected);
        }
      } else if ((e.key === "Delete" || e.key === "Backspace") && !selected.startsWith("@")) {
        e.preventDefault();
        apply("Supprimer", [{ op: "remove_element", slide: view.slide.id, element: selected }]);
        setSelected(null);
      } else if (e.key.startsWith("Arrow")) {
        const el = findElement(view.slide, selected);
        if (!el?.frame) return;
        e.preventDefault();
        const step = e.shiftKey ? 10 : 1;
        const f = { ...el.frame };
        if (e.key === "ArrowLeft") f.x -= step;
        if (e.key === "ArrowRight") f.x += step;
        if (e.key === "ArrowUp") f.y -= step;
        if (e.key === "ArrowDown") f.y += step;
        apply("Déplacer", [{ op: "set_frame", slide: view.slide.id, element: selected, frame: f }]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // --- Dictée : où va le texte ----------------------------------------------
  const [dictated, setDictated] = useState<{ seq: number; text: string } | null>(null);
  const target = view && selected ? view.targets.find((t) => t.id === selected) : undefined;
  const writable = target && target.text !== null ? target : undefined;
  const destination = editing
    ? "au curseur"
    : writable
      ? writable.target === "field"
        ? `remplace ${(FIELD_LABELS[writable.kind] ?? writable.kind).toLowerCase()}`
        : "à la suite du bloc sélectionné"
      : view?.slide.kind === "content"
        ? "nouveau bloc de texte"
        : "sélectionnez une zone";
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
      const keep = writable.target === "element" && previous.trim() !== "" && previous !== "Nouveau texte";
      apply("Dictée", textOps(view, writable, keep ? `${previous}\n${text}` : text));
      return;
    }
    if (view.slide.kind === "content") {
      const id = newId("t");
      const free = (view.slide.elements?.length ?? 0) > 0;
      const element = free
        ? { id, kind: "text", frame: { x: 40, y: 40, w: 640, h: 120 }, text: textToParagraphs(text) }
        : { id, kind: "text", text: textToParagraphs(text) };
      apply("Dictée", [{ op: "add_element", slide: view.slide.id, zone: free ? "elements" : "blocks", element }]).then(() => setSelected(id));
      return;
    }
    say(`Sélectionnez une zone pour y placer : « ${text} »`, true);
  };

  if (!snap) return <div className="table"><p className="empty" style={{ padding: 40 }}>Chargement…</p></div>;

  const cur = snap.slides.find((s) => s.id === current);
  const templateNames: Record<string, string> = Object.fromEntries(snap.slides.map((s) => [s.id, s.name ?? s.title ?? "Modèle"]));
  const index = snap.slides.findIndex((s) => s.id === current);
  const status = snap.dirty ? "modifications non enregistrées" : snap.path ? "enregistré" : "pas encore enregistré";

  return (
    <div className={`table${titlebarOverlay ? " overlay-titlebar" : ""}`}>
      {/* Bord supérieur : on y saisit la fenêtre pour la déplacer (barre de titre intégrée). */}
      {titlebarOverlay && <div className="drag-strip" data-tauri-drag-region />}
      {/* Étiquette du document, posée sur la table. */}
      <header className={`label object${snap.editing ? " editing-templates" : ""}`}>
        <span className="brand">{snap.editing ? "Écho · modèles" : "Écho"}</span>
        <TitleField
          value={snap.title_source ?? ""}
          display={snap.title ?? snap.name}
          onCommit={(v) => apply(snap.editing ? "Renommer l'ensemble" : "Renommer la présentation", [{ op: "set_param", key: "title", value: v.trim() || null }])}
        />
        <span className="caption" role="status">
          {snap.editing
            ? `${snap.slides.length} modèles · ${snap.editing.builtin ? "intégré : l'enregistrer crée une copie" : snap.dirty ? "modifications non enregistrées" : "enregistré"}`
            : <>{snap.path ? `${snap.name}.echo.pdf · ` : ""}{snap.slides.length} slides · {status}</>}
        </span>
      </header>

      {/* Outils posés en haut de la table. */}
      <div className="tray">
        <button className="object token" aria-label={snap.undo_label ? `Annuler : ${snap.undo_label}` : "Annuler"} title={snap.undo_label ?? "Annuler"} disabled={!snap.can_undo} onClick={() => run(api.undo())}>{Icon.undo}</button>
        <button className="object token" aria-label={snap.redo_label ? `Rétablir : ${snap.redo_label}` : "Rétablir"} title={snap.redo_label ?? "Rétablir"} disabled={!snap.can_redo} onClick={() => run(api.redo())}>{Icon.redo}</button>
        <div className="object theme-card">
          <label className="caption" htmlFor="theme">Thème</label>
          <select id="theme" className="select" value={snap.theme ?? ""} onChange={(e) => { e.currentTarget.blur(); apply("Changer de thème", [{ op: "set_theme", theme: e.target.value || null }]); }}>
            <option value="">Par défaut</option>
            {snap.themes.map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
        </div>
        {snap.editing ? (
          <>
            {!snap.editing.builtin && <button className="object btn" onClick={() => saveTemplates(true)}>Enregistrer une copie</button>}
            <button className="object btn primary" onClick={() => saveTemplates(false)} disabled={!snap.dirty && !snap.editing.builtin}>
              {snap.editing.builtin ? "Enregistrer une copie" : "Enregistrer"}
            </button>
            <button className="object btn" onClick={finishTemplates}>Terminer</button>
          </>
        ) : (
          <>
            <button className="object btn" onClick={editTemplates} title="Créer et modifier les modèles de slides">Modèles</button>
            <button className="object btn" onClick={open}>Ouvrir</button>
            <button className="object btn primary" onClick={() => save(false)} disabled={!snap.dirty && !!snap.path}>Enregistrer</button>
          </>
        )}
      </div>

      {/* Pile de slides, à gauche. */}
      <nav
        className={`pile${dragSlide ? " dragging" : ""}`}
        aria-label={snap.editing ? "Modèles" : "Slides"}
        onDragOver={(e) => {
          if (!dragSlide) return;
          e.preventDefault();
          e.dataTransfer.dropEffect = "move";
          // Place entre deux vignettes : selon la moitié survolée.
          const thumbs = [...e.currentTarget.querySelectorAll<HTMLElement>(".thumb")];
          let slot = thumbs.length;
          for (let i = 0; i < thumbs.length; i++) {
            const r = thumbs[i].getBoundingClientRect();
            if (e.clientY < r.top + r.height / 2) { slot = i; break; }
          }
          setDropAt(slot);
        }}
        onDragLeave={(e) => { if (!e.currentTarget.contains(e.relatedTarget as Node)) setDropAt(null); }}
        onDrop={(e) => {
          e.preventDefault();
          if (dragSlide && dropAt !== null) moveSlide(dragSlide, dropAt);
          setDragSlide(null);
          setDropAt(null);
        }}
      >
        {snap.slides.map((s, i) => {
          const url = svg(s.id, s.hash);
          const name = snap.editing ? templateNames[s.id] : undefined;
          return (
            <div key={s.id} className="thumb-slot">
              {dropAt === i && <span className="drop-line" aria-hidden />}
              <button
                className={`thumb${dragSlide === s.id ? " lifted" : ""}`}
                aria-current={s.id === current}
                draggable
                title="Glisser pour déplacer · ⌥↑ ⌥↓"
                onDragStart={(e) => {
                  e.dataTransfer.effectAllowed = "move";
                  e.dataTransfer.setData("text/plain", s.id);
                  setDragSlide(s.id);
                }}
                onDragEnd={() => { setDragSlide(null); setDropAt(null); }}
                onClick={() => { setCurrent(s.id); setSelected(null); setEditing(null); }}
              >
                <span className="thumb-n">{i + 1}</span>
                <span className="thumb-body">
                  {url ? <img className="thumb-img" src={url} alt={s.title ?? `Slide ${i + 1}`} draggable={false} /> : <span className="thumb-img" />}
                  {name && <span className="thumb-name">{name}</span>}
                </span>
              </button>
            </div>
          );
        })}
        {dropAt === snap.slides.length && <span className="drop-line" aria-hidden />}
        <button className="object btn add-slide" onClick={addSlide}>{snap.editing ? "+ Nouveau modèle" : "+ Nouvelle slide"}</button>
      </nav>

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
          <div className="stage-wrap empty">Rendu…</div>
        )}
      </main>

      {/* Post-it à droite de la feuille. */}
      <aside className="notes" aria-label="Outils de la slide">
        {snap.editing && view && (
          <div className="postit template-note" role="group" aria-label="Ce modèle">
            <span className="postit-title">Ce modèle</span>
            <NoteField
              label="Nom"
              value={view.slide.name ?? ""}
              onCommit={(v) => apply("Renommer le modèle", [{ op: "set_slide_field", slide: view.slide.id, field: "name", value: v.trim() || null }])}
            />
            <NoteField
              label="Description"
              value={view.slide.description ?? ""}
              onCommit={(v) => apply("Décrire le modèle", [{ op: "set_slide_field", slide: view.slide.id, field: "description", value: v.trim() || null }])}
            />
          </div>
        )}
        <div className="postit add-note" role="group" aria-label="Ajouter sur la slide">
          <span className="postit-title">Ajouter</span>
          <div className="add-row">
            {(["text", "card", "shape"] as const).map((k) => (
              <button key={k} className="linkish" disabled={!view || !(view.slide.kind === "content" || (k !== "text" && view.slide.kind === "blank"))} onClick={() => addElement(k)}>
                {{ text: "Texte", card: "Carte", shape: "Forme" }[k]}
              </button>
            ))}
          </div>
        </div>
        {gridCapable && (slideGrid ? (
          <div className="postit grid-note" role="group" aria-label="Grille de la slide">
            <span className="postit-title">Grille</span>
            <Stepper label="Colonnes" value={slideGrid.columns} onChange={(v) => setGrid(v, slideGrid.rows)} />
            <Stepper label="Lignes" value={slideGrid.rows} onChange={(v) => setGrid(slideGrid.columns, v)} />
            <label className="check postit-hint">
              <input type="checkbox" checked={gridAlways} onChange={(e) => setGridAlways(e.target.checked)} />
              Toujours visible
            </label>
            <button className="linkish" onClick={removeGrid}>Retirer la grille</button>
          </div>
        ) : (
          <button className="postit" onClick={() => setGrid(12, 6)}>
            <span className="postit-title">+ Grille</span>
            <span className="postit-hint">12 × 6 cases pour accrocher les éléments</span>
          </button>
        ))}
        {slideGrid && selectedFree && (
          <button className="postit" onClick={toggleAnchor}>
            <span className="postit-title">{selectedFree.cell ? "Détacher" : "Accrocher"}</span>
            <span className="postit-hint">{selectedFree.cell ? "Placer librement, au point près" : "Caler sur les cases de la grille"}</span>
          </button>
        )}
        <button className="postit" onClick={removeSlide} disabled={snap.slides.length < 2}>
          <span className="postit-title">{snap.editing ? "Supprimer le modèle" : "Supprimer la slide"}</span>
          <span className="postit-hint">Slide {index + 1} · annulable avec ⌘Z</span>
        </button>
        {!slideGrid && !snap.editing && <div className="postit note" role="note">
          <span className="postit-title">Astuce</span>
          <span className="postit-hint">⌘D pour dicter, dans la zone sélectionnée ou au curseur. Double-clic pour modifier un texte.</span>
        </div>}
      </aside>

      {/* Appareil de dictée, posé devant la feuille. */}
      <Dictation destination={destination} prompt={whisperPrompt} onText={onDictated} onError={(m) => say(m, true)} />

      {picking && (
        <TemplatePicker
          themeKey={appearanceKey}
          initialSet={snap.editing ? snap.editing.set : snap.templates}
          title={snap.editing ? "Nouveau modèle, à partir de…" : "Nouvelle slide"}
          onPick={insertTemplate}
          onSetChange={adoptSet}
          onClose={() => setPicking(false)}
        />
      )}

      {toast && <div className={`toast${toast.error ? " error" : ""}`} role={toast.error ? "alert" : "status"}>{toast.text}</div>}
    </div>
  );
}

/** Petit compteur − valeur + posé sur un post-it. */
function Stepper({ label, value, onChange, min = 1, max = 24 }: { label: string; value: number; onChange: (v: number) => void; min?: number; max?: number }) {
  return (
    <div className="stepper">
      <span className="postit-hint">{label}</span>
      <button aria-label={`${label} : une de moins`} disabled={value <= min} onClick={() => onChange(value - 1)}>−</button>
      <span className="stepper-value">{value}</span>
      <button aria-label={`${label} : une de plus`} disabled={value >= max} onClick={() => onChange(value + 1)}>+</button>
    </div>
  );
}

/** Champ d'un post-it, validé à la sortie ou par Entrée. */
function NoteField({ label, value, onCommit }: { label: string; value: string; onCommit: (v: string) => void }) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => {
    if (draft !== value) onCommit(draft);
  };
  return (
    <label className="note-field">
      <span className="postit-hint">{label}</span>
      <input
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
          else if (e.key === "Escape") { setDraft(value); e.currentTarget.blur(); }
          e.stopPropagation();
        }}
      />
    </label>
  );
}
