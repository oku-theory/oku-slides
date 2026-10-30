// Ce que l'agent change dans l'interface (outil `set_interface`, t-988), appliqué par les
// commandes de l'app : schéma et langue pour toutes les fenêtres, repères pour l'onglet.

import type { InterfaceChange } from "./api";
import type { SchemeChoice } from "./scheme";

/** Les commandes par lesquelles un changement passe ; injectées pour les tests. */
export interface InterfaceCommands {
  changeScheme: (choice: SchemeChoice) => Promise<void>;
  changeLocale: (code: string | null) => Promise<void>;
  setGhosts: (on: boolean) => void;
  /** Dit l'échec d'une commande. */
  fail: (err: unknown) => void;
}

/** Applique `c` : `system` vaut « celui du système » (`null`), un champ absent ne change rien. */
export function applyInterface(c: InterfaceChange, cmd: InterfaceCommands): void {
  if (c.scheme != null) cmd.changeScheme(c.scheme === "system" ? null : c.scheme).catch(cmd.fail);
  if (c.locale != null) cmd.changeLocale(c.locale === "system" ? null : c.locale).catch(cmd.fail);
  if (c.guides != null) cmd.setGhosts(c.guides);
}
