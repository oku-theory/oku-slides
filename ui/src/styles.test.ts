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
