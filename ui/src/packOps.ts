// Opérations sur le pack de modèles du deck.

import type { Op, PackInfo, PackRef } from "./api";

/** Le deck adopte ce pack : référence complète (id et version), jamais l'id seul. */
export function setTemplatesOp(pack: PackInfo): Op & { templates: PackRef } {
  return { op: "set_templates", templates: { id: pack.id, version: pack.version } };
}
