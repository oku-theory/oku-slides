// Réglages de l'hôte en mode modèles (modèle affiché, ou widget du sous-onglet
// actif) : une surface de réglages construite par la session, rendue au dock.
// Chaque champ porte sa clé ; une action part avec la slide et le widget de la
// surface affichée. Un réglage refusé relit la surface : le champ reprend la
// valeur de la session, celle que dessine l'aperçu.

import { useEffect, useRef, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Snapshot, Surface, TabApi } from "./api";
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
}

/** Réglages de l'hôte au dock : relus quand la slide, le widget ou le deck changent. */
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
  const active = !!current && !!snap?.editing && !snap.history;
  const slide = snap?.slides.find((s) => s.id === current);
  // Tout ce qui peut changer la carte : pack, essais, slide (rendu, nom), annulation.
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
    if (!now || s !== now.surface) return true;
    const { name, context } = fromA2ui(a);
    const field = context.key;
    if (name !== "param" || typeof field !== "string") return true;
    void run(
      s.id === HOST_WIDGET && now.widget
        ? api.setWidgetTrial(now.widget, field, context.value)
        : api.hostParam(now.slide, field, context.value),
    ).then((done) => {
      // Refusé (dit en toast par `run`) : l'instantané n'a pas changé, la surface est relue.
      if (!done) setSync((n) => n + 1);
    });
    return true;
  };
  return { surface: shown?.surface ?? null, onAction };
}
