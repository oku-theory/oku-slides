// Reformulation (`ask_rewrite`) : la session ne lit plus le markdown (décision 5,
// plan des sources Loro) ; seule l'interface le fait. Avant d'envoyer l'action
// `rewrite`, on y ajoute toujours les paragraphes du texte gardé : la session,
// qui sait si le champ adressé (`{owner, field}`) est `rich` ou `plain`, prend
// les paragraphes ou le texte.

import type { Surface } from "../api";
import type { QuickAction } from "./quick";
import { textToParagraphs } from "../deck";

/**
 * Ajoute `paragraphs` à une action `rewrite` ancrée qui garde une proposition ou
 * une édition. Toute autre action revient inchangée.
 */
export function withRewriteParagraphs(surface: Surface, a: QuickAction): QuickAction {
  if (a.name !== "rewrite" || a.context.kept === "original" || !surface.anchor) return a;
  return { ...a, context: { ...a.context, paragraphs: textToParagraphs(String(a.context.text)) } };
}
