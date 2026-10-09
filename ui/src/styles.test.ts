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

test("les pastilles sur plusieurs lignes s'alignent au début de la ligne (RTL compris)", () => {
  expect(ruleOf(own("styles.css"), ".chip")).toMatch(/text-align:\s*start/);
  expect(ruleOf(own("a2ui/a2ui.css"), ".a2ui-row .btn")).toMatch(/text-align:\s*start/);
});

test("« ↓ Tour en cours » est une pastille de texte, pas un rond d'icône", () => {
  const jump = ruleOf(own("a2ui/a2ui.css"), ".token.agent-jump");
  expect(jump).toMatch(/width:\s*auto/);
  expect(jump).toMatch(/white-space:\s*nowrap/);
});

test("la pile des slides repliée vers la gauche ne garde qu'un rail", () => {
  const rule = ruleOf(own("styles.css"), '.table:has(> .pile-column[data-folded="true"])');
  expect(rule).toMatch(/--pile-w:\s*var\(--pile-rail-w\)\s*!important/);
  expect(own("styles.css")).toMatch(/--pile-rail-w:\s*56px/);
});

test("la carte de la cible se pose sous la scène, défilant seule", () => {
  const rule = ruleOf(own("styles.css"), ".templates-panel > .host-settings, .templates-panel > .chart-panel");
  expect(rule).toMatch(/max-height:\s*40%/);
  // L'ombre de la feuille (positionnée) passe sous la carte, posée à distance de la slide.
  expect(rule).toMatch(/position:\s*relative/);
  expect(rule).toMatch(/z-index:\s*1/);
  expect(rule).toMatch(/margin:\s*var\(--space-10\) auto/);
});

test("la feuille prend le plus de place possible dans sa zone", () => {
  expect(ruleOf(own("styles.css"), ".sheet-area")).toMatch(/padding:\s*var\(--space-4\) var\(--space-6\)\s*;/);
});

test("les boutons slide précédente / suivante restent discrets hors survol", () => {
  expect(ruleOf(own("styles.css"), ".token.ghost.slide-nav")).toMatch(/opacity:\s*0\.35/);
  expect(ruleOf(own("styles.css"), ".slide-nav:disabled")).toMatch(/visibility:\s*hidden/);
});

test("le tiroir des variantes a sa place sous la scène, qui rétrécit, au lieu de la recouvrir", () => {
  const drawer = ruleOf(own("styles.css"), ".sheet-area > .variant-drawer");
  expect(drawer).toBeDefined();
  expect(drawer).toMatch(/grid-row:\s*2/);
  expect(drawer).toMatch(/overflow-y:\s*auto/);
  expect(own("styles.css")).not.toMatch(/\.variant-drawer\s*\{[^}]*position:\s*absolute/);
  const area = ruleOf(own("styles.css"), ".sheet-area");
  expect(area).toMatch(/display:\s*grid/);
  // La scène prend ce qui reste ; le tiroir, sa hauteur.
  expect(area).toMatch(/grid-template-rows:\s*minmax\(0,\s*1fr\)\s+auto/);
  expect(ruleOf(own("styles.css"), ".sheet-area > .templates-panel, .sheet-area > .widget-pane")).toMatch(/grid-row:\s*1/);
});
