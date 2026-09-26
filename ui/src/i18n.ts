// Langue de l'interface : les messages Fluent de `ui.ftl`, servis par le cœur
// Rust (requête « messages »). L'anglais est chargé d'abord ; la langue de
// l'utilisateur le recouvre, message par message.

import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";
import { workspace } from "./api";

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

/** Charge les messages de l'hôte ; à attendre avant le premier rendu. */
export async function loadMessages(): Promise<void> {
  const { locale: lang, resources } = await workspace.messages();
  setMessages(lang, resources);
}
