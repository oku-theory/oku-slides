// Élément neuf de la barre d'outils, par sorte : contenu de départ prêt à ajouter à une slide.

import type { Element } from "./api";
import { starterChart } from "./chart/chartDeck";
import { t } from "./i18n";
import type { AddKind } from "./slideTools";

/** Élément neuf pour la barre d'outils (`id` fourni par l'appelant), avec son contenu de départ. */
export function newElement(kind: AddKind, id: string): Element {
  switch (kind) {
    case "text":
      return { id, kind: "text", params: { body: [{ runs: [{ t: t("ui-new-text") }] }] } };
    case "card":
      return { id, kind: "card", params: { title: t("ui-new-card-title"), body: [{ runs: [{ t: t("ui-new-card-detail") }] }] } };
    case "shape":
      return { id, kind: "shape", style: { fill: "surface", radius: "md" } };
    case "chart":
      return { id, kind: "chart", params: { option: starterChart() } };
  }
}
