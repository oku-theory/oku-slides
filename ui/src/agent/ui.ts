// Le contexte d'une consigne : slide, élément, champ en édition, widget ouvert.

import type { AgentUi, FieldView } from "../api";

/** Ce que voit l'agent : slide, élément, champ, widget ouvert. */
export function agentUi(current: string | null, selected: string | null, editing: FieldView | null, widget: string | null): AgentUi {
  const element = editing ? (editing.owner_kind === "element" ? editing.owner : null) : selected;
  return { slide: current, element, field: editing?.field ?? null, editing: editing !== null, widget };
}
