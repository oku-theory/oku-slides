// Raccourcis de la dictée (⌘D, ⌘⇧D brouillon), de la saisie (⌘K) et ce que fait Échap.

/**
 * Où va une dictée : `auto` (⌘D) au curseur d'un texte en cours d'édition,
 * sinon au brouillon ; `draft` (⌘⇧D) toujours au brouillon.
 */
export type DictationMode = "auto" | "draft";

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
  return shortcutAction(e.shiftKey ? "draft" : "auto", { agentBusy, recording });
}

/** Ce que fait ⌘D (`auto`) ou ⌘⇧D (`draft`) : dicter, ou arrêter un tour qui n'écoute pas. */
export function shortcutAction(
  key: "auto" | "draft",
  { agentBusy, recording }: { agentBusy: boolean; recording: boolean },
): DictationMode | "agent-cancel" {
  return key === "draft" && agentBusy && !recording ? "agent-cancel" : key;
}

/**
 * ⌘K (Ctrl ailleurs) : ouvrir le brouillon pour écrire une consigne au clavier.
 * La lettre est lue par sa position (`KeyK`), comme ⌘D.
 */
export function typingShortcut(e: KeyboardEventLike): boolean {
  if (e.repeat || e.altKey || e.shiftKey || !(e.metaKey || e.ctrlKey)) return false;
  return e.code === "KeyK" || e.key.toLowerCase() === "k";
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

const TEXT_ENTRY = 'input, textarea, select, [contenteditable="true"]';

/**
 * Les touches du clavier appartiennent-elles à la cible, pas aux raccourcis de l'application ?
 * Toujours pour une saisie de texte ; dans une surface A2UI ou le dock, pour les touches
 * seules (Échap, flèches, Entrée) mais pas pour ⌘/Ctrl + touche : un bouton du dock
 * (un stepper, par exemple) laisse donc passer ⌘Z.
 */
export function targetOwnsKeys(target: HTMLElement | null, e: { metaKey: boolean; ctrlKey: boolean }): boolean {
  if (!target) return false;
  if (target.closest(TEXT_ENTRY)) return true;
  return !(e.metaKey || e.ctrlKey) && !!target.closest(".a2ui-surface, .agent-keys");
}
