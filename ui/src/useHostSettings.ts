// Réglages de l'hôte en mode modèles (modèle affiché, ou widget du sous-onglet
// actif, à ses valeurs d'exemple) : une surface de réglages construite par la
// session. Chaque champ porte sa clé ; une action part avec la slide et le
// widget de la surface affichée. Un réglage refusé relit la surface : le champ reprend la
// valeur de la session, celle que dessine l'aperçu.

import { useEffect, useRef, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { ListOp, Snapshot, Surface, TabApi } from "./api";
import { pinsKey } from "./packOps";
import type { ActionOutcome } from "./a2ui/actionOutcome";
import type { ListChange } from "./a2ui/ListTable";
import { fromA2ui } from "./agent/quick";

/** Surface des réglages du modèle affiché. */
export const HOST_TEMPLATE = "host-template";
/** Surface des réglages du widget du sous-onglet actif. */
export const HOST_WIDGET = "host-widget";
/** Surface des jetons du thème édité (onglet « Thème », routée par `useThemeSettings`). */
export const HOST_THEME = "host-theme";

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
  const slide = snap?.slides.find((s) => s.id === current);
  return useSessionSurface({
    ids: HOST_IDS,
    // Les jetons ont leur routeur (`useThemeSettings`).
    held: [HOST_THEME],
    active: !!current && !!snap?.editing && !snap.history,
    scope: { slide: current ?? "", widget },
    // Tout ce qui peut changer la carte : pack, valeurs d'exemple, slide (rendu, nom), annulation.
    key: [current, widget, snap?.editing?.base, pinsKey(snap?.widgets ?? {}), snap?.widget_trials, slide?.hash, slide?.name, snap?.undo_label, snap?.redo_label].join("|"),
    fetch: (scope) => api.hostSettings(scope.slide, scope.widget),
    send: (scope, field, context, surface) =>
      surface.id === HOST_WIDGET && scope.widget
        ? // Le widget que montre la surface.
          exampleValue(api, scope.widget, field, context)
        : api.hostParam(scope.slide, field, context.value),
    run,
    say,
  });
}

const HOST_IDS = [HOST_TEMPLATE, HOST_WIDGET] as const;

/**
 * Le cycle d'une surface de réglages construite par la session : lue quand `key` change,
 * gardée si son contenu est le même (sauf après un refus, qui force une relecture : le
 * modèle de données du champ porte la valeur refusée). Chaque action part vers la portée
 * de la surface affichée, gardée avec elle ; une surface remplacée n'envoie rien.
 */
export function useSessionSurface<S>(opts: {
  /** Ids des surfaces réclamées par ce hook. */
  ids: readonly string[];
  /**
   * Ids des surfaces tenues sans rien envoyer (routées par un autre hook, comme `host-theme`
   * par `useThemeSettings`) : leurs actions sont prises, jamais transmises à l'agent.
   */
  held?: readonly string[];
  active: boolean;
  /** La portée lue : la surface reçue est rattachée à elle (envoi des actions). */
  scope: S;
  /** Tout ce qui peut changer la surface, en une chaîne. */
  key: string;
  fetch: (scope: S) => Promise<Surface | null>;
  /** Envoie le réglage `field` ; `surface` : la surface affichée d'où part l'action. */
  send: (scope: S, field: string, context: Record<string, unknown>, surface: Surface) => Promise<Snapshot>;
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  say: (m: string, error?: boolean) => void;
}): HostSettings {
  const { ids, held = [], active, scope, key, fetch, send, run, say } = opts;
  // `sync` : le numéro de relecture forcée qui a produit la surface.
  const [shown, setShown] = useState<{ surface: Surface; scope: S; sync: number } | null>(null);
  // Lue au moment de l'action : une surface remplacée depuis n'envoie rien, quel que soit le rendu
  // dont vient le gestionnaire appelé.
  const shownRef = useRef(shown);
  shownRef.current = shown;
  // Relecture forcée après un refus : la surface est reconstruite même si son contenu n'a pas changé.
  const [sync, setSync] = useState(0);
  const outcome = useRef<Promise<boolean> | null>(null);
  useEffect(() => {
    if (!active) return setShown(null);
    let alive = true;
    const same = JSON.stringify(scope);
    fetch(scope)
      // Même contenu : la surface est gardée (SurfaceView reconstruit une surface reçue à nouveau),
      // sauf après un refus.
      .then((surface) =>
        alive &&
        setShown((prev) =>
          surface &&
          prev &&
          JSON.stringify(prev.scope) === same &&
          prev.sync === sync &&
          JSON.stringify(prev.surface) === JSON.stringify(surface)
            ? prev
            : surface
              ? { surface, scope, sync }
              : null,
        ),
      )
      .catch((e) => alive && say(String(e), true));
    return () => {
      alive = false;
    };
    // `key` porte la portée et tout ce qui change la surface ; `fetch` et `say` changent
    // d'identité à chaque rendu sans rien changer d'autre.
  }, [active, key, sync]);
  const onAction = (s: Surface, a: A2uiClientAction): boolean => {
    if (!ids.includes(s.id) && !held.includes(s.id)) return false;
    const now = shownRef.current;
    outcome.current = Promise.resolve(false);
    // Une surface tenue a son propre routeur : une action égarée ici ne part pas à l'agent.
    if (held.includes(s.id)) return true;
    if (!now || s !== now.surface) return true;
    const { name, context } = fromA2ui(a);
    const field = context.key;
    if (name !== "param" || typeof field !== "string") return true;
    outcome.current = run(send(now.scope, field, context, s)).then((done) => {
      // Refusé (dit en toast par `run`) : l'instantané n'a pas changé, la surface est relue.
      if (!done) setSync((n) => n + 1);
      return !!done;
    });
    return true;
  };
  return { surface: shown?.surface ?? null, onAction, outcome };
}

/**
 * Route un réglage de surface : une valeur (`setValue`), ou le changement d'une liste
 * (`change`, du tableau d'une liste) vers `editList` ; un collage (`replace`) remplace la
 * liste entière par ses items (`setValue`). Le changement d'une liste imbriquée (`nested`)
 * part vers `editList` avec son chemin `ligne/colonne`, à suffixer à la clé du paramètre.
 */
export function routeListChange(
  context: Record<string, unknown>,
  setValue: (value: unknown) => Promise<Snapshot>,
  editList: (change: ListOp, path?: string) => Promise<Snapshot>,
): Promise<Snapshot> {
  const change = context.change as ListChange | null | undefined;
  if (!change) return setValue(context.value);
  if (change.op === "replace") return setValue(change.items);
  if (change.op === "nested") return editList(change.change, change.path);
  return editList(change);
}

/** La clé de liste `key`, suivie du chemin `ligne/colonne` d'une liste imbriquée. */
export const listKey = (key: string, path?: string) => (path ? `${key}/${path}` : key);

/** Un réglage des valeurs d'exemple du widget (`routeListChange`). */
function exampleValue(
  api: TabApi,
  widget: string,
  param: string,
  context: Record<string, unknown>,
): Promise<Snapshot> {
  return routeListChange(
    context,
    (value) => api.setExampleValue(widget, param, value),
    (change, path) => api.editExampleList(widget, listKey(param, path), change),
  );
}
