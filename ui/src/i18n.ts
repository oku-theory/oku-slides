// Langue de l'interface : les messages Fluent de `ui.ftl`, servis par le cœur
// Rust (requête « messages »). L'anglais est chargé d'abord ; la langue de
// l'utilisateur le recouvre, message par message.

import { FluentBundle, FluentResource, type FluentValue, type FluentVariable, type Message } from "@fluent/bundle";
import { type Messages, inTauri, listenTauri, workspace } from "./api";

let bundle = newBundle("en");

/** Langues écrites de droite à gauche (parmi celles qu'on pourrait livrer). */
const RTL = new Set(["ar", "he", "fa", "ur"]);

/** Sens d'écriture d'une langue (`ar-EG` → `rtl`). */
export function direction(lang: string): "rtl" | "ltr" {
  return RTL.has(lang.split("-")[0].toLowerCase()) ? "rtl" : "ltr";
}

/** Remplace les messages : `resources` va de la référence à la langue choisie. */
export function setMessages(lang: string, resources: string[]): void {
  const next = newBundle(lang);
  for (const source of resources) next.addResource(isolatedResource(source), { allowOverrides: true });
  bundle = next;
  if (typeof document !== "undefined") {
    document.documentElement.lang = lang;
    // L'interface se retourne en arabe ; la scène (coordonnées de la slide) non.
    document.documentElement.dir = direction(lang);
  }
}

/** Code de la langue de l'interface (`en`, `fr`…). */
export function locale(): string {
  return bundle.locales[0] ?? "en";
}

/**
 * Bidi controls and invisible characters a label must not carry (t-959).
 *
 * Same set as `oku_slides_i18n::is_hidden_control`: bidi marks, embeddings, overrides and
 * isolates, U+061C, zero-width space, word joiners, BOM, U+180E, soft hyphen, the Hangul fillers
 * U+3164 and U+FFA0, tag characters. ZWJ and ZWNJ stay (Indic scripts, emoji sequences), as do
 * the variation selectors and the conjoining fillers U+115F, U+1160.
 */
const HIDDEN =
  /[\u061C\u200E\u200F\u202A-\u202E\u2066-\u2069\u200B\u2060-\u2064\uFEFF\u180E\u00AD\u3164\uFFA0\u{E0000}-\u{E007F}]/gu;

/**
 * Line breaks and terminal controls, turned into spaces in an argument (t-1379).
 *
 * Same set as `oku_slides_i18n::is_line_control`: the `Cc` controls (LF, CR, TAB, ESC, NEL…)
 * and U+2028, U+2029. A paragraph separator would end every isolate around it.
 */
const LINE = /[\u0000-\u001F\u007F-\u009F\u2028\u2029]/gu;

/** `s` made safe to interpolate: hidden controls removed, line controls turned into spaces. */
export function cleanArg(s: string): string {
  return s.replace(HIDDEN, "").replace(LINE, " ");
}

/** A pattern, as parsed by `@fluent/bundle` (not exported by its root). */
type Pattern = NonNullable<Message["value"]>;

/** A placeable of a pattern: variable, select, function or message reference, literal. */
type Placeable = Exclude<Exclude<Pattern, string>[number], string>;

/** `pattern` with each `{ $var }` as `{ ISOLATE($var) }`, in select variants too (t-1380). */
function isolatePlaceables(pattern: Pattern): Pattern {
  return typeof pattern === "string" ? pattern : pattern.map((e) => (typeof e === "string" ? e : isolatePlaceable(e)));
}

/** A bare variable placeable turned into `ISOLATE($var)`; selectors stay bare and match raw strings. */
function isolatePlaceable(e: Placeable): Placeable {
  if (e.type === "var") return { type: "func", name: "ISOLATE", args: [e] };
  if (e.type === "select") return { ...e, variants: e.variants.map((v) => ({ ...v, value: isolatePlaceables(v.value) })) };
  return e;
}

/**
 * `source` parsed, each message and term value printing its string variables isolated.
 *
 * Attributes are left as they are: `t()` formats message values only.
 */
function isolatedResource(source: string): FluentResource {
  const resource = new FluentResource(source);
  for (const entry of resource.body) if (entry.value !== null) entry.value = isolatePlaceables(entry.value);
  return resource;
}

/** A string that already is one isolate, such as an `isolate()` result passed as an argument. */
const ONE_ISOLATE = /^\u2068[^\u2068\u2069]*\u2069$/u;

/**
 * Wraps a string argument in FSI and PDI; other values pass through.
 *
 * Numbers, dates and a missing variable keep their own formatting.
 * The value was cleaned by `safeArg`, whose only isolates are balanced pairs.
 */
function isolateArg([value]: FluentValue[]): FluentValue {
  return typeof value === "string" && !ONE_ISOLATE.test(value) ? `\u2068${value}\u2069` : value;
}

/** A bundle for `lang`: isolation by `ISOLATE` on string variables only, not on every placeable. */
function newBundle(lang: string): FluentBundle {
  return new FluentBundle(lang, { useIsolating: false, functions: { ISOLATE: isolateArg } });
}

/** One pair an `isolate` call put around a value (several when isolated items were joined). */
const ISOLATED = /\u2068([^\u2068\u2069]*)\u2069/gu;

/** A string argument made safe: cleaned, keeping each balanced pair `isolate` calls put in it. */
function safeArg(v: FluentVariable): FluentVariable {
  if (typeof v !== "string") return v;
  let out = "";
  let last = 0;
  for (const m of v.matchAll(ISOLATED)) {
    out += cleanArg(v.slice(last, m.index)) + isolate(cleanArg(m[1] ?? ""));
    last = m.index + m[0].length;
  }
  return out + cleanArg(v.slice(last));
}

/**
 * Un message ; un identifiant inconnu se voit tel quel plutôt que de disparaître.
 *
 * String arguments are cleaned, so a forged name cannot reorder the sentence with overrides
 * (t-959) or inject a line (t-1379), and printed between FSI and PDI, so a right-to-left value
 * cannot reorder the text around it (t-1380).
 */
export function t(id: string, args?: Record<string, FluentVariable>): string {
  const message = bundle.getMessage(id);
  if (!message?.value) return id;
  const safe = args && Object.fromEntries(Object.entries(args).map(([k, v]) => [k, safeArg(v)]));
  return bundle.formatPattern(message.value, safe);
}

/** Isolation marks `t()` puts around string arguments. */
const ISOLATES = /[\u2068\u2069]/gu;

/** `t()` output without isolation marks, for text sent to the host (t-1380). */
export function tPlain(id: string, args?: Record<string, FluentVariable>): string {
  return t(id, args).replace(ISOLATES, "");
}

/** The bidi embedding, override and isolate controls, which a label must not carry into a dialog. */
const BIDI_CONTROLS = /[\u202A-\u202E\u2066-\u2069]/g;

/**
 * Wraps `s` in one FSI and one PDI, its own bidi controls removed first.
 *
 * The controls go because a PDI inside could close the isolate early. `t()` isolates each string
 * argument; a list of labels joined into one argument isolates each item, so one right-to-left
 * item cannot reorder its neighbours.
 */
export function isolate(s: string): string {
  return "\u2068" + s.replace(BIDI_CONTROLS, "") + "\u2069";
}

let chosen: string | null = null;
let shortcuts: Record<string, string> = {};

/** The theme shortcut's words in shipped locale `code` (`ui-shortcut-theme`), if it ships. */
export function shortcutWords(code: string): string | undefined {
  return Object.hasOwn(shortcuts, code) ? shortcuts[code] : undefined;
}
const listeners = new Set<() => void>();

/** Prévient quand la langue change ; rend de quoi se désabonner. */
export function onLocaleChange(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Installe les messages reçus de l'hôte, puis prévient. */
export function applyMessages(m: Messages): void {
  setMessages(m.locale, m.resources);
  chosen = m.chosen ?? null;
  shortcuts = m.shortcuts ?? {};
  for (const listener of listeners) listener();
}

/**
 * Génération des messages : avance à chaque demande et à chaque diffusion reçue. Une réponse
 * de l'hôte n'est appliquée que si rien de plus récent n'est arrivé entre-temps (réponses
 * dans le désordre, diffusion arrivée avant la réponse ou pendant le premier chargement).
 */
let generation = 0;

/** Charge les messages de l'hôte ; à attendre avant le premier rendu. */
export async function loadMessages(): Promise<void> {
  const asked = ++generation;
  const m = await workspace.messages();
  if (asked === generation) applyMessages(m);
}

/**
 * Messages diffusés par l'hôte après un choix de langue (seul `set_locale` les diffuse) :
 * appliqués s'ils changent la langue ou le choix, sinon ignorés. Quel que soit l'ordre de
 * la diffusion et de la réponse, la fenêtre qui a choisi n'est remontée qu'une fois.
 */
export function receiveMessages(m: Messages): boolean {
  generation++;
  if (m.locale === locale() && (m.chosen ?? null) === chosen) return false;
  applyMessages(m);
  return true;
}

/** Suit la langue choisie dans n'importe quelle fenêtre (`messages`, diffusé par l'hôte) ; rend de quoi arrêter. */
export function followLocale(): () => void {
  if (!inTauri) return () => {};
  return listenTauri(async () => {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<Messages>("messages", (e) => receiveMessages(e.payload));
  });
}

/** Change la langue de l'interface (`null` : celle du système). */
export async function changeLocale(code: string | null): Promise<void> {
  const asked = ++generation;
  const m = await workspace.setLocale(code);
  if (asked === generation) receiveMessages(m);
}
