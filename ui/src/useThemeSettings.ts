// Jetons du thème édité (onglet « Thème » du mode modèles) : la surface
// `host-theme` construite par la session, filtrée. Chaque ligne envoie
// l'évènement `token` ; le champ de filtre, `filter`. Même contrat que
// `useHostSettings` : un changement refusé relit la surface, le champ reprend
// la valeur de la session.

import { useEffect, useRef, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Snapshot, Surface, TabApi, TokenAction } from "./api";
import { fromA2ui } from "./agent/quick";
import { HOST_THEME, type HostSettings } from "./useHostSettings";

/**
 * Jetons du thème édité : relus après chaque changement accepté, et quand le brouillon
 * (base, nom, état), l'annulation, le pack ou le filtre changent. `onFilter` reçoit le filtre saisi dans la surface :
 * l'appelant le garde et le rend par `filter` (une seule source).
 */
export function useThemeSettings(
  api: TabApi,
  snap: Snapshot | null,
  filter: string,
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>,
  say: (m: string, error?: boolean) => void,
  onFilter: (filter: string) => void = () => {},
): HostSettings {
  // `sync` : le numéro de relecture forcée qui a produit la surface.
  const [shown, setShown] = useState<{ surface: Surface; sync: number } | null>(null);
  // Lue au moment de l'action : une surface remplacée depuis n'envoie rien.
  const shownRef = useRef(shown);
  shownRef.current = shown;
  // Relecture forcée après un refus : la surface est reconstruite même si son contenu n'a pas changé.
  const [sync, setSync] = useState(0);
  // Changements acceptés : chacun relit la surface. L'instantané seul ne suffit pas : deux actions
  // sur le même jeton (poser, puis ↺) gardent le même libellé d'annulation et le même `dirty`.
  const [edits, setEdits] = useState(0);
  const outcome = useRef<Promise<boolean> | null>(null);
  const active = !!snap?.editing && !snap.history;
  // Ce qui change les jetons hors de cette surface (annuler, rétablir, l'agent, l'en-tête du panneau).
  // Les ids des listes (polices, palette) changent à chaque relecture : toute étape d'annulation la provoque.
  const key = [JSON.stringify(snap?.editing?.theme ?? null), snap?.editing?.base, snap?.undo_label, snap?.redo_label, filter].join("|");
  useEffect(() => {
    if (!active) return setShown(null);
    let alive = true;
    api
      .hostTheme(filter)
      .then((surface) =>
        alive &&
        setShown((prev) =>
          surface && prev && prev.sync === sync && JSON.stringify(prev.surface) === JSON.stringify(surface)
            ? prev
            : surface
              ? { surface, sync }
              : null,
        ),
      )
      .catch((e) => alive && say(String(e), true));
    return () => {
      alive = false;
    };
    // `say` ne fait que poser un toast : son identité ne compte pas.
  }, [api, active, key, sync, edits]);
  const onAction = (s: Surface, a: A2uiClientAction): boolean => {
    if (s.id !== HOST_THEME) return false;
    outcome.current = Promise.resolve(false);
    const now = shownRef.current;
    if (!now || s !== now.surface) return true;
    const { name, context } = fromA2ui(a);
    if (name === "filter") {
      if (typeof context.filter === "string") onFilter(context.filter);
      return true;
    }
    // Le chemin du jeton : `token` (une clé `path` serait prise par @a2ui pour une liaison de données).
    const path = context.token;
    if (name !== "token" || !Array.isArray(path) || typeof context.action !== "string") return true;
    // « Ajouter la couleur » : le nom saisi prolonge le chemin ; un tableau envoie son changement.
    const full = typeof context.name === "string" ? [...(path as string[]), context.name] : (path as string[]);
    const value = context.change ?? context.value;
    outcome.current = run(api.hostToken(full, context.action as TokenAction, value)).then((done) => {
      // Refusé (dit en toast par `run`) : la surface est relue et reconstruite. Accepté : relue
      // (gardée si son contenu n'a pas changé).
      if (!done) setSync((n) => n + 1);
      else setEdits((n) => n + 1);
      return !!done;
    });
    return true;
  };
  return { surface: shown?.surface ?? null, onAction, outcome };
}
