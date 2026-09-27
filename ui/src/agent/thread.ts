// Le fil de l'agent : ce qui a été dit, ce qu'il fait, ce qu'il répond.

import type { AgentEvent, AgentState, PlanItem, StopReason, ToolStatus } from "../api";

/** Une ligne du fil : consigne, réponse, outil, plan, erreur ou fin de tour. */
export type Entry =
  | { kind: "user"; text: string }
  | { kind: "agent"; text: string }
  | { kind: "tool"; id: string; title: string; status: ToolStatus }
  | { kind: "plan"; entries: PlanItem[] }
  | { kind: "error"; text: string }
  | { kind: "stop"; stop: StopReason };

/** L'état de l'agent et les lignes du fil, dans l'ordre. */
export interface Thread {
  state: AgentState;
  entries: readonly Entry[];
}

/** Le fil d'une conversation qui commence. */
export const EMPTY_THREAD: Readonly<Thread> = Object.freeze({ state: "idle", entries: Object.freeze([]) });

/** Applique un évènement de l'agent au fil ; un évènement inconnu le laisse intact. */
export function threadReducer(t: Thread, e: AgentEvent): Thread {
  const entries = t.entries;
  switch (e.kind) {
    case "status":
      return { ...t, state: e.state };
    case "user_text":
      return { ...t, entries: [...entries, { kind: "user", text: e.text }] };
    case "chunk": {
      const last = entries.at(-1);
      if (last?.kind !== "agent") return { ...t, entries: [...entries, { kind: "agent", text: e.text }] };
      return { ...t, entries: [...entries.slice(0, -1), { kind: "agent", text: last.text + e.text }] };
    }
    case "tool_call":
      return { ...t, entries: [...entries, { kind: "tool", id: e.id, title: e.title, status: e.status }] };
    case "tool_update":
      return {
        ...t,
        entries: entries.map((x) =>
          x.kind === "tool" && x.id === e.id ? { ...x, title: e.title ?? x.title, status: e.status ?? x.status } : x,
        ),
      };
    case "plan": {
      const i = entries.findIndex((x) => x.kind === "plan");
      const plan: Entry = { kind: "plan", entries: e.entries };
      return { ...t, entries: i < 0 ? [...entries, plan] : entries.map((x, j) => (j === i ? plan : x)) };
    }
    case "turn_ended":
      return e.stop === "end_turn" ? t : { ...t, entries: [...entries, { kind: "stop", stop: e.stop }] };
    case "error":
      return { ...t, entries: [...entries, { kind: "error", text: e.message }] };
    case "conversation":
      return { ...t, entries: [] };
    case "thread":
      // Le fil gardé de l'onglet remplace l'état courant : rejoué depuis zéro.
      return e.entries.reduce(threadReducer, EMPTY_THREAD);
    default: {
      // Hôte plus récent que l'interface : la vérification ci-dessous garde le switch exhaustif.
      const unknown: never = e;
      void unknown;
      return t;
    }
  }
}

/** Un appel d'outil du fil. */
export type ToolEntry = Extract<Entry, { kind: "tool" }>;

/** Une ligne affichée : une entrée, ou des appels d'outils consécutifs ; `at` : indice de la première. */
export type ThreadItem = (Exclude<Entry, ToolEntry> | { kind: "tools"; tools: ToolEntry[] }) & { at: number };

/** Les lignes à afficher : les appels d'outils consécutifs tiennent en une seule. */
export function threadItems(entries: readonly Entry[]): ThreadItem[] {
  const items: ThreadItem[] = [];
  entries.forEach((e, at) => {
    const last = items.at(-1);
    if (e.kind !== "tool") items.push({ ...e, at });
    else if (last?.kind === "tools") last.tools.push(e);
    else items.push({ kind: "tools", tools: [e], at });
  });
  return items;
}
