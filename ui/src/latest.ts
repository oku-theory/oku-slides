// Réponses concurrentes : seule compte la plus récente des demandes.

/**
 * Numérote des demandes et dit si une réponse est encore à jour.
 *
 * Une réponse arrivée après celle d'une demande plus récente est dépassée :
 * l'appliquer ramènerait l'interface à un état plus ancien.
 */
export class Latest {
  private issued = 0;
  private applied = 0;

  /** Numéro d'une nouvelle demande. */
  begin(): number {
    this.issued += 1;
    return this.issued;
  }

  /** La réponse `n` est à jour ; si oui, elle devient la dernière appliquée. */
  accept(n: number): boolean {
    if (n <= this.applied) return false;
    this.applied = n;
    return true;
  }
}

/**
 * La vue à montrer : `next`, sauf si elle est plus ancienne que `current`.
 *
 * L'hôte numérote chaque vue (`generation`, croissant) : une réponse de
 * commande arrivée après un événement plus récent ne ramène pas la barre en
 * arrière. À numéro égal, `next` est prise (même état).
 */
export function newest<T extends { generation: number }>(current: T | null, next: T): T {
  return current && current.generation > next.generation ? current : next;
}
