// Le contexte d'une consigne : slide, élément ou champ sélectionné, édition en cours.

import type { AgentUi } from "../api";

/** Le bloc de contexte d'une consigne ; un `@nom` sélectionné est un champ. */
export function agentUi(current: string | null, selected: string | null, editing: string | null): AgentUi {
  // `undefined` : pas un champ ; `""` (un `@` seul) : un champ sans nom, ignoré.
  const name = selected?.startsWith("@") ? selected.slice(1) : undefined;
  return {
    slide: current,
    element: selected && name === undefined ? selected : null,
    field: name || null,
    editing: editing !== null && editing === selected,
  };
}
