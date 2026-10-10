// Le plein écran de la fenêtre : natif dans Tauri, l'API Fullscreen du navigateur en développement.

import { inTauri } from "./api";

/**
 * Le plein écran de la fenêtre.
 *
 * Implémenter cette interface pour remplacer l'écran de la fenêtre (tests).
 */
export interface Screen {
  /** Passe en plein écran ; `true` : c'est cet appel qui l'a fait (il ne l'était pas). */
  enter(): Promise<boolean>;
  /** Quitte le plein écran. */
  leave(): Promise<void>;
  /** Prévient quand la fenêtre quitte le plein écran (bouton vert, ⌃⌘F) ; rend l'arrêt de l'écoute. */
  onLeft(cb: () => void): () => void;
}

/**
 * Appelle `cb` à la sortie du plein écran, une fois qu'il a été atteint.
 * Pendant l'animation d'entrée, la fenêtre se dit encore fenêtrée : ce n'est pas une sortie.
 */
function afterReached(isFull: () => Promise<boolean>, cb: () => void): () => Promise<void> {
  let reached = false;
  return async () => {
    if (await isFull()) reached = true;
    else if (reached) cb();
  };
}

/** Le plein écran de la fenêtre Tauri. */
const tauriScreen: Screen = {
  async enter() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const w = getCurrentWindow();
    if (await w.isFullscreen()) return false;
    await w.setFullscreen(true);
    return true;
  },
  async leave() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().setFullscreen(false);
  },
  onLeft(cb) {
    let stop: (() => void) | undefined;
    let dead = false;
    import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) => {
        const w = getCurrentWindow();
        const check = afterReached(() => w.isFullscreen(), cb);
        return w.onResized(() => void check());
      })
      .then((un) => (dead ? un() : (stop = un)))
      .catch((e) => console.error("plein écran", e));
    return () => {
      dead = true;
      stop?.();
    };
  },
};

/** Le plein écran du document (navigateur, développement). */
const documentScreen: Screen = {
  async enter() {
    if (document.fullscreenElement) return false;
    await document.documentElement.requestFullscreen();
    return true;
  },
  async leave() {
    if (document.fullscreenElement) await document.exitFullscreen();
  },
  onLeft(cb) {
    const check = afterReached(async () => document.fullscreenElement !== null, cb);
    const on = () => void check();
    document.addEventListener("fullscreenchange", on);
    return () => document.removeEventListener("fullscreenchange", on);
  },
};

/** L'écran de cette fenêtre. */
export const windowScreen: Screen = inTauri ? tauriScreen : documentScreen;
