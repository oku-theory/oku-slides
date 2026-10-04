/**
 * Fichiers de widgets du pack édité : statuts, frappes, une file d'opérations.
 *
 * Partagé par la pile et les sous-onglets. Chaque fichier a son délai de
 * frappe (400 ms) ; toutes les opérations passent l'une après l'autre, dans
 * l'ordre. Règles de t-390 conservées : essai refusé (`saved: false`),
 * diagnostics du rendu gardés à la première relecture, frappe pendant un
 * renommage redirigée vers le nouveau nom.
 */
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { errorText, type TabApi, type WidgetStatus } from "./api";
import { PREVIEW_DELAY_MS, debounced } from "./debounce";
import { t } from "./i18n";
import type { Rewrite } from "./TypstEditor";
import { declaredAs, merged, ordered, skeleton, withTrial } from "./widgetList";

export interface WidgetFiles {
  /** Statuts du pack, `_commun` en tête ; `null` hors mode modèles ou pas encore lus. */
  statuses: WidgetStatus[] | null;
  /** La `revision` pour laquelle `statuses` a été lu ; `null` sans statuts. */
  statusesFor: string | null;
  /** Fichiers dont le texte tapé n'est pas enregistré. */
  unsaved: ReadonlySet<string>;
  /** Par fichier : le texte dont les statuts affichés sont le reflet. */
  checked: ReadonlyMap<string, string>;
  /** Par fichier : nombre d'écritures enregistrées (l'aperçu se redemande). */
  writes: ReadonlyMap<string, number>;
  /** Par fichier (nouveau nom) : remplacement à faire après un renommage. */
  rewrites: ReadonlyMap<string, Rewrite>;
  /** Frappe dans `name`, écrite PREVIEW_DELAY_MS après la dernière. */
  edit(name: string, text: string): void;
  /** Compteur des frappes et écritures de `name`, pour dater une lecture. */
  edits(name: string): number;
  /** Une source de `name` lue à `asked` est périmée : ne pas l'afficher. */
  stale(name: string, asked: number, source: string): boolean;
  /** La source de `name` vient d'être affichée. */
  loaded(name: string, source: string): void;
  /** Envoie la frappe en attente de `name`, ou de tous les fichiers. */
  flush(name?: string): void;
  /** Toutes les frappes envoyées et la file vide. */
  settle(): Promise<void>;
  /** Crée `name` avec son squelette ; vrai si créé. */
  create(name: string): Promise<boolean>;
  /** Renomme ; vrai si fait. */
  rename(from: string, to: string): Promise<boolean>;
  /** Supprime ; vrai si fait. */
  remove(name: string): Promise<boolean>;
  /** Dit une erreur (toast du parent). */
  fail(e: unknown): void;
  /** Frappes non envoyées, écriture en cours, ou dernière écriture non enregistrée. */
  pending(name: string): boolean;
  /** Les frappes de `name` gagnent : envoyées tout de suite, ou le dernier texte tenté renvoyé. */
  keep(name: string): void;
  /** Abandonne les frappes de `name` et ses écritures en file (fichier supprimé ailleurs). */
  forget(name: string): void;
}

type Push = ReturnType<typeof debounced<[string]>>;
interface Trial { statuses: WidgetStatus[]; text: string }
interface Callbacks { changed(): void; error(message: string): void; renamed(from: string, to: string): void }

/** `map` où `key` vaut `value` (nouvelle table, pour React). */
function withEntry<V>(map: ReadonlyMap<string, V>, key: string, value: V): ReadonlyMap<string, V> {
  const next = new Map(map);
  next.set(key, value);
  return next;
}

/** `set` avec ou sans `name` ; le même objet s'il ne change pas. */
function toggled(set: ReadonlySet<string>, name: string, on: boolean): ReadonlySet<string> {
  if (set.has(name) === on) return set;
  const next = new Set(set);
  if (on) next.add(name);
  else next.delete(name);
  return next;
}

const EMPTY_SET: ReadonlySet<string> = new Set();
const EMPTY_MAP = new Map<string, never>();

/**
 * `pack` : le pack édité (`null` hors mode modèles : rien n'est lu). Un
 * autre pack remet tout à zéro, frappes en attente annulées.
 * `on.renamed` est appelé dans la file, avant les nouveaux statuts.
 */
export function useWidgetFiles(api: TabApi, pack: string | null, revision: string, on: Callbacks): WidgetFiles {
  const [statuses, setStatuses] = useState<WidgetStatus[] | null>(null);
  const [statusesFor, setStatusesFor] = useState<string | null>(null);
  const [unsaved, setUnsaved] = useState<ReadonlySet<string>>(EMPTY_SET);
  const [checked, setChecked] = useState<ReadonlyMap<string, string>>(EMPTY_MAP);
  const [writes, setWrites] = useState<ReadonlyMap<string, number>>(EMPTY_MAP);
  const [rewrites, setRewrites] = useState<ReadonlyMap<string, Rewrite>>(EMPTY_MAP);
  /** Copie synchrone de `unsaved`, lue dans la file (jamais pendant le rendu). */
  const unsavedNow = useRef(new Set<string>());
  /** Dernier texte enregistré (ou lu) de chaque fichier. */
  const sent = useRef(new Map<string, string>());
  /** Fichiers dont une frappe attend son écriture. */
  const unsent = useRef(new Set<string>());
  /** Par fichier : frappes et écritures (date une lecture), écritures en cours. */
  const edits = useRef(new Map<string, number>());
  const writing = useRef(new Map<string, number>());
  const inflight = useRef<Promise<unknown>>(Promise.resolve());
  const moves = useRef(new Map<string, { to: string; done: boolean }>());
  const trials = useRef(new Map<string, Trial>());
  /** Essais pris par la relecture en vol : rendus à la suivante si elle la remplace. */
  const reread = useRef(new Map<string, Trial>());
  const pushes = useRef(new Map<string, Push>());
  /** Par fichier : le dernier texte tapé, tant qu'il attend son délai. */
  const typing = useRef(new Map<string, string>());
  /** Dernier texte tenté par fichier (enregistré ou non). */
  const attempted = useRef(new Map<string, string>());
  /** Génération d'abandon par fichier : une écriture en file d'une génération passée ne part pas. */
  const forgotten = useRef(new Map<string, number>());
  const refreshSeq = useRef(0);
  /** Une relecture des statuts est en vol. */
  const reading = useRef(false);
  /** Change à chaque changement de pack : le travail en vol d'un ancien pack est ignoré. */
  const generation = useRef(0);
  const onRef = useRef(on);
  const apiRef = useRef(api);
  const revisionRef = useRef(revision);
  useLayoutEffect(() => {
    onRef.current = on;
    apiRef.current = api;
    revisionRef.current = revision;
  });

  const bump = (m: Map<string, number>, name: string, by: number) => m.set(name, (m.get(name) ?? 0) + by);
  const fail = useCallback((e: unknown) => onRef.current.error(errorText(e)), []);
  const markUnsaved = useCallback((name: string, yes: boolean) => {
    if (yes) unsavedNow.current.add(name);
    else unsavedNow.current.delete(name);
    setUnsaved((u) => toggled(u, name, yes));
  }, []);
  const check = useCallback((name: string, text: string) => setChecked((c) => withEntry(c, name, text)), []);

  // Un autre pack (ou la sortie du mode modèles) : tout repart de zéro.
  useEffect(() => {
    generation.current++;
    refreshSeq.current++;
    reading.current = false;
    pushes.current.forEach((p) => p.cancel());
    pushes.current.clear();
    for (const r of [sent, unsent, edits, writing, moves, trials, reread, typing, attempted]) r.current.clear();
    unsavedNow.current.clear();
    setStatuses(null);
    setStatusesFor(null);
    setUnsaved(EMPTY_SET);
    setChecked(EMPTY_MAP);
    setWrites(EMPTY_MAP);
    setRewrites(EMPTY_MAP);
  }, [pack]);

  /** Relit les statuts pour la revision courante ; une réponse plus ancienne qu'une autre est ignorée. */
  const refresh = useCallback(() => {
    const seq = ++refreshSeq.current;
    const rev = revisionRef.current;
    const read = apiRef.current;
    reading.current = true;
    // Une relecture remplacée rend ses essais ; un essai plus récent du même fichier l'emporte.
    for (const [file, tr] of reread.current) if (!trials.current.has(file)) trials.current.set(file, tr);
    const pending = [...trials.current];
    trials.current.clear();
    reread.current = new Map(pending);
    const written = Promise.all(pending.map(([file]) => read.widgetSource(file).catch(() => null)));
    Promise.all([read.widgetStatus(), written])
      .then(([list, sources]) => {
        if (seq !== refreshSeq.current) return;
        reading.current = false;
        reread.current.clear();
        let shown = list;
        pending.forEach(([, tr], i) => {
          if (sources[i] === tr.text) shown = withTrial(shown, tr.statuses);
        });
        setStatuses(ordered(shown));
        setStatusesFor(rev);
      })
      .catch((e: unknown) => {
        if (seq !== refreshSeq.current) return;
        reading.current = false;
        reread.current.clear();
        fail(e);
      });
  }, [fail]);
  /**
   * Un changement local vient d'arriver : une relecture en vol rendrait la liste
   * d'avant (un sous-onglet renommé élagué, un widget supprimé revenu), on la relance.
   */
  const invalidate = useCallback(() => {
    if (reading.current) refresh();
  }, [refresh]);

  // Statuts relus quand le pack ou la revision change.
  useEffect(() => {
    if (pack !== null) refresh();
  }, [api, pack, revision, refresh]);

  /** `task` à la suite de la file ; sa valeur, ou `false` après un échec dit. */
  const enqueue = useCallback(
    <T,>(task: (live: () => boolean) => Promise<T>): Promise<T | false> => {
      const born = generation.current;
      const live = () => born === generation.current;
      const run = inflight.current
        .then((): Promise<T | false> | false => (live() ? task(live) : false))
        .catch((e: unknown) => {
          if (live()) fail(e);
          return false as const;
        });
      inflight.current = run;
      return run;
    },
    [fail],
  );

  /** Écrit `text` dans `name` : statuts, essai, marque, compte des écritures. */
  const write = useCallback(
    (name: string, text: string, live: () => boolean) => {
      attempted.current.set(name, text);
      bump(writing.current, name, 1);
      return apiRef.current
        .setWidget(name, text)
        .then(
          (change) => {
            if (!live()) return;
            setStatuses((s) => merged(s, [change.status, ...change.broken]));
            check(name, text);
            if (!change.saved) {
              markUnsaved(name, true);
              return;
            }
            sent.current.set(name, text);
            markUnsaved(name, false);
            trials.current.set(name, { statuses: [change.status, ...change.broken], text });
            setWrites((w) => withEntry(w, name, (w.get(name) ?? 0) + 1));
            invalidate();
            onRef.current.changed();
          },
          (e: unknown) => {
            if (live()) markUnsaved(name, true);
            throw e;
          },
        )
        .finally(() => {
          if (live()) bump(writing.current, name, -1);
        });
    },
    [check, markUnsaved, invalidate],
  );

  /**
   * Met en file l'écriture `task` de `names` (le fichier tapé, et sa cible s'il est renommé) :
   * elle ne part pas si l'un d'eux est oublié (`forget`) avant son tour.
   */
  const enqueueWrite = useCallback(
    (names: string[], task: (live: () => boolean) => Promise<unknown>) => {
      const born = names.map((n) => forgotten.current.get(n) ?? 0);
      const kept = () => names.every((n, i) => (forgotten.current.get(n) ?? 0) === born[i]);
      return enqueue((live) => (kept() ? task(live) : Promise.resolve(false)));
    },
    [enqueue],
  );

  /** Le délai de frappe de `name`, créé au premier besoin. */
  const pushOf = useCallback(
    (name: string): Push => {
      let p = pushes.current.get(name);
      if (!p) {
        p = debounced((text: string) => {
          unsent.current.delete(name);
          typing.current.delete(name);
          // Un fichier vide vaudrait suppression : on attend du texte.
          if (text === "") return;
          bump(edits.current, name, 1);
          const move = moves.current.get(name);
          void enqueueWrite(move ? [name, move.to] : [name], (live) =>
            move?.done ? write(move.to, declaredAs(text, name, move.to), live) : write(name, text, live),
          );
        }, PREVIEW_DELAY_MS);
        pushes.current.set(name, p);
      }
      return p;
    },
    [enqueueWrite, write],
  );
  // Démontage : les frappes partent quand même.
  useEffect(() => () => pushes.current.forEach((p) => p.flush()), []);

  const edit = useCallback(
    (name: string, text: string) => {
      bump(edits.current, name, 1);
      unsent.current.add(name);
      typing.current.set(name, text);
      pushOf(name)(text);
    },
    [pushOf],
  );

  const flush = useCallback((name?: string) => {
    if (name === undefined) pushes.current.forEach((p) => p.flush());
    else pushes.current.get(name)?.flush();
  }, []);

  const pending = useCallback(
    (name: string) => unsent.current.has(name) || unsavedNow.current.has(name) || (writing.current.get(name) ?? 0) > 0,
    [],
  );
  const keep = useCallback(
    (name: string) => {
      if (unsent.current.has(name)) {
        pushes.current.get(name)?.flush();
        return;
      }
      // Non enregistrée, ou en vol (l'agent a pu écrire après) : le dernier texte tenté
      // repart, en file derrière l'écriture en vol (au pire une écriture de trop).
      const last = attempted.current.get(name);
      const inFlight = (writing.current.get(name) ?? 0) > 0;
      if ((unsavedNow.current.has(name) || inFlight) && last !== undefined) {
        edit(name, last);
        pushes.current.get(name)?.flush();
      }
    },
    [edit],
  );
  const forget = useCallback(
    (name: string) => {
      forgotten.current.set(name, (forgotten.current.get(name) ?? 0) + 1);
      pushes.current.get(name)?.cancel();
      pushes.current.delete(name);
      // `moves` aussi : un renommage en cours de `name` ne redirigera plus sa frappe vers le nouveau nom.
      for (const r of [unsent, typing, sent, attempted, moves, trials, reread]) r.current.delete(name);
      markUnsaved(name, false);
    },
    [markUnsaved],
  );

  const settle = useCallback(async () => {
    // Une opération mise en file (ou une frappe arrivée) pendant l'attente est attendue aussi.
    let seen: Promise<unknown>;
    do {
      flush();
      seen = inflight.current;
      await seen;
    } while (seen !== inflight.current || unsent.current.size > 0);
  }, [flush]);

  const create = useCallback(
    (name: string) => {
      flush();
      return enqueue((live) =>
        apiRef.current.setWidget(name, skeleton(name)).then((c) => {
          if (!live()) return false;
          if (!c.saved) {
            onRef.current.error(c.status.diagnostics[0]?.message ?? t("ui-widgets-unsaved"));
            return false;
          }
          markUnsaved(name, false);
          sent.current.set(name, skeleton(name));
          trials.current.set(name, { statuses: [c.status, ...c.broken], text: skeleton(name) });
          setStatuses((s) => merged(s, [c.status, ...c.broken]));
          invalidate();
          onRef.current.changed();
          return true;
        }),
      ).then((ok) => ok === true);
    },
    [enqueue, flush, markUnsaved, invalidate],
  );

  const rename = useCallback(
    (from: string, to: string) => {
      const move = { to, done: false };
      moves.current.set(from, move);
      const typedSince = edits.current.get(from) ?? 0;
      return enqueue((live) =>
        apiRef.current
          .renameWidget(from, to)
          .then((list) => {
            if (!live()) return false;
            move.done = true;
            const keep = unsavedNow.current.has(from);
            // La frappe encore en attente part maintenant, vers `to`.
            pushes.current.get(from)?.flush();
            pushes.current.delete(from);
            if (keep) {
              markUnsaved(from, false);
              markUnsaved(to, true);
            }
            const find = `widget("${from}"`;
            const replace = `widget("${to}"`;
            const known = sent.current.get(from);
            const text = known === undefined ? undefined : declaredAs(known, from, to);
            if (text !== undefined) sent.current.set(to, text);
            sent.current.delete(from);
            if (text === undefined || keep || (edits.current.get(from) ?? 0) !== typedSince) {
              setRewrites((r) => withEntry(r, to, { find, replace }));
            } else {
              // Rien de tapé : l'éditeur montre ce que la session a écrit, sans écriture de plus.
              check(to, text);
              setRewrites((r) => withEntry(r, to, { find, replace, silent: true }));
            }
            edits.current.set(to, (edits.current.get(from) ?? 0) + 1);
            // Le sous-onglet suit avant les statuts sans `from` : il n'est pas élagué.
            onRef.current.renamed(from, to);
            setStatuses(ordered(list));
            invalidate();
            onRef.current.changed();
            return true;
          })
          .finally(() => {
            if (live()) moves.current.delete(from);
          }),
      ).then((ok) => ok === true);
    },
    [enqueue, check, markUnsaved, invalidate],
  );

  const remove = useCallback(
    (name: string) => {
      // Le fichier part : sa frappe en attente aussi ; l'écriture déjà partie est attendue.
      const held = unsent.current.has(name) ? typing.current.get(name) : undefined;
      const typedAt = edits.current.get(name) ?? 0;
      const born = generation.current;
      pushes.current.get(name)?.cancel();
      pushes.current.delete(name);
      unsent.current.delete(name);
      typing.current.delete(name);
      return enqueue((live) =>
        apiRef.current.removeWidget(name).then((list) => {
          if (!live()) return false;
          markUnsaved(name, false);
          sent.current.delete(name);
          setStatuses(ordered(list));
          invalidate();
          onRef.current.changed();
          return true;
        }),
      ).then((ok) => {
        if (ok === true) return true;
        // Refusé (un modèle l'utilise) : la frappe annulée revient, sauf si une plus récente la remplace.
        if (held && born === generation.current && (edits.current.get(name) ?? 0) === typedAt) {
          markUnsaved(name, true);
          bump(edits.current, name, 1);
          void enqueueWrite([name], (live) => write(name, held, live));
        }
        return false;
      });
    },
    [enqueue, enqueueWrite, markUnsaved, invalidate, write],
  );

  const loaded = useCallback(
    (name: string, source: string) => {
      sent.current.set(name, source);
      check(name, source);
    },
    [check],
  );

  const stale = useCallback(
    (name: string, asked: number, source: string) =>
      unsent.current.has(name) ||
      (edits.current.get(name) ?? 0) !== asked ||
      (writing.current.get(name) ?? 0) > 0 ||
      moves.current.has(name) ||
      source === sent.current.get(name),
    [],
  );

  const editsOf = useCallback((name: string) => edits.current.get(name) ?? 0, []);

  return useMemo(
    () => ({
      statuses, statusesFor, unsaved, checked, writes, rewrites,
      edit, edits: editsOf, stale, loaded, flush, settle, create, rename, remove, fail, pending, keep, forget,
    }),
    [statuses, statusesFor, unsaved, checked, writes, rewrites, edit, editsOf, stale, loaded, flush, settle, create, rename, remove, fail, pending, keep, forget],
  );
}
