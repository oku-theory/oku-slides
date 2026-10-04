// Les feuilles de style de l'app : plus de post-it, une seule définition de la carte.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const own = (f: string) => readFileSync(join(import.meta.dir, f), "utf8");
const css = ["styles.css", "glass.css", "a2ui/a2ui.css", "chart/ChartDataGrid.css"].map(own).join("\n");

test("aucun style post-it ne reste dans les feuilles de style", () => {
  expect(css).not.toMatch(/postit|--tilt|shadow-note/);
});

test("la carte n'est définie que dans a2ui.css", () => {
  expect(own("a2ui/a2ui.css")).toMatch(/\.card\b/);
  // Seule exception : la mise en page du brouillon, plus spécifique que `.card.prompt`.
  const rest = own("styles.css").replace(/\.draft\.card\.prompt/g, "");
  expect(rest).not.toMatch(/\.card\b/);
});

test("la consigne se colle au bord de fin, aussi en RTL", () => {
  const rule = own("a2ui/a2ui.css").match(/\.card\.prompt,\s*\.a2ui-prompt\s*\{([^}]*)\}/);
  expect(rule?.[1]).toMatch(/margin-inline-start:\s*auto/);
  expect(rule?.[1]).not.toMatch(/margin:\s*0 0 0 auto/);
});

test("le brouillon garde sa mise en colonne malgré .card.prompt", () => {
  const rule = own("styles.css").match(/\.draft\.card\.prompt\s*\{([^}]*)\}/);
  expect(rule?.[1]).toMatch(/display:\s*flex/);
});

/** Le corps de la première règle dont le sélecteur correspond exactement à `selector` (commentaires ôtés). */
const ruleOf = (sheet: string, selector: string) => {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\s*,\s*/g, ",\\s*");
  const bare = sheet.replace(/\/\*[\s\S]*?\*\//g, "");
  return bare.match(new RegExp(`(?:^|\\})\\s*${escaped}\\s*\\{([^}]*)\\}`))?.[1];
};

test("la hauteur du dock se mesure au panneau entier, pas au fil ni à la fenêtre", () => {
  expect(ruleOf(own("styles.css"), ".notes")).toMatch(/container-type:\s*size/);
  const dock = ruleOf(own("a2ui/a2ui.css"), ".agent-dock");
  expect(dock).toMatch(/max-height:\s*45cqh/);
  expect(dock).not.toMatch(/max-height:\s*[\d.]+(%|vh|dvh)/);
});

test("les pastilles sur plusieurs lignes s'alignent au début de la ligne (RTL compris)", () => {
  expect(ruleOf(own("styles.css"), ".chip")).toMatch(/text-align:\s*start/);
  expect(ruleOf(own("a2ui/a2ui.css"), ".a2ui-row .btn")).toMatch(/text-align:\s*start/);
});

test("fil et dock vides ne prennent aucune place mais restent dans l'arbre d'accessibilité", () => {
  const empty = ruleOf(own("a2ui/a2ui.css"), ".agent-thread.vacant, .agent-dock.vacant");
  expect(empty).toMatch(/position:\s*absolute/);
  expect(empty).not.toMatch(/display:\s*none|visibility:\s*hidden/);
});

test("en Verre, le dock flotte : marge intérieure et coins arrondis, pas de filet", () => {
  const dock = ruleOf(own("glass.css"), ".agent-dock");
  expect(dock).toMatch(/padding:/);
  expect(dock).toMatch(/border-radius:/);
  expect(dock).toMatch(/border-top:\s*0/);
});

test("« ↓ Tour en cours » est une pastille de texte, pas un rond d'icône", () => {
  const jump = ruleOf(own("a2ui/a2ui.css"), ".token.agent-jump");
  expect(jump).toMatch(/width:\s*auto/);
  expect(jump).toMatch(/white-space:\s*nowrap/);
});
