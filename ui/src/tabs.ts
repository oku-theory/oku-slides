// Barre d'onglets : logique pure (places de dépôt, raccourcis, navigation).

/**
 * Place de dépôt (0 = avant le premier onglet), d'après le milieu de chaque onglet.
 *
 * `rtl` : les onglets s'affichent de droite à gauche (l'ordre du tableau reste
 * logique) ; « avant » un point se lit alors à sa droite, pas à sa gauche.
 */
export function slotAt(mids: number[], x: number, rtl = false): number {
  return mids.filter((m) => (rtl ? m > x : m < x)).length;
}

/** Index final d'un onglet parti de `from` et déposé à la place `slot`. */
export function finalIndex(from: number, slot: number): number {
  return slot > from ? slot - 1 : slot;
}

/** Onglet voisin, en boucle ; `undefined` si `active` n'est pas dans la liste. */
export function cycle(tabs: string[], active: string, delta: number): string | undefined {
  const n = tabs.length;
  const i = tabs.indexOf(active);
  if (n === 0 || i < 0) return undefined;
  return tabs[(((i + delta) % n) + n) % n];
}

/** Onglet n (1 à 8) ; 9 : le dernier. */
export function nth(tabs: string[], n: number): string | undefined {
  return n === 9 ? tabs[tabs.length - 1] : tabs[n - 1];
}

type TabAction =
  | { kind: "new-tab" }
  | { kind: "close-tab" }
  | { kind: "new-window" }
  | { kind: "cycle"; delta: 1 | -1 }
  | { kind: "nth"; n: number }
  | { kind: "help" };

type Keys = Pick<KeyboardEvent, "code" | "key" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey" | "repeat">;

/**
 * Raccourci des onglets : ⌘ sur macOS, Ctrl ailleurs ; Ctrl+Tab partout.
 *
 * Lettres par caractère (`key`), comme les raccourcis d'`App` et les menus :
 * ⌘Z reste « annuler » en AZERTY. Chiffres par touche physique (`code`) :
 * ⌘1 reste ⌘1 en AZERTY, où `key` vaut « & ».
 */
export function tabShortcut(e: Keys, mac: boolean): TabAction | null {
  if (e.repeat || e.altKey) return null;
  if (e.ctrlKey && e.key === "Tab") return { kind: "cycle", delta: e.shiftKey ? -1 : 1 };
  if (!(mac ? e.metaKey : e.ctrlKey)) return null;
  const letter = e.key.toLowerCase();
  if (letter === "t" && !e.shiftKey) return { kind: "new-tab" };
  if (letter === "w" && !e.shiftKey) return { kind: "close-tab" };
  if (letter === "n" && e.shiftKey) return { kind: "new-window" };
  // L'aide : ⌘? comme sur macOS, ⌘/ aussi ; par caractère, où qu'il soit sur le clavier.
  if (e.key === "?" || e.key === "/") return { kind: "help" };
  const digit = /^Digit([1-9])$/.exec(e.code);
  return digit && !e.shiftKey ? { kind: "nth", n: Number(digit[1]) } : null;
}

/** Le point (coordonnées de la page) est hors de la fenêtre. */
export function outside(x: number, y: number, width: number, height: number): boolean {
  return x < 0 || y < 0 || x >= width || y >= height;
}

/** Coin d'un menu `w` × `h` ouvert vers le haut depuis le pointeur, gardé dans la fenêtre. */
export function menuAbove(x: number, y: number, w: number, h: number, width: number, height: number): { x: number; y: number } {
  const margin = 8;
  return {
    x: Math.max(margin, Math.min(x, width - w - margin)),
    y: Math.max(margin, Math.min(y - h, height - h - margin)),
  };
}

/**
 * Appels regroupés : un seul en cours ; ceux arrivés entre-temps n'en font qu'un.
 *
 * Pour suivre le pointeur sans empiler les requêtes à l'hôte : la dernière
 * position est toujours envoyée, les intermédiaires sont sautées. `idle` :
 * plus rien en cours ni en attente (avant un appel qui doit passer après).
 */
export function coalesce(call: () => Promise<unknown>): { run: () => void; idle: () => Promise<void> } {
  let current: Promise<void> | null = null;
  let again = false;
  const run = () => {
    if (current) {
      again = true;
      return;
    }
    again = false;
    current = call()
      .then(
        () => {},
        (e) => console.warn("appel regroupé en échec", e),
      )
      .finally(() => {
        current = null;
        if (again) run();
      });
  };
  const idle = async () => {
    while (current) await current;
  };
  return { run, idle };
}

/**
 * Place d'un onglet arrivé d'une autre fenêtre (déjà en fin de barre).
 *
 * `mids` : milieu des autres onglets. `null` : il est déjà à sa place.
 */
export function arrivalIndex(mids: number[], x: number, rtl = false): number | null {
  const slot = slotAt(mids, x, rtl);
  return slot < mids.length ? slot : null;
}
