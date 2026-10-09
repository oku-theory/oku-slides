// Ce que la région annoncée du fil lit d'une surface (t-457).
//
// La liste des tours n'est pas une région `aria-live` : une surface A2UI reconstruite
// (resynchronisation, reçu remplacé par sa version sans suites) recrée tout son DOM, qu'une
// région « additions » relirait. Le fil annonce à la place, dans une région masquée, le texte
// lisible de chaque surface qui est nouveau par rapport à ce qu'il en a déjà lu.

import { useEffect, useRef, useState } from "react";
import type { Surface } from "../api";
import type { Component } from "./a2uiModel";
import { components, dataModel, isPath, read } from "./a2uiModel";

/** Composants dont le texte (`text`) est lu. */
const TEXTS = new Set(["Text", "Markdown", "Prompt", "Badge"]);

const lines = new WeakMap<Surface, string[]>();

/**
 * Les textes lisibles de `s`, dans l'ordre de l'arbre : textes, consigne, badge, titre et faits d'un
 * reçu. Ni boutons (un reçu sans ses suites dit la même chose), ni détail replié, ni timeline.
 */
export function spokenLines(s: Surface): string[] {
  const hit = lines.get(s);
  if (hit) return hit;
  const byId = new Map(components(s).map((c) => [c.id, c]));
  const model = dataModel(s);
  const text = (v: unknown) => {
    const value = isPath(v) ? read(model, v.path) : v;
    return typeof value === "string" ? value.trim() : "";
  };
  const out: string[] = [];
  const seen = new Set<string>();
  const visit = (id: unknown) => {
    if (typeof id !== "string" || seen.has(id)) return;
    seen.add(id);
    const c: Component | undefined = byId.get(id);
    if (!c) return;
    if (TEXTS.has(c.component)) out.push(text(c.text));
    else if (c.component === "Receipt") out.push(text(c.title), ...(Array.isArray(c.facts) ? c.facts.map(text) : []));
    else if (c.component === "Disclosure" || c.component === "Timeline") return;
    visit(c.child);
    if (Array.isArray(c.children)) c.children.forEach(visit);
  };
  visit("root");
  const result = out.filter(Boolean);
  lines.set(s, result);
  return result;
}

/** Une ligne sans l'ellipse d'une coupe (`first_line` côté hôte) : « abc… » est le début de « abcd ». */
const stem = (line: string) => (line.endsWith("…") ? line.slice(0, -1).trimEnd() : line);

/**
 * Ce qui a été entendu d'une ligne lue, pour lire la suite d'une ligne qui la prolonge : une ligne
 * coupée (« … ») l'a peut-être été au milieu d'un mot, son dernier mot ne compte pas (la suite
 * reprend au début du mot). Coupée dans son premier mot : rien.
 */
function heardPrefix(line: string): string {
  const s = stem(line);
  if (s === line) return s;
  // Coupée juste après une espace (« le … ») : le dernier mot est entier.
  if (/\s…$/u.test(line)) return s;
  const space = s.search(/\s\S*$/u);
  return space < 0 ? "" : s.slice(0, space).trimEnd();
}

/**
 * Ce qui est nouveau dans `now` par rapport à `before` (déjà lu) : les lignes jamais lues ; d'une
 * ligne qui en prolonge une déjà lue (narration en flux), seulement la suite, depuis le début d'un
 * mot. Une ligne coupée (« … ») dont le début est déjà lu n'apporte rien ; une ligne entière qui
 * n'est que le début d'une ligne lue (« Sable » après « Sable clair ») est lue.
 */
export function newSpeech(before: string[], now: string[]): string {
  const heard = before.map(stem).filter(Boolean);
  const prefixes = before.map(heardPrefix).filter(Boolean);
  const parts: string[] = [];
  for (const line of now) {
    const s = stem(line);
    // Seule une ligne coupée peut n'être que le début d'une ligne lue ; entière, elle est nouvelle.
    const cut = s !== line;
    if (heard.some((h) => (cut ? h.startsWith(s) : h === s))) continue;
    const grown = prefixes.filter((h) => s.startsWith(h)).sort((a, b) => b.length - a.length)[0];
    parts.push(grown ? s.slice(grown.length).trim() : line);
  }
  return parts.filter(Boolean).join(" ");
}

/**
 * Ce qui reste entendu d'une surface après lecture de `now` : ses lignes, et pour chacune au plus la
 * plus longue ligne lue qui la prolonge (une coupe « … » d'une ligne déjà lue ne se relit pas). Le
 * reste ne sert plus à aucune comparaison : la mémoire reste bornée par la surface (au plus deux
 * fois ses lignes), même pour une narration en flux de mille morceaux.
 */
export function mergeHeard(before: string[], now: string[]): string[] {
  const longer = now.map((n) => {
    const s = stem(n);
    return before.filter((b) => b !== n && stem(b).length > s.length && stem(b).startsWith(s)).sort((a, b) => b.length - a.length)[0];
  });
  return [...new Set([...longer.filter((b): b is string => b !== undefined), ...now])];
}

/** Attente sans nouveau texte avant de lire une phrase inachevée (narration en flux). */
export const QUIET_MS = 1000;
/** Fin de phrase : le texte se lit sans attendre. */
const ENDED = /[.!?…:]["»”')\]]*$/u;

/** Annonces gardées dans la région (les plus anciennes sortent ; seules les ajoutées sont lues). */
const KEPT = 20;

/** Une annonce de la région ; sa clé ne sert qu'une fois. */
export interface Spoken {
  key: string;
  text: string;
}

/**
 * Ce que le fil a lu de chaque surface, et ce qu'il lit ensuite. Le texte nouveau se lit d'un coup
 * quand il finit une phrase, sinon après `QUIET_MS` sans nouveau texte : une narration en flux ne
 * se lit pas morceau par morceau.
 */
export class Speech {
  private heard = new Map<string, string[]>();
  private thread: Surface[] = [];
  private generation: number | null = null;
  /** Le tour ouvert à la dernière mise à jour. */
  private openTurn: number | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private next = 0;

  constructor(private readonly say: (spoken: Spoken[]) => void) {}

  /**
   * Les surfaces du fil (lues) et celles du dock (tenues pour lues, sa propre région les annonce :
   * une question qui s'y fige ne relit pas sa question). La première mise à jour (onglet rouvert)
   * tient tout pour lu : un onglet rouvert ne relit pas son fil. Une resynchronisation (nouvelle
   * `generation` : Lagged, reconnexion) aussi, sauf les surfaces du tour ouvert (`openTurn`, ou
   * celui qui l'était avant la coupure) : la suite de sa narration et son reçu sont encore lus.
   */
  update(thread: Surface[], dock: Surface[], generation: number, openTurn: number | null = null) {
    const present = new Set([...thread, ...dock].map((s) => s.id));
    for (const id of [...this.heard.keys()]) if (!present.has(id)) this.heard.delete(id);
    this.hear(dock);
    this.thread = thread;
    this.cancel();
    const wasOpen = this.openTurn;
    this.openTurn = openTurn;
    if (generation !== this.generation) {
      const first = this.generation === null;
      this.generation = generation;
      const open = first ? [] : [wasOpen, openTurn].filter((n): n is number => n !== null);
      this.hear(thread.filter((s) => s.turn === undefined || !open.includes(s.turn)));
      if (open.length === 0) return;
    }
    const pending = this.pending();
    if (pending.length === 0) return;
    if (pending.every((p) => ENDED.test(p.text))) this.flush();
    else this.timer = setTimeout(() => this.flush(), QUIET_MS);
  }

  dispose() {
    this.cancel();
  }

  private hear(surfaces: Surface[]) {
    for (const s of surfaces) this.heard.set(s.id, mergeHeard(this.heard.get(s.id) ?? [], spokenLines(s)));
  }

  private pending() {
    return this.thread
      .map((s) => ({ s, text: newSpeech(this.heard.get(s.id) ?? [], spokenLines(s)) }))
      .filter((p) => p.text);
  }

  private flush() {
    this.timer = null;
    const pending = this.pending();
    if (pending.length === 0) return;
    this.hear(pending.map((p) => p.s));
    this.say(pending.map((p) => ({ key: `${p.s.id}:${this.next++}`, text: p.text })));
  }

  private cancel() {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }
}

/**
 * Les annonces du fil (voir `Speech`) ; `generation` change à chaque resynchronisation, `openTurn`
 * est le tour en cours (`null` : aucun).
 */
export function useSpeech(thread: Surface[], dock: Surface[], generation: number, openTurn: number | null = null): Spoken[] {
  const [said, setSaid] = useState<Spoken[]>([]);
  const speech = useRef<Speech | null>(null);
  useEffect(() => {
    const s = new Speech((spoken) => setSaid((list) => [...list, ...spoken].slice(-KEPT)));
    speech.current = s;
    return () => s.dispose();
  }, []);
  useEffect(() => speech.current?.update(thread, dock, generation, openTurn), [thread, dock, generation, openTurn]);
  return said;
}
