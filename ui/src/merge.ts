// Enregistrer sans écraser un fichier remplacé par un autre deck, ouvrir
// une autre copie d'un deck déjà ouvert : les questions, sur le modèle de `close.ts`.

import { inTauri, pickSavePath, type Snapshot, type TabApi, tabApi, workspace } from "./api";
import { t } from "./i18n";

/** Que faire d'un fichier remplacé sur disque par un autre deck. */
export type ReplacedChoice = "overwrite" | "save-as" | "cancel";
/** Pose la question d'un fichier remplacé (`file` : son nom) ; le choix de l'utilisateur. */
export type AskReplaced = (file: string) => Promise<ReplacedChoice>;
/** Choisit où enregistrer ; `null` : renoncé. */
export type PickSavePath = (name: string) => Promise<string | null>;

/** Le nom du fichier de `path`, sinon `name`. */
const fileName = (path: string | null, name: string) => (path ? (path.split(/[\\/]/).pop() ?? name) : name);

/** Demande quoi faire d'un fichier remplacé sur disque par un autre deck. */
async function askReplaced(file: string): Promise<ReplacedChoice> {
  const text = t("ui-disk-replaced-question", { file });
  // Navigateur (développement) : `window.confirm` n'a que deux boutons, pas d'« Enregistrer sous… ».
  if (!inTauri) return window.confirm(text) ? "overwrite" : "cancel";
  const labels = { yes: t("ui-disk-replaced-overwrite"), no: t("ui-disk-replaced-save-as"), cancel: t("ui-close-cancel") };
  const { message } = await import("@tauri-apps/plugin-dialog");
  const r = await message(text, { kind: "warning", buttons: labels });
  if (r === labels.yes || r === "Yes") return "overwrite";
  if (r === labels.no || r === "No") return "save-as";
  return "cancel";
}

/**
 * Enregistre `snap` à sa place (un deck sans fichier va au dossier par
 * défaut) ; un fichier remplacé par un autre deck fait poser la question.
 * `null` : rien n'a été enregistré. `ask` et `pick` s'injectent pour les
 * tests (par défaut, la question et le dialogue réels).
 */
export async function saveGuarded(
  api: TabApi,
  snap: Snapshot,
  ask: AskReplaced = askReplaced,
  pick: PickSavePath = pickSavePath,
): Promise<Snapshot | null> {
  const s = await api.save(null, false);
  if (!s.disk_replaced) return s;
  const choice = await ask(fileName(snap.path, snap.name));
  let last: Snapshot | null = null;
  if (choice === "overwrite") last = await api.save(null, true);
  if (choice === "save-as") {
    // Le dialogue du système a déjà fait confirmer le remplacement d'un
    // fichier existant, fût-ce celui-là même.
    const other = await pick(snap.name);
    last = other ? await api.save(other, true) : null;
  }
  // Toujours refusé : rien n'a été écrit.
  return last && !last.disk_replaced ? last : null;
}

/** Que faire d'une autre copie d'un deck déjà ouvert. */
export type SameDeckChoice = "merge" | "apart" | "cancel";

/** Ce dont `openGuarded` a besoin ; injecté pour les tests. */
export interface OpenDeps {
  sameDeck: (path: string) => Promise<{ tab: string | null; title: string | null }>;
  openTab: (window: string, path: string) => Promise<unknown>;
  /** Fusionne ; la réponse porte l'avis de fusion (montré une fois). */
  merge: (tab: string, path: string) => Promise<{ notice: string | null } | undefined>;
  activate: (tab: string) => Promise<unknown>;
  ask: (file: string, title: string) => Promise<SameDeckChoice>;
}

/** Demande s'il faut fusionner `file` dans l'onglet `title`, ou l'ouvrir à part. */
async function askSameDeck(file: string, title: string): Promise<SameDeckChoice> {
  const text = t("ui-same-deck-question", { file, title });
  // Navigateur (développement) : `window.confirm` n'a que deux boutons, pas d'annulation.
  if (!inTauri) return window.confirm(text) ? "merge" : "apart";
  const labels = { yes: t("ui-same-deck-merge"), no: t("ui-same-deck-apart"), cancel: t("ui-close-cancel") };
  const { message } = await import("@tauri-apps/plugin-dialog");
  const r = await message(text, { kind: "info", buttons: labels });
  if (r === labels.yes || r === "Yes") return "merge";
  if (r === labels.no || r === "No") return "apart";
  return "cancel";
}

/** Les dépendances réelles de `openGuarded`. */
export const openDeps: OpenDeps = {
  sameDeck: (path) => workspace.sameDeck(path),
  openTab: (window, path) => workspace.openTab(window, path),
  merge: (tab, path) => tabApi(tab).merge(path),
  activate: (tab) => workspace.activate(tab),
  ask: askSameDeck,
};

/**
 * Ouvre `path` dans la fenêtre `window` ; s'il est une autre copie d'un deck
 * déjà ouvert, demande d'abord : fusionner dans cet onglet, ou ouvrir à part.
 * Rend l'avis de fusion et son onglet (la réponse de la fusion est la seule
 * à le porter), `null` sans fusion.
 */
export async function openGuarded(
  window: string,
  path: string,
  deps: OpenDeps = openDeps,
): Promise<{ tab: string; text: string } | null> {
  const same = await deps.sameDeck(path);
  if (!same.tab) {
    await deps.openTab(window, path);
    return null;
  }
  const choice = await deps.ask(fileName(path, path), same.title ?? "");
  if (choice === "merge") {
    const merged = await deps.merge(same.tab, path);
    await deps.activate(same.tab);
    return merged?.notice ? { tab: same.tab, text: merged.notice } : null;
  }
  if (choice === "apart") await deps.openTab(window, path);
  return null;
}

/**
 * L'avis de fusion à montrer par l'onglet actif de la fenêtre `win` :
 * l'onglet fusionné une fois actif, ou, s'il est dans une autre fenêtre,
 * l'onglet actif d'ici (sinon l'avis serait perdu) ; `null` : pas encore.
 */
export function mergedNotice(
  merged: { tab: string; text: string } | null,
  win: { active: string; tabs: { id: string }[] },
): string | null {
  if (!merged) return null;
  const here = win.tabs.some((x) => x.id === merged.tab);
  return !here || win.active === merged.tab ? merged.text : null;
}
