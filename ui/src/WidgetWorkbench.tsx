/**
 * Panneau Widgets : les fichiers du pack en cours (mode modèles ou deck),
 * l'éditeur Typst du fichier choisi et l'aperçu du widget seul.
 *
 * La frappe passe par `set_widget`, 400 ms après la dernière touche ; ses
 * statuts viennent de la réponse, puis l'aperçu est redemandé. L'agent passe par les mêmes opérations : quand
 * `revision` (version du pack) change, la liste est relue, et la source
 * aussi si ce n'est pas notre propre écriture.
 */
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { errorText, type TabApi, type WidgetPreview, type WidgetStatus } from "./api";
import { croppedSvg } from "./widgetPreview";
import { PREVIEW_DELAY_MS, debounced } from "./debounce";
import { t } from "./i18n";
import { Icon } from "./icons";
import TypstEditor from "./TypstEditor";
import { COMMON, declaredAs, diagnosticPlace, fileDiagnostics, listedDiagnostics, merged, nameAvailable, ordered, skeleton, stateOf, widgetLabel, withTrial } from "./widgetList";

interface Props {
  api: TabApi;
  /** Change avec la version du pack en cours (le deck ou les modèles). */
  revision: string;
  /** Une écriture a abouti : le snapshot est à relire. */
  onChanged: () => void;
  onClose: () => void;
  onError: (message: string) => void;
}

export default function WidgetWorkbench({ api, revision, onChanged, onClose, onError }: Props) {
  const [statuses, setStatuses] = useState<WidgetStatus[] | null>(null);
  const [current, setCurrent] = useState<string | null>(null);
  const [source, setSource] = useState<string | null>(null);
  const [editorKey, setEditorKey] = useState(0);
  const [preview, setPreview] = useState<WidgetPreview | null>(null);
  const [naming, setNaming] = useState<{ from: string | null; value: string } | null>(null);
  /** Fichiers dont la dernière écriture a échoué : le texte tapé n'est pas enregistré. */
  const [unsaved, setUnsaved] = useState<ReadonlySet<string>>(new Set());
  const markUnsaved = useCallback((name: string, on: boolean) => {
    setUnsaved((u) => {
      if (u.has(name) === on) return u;
      const next = new Set(u);
      if (on) next.add(name);
      else next.delete(name);
      return next;
    });
  }, []);
  /** Fichier dont la source est affichée, et son dernier texte enregistré. */
  const loaded = useRef<string | null>(null);
  const sent = useRef<string | null>(null);
  /** Le texte dont les statuts affichés sont le reflet (dernier lu ou écrit). */
  const [checked, setChecked] = useState<string | null>(null);
  /** Compteur des frappes et écritures : une source demandée avant lui est périmée. */
  const edits = useRef(0);
  /** Une frappe attend son écriture. */
  const unsent = useRef(false);
  /**
   * Renommages dans la file, par ancien nom. Une frappe tapée pendant l'un
   * d'eux vise encore l'ancien nom : s'il aboutit, elle est écrite sous le
   * nouveau, son nom déclaré suivant, pour ne pas recréer l'ancien fichier.
   */
  const moves = useRef(new Map<string, { to: string; done: boolean }>());
  /** Nom déclaré à corriger dans l'éditeur après un renommage pendant une frappe. */
  const [rewrite, setRewrite] = useState<{ find: string; replace: string; silent?: boolean } | null>(null);
  /** Opérations en cours (écritures, suppressions, renommages) ; la dernière de la file. */
  const writing = useRef(0);
  const inflight = useRef<Promise<unknown>>(Promise.resolve());
  /** Numéro du dernier aperçu demandé : un aperçu plus ancien est ignoré. */
  const previewSeq = useRef(0);
  // Rappels du parent, souvent écrits en ligne : lus par référence, pour que
  // l'écriture retardée ne soit ni recréée ni annulée à chaque rendu.
  const changedRef = useRef(onChanged);
  const errorRef = useRef(onError);
  useLayoutEffect(() => {
    changedRef.current = onChanged;
    errorRef.current = onError;
  });
  const changed = useCallback(() => changedRef.current(), []);

  const fail = useCallback((e: unknown) => errorRef.current(errorText(e)), []);
  /**
   * Statuts rendus par la dernière écriture, et le texte écrit. Ils portent
   * les diagnostics du rendu (mode modèles), que la relecture
   * (`widget_status`, la sonde) n'a pas. La première relecture qui suit les
   * garde si le fichier a bien ce texte (enregistré, pas un essai refusé du
   * mode deck), puis ils sont oubliés.
   */
  const trial = useRef<{ statuses: WidgetStatus[]; file: string; text: string } | null>(null);
  const refresh = useCallback(() => {
    const t = trial.current;
    trial.current = null;
    const written = t ? api.widgetSource(t.file).catch(() => null) : Promise.resolve(null);
    return Promise.all([api.widgetStatus(), written]).then(([list, source]) => {
      setStatuses(ordered(t && source === t.text ? withTrial(list, t.statuses) : list));
    });
  }, [api]);
  const clearPreview = useCallback(() => {
    previewSeq.current += 1;
    setPreview(null);
  }, []);
  const showPreview = useCallback(
    (name: string) => {
      if (name === COMMON) return clearPreview();
      const seq = ++previewSeq.current;
      api
        .previewWidget(name)
        .then((p) => {
          if (seq === previewSeq.current) setPreview(p);
        })
        .catch(fail);
    },
    [api, fail, clearPreview],
  );

  useEffect(() => {
    refresh().catch(fail);
  }, [refresh, revision, fail]);

  // Fichier choisi, ou pack changé ailleurs : sa source, sauf si l'éditeur a
  // changé depuis la demande (frappe ou écriture) ou que c'est notre texte.
  useEffect(() => {
    if (loaded.current !== current) {
      loaded.current = null;
      setSource(null);
      clearPreview();
    }
    if (!current) return;
    let alive = true;
    const asked = edits.current;
    api
      .widgetSource(current)
      .then((s) => {
        if (!alive) return;
        const stale = unsent.current || edits.current !== asked || writing.current > 0 || s === sent.current;
        if (loaded.current === current && stale) return;
        loaded.current = current;
        sent.current = s;
        setChecked(s);
        setSource(s);
        setEditorKey((k) => k + 1);
        showPreview(current);
      })
      .catch(fail);
    return () => {
      alive = false;
    };
  }, [api, current, revision, fail, showPreview, clearPreview]);

  /**
   * Met `task` à la suite des opérations en cours (écritures, suppressions,
   * renommages) : une à la fois, dans l'ordre, même si l'une est lente.
   * Un échec est dit, et la file continue.
   */
  const enqueue = useCallback(
    (task: () => Promise<unknown>) => {
      writing.current += 1;
      inflight.current = inflight.current
        .then(task)
        .catch(fail)
        .finally(() => {
          writing.current -= 1;
        });
    },
    [fail],
  );

  /** Écrit `text` dans `name` ; ses statuts, son aperçu, sa marque « non enregistré ». */
  const write = useCallback(
    (name: string, text: string) =>
      api.setWidget(name, text).then(
        (change) => {
          // Les statuts rendus sont ceux de `text` (même un essai refusé en mode deck) :
          // pas de relecture ici, celle du pack changé suffit.
          setStatuses((s) => merged(s, [change.status, ...change.broken]));
          setChecked(text);
          if (!change.saved) {
            // Essai refusé (mode deck, rendu en échec) : rien d'enregistré, le pack n'a pas
            // changé. Le texte tapé reste non enregistré, et une relecture ne l'écrase pas.
            markUnsaved(name, true);
            return;
          }
          // Enregistré seulement maintenant : un échec garde le texte tapé, non enregistré.
          sent.current = text;
          markUnsaved(name, false);
          trial.current = { statuses: [change.status, ...change.broken], file: name, text };
          showPreview(name);
          changed();
        },
        (e: unknown) => {
          markUnsaved(name, true);
          throw e;
        },
      ),
    [api, changed, showPreview, markUnsaved],
  );

  const push = useMemo(
    () =>
      debounced((name: string, text: string) => {
        unsent.current = false;
        // Un fichier vide vaudrait suppression : on attend du texte.
        if (text === "") return;
        edits.current += 1;
        const move = moves.current.get(name);
        // Renommé entre-temps : la frappe suit le fichier, nom déclaré compris.
        enqueue(() => (move?.done ? write(move.to, declaredAs(text, name, move.to)) : write(name, text)));
      }, PREVIEW_DELAY_MS),
    [enqueue, write],
  );
  // Fermeture : la dernière frappe part quand même.
  useEffect(() => () => push.flush(), [push]);

  /** Change de fichier ; la frappe en attente part d'abord. */
  const select = (name: string | null) => {
    push.flush();
    setCurrent(name);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || naming) return;
      e.preventDefault();
      e.stopPropagation();
      onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose, naming]);

  /** Le nom saisi : admis, et libre ; la liste doit être connue. */
  const nameOk = (n: { from: string | null; value: string }) => nameAvailable(statuses, n.value, n.from);

  const commitName = () => {
    if (!naming || !nameOk(naming)) return;
    const { from, value } = naming;
    setNaming(null);
    push.flush();
    if (from) rename(from, value);
    else
      enqueue(() =>
        api.setWidget(value, skeleton(value)).then((c) => {
          // Refusé (mode deck, rendu en échec) : rien n'est créé ; le premier diagnostic dit pourquoi.
          if (!c.saved) return fail(c.status.diagnostics[0]?.message ?? t("ui-widgets-unsaved"));
          markUnsaved(value, false);
          trial.current = { statuses: [c.status, ...c.broken], file: value, text: skeleton(value) };
          setStatuses((s) => merged(s, [c.status, ...c.broken]));
          changed();
          setCurrent(value);
        }),
      );
  };

  /**
   * Renomme `from` en `to`. L'éditeur ouvert sur `from` reste ouvert, texte
   * tapé compris : les frappes de l'attente vont à `to`, nom déclaré corrigé.
   */
  const rename = (from: string, to: string) => {
    const move = { to, done: false };
    moves.current.set(from, move);
    const typedSince = edits.current;
    const keep = unsaved.has(from);
    enqueue(() =>
      api
        .renameWidget(from, to)
        .then((list) => {
          move.done = true;
          // La frappe encore en attente part maintenant, vers `to`.
          push.flush();
          setUnsaved((u) => {
            if (!u.has(from)) return u;
            const next = new Set(u);
            next.delete(from);
            next.add(to);
            return next;
          });
          setStatuses(ordered(list));
          if (loaded.current === from) {
            loaded.current = to;
            // Texte tapé (ou non enregistré) : gardé, son nom déclaré suit, comme une frappe.
            const find = `widget("${from}"`;
            const replace = `widget("${to}"`;
            if (keep || edits.current !== typedSince) setRewrite({ find, replace });
            else {
              // Rien de tapé : l'éditeur montre ce que la session a écrit, sans écriture de plus
              // (la relecture de `to`, peut-être jugée périmée file non vide, n'y suffit pas).
              const renamed = declaredAs(sent.current ?? "", from, to);
              sent.current = renamed;
              setChecked(renamed);
              setRewrite({ find, replace, silent: true });
            }
          }
          changed();
          setCurrent((c) => (c === from ? to : c));
        })
        .finally(() => moves.current.delete(from)),
    );
  };

  const remove = () => {
    if (!current) return;
    // Le fichier part : sa frappe en attente aussi ; l'écriture déjà partie est attendue.
    push.cancel();
    unsent.current = false;
    const name = current;
    enqueue(() =>
      api.removeWidget(name).then((list) => {
        markUnsaved(name, false);
        setStatuses(ordered(list));
        changed();
        setCurrent((c) => (c === name ? null : c));
      }),
    );
  };

  const currentStatus = statuses?.find((s) => s.name === current);
  const editorDiagnostics = useMemo(() => (current ? fileDiagnostics(currentStatus, current) : []), [currentStatus, current]);
  const previewUrl = useMemo(
    () => {
      if (!preview?.svg) return null;
      // Un widget d'élément est cadré sur ce qu'il dessine, pas sur la slide entière.
      const svg = croppedSvg(preview.svg, preview.bbox ?? null);
      return URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
    },
    [preview],
  );
  useEffect(() => () => {
    if (previewUrl) URL.revokeObjectURL(previewUrl);
  }, [previewUrl]);

  return (
    <div className="picker-scrim" onPointerDown={onClose}>
      <div className="picker workbench" role="dialog" aria-modal="true" aria-labelledby="workbench-title" onPointerDown={(e) => e.stopPropagation()}>
        <div className="picker-head">
          <h2 id="workbench-title">{t("ui-widgets-title")}</h2>
          <span className="caption">{current && unsaved.has(current) ? t("ui-widgets-unsaved") : ""}</span>
          <button className="object token" aria-label={t("ui-widgets-close")} title={t("ui-widgets-close")} onClick={onClose}>{Icon.close}</button>
        </div>
        <div className="workbench-body">
          <nav className="workbench-list" aria-label={t("ui-widgets-files")}>
            <ul>
              {(statuses ?? []).map((s) => {
                const state = stateOf(s);
                return (
                  <li key={s.name}>
                    <button className="workbench-file" aria-current={s.name === current} onClick={() => select(s.name)}>
                      <span className={`pill ${state}`} role="img" aria-label={t(state === "valid" ? "ui-widgets-valid" : "ui-widgets-errors")} />
                      {widgetLabel(s.name)}
                      {unsaved.has(s.name) && (
                        <span className="workbench-unsaved" role="img" aria-label={t("ui-widgets-unsaved")} title={t("ui-widgets-unsaved")} />
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
            {statuses?.length === 0 && <p className="caption">{t("ui-widgets-empty")}</p>}
            {naming ? (
              <form className="workbench-name" onSubmit={(e) => { e.preventDefault(); commitName(); }}>
                <input
                  className="select"
                  autoFocus
                  aria-label={t("ui-widgets-name")}
                  aria-invalid={naming.value !== "" && !nameOk(naming)}
                  value={naming.value}
                  onChange={(e) => setNaming({ ...naming, value: e.target.value })}
                  onKeyDown={(e) => { if (e.key === "Escape") setNaming(null); }}
                />
              </form>
            ) : (
              <div className="workbench-actions">
                <button className="btn" disabled={statuses === null} onClick={() => setNaming({ from: null, value: "" })}>{t("ui-widgets-new")}</button>
                <button className="btn" disabled={statuses === null || !current || current === COMMON} onClick={() => current && setNaming({ from: current, value: current })}>{t("ui-widgets-rename")}</button>
                <button className="btn danger" disabled={!current} onClick={remove}>{t("ui-widgets-delete")}</button>
              </div>
            )}
          </nav>
          <section className="workbench-editor">
            {current && source !== null ? (
              <TypstEditor
                key={editorKey}
                rewrite={rewrite}
                text={source}
                diagnostics={editorDiagnostics}
                diagnosticsFor={checked}
                label={widgetLabel(current)}
                onChange={(text) => {
                  edits.current += 1;
                  unsent.current = true;
                  push(current, text);
                }}
              />
            ) : (
              <p className="caption">{t("ui-widgets-pick")}</p>
            )}
          </section>
          <aside className="workbench-preview" aria-label={t("ui-widgets-preview")}>
            {previewUrl ? <img src={previewUrl} alt={t("ui-widgets-preview")} /> : <p className="caption">{t("ui-widgets-no-preview")}</p>}
            <ul className="workbench-diagnostics">
              {listedDiagnostics(currentStatus, preview?.diagnostics ?? []).map((d, i) => {
                const place = diagnosticPlace(d);
                return (
                  <li key={i} className={d.error ? "error" : undefined}>
                    {place ? `${place} ` : ""}{d.message}
                  </li>
                );
              })}
            </ul>
          </aside>
        </div>
      </div>
    </div>
  );
}
