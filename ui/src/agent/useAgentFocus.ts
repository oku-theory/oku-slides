// Le focus demandé par l'agent (évènement `focus` de ses outils) : montrer une slide, choisir un élément.

import { useEffect, useRef, useState } from "react";
import type { SlideSummary, SlideView } from "../api";
import { findElement } from "../deck";

type Focus = { slide: string; element: string | null };

/**
 * Le rappel d'un focus de l'agent. Pendant l'édition d'un texte, il est différé : seul le dernier
 * reçu compte, appliqué quand l'édition se termine (validation ou annulation), sauf si l'utilisateur
 * a changé de slide entre-temps ; l'agent ne change pas de slide sous le curseur. Une slide inconnue
 * du deck est ignorée. L'élément est cherché dans une vue de sa slide plus récente que le focus si
 * celle d'alors ne l'a pas (l'agent vient de le créer) ; introuvable, la sélection reste en place.
 * `editing`, `slides`, `current` et `view` sont lus par des références : un abonnement posé une fois
 * peut appeler une ancienne version du rappel.
 */
export function useAgentFocus({ slides, view, current, editing, show, select }: {
  slides: SlideSummary[] | undefined;
  view: SlideView | null;
  current: string | null;
  editing: string | null;
  show: (slide: string) => void;
  select: (element: string | null) => void;
}): (slide: string, element: string | null) => void {
  const seen = useRef({ editing, slides, current, view });
  seen.current = { editing, slides, current, view };
  /** Le dernier focus reçu pendant l'édition, et la slide affichée à ce moment. */
  const deferred = useRef<{ focus: Focus; from: string | null } | null>(null);
  /** Le focus appliqué dont l'élément attend une vue de sa slide ; `stale` : la vue affichée à sa réception. */
  const [want, setWant] = useState<(Focus & { stale: SlideView | null }) | null>(null);
  const apply = (f: Focus) => {
    if (!seen.current.slides?.some((s) => s.id === f.slide)) return;
    show(f.slide);
    setWant({ ...f, stale: seen.current.view });
  };
  useEffect(() => {
    const d = deferred.current;
    if (editing !== null || !d) return;
    deferred.current = null;
    // L'utilisateur a changé de slide pendant l'édition : il n'y est pas ramené.
    if (current === d.from) apply(d.focus);
  }, [editing]);
  useEffect(() => {
    if (!want) return;
    // L'utilisateur est allé ailleurs entre-temps : l'élément n'est plus attendu.
    if (current !== want.slide) return setWant(null);
    if (view?.slide.id !== want.slide) return;
    if (want.element === null) {
      setWant(null);
      select(null);
    } else if (findElement(view.slide, want.element)) {
      setWant(null);
      select(want.element);
    } else if (view !== want.stale) {
      // Une vue plus récente que le focus ne l'a pas non plus : identifiant inconnu, ignoré.
      setWant(null);
    }
  }, [want, view, current]);
  return (slide, element) => {
    const focus = { slide, element };
    if (seen.current.editing !== null) deferred.current = { focus, from: seen.current.current };
    else apply(focus);
  };
}
