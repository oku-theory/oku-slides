// Les raccourcis des onglets d'une fenêtre (⌘T, ⌘W, ⌘1…9, Ctrl+Tab…).

import { useEffect, useRef } from "react";
import { tabShortcut, type TabAction } from "./tabs";

/** Ce que les raccourcis consultent à chaque touche. */
export interface TabShortcutOptions {
  /** ⌘ sur macOS, Ctrl ailleurs. */
  mac: boolean;
  /** La fenêtre peut en ouvrir une autre (Tauri) : ⌘⇧N compte. */
  windows: boolean;
  /** Un dialogue de la fenêtre est ouvert : aucun raccourci d'onglet ne passe derrière lui. */
  held: () => boolean;
  onAction: (action: TabAction) => void;
}

/**
 * Écoute les raccourcis des onglets tant que `enabled`. Retenus quand `held()` ; pendant une
 * saisie (texte d'une slide, champ), changer d'onglet perdrait ce qui n'est pas validé : seul
 * Ctrl+Tab reste actif. Une touche prise est `preventDefault`.
 */
export function useTabShortcuts(enabled: boolean, options: TabShortcutOptions) {
  const latest = useRef(options);
  latest.current = options;
  useEffect(() => {
    if (!enabled) return;
    const onKey = (e: KeyboardEvent) => {
      const { mac, windows, held, onAction } = latest.current;
      const action = tabShortcut(e, mac);
      if (!action || held()) return;
      if (action.kind === "new-window" && !windows) return;
      const typing = (e.target as HTMLElement | null)?.closest?.("input, textarea, select, [contenteditable]");
      if (typing && action.kind !== "cycle") return;
      e.preventDefault();
      onAction(action);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enabled]);
}
