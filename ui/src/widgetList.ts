/** Liste des fichiers de widgets : ordre, libellés, états, squelettes. */
import type { WidgetDiagnostic, WidgetStatus } from "./api";
import { t } from "./i18n";

/** Le fichier de code partagé du pack. */
export const COMMON = "_commun";

const NAME = /^[a-z][a-z0-9-]{0,63}$/;

/** Un nom de fichier admis par la session. */
export function isWidgetName(name: string): boolean {
  return name === COMMON || NAME.test(name);
}

/**
 * Un nom saisi pour Nouveau ou Renommer (`from`) : admis, et libre dans
 * `list`. `_commun` se crée (s'il manque) mais ne se prend pas par un
 * renommage. Faux tant que la liste est inconnue.
 */
export function nameAvailable(list: WidgetStatus[] | null, value: string, from: string | null): boolean {
  if (list === null || !isWidgetName(value) || (value === COMMON && from !== null)) return false;
  return !list.some((s) => s.name === value && s.name !== from);
}

/** Libellé d'un fichier : « Commun » pour `_commun`. */
export function widgetLabel(name: string): string {
  return name === COMMON ? t("ui-widgets-common") : name;
}

export type WidgetState = "valid" | "errors";

export function stateOf(s: WidgetStatus): WidgetState {
  return s.diagnostics.some((d) => d.error) ? "errors" : "valid";
}

/** `_commun` en tête, puis par nom. */
export function ordered(list: WidgetStatus[]): WidgetStatus[] {
  return [...list].sort((a, b) => (a.name === COMMON ? -1 : b.name === COMMON ? 1 : a.name.localeCompare(b.name)));
}

/** `list` où les statuts `fresh` remplacent ceux de même nom, ou s'ajoutent ; ordonnés. */
export function merged(list: WidgetStatus[] | null, fresh: WidgetStatus[]): WidgetStatus[] {
  const names = new Set(fresh.map((s) => s.name));
  return ordered([...(list ?? []).filter((s) => !names.has(s.name)), ...fresh]);
}

/**
 * Statuts relus (`list`, la sonde des fichiers) complétés par ceux d'un
 * essai (`trial`, réponse d'une écriture) : les diagnostics de l'essai
 * absents de la relecture, ceux du rendu, s'ajoutent à leur fichier.
 */
export function withTrial(list: WidgetStatus[], trial: WidgetStatus[]): WidgetStatus[] {
  return list.map((s) => {
    const extra = trial.find((x) => x.name === s.name)?.diagnostics ?? [];
    const seen = new Set(s.diagnostics.map(diagnosticKey));
    const added = extra.filter((d) => !seen.has(diagnosticKey(d)));
    return added.length ? { ...s, diagnostics: [...s.diagnostics, ...added] } : s;
  });
}

/** Les diagnostics de `s` placés dans le fichier `file`. */
export function fileDiagnostics(s: WidgetStatus | undefined, file: string): WidgetDiagnostic[] {
  return (s?.diagnostics ?? []).filter((d) => d.file === file);
}

/**
 * Diagnostics listés pour un fichier : ceux de son statut (ceux de
 * `_commun` qu'il importe compris), puis ceux de l'aperçu, sans doublon.
 */
export function listedDiagnostics(s: WidgetStatus | undefined, preview: WidgetDiagnostic[]): WidgetDiagnostic[] {
  const out: WidgetDiagnostic[] = [];
  const seen = new Set<string>();
  for (const d of [...(s?.diagnostics ?? []), ...preview]) {
    const key = diagnosticKey(d);
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(d);
  }
  return out;
}

/** Identité d'un diagnostic, pour les dédoublonner. */
function diagnosticKey(d: WidgetDiagnostic): string {
  return JSON.stringify([d.file, d.line, d.column, d.error, d.message]);
}

/** Lieu d'un diagnostic : `fichier:ligne:colonne`, le fichier seul, ou rien. */
export function diagnosticPlace(d: WidgetDiagnostic): string {
  const file = d.file ?? "";
  return d.line == null ? file : `${file}:${d.line}:${d.column ?? 1}`;
}

/** `text` où la déclaration `widget("<from>"` devient `widget("<to>"` (la première, comme la session). */
export function declaredAs(text: string, from: string, to: string): string {
  return text.replace(`widget("${from}"`, `widget("${to}"`);
}

/** Squelette d'un nouveau fichier : imports, déclaration, un champ éditable. */
export function skeleton(name: string): string {
  if (name === COMMON) return '// Code partagé : #import "_commun.typ": …\n#import "/oku-slides/lib.typ": color-of, size-of\n';
  return [
    '#import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of',
    "",
    `#let widget = widget("${name}", params: (title: p.plain()),`,
    '  (args, ctx) => field(ctx, "title", { set text(fill: color-of(ctx.tokens, "accent")); args.title }))',
    "",
  ].join("\n");
}
