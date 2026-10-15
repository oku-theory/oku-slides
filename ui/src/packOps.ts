// Opérations sur le pack de modèles du deck.

import type { Op, PackInfo, PackRef } from "./api";
import { t } from "./i18n";

/** Le deck adopte ce pack : référence complète (id et version), jamais l'id seul. */
export function setTemplatesOp(pack: PackInfo): Op & { templates: PackRef } {
  return { op: "set_templates", templates: { id: pack.id, version: pack.version } };
}

/** Nom d'un pack dans la liste des packs : intégré tel quel, sinon dit reçu (arrivé avec un deck) ou à moi. */
export function packOptionLabel(pack: PackInfo): string {
  if (pack.builtin) return pack.name;
  return pack.received ? t("ui-picker-set-received", { name: pack.name }) : t("ui-picker-set-mine", { name: pack.name });
}
