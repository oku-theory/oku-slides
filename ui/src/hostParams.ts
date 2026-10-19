// Paramètres d'hôte des widgets d'élément : quel panneau ouvrir, sur quel paramètre.

import type { HostParams, HostType } from "./api";

/** Le paramètre d'hôte `host` du widget `kind`, ou null (propriétés propres seulement). */
function hostParam(params: HostParams, kind: string, host: HostType): string | null {
  if (!Object.hasOwn(params, kind)) return null;
  const hosts = params[kind];
  return Object.hasOwn(hosts, host) ? (hosts[host] ?? null) : null;
}

/** Un panneau propre et le paramètre qu'il lit et écrit. */
export type OwnPanel = { panel: Exclude<HostType, "path">; param: string };

/** Le panneau propre d'un élément : graphique, sinon tableau ; null pour les autres. */
export function ownPanel(params: HostParams, kind: string | undefined): OwnPanel | null {
  if (kind === undefined) return null;
  const chart = hostParam(params, kind, "chart");
  if (chart !== null) return { panel: "chart", param: chart };
  const table = hostParam(params, kind, "table");
  return table !== null ? { panel: "table", param: table } : null;
}
