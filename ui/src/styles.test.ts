// Les feuilles de style de l'app : plus de post-it, une seule définition de la carte.

import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { contrast, luminance } from "./color";

const own = (f: string) => readFileSync(join(import.meta.dir, f), "utf8");
/** The app's sheets, loaded in turn by main.tsx (styles.css continued by import.css, pack.css and inspector.css). */
const APP = ["styles.css", "import.css", "pack.css", "inspector.css"].map(own).join("\n");
/** The sheet's grid: stage, drawer and data pane in the first column, the inspector in the second. */
const GRID = own("inspector.css");
const css = [APP, ...["glass.css", "a2ui/a2ui.css"].map(own)].join("\n");

test("aucun style post-it ne reste dans les feuilles de style", () => {
  expect(css).not.toMatch(/postit|--tilt|shadow-note/);
});

test("la carte n'est définie que dans a2ui.css", () => {
  // `.card` seule, pas `.card-name` (le nom d'une carte de sélecteur).
  const card = /\.card(?![\w-])/;
  expect(own("a2ui/a2ui.css")).toMatch(card);
  // Seule exception : la mise en page du brouillon, plus spécifique que `.card.prompt`.
  const rest = APP.replace(/\.draft\.card\.prompt/g, "");
  expect(rest).not.toMatch(card);
});

test("la consigne se colle au bord de fin, aussi en RTL", () => {
  const rule = own("a2ui/a2ui.css").match(/\.card\.prompt,\s*\.a2ui-prompt\s*\{([^}]*)\}/);
  expect(rule?.[1]).toMatch(/margin-inline-start:\s*auto/);
  expect(rule?.[1]).not.toMatch(/margin:\s*0 0 0 auto/);
});

test("le brouillon garde sa mise en colonne malgré .card.prompt", () => {
  const rule = APP.match(/\.draft\.card\.prompt\s*\{([^}]*)\}/);
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

test("sans colonne de gauche, la feuille reprend sa largeur", () => {
  expect(ruleOf(APP, ".table:not(:has(> .pile-column, > .side-panel))")).toMatch(/--pile-w:\s*0px\s*!important/);
});

test("la bande des miniatures défile à l'horizontale sous la scène", () => {
  const rule = ruleOf(APP, ".slide-strip .pile");
  expect(rule).toMatch(/flex-direction:\s*row/);
  expect(rule).toMatch(/overflow-x:\s*auto/);
  expect(ruleOf(APP, ".slide-strip")).toMatch(/flex:\s*none/);
});

test("le bouton d'ajout se centre verticalement sur les miniatures de la bande", () => {
  expect(ruleOf(APP, ".slide-strip .add-slide")).toMatch(/align-self:\s*center/);
});

test("la bande des miniatures laisse la place à l'anneau, à l'ombre et au soulèvement de la courante", () => {
  // Défilant à l'horizontale, la bande rogne ce qui dépasse de sa boîte de contenu : la marge
  // intérieure doit loger l'ombre soulevée (et l'anneau de 3 px) en haut comme en bas.
  const rule = ruleOf(APP, ".slide-strip .pile") ?? "";
  expect(rule).toMatch(/padding:\s*var\(--space-4\)\s+var\(--space-4\)\s+var\(--space-6\)\s*;/);
});

test("les miniatures de la bande portent une ombre basse, contenue par sa marge (pas de bord net)", () => {
  // À l'élévation des slides (2.4), l'ombre soulevée descend de ~77 px : la bande la couperait net.
  expect(ruleOf(APP, ".slide-strip")).toMatch(/--elev-sheet:\s*1\s*;/);
  // Déclarées aussi sur la bande : sinon elles gardent l'élévation calculée à la racine.
  const shadows = ruleOf(APP, ":root, .slide-strip") ?? "";
  expect(shadows).toMatch(/--shadow-sheet-small:/);
  expect(shadows).toMatch(/--shadow-sheet-lifted:/);
});

test("la bande des miniatures laisse de l'air sous la slide", () => {
  expect(ruleOf(APP, ".slide-strip")).toMatch(/margin:\s*var\(--space-4\)\s+var\(--space-4\)\s+0\s*;/);
});

/** Les variables posées dans le code (`style={{ "--x": … }}`, `setProperty("--x", …)`). */
function setInCode(dir = import.meta.dir): Set<string> {
  const out = new Set<string>();
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, e.name);
    if (e.isDirectory()) setInCode(path).forEach((v) => out.add(v));
    else if (/\.tsx?$/.test(e.name)) for (const m of readFileSync(path, "utf8").matchAll(/["'](--[\w-]+)["']/g)) out.add(m[1]);
  }
  return out;
}

test("chaque var() sans repli lit une variable définie (une seule inconnue annule toute la déclaration)", () => {
  const declared = new Set([...css.matchAll(/(--[\w-]+)\s*:/g)].map((m) => m[1]));
  setInCode().forEach((v) => declared.add(v));
  const unknown = [...new Set([...css.matchAll(/var\(\s*(--[\w-]+)\s*\)/g)].map((m) => m[1]))].filter((v) => !declared.has(v));
  expect(unknown).toEqual([]);
});


test("la feuille prend le plus de place possible dans sa zone", () => {
  expect(ruleOf(APP, ".sheet-area")).toMatch(/padding:\s*var\(--space-4\) var\(--space-6\)\s*;/);
});

test("les boutons slide précédente / suivante restent discrets hors survol", () => {
  expect(ruleOf(APP, ".token.ghost.slide-nav")).toMatch(/opacity:\s*0\.35/);
  expect(ruleOf(APP, ".slide-nav:disabled")).toMatch(/visibility:\s*hidden/);
});

test("le tiroir des variantes a sa place sous la scène, qui rétrécit, au lieu de la recouvrir", () => {
  const drawer = ruleOf(APP, ".sheet-area > .variant-drawer");
  expect(drawer).toBeDefined();
  expect(drawer).toMatch(/overflow-y:\s*auto/);
  expect(ruleOf(GRID, ".sheet-area > .variant-drawer")).toMatch(/grid-row:\s*2/);
  expect(APP).not.toMatch(/\.variant-drawer\s*\{[^}]*position:\s*absolute/);
  expect(ruleOf(APP, ".sheet-area")).toMatch(/display:\s*grid/);
  // La scène prend ce qui reste ; le tiroir puis le volet des données, leur hauteur.
  expect(ruleOf(GRID, ".sheet-area")).toMatch(/grid-template-rows:\s*minmax\(0,\s*1fr\)\s+auto\s+auto/);
  expect(ruleOf(GRID, ".sheet-area > .stage-panel, .sheet-area > .widget-pane")).toMatch(/grid-row:\s*1/);
});

test("the data pane sits under the stage in the sheet's grid, never over it", () => {
  const pane = ruleOf(GRID, ".sheet-area > .data-pane");
  expect(pane).toMatch(/grid-row:\s*3/);
  expect(pane).toMatch(/max-height:\s*40vh/);
  expect(APP).not.toMatch(/\.data-pane\s*\{[^}]*position:\s*absolute/);
});

test("the inspector is not in the sheet's grid: it is a tab of the side panel", () => {
  expect(GRID).not.toMatch(/\.sheet-area\s*>\s*\.inspector|--inspector-w/);
});

test("the side panel takes the table's left column", () => {
  expect(ruleOf(GRID, ".side-panel")).toMatch(/grid-area:\s*pile/);
});

test("the folded side panel keeps only its rail's width, over a width chosen at the divider", () => {
  expect(ruleOf(GRID, ".table:has(> .side-panel:not(.open))")).toMatch(/--pile-w:\s*48px\s*!important/);
});

test("the open side panel defaults to 320px and yields to the divider's inline width", () => {
  const open = ruleOf(GRID, ".table:has(> .side-panel.open)") ?? "";
  expect([/--pile-w:\s*320px/.test(open), /!important/.test(open)]).toEqual([true, false]);
});

test("the pile divider shows only beside a pile column or an open side panel", () => {
  expect(ruleOf(APP, ".table:not(:has(> .pile-column, > .side-panel.open)) .col-divider[data-side=\"pile\"]")).toMatch(/display:\s*none/);
});

test("below 1100 px the open side panel overlays the stage instead of taking a column", () => {
  const narrow = GRID.match(/@media \(max-width: 1100px\)\s*\{([\s\S]*?)\n\}/)?.[1] ?? "";
  expect(ruleOf(narrow, ".table:has(> .side-panel.open)")).toMatch(/--pile-w:\s*48px\s*!important/);
  const overlay = ruleOf(narrow, ".side-panel.open > .side-body") ?? "";
  expect([/position:\s*absolute/.test(overlay), /inset-inline-start:\s*48px/.test(overlay)]).toEqual([true, true]);
});

test("side panel body lies unframed on the table", () => {
  const rules = [".side-packs-box, .side-themes-box", ".side-body > .inspector"].map((sel) => ruleOf(GRID, sel) ?? "");
  expect(rules.map((r) => r !== "" && !/box-shadow:\s*var/.test(r))).toEqual([true, true]);
});

test("side panel cards are a flat tint, without any shadow", () => {
  const card = ruleOf(GRID, ".side-body .a2ui-card") ?? "";
  expect([/background:\s*color-mix/.test(card), /box-shadow:\s*none/.test(card)]).toEqual([true, true]);
});

test("side panel rhythm is set once on its body", () => {
  const body = ruleOf(GRID, ".side-body") ?? "";
  const tokens = ["--side-title-gap: var(--space-6)", "--side-section-gap: var(--space-10)", "--side-item-gap: var(--space-4)"];
  expect(tokens.map((t) => body.includes(t))).toEqual([true, true, true]);
});

test("side panel titles head their content with the same gap in every tab", () => {
  const gaps = [".side-themes-box .a2ui-column", ".side-body > .inspector"].map((sel) => ruleOf(GRID, sel) ?? "");
  expect(gaps.map((r) => /gap:\s*var\(--side-title-gap\)/.test(r))).toEqual([true, true]);
});

test("side panel packs sections stand a section gap apart", () => {
  const rule = ruleOf(GRID, '.side-packs-box .a2ui-column:has(> [data-component-id="layouts"])');
  expect(rule).toMatch(/gap:\s*var\(--side-section-gap\)/);
});

test("side panel packs section titles stand a title gap above their cards", () => {
  const rule = ruleOf(GRID, '.side-packs-box .a2ui-slot[data-component-id$="-title"]') ?? "";
  expect(rule).toMatch(/margin-block-end:\s*calc\(var\(--side-title-gap\) - var\(--space-2\)\)/);
});

test("side panel pack cards stand an item gap apart", () => {
  const sel = ".side-packs-box .a2ui-slot:has(> * > .a2ui-card) + .a2ui-slot:has(> * > .a2ui-card)";
  expect(ruleOf(GRID, sel)).toMatch(/margin-block-start:\s*calc\(var\(--side-item-gap\) - var\(--space-2\)\)/);
});

test("side panel inspector title is set like the other tabs' titles", () => {
  expect(ruleOf(GRID, ".side-body > .inspector .pane-title")).toMatch(/font:\s*600 14px\/20px var\(--sans\)/);
});

test("side panel theme thumbnails are flat", () => {
  expect(ruleOf(GRID, ".side-themes-box .theme-choice :is(img, .theme-choice-blank)")).toMatch(/box-shadow:\s*none/);
});

test("a pill button shares a surface row's button rule", () => {
  expect(ruleOf(own("a2ui/a2ui.css"), ".a2ui-row .btn, .btn.pill")).toMatch(/border-radius:\s*999px/);
});

test("narrow side panel overlay paints its own background", () => {
  const narrow = GRID.match(/@media \(max-width: 1100px\)\s*\{([\s\S]*?)\n\}/)?.[1] ?? "";
  expect(ruleOf(narrow, ".side-panel.open > .side-body")).toMatch(/background:\s*var\(--paper-000\)/);
});

/** The `color.bg`-style token values of the built-in default theme (Papier), the source of the app's defaults. */
const papier = JSON.parse(readFileSync(join(import.meta.dir, "../../typst/oku-slides/themes/papier.json"), "utf8")).tokens as {
  app: { desk: string; piece: string };
  "app-dark": { desk: string; piece: string };
};

/** The hex a custom property is set to in `block` of styles.css. */
const hexIn = (block: string, name: string) => block.match(new RegExp(`${name}:\\s*(#[0-9a-fA-F]{3,6})\\b`))?.[1];

// Covers the built-in light and dark defaults only: a theme's `--app-*` overrides set by
// `applyAppearance` are not checked, and the bar's translucent `--shade` is not a hex.
test("tab kind borders reach 3:1 against the default tab backgrounds in light and dark", () => {
  const root = css.match(/(?:^|\n):root\s*\{([^}]*)\}/)?.[1] ?? "";
  const dark = css.match(/:root\[data-scheme="dark"\]\s*\{([^}]*)\}/)?.[1] ?? "";
  const schemes = [
    { name: "light", block: root, backgrounds: [papier.app.piece, papier.app.desk] },
    { name: "dark", block: dark, backgrounds: [papier["app-dark"].piece, papier["app-dark"].desk] },
  ];
  for (const { name, block, backgrounds } of schemes) {
    for (const token of ["--tab-pack", "--tab-theme"]) {
      const hex = hexIn(block, token);
      expect(hex, `${token} in ${name}`).toBeDefined();
      for (const bg of backgrounds) {
        expect(contrast(luminance(hex!)!, luminance(bg)!), `${token} on ${bg} in ${name}`).toBeGreaterThanOrEqual(3);
      }
    }
  }
});

test("a wide Markdown table scrolls sideways in its box and keeps its table display", () => {
  const box = ruleOf(APP, ".md .md-table");
  expect(box).toMatch(/overflow-x:\s*auto/);
  expect(box).toMatch(/max-width:\s*100%/);
  expect(ruleOf(APP, ".md table")).not.toMatch(/display:/);
});

test("history bar splits text and slider 25/75, ignoring the version text and theme font", () => {
  // A content-sized text track (e.g. `minmax(180px, auto)`) lets each version's caption
  // and the theme's serif resize the slider while scrolling: both tracks need a fixed minimum.
  const columns = ruleOf(APP, ".history-bar")?.match(/grid-template-columns:\s*([^;]+);/)?.[1];
  const [version, track] = (columns ?? "").match(/minmax\([^)]*\)|\S+/g) ?? [];
  expect(version).toMatch(/^minmax\((0|\d+px),\s*25fr\)$/);
  expect(track).toMatch(/^minmax\(\d+px,\s*75fr\)$/);
});
