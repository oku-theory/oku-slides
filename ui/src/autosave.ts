// L'enregistrement automatique : on n'enregistre jamais à la main.
//
// Chaque modification programme un enregistrement, repoussé tant que les
// modifications s'enchaînent ; jamais deux à la fois, et celle arrivée
// pendant une écriture est enregistrée juste après.

/** Délai après la dernière modification avant d'enregistrer (ms). */
export const AUTOSAVE_DELAY_MS = 1500;

/** Ce dont la décision a besoin dans un instantané. */
interface Saveable {
  dirty: boolean;
  path: string | null;
  /**
   * The pack being edited: saving creates the session's version, then replaces it. `leased`:
   * the tab may write it (it holds the lease, or needs none).
   */
  editing: { leased: boolean } | null;
  /** A theme tab's draft: saved only while the tab holds its theme's lease. */
  theming?: { leased: boolean } | null;
  /** Historique ouvert : la version montrée n'est qu'un aperçu. */
  history?: unknown;
}

/**
 * Should the tab save now? A modified deck, with or without a file (it goes to the default
 * folder); a modified pack, built-in included: one version per editing session, created at the
 * first save then replaced. `paused`: a save was refused (file replaced). Never while history
 * is open (the session would refuse; the deck set aside saves when it closes), and never for a
 * pack or theme without its lease (fail closed): such a tab would make its pre-restart work
 * current on its own once the holder closes.
 */
export function wantsAutosave(s: Saveable, paused: boolean): boolean {
  if (paused || !s.dirty || s.history) return false;
  if (s.theming) return s.theming.leased === true;
  return s.editing === null || s.editing.leased === true;
}

/** Enregistrements différés : `schedule` à chaque modification, `flush` avant de partir. */
export interface Autosave {
  schedule(): void;
  /** Enregistre tout de suite ce qui attend ; rien s'il n'y a rien. */
  flush(): Promise<void>;
  cancel(): void;
}

/** Programme `save` après `delay` sans modification ; les erreurs restent à `save`. */
export function createAutosave(save: () => Promise<void>, delay = AUTOSAVE_DELAY_MS): Autosave {
  let timer: ReturnType<typeof setTimeout> | null = null;
  /** L'écriture en cours, et celles qui la suivent (une modification arrivée pendant). */
  let cycle: Promise<void> | null = null;
  let again = false;

  const run = (): Promise<void> => {
    timer = null;
    if (cycle) {
      again = true;
      return cycle;
    }
    cycle = (async () => {
      do {
        again = false;
        await save().catch(() => {});
      } while (again);
      cycle = null;
    })();
    return cycle;
  };

  return {
    schedule() {
      if (timer) clearTimeout(timer);
      timer = setTimeout(run, delay);
    },
    async flush() {
      if (timer) {
        clearTimeout(timer);
        await run();
      } else if (cycle) await cycle;
    },
    cancel() {
      if (timer) clearTimeout(timer);
      timer = null;
    },
  };
}
