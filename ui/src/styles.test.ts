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

// L'alignement des pastilles sur plusieurs lignes (RTL compris) et la pastille « ↓ Tour en cours »
// se vérifient sur la mise en page, dans e2e/060-agent.cjs (`assertPills`).

test("la pile des slides repliée vers la gauche ne garde qu'un rail", () => {
  const rule = ruleOf(own("styles.css"), '.table:has(> .pile-column[data-folded="true"])');
  expect(rule).toMatch(/--pile-w:\s*var\(--pile-rail-w\)\s*!important/);
  expect(own("styles.css")).toMatch(/--pile-rail-w:\s*56px/);
});

test("la carte de la cible se pose sous la scène, défilant seule", () => {
  const rule = ruleOf(own("styles.css"), ".settings-band > .host-settings, .settings-band > .chart-panel");
  // Dans la bande sous la scène : la carte rétrécit (et défile) plutôt que de déborder.
  expect(rule).toMatch(/flex:\s*0 1 auto/);
  expect(rule).toMatch(/min-height:\s*0/);
  expect(rule).toMatch(/overflow-y:\s*auto/);
  // L'ombre de la feuille (positionnée) passe sous la carte, posée à distance de la slide.
  expect(rule).toMatch(/position:\s*relative/);
  expect(rule).toMatch(/z-index:\s*1/);
  // La distance à la slide est celle du séparateur au-dessus de la carte (`.band-divider`).
  expect(rule).toMatch(/margin:\s*0 auto/);
  // Bande de hauteur fixe sous la scène : la boîte de la slide ne suit pas la hauteur de la carte (t-647) ;
  // la hauteur choisie au séparateur (`--band-h`) l'emporte sur le défaut.
  const band = ruleOf(own("styles.css"), ".settings-band");
  expect(band).toMatch(/flex:\s*none/);
  expect(band).toMatch(/height:\s*var\(--band-h,\s*clamp\([^)]*vh[^)]*\)\)/);
});

test("le panneau d'un graphique ou d'un tableau remplit la bande, sans plafond propre", () => {
  expect(ruleOf(own("chart/ChartDataGrid.css"), ".chart-panel")).not.toMatch(/max-height/);
});

test("la bascule graphique / tableau se cale en fin de rangée, de droite à gauche aussi", () => {
  expect(ruleOf(own("chart/ChartDataGrid.css"), ".panel-toolbar > .show-as")).toMatch(/margin-inline-start:\s*auto/);
});

test("le séparateur de la bande garde à la carte la distance d'avant à la slide", () => {
  const rule = ruleOf(own("styles.css"), ".band-divider");
  expect(rule).toMatch(/height:\s*12px/);
  expect(rule).toMatch(/margin-block:\s*calc\(\(var\(--space-10\) - 12px\) \/ 2\)/);
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
