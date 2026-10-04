// Une surface A2UI (réglages du dock ou agent), rendue avec le catalogue v3.
// Une liste de messages qui prolonge la précédente n'est traitée que pour sa
// suite (la surface garde son état : détail ouvert, saisie) ; toute autre liste
// recrée la surface et libère l'ancienne (ce qui était déplié le reste : `openMemory`).

import { useEffect, useRef, useState } from "react";
import { A2uiSurface } from "@a2ui/react/v0_9";
import type { A2uiClientAction, A2uiMessage, ComponentApi, SurfaceModel } from "@a2ui/web_core/v0_9";
import { MessageProcessor } from "@a2ui/web_core/v0_9";
import { catalog } from "./catalog";
import type { OpenMemory } from "./openMemory";
import { OpenMemoryContext } from "./openMemory";

/** Une surface construite (et son processeur, pour la prolonger), ou l'erreur de validation de ses messages. */
export interface Built {
  processor: MessageProcessor<ComponentApi> | null;
  surface: SurfaceModel<never> | null;
  error: string | null;
}

/** Construit la surface `surfaceId` à partir de `messages` ; n'émet rien, ne lève rien. */
export function buildSurface(
  messages: A2uiMessage[],
  surfaceId: string,
  onAction: (action: A2uiClientAction) => void,
): Built {
  if (messages.length === 0) return { processor: null, surface: null, error: null };
  const processor = new MessageProcessor([catalog], onAction);
  const get = () => (processor.model.surfacesMap.get(surfaceId) as SurfaceModel<never> | undefined) ?? null;
  try {
    processor.processMessages(messages);
  } catch (e) {
    // Une surface a pu être créée avant l'erreur : elle ne doit pas fuir.
    get()?.dispose();
    return { processor: null, surface: null, error: String(e) };
  }
  return { processor, surface: get(), error: null };
}

/** La surface vivante : son processeur, la surface construite et les messages déjà traités. */
interface Live {
  surfaceId: string;
  processor: MessageProcessor<ComponentApi>;
  surface: SurfaceModel<never> | null;
  done: A2uiMessage[];
}

/**
 * `next` prolonge-t-elle `done` ? Les réducteurs (`surfacesReducer`) n'ajoutent qu'en fin
 * en gardant les mêmes objets : comparer les bornes suffit (temps constant).
 */
const prolongs = (done: A2uiMessage[], next: A2uiMessage[]) =>
  done.length > 0 && next.length > done.length && next[0] === done[0] && next[done.length - 1] === done[done.length - 1];

/** Rend la surface `surfaceId` décrite par `messages` ; une surface invalide va à `onError`. */
export function SurfaceView({
  messages,
  surfaceId,
  onAction,
  onError,
}: {
  messages: A2uiMessage[];
  surfaceId: string;
  onAction: (action: A2uiClientAction) => void;
  onError: (message: string) => void;
}) {
  // Les gestionnaires changent à chaque rendu ; la surface, seulement avec les messages. Ceux
  // d'un rendu aux messages nouveaux n'en prennent la place qu'avec la surface qu'ils décrivent :
  // un champ validé entre ce rendu et la reconstruction part avec la surface affichée.
  const latest = useRef({ onAction, onError });
  latest.current = { onAction, onError };
  const handlers = useRef(latest.current);
  const [surface, setSurface] = useState<SurfaceModel<never> | null>(null);
  const live = useRef<Live | null>(null);
  if (!live.current || live.current.done === messages) handlers.current = latest.current;
  // Ce qui est déplié survit aux reconstructions de la même surface (`openMemory`), pas à un changement d'identifiant.
  const opened = useRef<{ surfaceId: string; memory: OpenMemory } | null>(null);
  if (opened.current?.surfaceId !== surfaceId) opened.current = { surfaceId, memory: new Map() };
  // Aucun nettoyage d'effet ne libère la surface : il voit la liste du rendu précédent, jamais la
  // suivante, et ne peut pas savoir s'il y aura prolongation. Le corps libère l'ancienne quand il
  // reconstruit ; le second effet la libère au démontage.
  useEffect(() => {
    handlers.current = latest.current;
    const current = live.current;
    if (current && current.surfaceId === surfaceId && prolongs(current.done, messages)) {
      try {
        current.processor.processMessages(messages.slice(current.done.length));
      } catch (e) {
        // Refusé : signalé une fois. La validation précède toute modification du modèle :
        // la surface affichée reste valide, et la suite refusée n'est pas rejouée.
        handlers.current.onError(String(e));
      }
      current.done = messages;
      // La suite a pu créer la surface (première liste sans `createSurface` pour cet identifiant).
      const now = (current.processor.model.surfacesMap.get(surfaceId) as SurfaceModel<never> | undefined) ?? null;
      if (now !== current.surface) {
        current.surface = now;
        setSurface(now);
      }
      return;
    }
    current?.surface?.dispose();
    live.current = null;
    const built = buildSurface(messages, surfaceId, (action) => handlers.current.onAction(action));
    // Signalée après le rendu : le parent peut mettre son état à jour sans avertissement de React.
    if (built.error) handlers.current.onError(built.error);
    if (built.processor) live.current = { surfaceId, processor: built.processor, surface: built.surface, done: messages };
    setSurface(built.surface);
  }, [messages, surfaceId]);
  // Démontage (et premier démontage de StrictMode) : libère la surface ; un remontage la reconstruit.
  useEffect(
    () => () => {
      live.current?.surface?.dispose();
      live.current = null;
    },
    [],
  );
  // `.a2ui-surface` : les raccourcis de l'app ignorent les touches tapées dans une surface.
  return surface ? (
    <div className="a2ui-surface">
      <OpenMemoryContext.Provider value={opened.current.memory}>
        <A2uiSurface surface={surface} />
      </OpenMemoryContext.Provider>
    </div>
  ) : null;
}
