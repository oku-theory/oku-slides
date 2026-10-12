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
 * `m` relevé à sa clé racine d'après `current`, la valeur au pointeur racine (`/steps`).
 *
 * Rendu tel quel si ce n'est pas une écriture imbriquée, si son chemin ne mène pas à un objet
 * ou une liste, ou passe par un segment refusé. Une suppression imbriquée n'est pas relevée :
 * l'hôte n'en envoie pas (une clé perdue réécrit son parent entier).
 */
export function liftPatch(m: A2uiMessage, current: (pointer: string) => unknown): A2uiMessage {
  const u = dataUpdate(m);
  const raw = u?.path?.split("/").slice(1);
  if (!u || !raw || raw.length < 2 || u.value === undefined) return m;
  const segments = raw.map(decode);
  if (segments.some((s) => FORBIDDEN.has(s))) return m;
  const pointer = `/${raw[0]}`;
  const root: unknown = structuredClone(current(pointer));
  const last = segments[segments.length - 1];
  let target: unknown = root;
  for (const s of segments.slice(1, -1)) {
    const own = target !== null && typeof target === "object" && Object.hasOwn(target, s);
    target = own ? (target as Record<string, unknown>)[s] : undefined;
  }
  if (Array.isArray(target)) {
    if (!INDEX.test(last) || Number(last) > target.length) return m;
    target[Number(last)] = u.value;
  } else if (target !== null && typeof target === "object") {
    (target as Record<string, unknown>)[last] = u.value;
  } else {
    return m;
  }
  return { ...m, updateDataModel: { ...u, path: pointer, value: root } } as unknown as A2uiMessage;
}
