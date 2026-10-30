// Curseur de l'historique : repères, déplacements au clavier, et envoi du
// dernier cran demandé (les réponses dépassées sont ignorées).

import type { SlideChanges, Version } from "./api";
import { Latest } from "./latest";

/** Rendu pendant le glissement (sinon au relâchement) : un cran coûte 5,3 ms (p50, tâche 3). */
export const RENDER_WHILE_DRAGGING = true;

/** Les repères du curseur : début de chaque session, et chaque fusion. */
export function markers(versions: Version[]): Marker[] {
  return versions.flatMap<Marker>((v, index) =>
    v.merge ? [{ index, kind: "merge" }] : v.session_start ? [{ index, kind: "session" }] : [],
  );
}

export interface Marker {
  index: number;
  kind: "session" | "merge";
}

/** Le cran après une flèche : une version, ou (⇧) le début de session précédent ou suivant. */
export function step(index: number, versions: Version[], key: "ArrowLeft" | "ArrowRight", shift: boolean): number {
  const last = versions.length - 1;
  if (!shift) return Math.max(0, Math.min(last, index + (key === "ArrowRight" ? 1 : -1)));
  if (key === "ArrowRight") {
    for (let i = index + 1; i <= last; i++) if (versions[i].session_start) return i;
    return last;
  }
  for (let i = index - 1; i >= 0; i--) if (versions[i].session_start) return i;
  return 0;
}

/** Les marques de la pile : slide supprimée ou modifiée depuis la version montrée. */
export function pileMarks(changes: SlideChanges): Record<string, "removed" | "changed"> {
  const marks: Record<string, "removed" | "changed"> = {};
  for (const id of changes.changed_since) marks[id] = "changed";
  for (const id of changes.removed_since) marks[id] = "removed";
  return marks;
}

/** Envoie les crans demandés ; seule la réponse du plus récent est montrée. */
export class Seeker<T> {
  private latest = new Latest();

  constructor(
    private seek: (index: number) => Promise<T>,
    private show: (answer: T) => void,
  ) {}

  async go(index: number): Promise<void> {
    const n = this.latest.begin();
    const answer = await this.seek(index);
    if (this.latest.accept(n)) this.show(answer);
  }

  /** Steps still in flight will not show (before Close or Restore). */
  cancel(): void {
    this.latest.accept(this.latest.begin());
  }
}
