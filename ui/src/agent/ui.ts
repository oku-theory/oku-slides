// Le contexte d'une consigne : slide, élément sélectionné ou en édition, champ en édition.

import type { AgentUi, FieldView } from "../api";

/** Ce que voit l'agent : slide, élément sélectionné ou en édition, champ en édition. */
export function agentUi(current: string | null, selected: string | null, editing: FieldView | null): AgentUi {
  const element = editing ? (editing.owner_kind === "element" ? editing.owner : null) : selected;
  return { slide: current, element, field: editing?.field ?? null, editing: editing !== null };
}
