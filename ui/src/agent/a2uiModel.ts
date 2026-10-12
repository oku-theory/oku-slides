// Lecture d'une surface A2UI telle qu'écrite dans ses messages : composants et modèle de données,
// les mises à jour successives appliquées.

import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import { dataUpdate, decode, isNested, liftPatch } from "../a2ui/liftPatch";
import type { Surface } from "../api";

/** Un composant A2UI tel qu'écrit dans les messages. */
export type Component = { id: string; component: string; [k: string]: unknown };
type Message = {
  updateComponents?: { components?: unknown };
  updateDataModel?: { path?: string; value?: unknown };
};

/** Composants de la surface, les mises à jour successives appliquées (la dernière gagne). */
export function components(s: Surface): Component[] {
  const byId = new Map<string, Component>();
  for (const m of s.messages as unknown as Message[]) {
    const list = m.updateComponents?.components;
    if (!Array.isArray(list)) continue;
    for (const c of list) {
      if (c && typeof c === "object" && typeof (c as Component).id === "string") byId.set((c as Component).id, c as Component);
    }
  }
  return [...byId.values()];
}

/** La clé racine désignée par un pointeur à un segment (`/a~1b` → `a/b`). */
const keyOf = (pointer: string) => decode(pointer.slice(1));

/** Modèle de données de la surface, les mises à jour successives appliquées. */
export function dataModel(s: Surface): Record<string, unknown> {
  let root: Record<string, unknown> = {};
  for (const m of s.messages as unknown as Message[]) {
    const u = m.updateDataModel;
    if (!u) continue;
    if (!u.path || u.path === "/") root = u.value && typeof u.value === "object" ? { ...(u.value as Record<string, unknown>) } : {};
    else {
      // Une écriture imbriquée (`/steps/1`) se relève à sa clé racine, comme dans `SurfaceView` ;
      // une qui ne mène nulle part est ignorée.
      const read = (pointer: string) => (Object.hasOwn(root, keyOf(pointer)) ? root[keyOf(pointer)] : undefined);
      const lifted = dataUpdate(liftPatch(m as unknown as A2uiMessage, read));
      if (!lifted || isNested(lifted)) continue;
      root = { ...root, [keyOf(lifted.path ?? "")]: lifted.value };
    }
  }
  return root;
}

/** La valeur d'un chemin (`/a/b`) dans le modèle ; `undefined` si absente. */
export function read(model: Record<string, unknown>, path: string): unknown {
  let v: unknown = model;
  for (const part of path.split("/").filter(Boolean)) {
    if (!v || typeof v !== "object") return undefined;
    v = (v as Record<string, unknown>)[part];
  }
  return v;
}

export const isPath = (v: unknown): v is { path: string } =>
  !!v && typeof v === "object" && typeof (v as { path?: unknown }).path === "string";
