// Le mode présentation : la slide seule, sur tout l'écran ou toute la fenêtre.

import { useEffect, useRef, useState } from "react";
import type { Timers } from "./debounce";
import { REAL_TIMERS } from "./debounce";
import type { Screen } from "./fullscreen";
import { windowScreen } from "./fullscreen";
import { t } from "./i18n";
import type { PresentMode } from "./present";
import { presentKey } from "./present";
import { slideStep } from "./slideKeys";
import "./Presenter.css";

/** Délai d'immobilité de la souris avant de cacher le curseur. */
const IDLE_MS = 2000;

/** Attente maximale d'une étape du plein écran avant de passer à la suivante. */
const SCREEN_STEP_MS = 3000;

/**
 * Entrées et sorties du plein écran, l'une après l'autre (sortir puis vite représenter).
 * Une étape qui ne répond jamais (requête refusée en silence) ne bloque pas les suivantes.
 */
let screenQueue: Promise<unknown> = Promise.resolve();
function serial<T>(step: () => Promise<T>): Promise<T> {
  const next = screenQueue.then(step, step);
  screenQueue = Promise.race([next.catch(() => {}), new Promise((r) => setTimeout(r, SCREEN_STEP_MS))]);
  return next;
}

/**
 * Le plein écran d'une présentation qui vient de se démonter, quitté seulement après le commit.
 * Un remontage aussitôt (StrictMode, déplacement dans l'arbre) le reprend : sous macOS,
 * entrer, sortir et rentrer pendant l'animation laisse la fenêtre fenêtrée.
 */
let releasing: { screen: Screen; made: Promise<boolean>; kept: boolean } | null = null;

interface Props {
  mode: PresentMode;
  slides: readonly { id: string; hash: string }[];
  /** Slide montrée ; `null` : la première. */
  current: string | null;
  /** URL de l'image d'une slide (voir `useSvgCache`). */
  svg: (id: string, hash: string) => string | null;
  onShow: (id: string) => void;
  onExit: () => void;
  /** Le plein écran de la fenêtre (tests). */
  screen?: Screen;
  timers?: Timers;
}

/**
 * La slide `current` seule, à l'échelle de la place, sur fond noir.
 *
 * ← → Espace Page↑ Page↓ Début Fin et le clic naviguent, Échap sort ; ces
 * touches ne vont à rien d'autre. En mode `screen`, la fenêtre passe en plein
 * écran le temps de la présentation, et en sortir par le système la termine.
 */
export default function Presenter({ mode, slides, current, svg, onShow, onExit, screen = windowScreen, timers = REAL_TIMERS }: Props) {
  const index = Math.max(0, slides.findIndex((s) => s.id === current));
  const slide = slides[index];
  const url = slide ? svg(slide.id, slide.hash) : null;
  // La suivante se rend d'avance : pas d'attente au passage.
  const next = slides[index + 1];
  if (next) svg(next.id, next.hash);

  const latest = useRef({ index, count: slides.length, onShow, onExit });
  latest.current = { index, count: slides.length, onShow, onExit };
  const show = (to: number | null) => {
    if (to !== null) latest.current.onShow(slides[to].id);
  };

  // Les touches de la présentation, avant tous les autres raccourcis de la fenêtre.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const { index, count, onShow, onExit } = latest.current;
      const action = presentKey(e, index, count);
      if (!action) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      if (action.kind === "exit") onExit();
      else if (action.kind === "show") onShow(slides[action.index].id);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [slides]);

  // Plein écran le temps de la présentation ; quitté seulement s'il est de nous.
  useEffect(() => {
    if (mode !== "screen") return;
    const stop = screen.onLeft(() => latest.current.onExit());
    const prior = releasing?.screen === screen ? releasing : null;
    if (prior) prior.kept = true;
    releasing = null;
    const made =
      prior?.made ??
      serial(() => screen.enter()).catch((e) => {
        console.error("plein écran", e);
        return false;
      });
    return () => {
      stop();
      const release = { screen, made, kept: false };
      releasing = release;
      queueMicrotask(() => {
        if (release.kept) return;
        if (releasing === release) releasing = null;
        void serial(async () => {
          if (await made) await screen.leave();
        }).catch((e) => console.error("plein écran", e));
      });
    };
  }, [mode, screen]);

  // Le curseur se cache quand la souris ne bouge plus.
  const [idle, setIdle] = useState(false);
  const idleTimer = useRef<unknown>(null);
  const wake = () => {
    setIdle(false);
    if (idleTimer.current !== null) timers.clear(idleTimer.current);
    idleTimer.current = timers.set(() => setIdle(true), IDLE_MS);
  };
  useEffect(() => {
    wake();
    return () => {
      if (idleTimer.current !== null) timers.clear(idleTimer.current);
    };
  }, [timers]);

  return (
    <div
      className={`presenter${idle ? " idle" : ""}`}
      role="dialog"
      aria-modal="true"
      aria-label={t("ui-present")}
      onMouseMove={wake}
      onClick={() => show(slideStep(index, 1, slides.length))}
    >
      {url && <img src={url} alt={t("ui-present-slide", { index: index + 1, count: slides.length })} draggable={false} />}
    </div>
  );
}
