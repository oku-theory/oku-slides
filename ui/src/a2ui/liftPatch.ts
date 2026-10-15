// Relève une écriture imbriquée du modèle de données à sa clé racine.
//
// `/steps/4/calls/2/status` devient une écriture de `/steps` avec une valeur neuve : `@a2ui`
// compare les props par structure et modifie le modèle sur place, sans copie la timeline ne
// verrait pas qu'une étape a changé.

import type { A2uiMessage } from "@a2ui/web_core/v0_9";

/** Une écriture de modèle de données, telle qu'écrite dans un message. */
export type DataUpdate = { surfaceId: string; path?: string; value?: unknown };

/** Segments refusés, comme le fait le modèle de `@a2ui` : ils mèneraient au prototype. */
const FORBIDDEN = new Set(["__proto__", "constructor", "prototype"]);

/** Un index de liste au sens de JSON Pointer : pas de signe, pas de zéro en tête. */
const INDEX = /^(0|[1-9]\d*)$/;

/** Un segment de JSON Pointer, décodé (`~1` → `/`, `~0` → `~`). */
export const decode = (s: string) => s.replace(/~1/g, "/").replace(/~0/g, "~");

/** L'écriture de modèle portée par `m`, s'il en est une. */
export const dataUpdate = (m: A2uiMessage): DataUpdate | undefined =>
  (m as unknown as { updateDataModel?: DataUpdate }).updateDataModel;

/** Une écriture sous la racine : un chemin d'au moins deux segments. */
export const isNested = (u: DataUpdate) => (u.path?.split("/").length ?? 0) > 2;

/**
 * Le pointeur de la clé racine (`/steps`) d'une écriture imbriquée relevable, sinon `undefined`
 * (y compris pour un chemin qui passe par un segment refusé : le modèle n'est alors pas lu).
 */
function rootOf(m: A2uiMessage): string | undefined {
  const u = dataUpdate(m);
  const raw = u?.path?.split("/").slice(1);
  if (!u || !raw || raw.length < 2 || u.value === undefined) return undefined;
  if (raw.some((s) => FORBIDDEN.has(decode(s)))) return undefined;
  return `/${raw[0]}`;
}

/** Écrit `u` dans `root` (la valeur de sa clé racine, déjà copiée) ; faux si son chemin ne mène nulle part. */
function writeInto(root: unknown, u: DataUpdate): boolean {
  const segments = (u.path ?? "").split("/").slice(1).map(decode);
  const last = segments[segments.length - 1];
  let target: unknown = root;
  for (const s of segments.slice(1, -1)) {
    const own = target !== null && typeof target === "object" && Object.hasOwn(target, s);
    target = own ? (target as Record<string, unknown>)[s] : undefined;
  }
  if (Array.isArray(target)) {
    if (!INDEX.test(last) || Number(last) > target.length) return false;
    target[Number(last)] = u.value;
  } else if (target !== null && typeof target === "object") {
    (target as Record<string, unknown>)[last] = u.value;
  } else {
    return false;
  }
  return true;
}

/**
 * `messages` en groupes, dans l'ordre : les écritures imbriquées consécutives d'une même clé
 * racine forment un groupe, tout autre message est seul dans le sien.
 */
export function groupByRoot(messages: A2uiMessage[]): A2uiMessage[][] {
  const groups: A2uiMessage[][] = [];
  let open: string | undefined;
  for (const m of messages) {
    const root = rootOf(m);
    if (root !== undefined && root === open) groups[groups.length - 1].push(m);
    else groups.push([m]);
    open = root;
  }
  return groups;
}

/**
 * Un groupe de `groupByRoot` relevé en une écriture de sa clé racine, d'après `current`, la
 * valeur au pointeur racine (`/steps`) : une seule copie pour tout le groupe.
 *
 * Un message seul qui n'est pas une écriture imbriquée est rendu tel quel. Les écritures dont le
 * chemin ne mène pas à un objet ou une liste, ou passe par un segment refusé, sont écartées ;
 * `null` si aucune ne reste. Une suppression imbriquée n'est pas relevée : l'hôte n'en envoie pas
 * (une clé perdue réécrit son parent entier).
 */
export function liftGroup(group: A2uiMessage[], current: (pointer: string) => unknown): A2uiMessage | null {
  const first = group[0];
  const pointer = first ? rootOf(first) : undefined;
  if (!first || pointer === undefined) return first ?? null;
  const root: unknown = structuredClone(current(pointer));
  let applied = false;
  for (const m of group) {
    const u = dataUpdate(m);
    if (u && writeInto(root, u)) applied = true;
  }
  if (!applied) return null;
  const u = dataUpdate(first);
  return { ...first, updateDataModel: { ...u, path: pointer, value: root } } as unknown as A2uiMessage;
}

/**
 * `m` relevé à sa clé racine d'après `current`, la valeur au pointeur racine (`/steps`).
 *
 * Rendu tel quel si ce n'est pas une écriture imbriquée, si son chemin ne mène pas à un objet
 * ou une liste, ou passe par un segment refusé.
 */
export function liftPatch(m: A2uiMessage, current: (pointer: string) => unknown): A2uiMessage {
  return liftGroup([m], current) ?? m;
}
