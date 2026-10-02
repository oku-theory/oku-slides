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
  /** Modèles en cours d'édition : enregistrer crée la version de la session, puis la remplace. */
  editing: unknown;
  /** Historique ouvert : la version montrée n'est qu'un aperçu. */
  history?: unknown;
}

/**
 * Faut-il enregistrer ? Un deck modifié, avec ou sans fichier (il ira au
 * dossier par défaut) ; des modèles modifiés, intégrés compris : une version
 * par session d'édition, créée au premier enregistrement puis remplacée.
 * `paused` : un enregistrement a été refusé (fichier remplacé). Jamais
 * pendant l'historique (la session le refuserait) : le deck mis de côté
 * s'enregistre à sa fermeture.
 */
export function wantsAutosave(s: Saveable, paused: boolean): boolean {
  return !paused && s.dirty && !s.history;
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
