// Recherche de mise à jour demandée par le menu de l'app, faite par l'onglet actif ; son avis dit le résultat.

import { useCallback, useEffect, useRef } from "react";
import { errorText, type UpdateStatus } from "./api";
import { t } from "./i18n";

/** Ce que la fenêtre confie à son onglet actif pour chercher une mise à jour. */
export interface AppUpdates {
  check(): Promise<UpdateStatus | null>;
  /** Recherches demandées par le menu de l'app ; chaque hausse en lance une. */
  requested: number;
}

/** Lance une recherche à chaque demande du menu de l'app ; rien sans hôte. */
export function useUpdateCheck(updates: AppUpdates | undefined, say: (text: string, error?: boolean) => void): void {
  const check = updates?.check;
  const run = useCallback(() => {
    if (!check) return;
    check().then(
      (s) => s && !s.offer && say(t("ui-update-up-to-date")),
      (e) => say(errorText(e), true),
    );
  }, [check, say]);
  // Au montage, les demandes déjà comptées ne relancent rien.
  const seen = useRef(updates?.requested ?? 0);
  useEffect(() => {
    const n = updates?.requested ?? 0;
    if (n > seen.current) run();
    seen.current = n;
  }, [updates?.requested, run]);
}
