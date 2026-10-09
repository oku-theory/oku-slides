// Côté fenêtre principale, l'agent détaché : contexte publié, textes à écrire, raccourcis relayés.
// Les raccourcis partent bruts : la fenêtre de l'agent les interprète avec son état (écoute, tour).

import { useEffect, useRef } from "react";
import { dictationShortcut, typingShortcut } from "./keys";
import { onlyFor, type LinkContext, type LinkShortcut, type LinkTransport } from "./link";

export interface MainLinkInput {
  tab: string;
  /** L'agent est dans sa fenêtre : publier, répondre, relayer. Sinon rien. */
  detached: boolean;
  context: LinkContext;
  /** Écrit le texte au curseur, dans la zone sélectionnée ou en nouveau bloc ; `false` : nulle part. */
  insert: (text: string) => boolean;
  transport: LinkTransport;
}

/** Agent détaché : publie le contexte, répond à hello et insert, relaie ⌘D / ⌘⇧D / ⌘K bruts. */
export function useMainLink({ tab, detached, context, insert, transport }: MainLinkInput): void {
  const latest = useRef({ context, insert });
  latest.current = { context, insert };
  // Clé du contexte, calculée seulement détaché : une égalité profonde en guise de dépendance.
  const contextKey = detached ? JSON.stringify(context) : "";
  useEffect(() => {
    if (detached) transport.send({ to: "agent", kind: "context", tab, context: latest.current.context });
  }, [detached, tab, contextKey, transport]);
  useEffect(() => {
    if (!detached) return;
    // Insertions annulées par la fenêtre de l'agent avant d'arriver ici ; bornées : une annulation
    // ne sert qu'à l'insertion qui la suit de près.
    const cancelled: string[] = [];
    return transport.listen(onlyFor("main", tab, (m) => {
      if (m.kind === "hello") transport.send({ to: "agent", kind: "context", tab, context: latest.current.context });
      if (m.kind === "insert-cancel") {
        cancelled.push(m.id);
        if (cancelled.length > 64) cancelled.shift();
      }
      if (m.kind === "insert") {
        const at = cancelled.indexOf(m.id);
        if (at >= 0) {
          cancelled.splice(at, 1);
          return;
        }
        const ok = latest.current.insert(m.text);
        transport.send({ to: "agent", kind: "insert-ack", tab, id: m.id, ok });
      }
    }));
  }, [detached, tab, transport]);
  useEffect(() => {
    if (!detached) return;
    const onKey = (e: KeyboardEvent) => {
      const key: LinkShortcut | null = typingShortcut(e)
        ? "type"
        : dictationShortcut(e, { agentBusy: false, recording: false }) === null
          ? null
          : e.shiftKey ? "draft" : "auto";
      if (!key) return;
      e.preventDefault();
      e.stopPropagation();
      transport.send({ to: "agent", kind: "shortcut", tab, key });
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [detached, tab, transport]);
}
