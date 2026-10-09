// La portée de l'agent : les slides, l'atelier d'un pack, ou un widget.

import type { AgentScope, AgentState } from "../api";

/** Ce qui décide de la portée : mode, historique, pack édité, widget actif. */
export interface ScopeView {
  templates: boolean;
  history: boolean;
  pack: string | null;
  widget: string | null;
}

/** L'état de l'agent et la portée de son tour (absente au repos). */
export interface AgentStatus {
  state: AgentState;
  scope?: AgentScope;
}

/** La portée de ce que l'utilisateur regarde. */
export function agentScope(v: ScopeView): AgentScope {
  if (!v.templates || v.history || v.pack === null) return { kind: "slides" };
  return v.widget === null ? { kind: "atelier", pack: v.pack } : { kind: "widget", pack: v.pack, name: v.widget };
}

/** Une clé stable : deux portées égales ont la même. */
export function scopeKey(s: AgentScope): string {
  return s.kind === "slides" ? "slides" : s.kind === "atelier" ? `atelier:${s.pack}` : `widget:${s.pack}:${s.name}`;
}

/** Deux portées désignent-elles la même conversation ? */
export function sameScope(a: AgentScope, b: AgentScope): boolean {
  return scopeKey(a) === scopeKey(b);
}

/** La portée d'un tour en cours ailleurs que dans `scope` ; `null` sinon. */
export function busyElsewhere(status: AgentStatus, scope: AgentScope): AgentScope | null {
  if (status.state === "idle" || !status.scope) return null;
  return sameScope(status.scope, scope) ? null : status.scope;
}

/** Le nom d'une portée dans une phrase : « les slides », « l'atelier », ou le widget. */
export function scopePlace(s: AgentScope, tr: (key: string) => string): string {
  if (s.kind === "widget") return s.name;
  return tr(s.kind === "slides" ? "ui-agent-scope-slides" : "ui-agent-scope-atelier");
}
