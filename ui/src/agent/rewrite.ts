// Reformulation d'un élément (`ask_rewrite`) : la session ne lit plus le
// markdown (décision 5, plan des sources Loro) ; seule l'interface le fait.
// Avant d'envoyer l'action `rewrite`, on y ajoute les paragraphes du texte
// gardé, pour qu'un champ (`@title`…) n'ait qu'à envoyer `text`.

import type { Surface } from "../api";
import type { QuickAction } from "./quick";
import { textToParagraphs } from "../deck";

/** Ancre de champ : `@title`, `@subtitle`, `@kicker`, `@text`. */
const isField = (element: string) => element.startsWith("@");

/**
 * Ajoute `paragraphs` à une action `rewrite` qui garde une proposition ou une
 * édition d'un élément (pas un champ). Toute autre action revient inchangée.
 */
export function withRewriteParagraphs(surface: Surface, a: QuickAction): QuickAction {
  if (a.name !== "rewrite" || a.context.kept === "original") return a;
  const element = surface.anchor?.element;
  if (!element || isField(element)) return a;
  return { ...a, context: { ...a.context, paragraphs: textToParagraphs(String(a.context.text)) } };
}
