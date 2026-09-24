import { useCallback, useEffect, useRef, useState } from "react";
import type { Op, SlideView, Snapshot } from "./api";
import { api, applyAppearance, findElement, newId, pickOpenPath, pickSavePath } from "./api";
import Stage from "./Stage";

const Icon = {
  undo: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M9 14L4 9l5-5" /><path d="M4 9h11a5 5 0 0 1 0 10h-3" /></svg>,
  redo: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M15 14l5-5-5-5" /><path d="M20 9H9a5 5 0 0 0 0 10h3" /></svg>,
  mic: <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="3" width="6" height="11" rx="3" /><path d="M5 11a7 7 0 0 0 14 0" /><path d="M12 18v3" /></svg>,
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

  const addText = () => {
    if (!view) return;
    const slide = view.slide;
    const id = newId("t");
    const free = (slide.elements?.length ?? 0) > 0;
    const element = free
      ? { id, kind: "text", frame: { x: 40, y: 40, w: 640, h: 120 }, text: [{ runs: [{ t: "Nouveau texte" }] }] }
      : { id, kind: "text", text: [{ runs: [{ t: "Nouveau texte" }] }] };
    apply("Ajouter un texte", [{ op: "add_element", slide: slide.id, zone: free ? "elements" : "blocks", element }]).then(() => {
      setSelected(id);
      setEditing(id);
    });
  };

  const addSlide = () => {
    if (!snap) return;
    const index = snap.slides.findIndex((s) => s.id === current) + 1;
    const id = newId("s");
    const slide = { id, kind: "content", title: "Nouvelle slide", blocks: [{ id: newId("b"), kind: "text", text: [{ runs: [{ t: "…" }] }] }] };
    apply("Nouvelle slide", [{ op: "add_slide", index, slide }]).then(() => setCurrent(id));
  };

  const removeSlide = () => {
    if (!snap || !current || snap.slides.length < 2) return;
    apply("Supprimer la slide", [{ op: "remove_slide", slide: current }]);
  };

  // Raccourcis clavier.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (editing || (e.target as HTMLElement).closest("input, textarea, select")) return;
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

  if (!snap) return <div className="app"><p className="empty" style={{ padding: 40 }}>Chargement…</p></div>;

  const cur = snap.slides.find((s) => s.id === current);
  const index = snap.slides.findIndex((s) => s.id === current);

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">Slider</div>
        <div className="doc">
          <div className="doc-name">{snap.name}</div>
          <div className="caption" role="status">
            {snap.slides.length} slides · {snap.dirty ? "modifications non enregistrées" : snap.path ? "enregistré" : "pas encore enregistré"}
          </div>
        </div>
        <div className="spacer" />
        <div className="actions">
          <button className="btn icon" aria-label={snap.undo_label ? `Annuler : ${snap.undo_label}` : "Annuler"} title={snap.undo_label ?? undefined} disabled={!snap.can_undo} onClick={() => run(api.undo())}>{Icon.undo}</button>
          <button className="btn icon" aria-label={snap.redo_label ? `Rétablir : ${snap.redo_label}` : "Rétablir"} title={snap.redo_label ?? undefined} disabled={!snap.can_redo} onClick={() => run(api.redo())}>{Icon.redo}</button>
          <label className="caption" htmlFor="theme">Thème</label>
          <select id="theme" className="select" value={snap.theme ?? ""} onChange={(e) => apply("Changer de thème", [{ op: "set_theme", theme: e.target.value || null }])}>
            <option value="">Par défaut</option>
            {snap.themes.map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
          <button className="btn piece" onClick={open}>Ouvrir</button>
          <button className="btn piece" onClick={() => save(false)} disabled={!snap.dirty && !!snap.path}>Enregistrer</button>
        </div>
      </header>

      <div className="body">
        <nav className="thumbs" aria-label="Slides">
          {snap.slides.map((s, i) => {
            const url = svg(s.id, s.hash);
            return (
              <button key={s.id} className="thumb" aria-current={s.id === current} onClick={() => { setCurrent(s.id); setSelected(null); setEditing(null); }}>
                <span className="thumb-n">{i + 1}</span>
                {url ? <img className="thumb-img" src={url} alt={s.title ?? `Slide ${i + 1}`} /> : <span className="thumb-img" />}
              </button>
            );
          })}
          <button className="btn add-slide" onClick={addSlide}>+ Nouvelle slide</button>
        </nav>

        <main className="sheet">
          <div className="sheet-head">
            <span className="caption">Slide {index + 1} sur {snap.slides.length}{cur ? ` · ${cur.kind}` : ""}</span>
            <div className="tools">
              <button className="btn" onClick={addText} disabled={!view || view.slide.kind !== "content"}>+ Texte</button>
              <button className="btn" onClick={removeSlide} disabled={snap.slides.length < 2}>Supprimer la slide</button>
            </div>
          </div>
          {view && view.slide.id === current ? (
            <Stage
              view={view}
              svgUrl={cur ? svg(cur.id, cur.hash) : null}
              selected={selected}
              editing={editing}
              onSelect={setSelected}
              onEdit={setEditing}
              onApply={apply}
            />
          ) : (
            <div className="stage-wrap empty">Rendu…</div>
          )}
          <div className="dictation" aria-label="Dictée">
            <button className="mic" disabled aria-label="Dicter (bientôt)">{Icon.mic}</button>
            <div className="spacer caption">Double-cliquez sur un texte pour le modifier · glissez un élément libre pour le déplacer</div>
            <span className="caption"><span className="key">⌘Z</span> annuler</span>
          </div>
        </main>
      </div>

      {toast && <div className={`toast${toast.error ? " error" : ""}`} role={toast.error ? "alert" : "status"}>{toast.text}</div>}
    </div>
  );
}
