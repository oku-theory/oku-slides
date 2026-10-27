// Les actions de fichier d'un onglet : ouvrir (un .pptx passe par l'import), copier, et
// l'enregistrement automatique. Déplacer et fusionner se demandent à l'agent (t-988).

import { useEffect, useMemo, useRef, useState } from "react";
import type { RefObject } from "react";
import { pickOpenPath, pickSavePath, type Snapshot, type TabApi } from "./api";
import { createAutosave, wantsAutosave } from "./autosave";
import { t } from "./i18n";
import { isPptx } from "./importFile";
import { saveGuarded } from "./merge";

/** Ce dont les actions de fichier ont besoin de l'onglet. */
export interface FileActionsContext {
  api: TabApi;
  snap: Snapshot | null;
  /** Le dernier instantané accepté, à jour avant le rendu qui suit. */
  snapRef: RefObject<Snapshot | null>;
  /** Applique une requête de l'onglet ; `null` : refusée (déjà dite) ou abandonnée. */
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  say: (text: string, error?: boolean) => void;
  /** Ouvrir un deck : l'espace de travail choisit l'onglet. */
  onOpen: (path: string) => void;
  /** Un `.pptx` choisi : l'espace de travail ouvre son dialogue d'import. */
  onImportPick: (path: string) => void;
}

/** Les actions de fichier de l'onglet et son enregistrement automatique. */
export function useFileActions({ api, snap, snapRef, run, say, onOpen, onImportPick }: FileActionsContext) {
  // Un .pptx ne s'ouvre pas : le dialogue d'import choisit d'abord comment le reprendre.
  const open = async () => {
    const path = await pickOpenPath();
    if (path && isPptx(path)) onImportPick(path);
    else if (path) onOpen(path);
  };

  // --- Enregistrement automatique -----------------------------------------
  // On n'enregistre jamais à la main : chaque modification part au fichier
  // (au dossier par défaut pour un deck neuf) peu après la dernière.
  // Un fichier remplacé par un autre deck, et l'écrasement refusé : plus
  // d'enregistrement automatique jusqu'à un déplacement (l'agent, outil `move`).
  const [autosavePaused, setAutosavePaused] = useState(false);
  const pausedRef = useRef(autosavePaused);
  pausedRef.current = autosavePaused;
  const autosave = useMemo(
    () =>
      createAutosave(async () => {
        const s = snapRef.current;
        if (!s || !wantsAutosave(s, pausedRef.current)) return;
        if (s.editing) {
          await run(api.savePack());
          return;
        }
        // A theme tab saves its draft; the answer is a pin, the snapshot is read after it.
        if (s.theming) {
          await run(api.saveTheme().then(() => api.snapshot()));
          return;
        }
        const saved = await run(saveGuarded(api, s));
        if (!saved) setAutosavePaused(true);
      }),
    [api, run],
  );
  useEffect(() => {
    if (snap && wantsAutosave(snap, autosavePaused)) autosave.schedule();
  }, [snap, autosavePaused, autosave]);
  // Le deck a changé de fichier (déplacé) : l'enregistrement automatique reprend.
  const path = snap?.path ?? null;
  useEffect(() => {
    setAutosavePaused(false);
  }, [path]);
  // Onglet quitté (`App` remonté) : ce qui attend part tout de suite.
  useEffect(() => () => void autosave.flush(), [autosave]);

  /** Écrit une copie du deck ailleurs ; il garde son fichier. */
  const saveCopy = async () => {
    if (!snap) return;
    const path = await pickSavePath(snap.name);
    if (!path) return;
    if (await run(api.saveCopy(path))) say(t("ui-toast-copied"));
  };

  return { open, autosave, saveCopy };
}
