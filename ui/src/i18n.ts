// Langue de l'interface : les messages Fluent de `ui.ftl`, servis par le cœur
// Rust (requête « messages »). L'anglais est chargé d'abord ; la langue de
// l'utilisateur le recouvre, message par message.

import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";
import { type Messages, inTauri, listenTauri, workspace } from "./api";

let bundle = new FluentBundle("en", { useIsolating: false });

/** Langues écrites de droite à gauche (parmi celles qu'on pourrait livrer). */
const RTL = new Set(["ar", "he", "fa", "ur"]);

/** Sens d'écriture d'une langue (`ar-EG` → `rtl`). */
export function direction(lang: string): "rtl" | "ltr" {
  return RTL.has(lang.split("-")[0].toLowerCase()) ? "rtl" : "ltr";
}

/** Remplace les messages : `resources` va de la référence à la langue choisie. */
export function setMessages(lang: string, resources: string[]): void {
  const next = new FluentBundle(lang, { useIsolating: false });
  for (const source of resources) next.addResource(new FluentResource(source), { allowOverrides: true });
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

/** Un message ; un identifiant inconnu se voit tel quel plutôt que de disparaître. */
export function t(id: string, args?: Record<string, FluentVariable>): string {
  const message = bundle.getMessage(id);
  return message?.value ? bundle.formatPattern(message.value, args) : id;
}

let available: string[] = [];
let chosen: string | null = null;
const listeners = new Set<() => void>();

/** Langues livrées par l'hôte, l'anglais d'abord. */
export function availableLocales(): string[] {
  return available;
}

/** Langue choisie dans les réglages ; `null` : celle du système. */
export function chosenLocale(): string | null {
  return chosen;
}

/** Prévient quand la langue change ; rend de quoi se désabonner. */
export function onLocaleChange(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Installe les messages reçus de l'hôte, puis prévient. */
export function applyMessages(m: Messages): void {
  setMessages(m.locale, m.resources);
  available = m.available ?? [];
  chosen = m.chosen ?? null;
  for (const listener of listeners) listener();
}

/** Le nom d'une langue dans elle-même (`de` → « Deutsch »). */
export function languageName(code: string): string {
  let name = code;
  try {
    name = new Intl.DisplayNames([code], { type: "language" }).of(code) ?? code;
  } catch {
    /* moteur sans noms de langues : le code */
  }
  return name.charAt(0).toLocaleUpperCase(code) + name.slice(1);
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
