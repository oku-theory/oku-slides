// Réglages de l'hôte en mode modèles (modèle affiché, ou widget du sous-onglet
// actif, à ses valeurs d'exemple) : une surface de réglages construite par la
// session. Chaque champ porte sa clé ; une action part avec la slide et le
// widget de la surface affichée. Un réglage refusé relit la surface : le champ reprend la
// valeur de la session, celle que dessine l'aperçu.

import { useEffect, useRef, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Snapshot, Surface, TabApi } from "./api";
import type { ActionOutcome } from "./a2ui/actionOutcome";
import type { ListChange } from "./a2ui/ListTable";
import { fromA2ui } from "./agent/quick";

/** Surface des réglages du modèle affiché. */
export const HOST_TEMPLATE = "host-template";
/** Surface des réglages du widget du sous-onglet actif. */
export const HOST_WIDGET = "host-widget";

/** La surface de réglages de l'hôte, et le routage de ses actions. */
export interface HostSettings {
  surface: Surface | null;
  /**
   * Vrai pour toute surface de l'hôte (jamais transmise à l'agent) : l'action part si `s`
   * est la surface affichée et porte une clé ; une surface remplacée n'envoie rien.
   */
  onAction(s: Surface, a: A2uiClientAction): boolean;
  /**
   * Issue de l'action en cours d'envoi, posée par `onAction` (à fournir aux champs par
   * `ActionOutcomeContext`) : vraie si la session l'a acceptée ; fausse si refusée ou si rien n'est parti.
   */
  outcome: ActionOutcome;
}

/** Réglages de l'hôte : relus quand la slide, le widget, ses valeurs d'exemple ou le deck changent. */
export function useHostSettings(
  api: TabApi,
  snap: Snapshot | null,
  current: string | null,
  widget: string | null,
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>,
  say: (m: string, error?: boolean) => void,
): HostSettings {
  // `sync` : le numéro de relecture forcée qui a produit la surface.
  const [shown, setShown] = useState<{ surface: Surface; slide: string; widget: string | null; sync: number } | null>(null);
  // Lue au moment de l'action : une surface remplacée depuis n'envoie rien, quel que soit le rendu
  // dont vient le gestionnaire appelé.
  const shownRef = useRef(shown);
  shownRef.current = shown;
  // Relecture forcée après un refus : la surface est reconstruite même si son contenu n'a pas changé
  // (le modèle de données du champ porte la valeur refusée).
  const [sync, setSync] = useState(0);
  const outcome = useRef<Promise<boolean> | null>(null);
  const active = !!current && !!snap?.editing && !snap.history;
  const slide = snap?.slides.find((s) => s.id === current);
  // Tout ce qui peut changer la carte : pack, valeurs d'exemple, slide (rendu, nom), annulation.
  const key = [snap?.editing?.base, snap?.templates?.version, snap?.widget_trials, slide?.hash, slide?.name, snap?.undo_label, snap?.redo_label].join("|");
  useEffect(() => {
    if (!active || !current) return setShown(null);
    let alive = true;
    api
      .hostSettings(current, widget)
      // Même contenu : la surface est gardée (SurfaceView reconstruit une surface reçue à nouveau),
      // sauf après un refus.
      .then((surface) =>
        alive &&
        setShown((prev) =>
          surface &&
          prev &&
          prev.slide === current &&
          prev.widget === widget &&
          prev.sync === sync &&
          JSON.stringify(prev.surface) === JSON.stringify(surface)
            ? prev
            : surface
              ? { surface, slide: current, widget, sync }
              : null,
        ),
      )
      .catch((e) => alive && say(String(e), true));
    return () => {
      alive = false;
    };
    // `say` ne fait que poser un toast : son identité ne compte pas.
  }, [api, active, current, widget, key, sync]);
  const onAction = (s: Surface, a: A2uiClientAction): boolean => {
    if (s.id !== HOST_TEMPLATE && s.id !== HOST_WIDGET) return false;
    const now = shownRef.current;
    outcome.current = Promise.resolve(false);
    if (!now || s !== now.surface) return true;
    const { name, context } = fromA2ui(a);
    const field = context.key;
    if (name !== "param" || typeof field !== "string") return true;
    outcome.current = run(
      s.id === HOST_WIDGET && now.widget
        ? // Le widget que montre la surface.
          exampleValue(api, now.widget, field, context)
        : api.hostParam(now.slide, field, context.value),
    ).then((done) => {
      // Refusé (dit en toast par `run`) : l'instantané n'a pas changé, la surface est relue.
      if (!done) setSync((n) => n + 1);
      return !!done;
    });
    return true;
  };
  return { surface: shown?.surface ?? null, onAction, outcome };
}

/**
 * Un réglage des valeurs d'exemple du widget : une valeur, ou le
 * changement d'une liste (`change`, du tableau d'une liste) ; un collage
 * remplace la liste entière.
 */
function exampleValue(
  api: TabApi,
  widget: string,
  param: string,
  context: Record<string, unknown>,
): Promise<Snapshot> {
  const change = context.change as ListChange | null | undefined;
  if (!change) return api.setExampleValue(widget, param, context.value);
  return change.op === "replace"
    ? api.setExampleValue(widget, param, change.items)
    : api.editExampleList(widget, param, change);
}
