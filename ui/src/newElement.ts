// Élément neuf de la barre d'outils, par nom : contenu de départ prêt à ajouter à une slide.

import type { Element } from "./api";
import { BASE } from "./baseKinds";
import { starterChart } from "./chart/chartDeck";
import { t } from "./i18n";
import { starterTable } from "./table/tableDeck";
import type { AddName } from "./slideTools";

/** Élément neuf pour la barre d'outils (`id` fourni par l'appelant), avec son contenu de départ. */
export function newElement(name: AddName, id: string): Element {
  const kind = BASE[name];
  switch (name) {
    case "text":
      return { id, kind, params: { body: [{ runs: [{ t: t("ui-new-text") }] }] } };
    case "card":
      return { id, kind, params: { title: t("ui-new-card-title"), body: [{ runs: [{ t: t("ui-new-card-detail") }] }] } };
    case "shape":
      return { id, kind, style: { fill: "surface", radius: "md" } };
    case "chart":
      return { id, kind, params: { option: starterChart() } };
    case "table":
      return { id, kind, params: { data: starterTable() } };
  }
}

/** Widget du pack neuf : son nom en `kind`, sans params (ses défauts valent). */
export function newPackWidget(name: string, id: string): Element {
  return { id, kind: name };
}
