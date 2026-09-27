// Raccourcis de la dictée (⌘D texte, ⌘⇧D agent) et ce que fait Échap.

/** Où va une dictée : dans le deck (texte) ou à l'agent (consigne). */
export type DictationMode = "text" | "agent";

/** Ce que les raccourcis lisent d'un évènement clavier. */
export interface KeyboardEventLike {
  key: string;
  code: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  repeat: boolean;
}

/**
 * Le raccourci de dictée d'une touche ; `null` si ce n'en est pas un.
 * Pendant une écoute, ⌘D comme ⌘⇧D l'arrêtent : la dictée garde le mode de
 * son début. La lettre est lue par sa position (`KeyD`) : ⌘D marche aussi
 * sur un clavier cyrillique ou thaï.
 */
export function dictationShortcut(
  e: KeyboardEventLike,
  { agentBusy, recording }: { agentBusy: boolean; recording: boolean },
): DictationMode | "agent-cancel" | null {
  if (e.repeat || e.altKey || !(e.metaKey || e.ctrlKey)) return null;
  if (e.code !== "KeyD" && e.key.toLowerCase() !== "d") return null;
  if (!e.shiftKey) return "text";
  return agentBusy && !recording ? "agent-cancel" : "agent";
}

/**
 * Ce que fait Échap dans l'app, une fois écartés les champs, l'édition d'un
 * texte et le sélecteur de modèles (qui gardent la touche) : désélectionner
 * d'abord, arrêter le tour de l'agent en dernier recours.
 */
export function escapeAction({ selected, agentBusy }: { selected: boolean; agentBusy: boolean }): "deselect" | "agent-cancel" | null {
  if (selected) return "deselect";
  return agentBusy ? "agent-cancel" : null;
}
