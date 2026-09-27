// Surfaces produites par la session Rust (fixture partagée, voir forms_tests.rs), pour les tests.

import type { Surface } from "../../api";
import fixture from "./agent-surfaces.json";

/** La surface `id` de la fixture ; lève si elle n'y est pas. */
export function realSurface(id: string): Surface {
  const s = (fixture as unknown as Surface[]).find((x) => x.id === id);
  if (!s) throw new Error(`surface absente de la fixture : ${id}`);
  return s;
}
