# Éditeur de texte avec rendu markdown — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remplacer la `<textarea>` de l'éditeur posé sur la slide par CodeMirror 6, qui garde les marques markdown visibles mais affiche gras, italique, liens et pastilles.

**Architecture:** Le parseur markdown existant (`ui/src/markdown.ts`) renvoie aussi des plages positionnées (`markdownSpans`) et le préfixe de liste (`listPrefix`) ; une fonction pure les traduit en décorations de ligne (`lineDecorations`), qu'un `ViewPlugin` CodeMirror applique. Un composant `TextEditor` enveloppe CodeMirror (touches, historique, dictée) et remplace la textarea dans `Stage.tsx` ; il transmet son texte à la validation, seule source de vérité.

**Tech Stack:** React 19.3 (`useEffectEvent`, 19.2 au moins), TypeScript strict, Bun (tests), CodeMirror 6 (`@codemirror/state` ^6.7.6, `@codemirror/view` ^6.43.13, `@codemirror/commands` ^6.11.1), Playwright (e2e, Chromium), Tauri (WebKit, vérification manuelle).

**Spec:** `docs/superpowers/specs/2026-09-26-editeur-markdown-design.md`

## Global Constraints

- Pas de `@codemirror/lang-markdown` : les styles viennent de `markdownSpans` ; une seule implémentation du parseur et une seule règle des préfixes de liste (`listPrefix`).
- Les champs (titre, sous-titre, kicker) restent du texte brut sur une ligne : pas de décorations, Entrée et Maj+Entrée valident.
- Marques toujours visibles. Pastille dans l'éditeur : fond seul, sans coins arrondis ni marge.
- Touches de base : `standardKeymap`, jamais `defaultKeymap` (commandes d'éditeur de code).
- `main.tsx` active `<StrictMode>` : tout effet doit supporter montage → démontage → montage.
- Tâche Oku parente : `t-126` ; chaque commit se termine par `(t-126)`, message en français, format `type(portée) : description`. Pas de ligne `Co-Authored-By`. Chaque tâche ferme sa sous-tâche Oku dans son dernier step.
- Commentaires et noms de tests en français ; première phrase d'une doc sous 15 mots ; `import type` sur une ligne à part, comme dans le code existant.
- Tests Bun côte à côte (`foo.test.ts` à côté de `foo.ts`), exclus de `tsc`. Commandes UI depuis `ui/` (`bun test`, `bun run typecheck`), le reste depuis la racine.
- « Déplacer tel quel » : le bloc déplacé **remplace** le commentaire qui l'annonce dans les exemples de code ci-dessous.

---

### Task 1 : Parseur — plages positionnées et préfixe de liste (sous-tâche `t-136`)

**Files:**
- Modify: `ui/src/markdown.ts` (doc du module, `parse`, nouvelles exportations), `ui/src/deck.ts:27-49` (`textToParagraphs` utilise `listPrefix`)
- Test: `ui/src/markdown.test.ts`, `ui/src/deck.test.ts`

**Interfaces:**
- Produces :
  ```ts
  export type SpanKind = "mark" | "bold" | "italic" | "link" | "url" | "highlight";
  export interface Span { from: number; to: number; kind: SpanKind; highlight?: number }
  export function markdownSpans(line: string): Span[] // non vides, triées par from croissant puis to décroissant, ordre d'ajout sinon
  export function listPrefix(line: string): { list?: "bullet" | "number"; length: number }
  ```
  Offsets en unités UTF-16 (indices de chaîne JS, comme les positions CodeMirror), relatifs à `line`.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `ui/src/markdown.test.ts`, ajouter `listPrefix, markdownSpans` à l'import depuis `./markdown`, puis :

```ts
describe("markdownSpans", () => {
  test("gras : marques autour, gras dedans", () => {
    expect(markdownSpans("a **b** c")).toEqual([
      { from: 2, to: 4, kind: "mark" },
      { from: 4, to: 5, kind: "bold" },
      { from: 5, to: 7, kind: "mark" },
    ]);
  });

  test("triple étoile : gras puis italique sur la même plage", () => {
    expect(markdownSpans("***x***")).toEqual([
      { from: 0, to: 3, kind: "mark" },
      { from: 3, to: 4, kind: "bold" },
      { from: 3, to: 4, kind: "italic" },
      { from: 4, to: 7, kind: "mark" },
    ]);
  });

  test("italique dans du gras : l'englobante avant l'englobée", () => {
    expect(markdownSpans("**a *b* c**")).toEqual([
      { from: 0, to: 2, kind: "mark" },
      { from: 2, to: 9, kind: "bold" },
      { from: 4, to: 5, kind: "mark" },
      { from: 5, to: 6, kind: "italic" },
      { from: 6, to: 7, kind: "mark" },
      { from: 9, to: 11, kind: "mark" },
    ]);
  });

  test("underscore autour d'un mot : italique", () => {
    expect(markdownSpans("_mot_")).toEqual([
      { from: 0, to: 1, kind: "mark" },
      { from: 1, to: 4, kind: "italic" },
      { from: 4, to: 5, kind: "mark" },
    ]);
  });

  test("lien : texte, adresse et marques", () => {
    expect(markdownSpans("[le site](https://a.b)")).toEqual([
      { from: 0, to: 1, kind: "mark" },
      { from: 1, to: 8, kind: "link" },
      { from: 8, to: 10, kind: "mark" },
      { from: 10, to: 21, kind: "url" },
      { from: 21, to: 22, kind: "mark" },
    ]);
  });

  test("lien en gras : le gras est dans le lien", () => {
    expect(markdownSpans("[**a**](u)")).toEqual([
      { from: 0, to: 1, kind: "mark" },
      { from: 1, to: 6, kind: "link" },
      { from: 1, to: 3, kind: "mark" },
      { from: 3, to: 4, kind: "bold" },
      { from: 4, to: 6, kind: "mark" },
      { from: 6, to: 8, kind: "mark" },
      { from: 8, to: 9, kind: "url" },
      { from: 9, to: 10, kind: "mark" },
    ]);
  });

  test("lien au texte vide : pas de plage vide", () => {
    expect(markdownSpans("[](u)")).toEqual([
      { from: 0, to: 1, kind: "mark" },
      { from: 1, to: 3, kind: "mark" },
      { from: 3, to: 4, kind: "url" },
      { from: 4, to: 5, kind: "mark" },
    ]);
  });

  test("pastille avec couleur : la couleur fait partie de la marque fermante", () => {
    expect(markdownSpans("==上手=={2}")).toEqual([
      { from: 0, to: 2, kind: "mark" },
      { from: 2, to: 4, kind: "highlight", highlight: 2 },
      { from: 4, to: 9, kind: "mark" },
    ]);
  });

  test("pastille sans couleur : la première", () => {
    expect(markdownSpans("==x==")).toEqual([
      { from: 0, to: 2, kind: "mark" },
      { from: 2, to: 3, kind: "highlight", highlight: 1 },
      { from: 3, to: 5, kind: "mark" },
    ]);
  });

  test("emoji avant du gras : décalages en unités UTF-16", () => {
    expect(markdownSpans("😀 **a**")).toEqual([
      { from: 3, to: 5, kind: "mark" },
      { from: 5, to: 6, kind: "bold" },
      { from: 6, to: 8, kind: "mark" },
    ]);
  });

  test("échappement dans du gras : la barre oblique est une marque", () => {
    expect(markdownSpans("**\\*x**")).toEqual([
      { from: 0, to: 2, kind: "mark" },
      { from: 2, to: 5, kind: "bold" },
      { from: 2, to: 3, kind: "mark" },
      { from: 5, to: 7, kind: "mark" },
    ]);
  });

  test("délimiteurs sans fermeture, quadruple étoile, snake_case : aucune plage", () => {
    expect(markdownSpans("a **b et [c](")).toEqual([]);
    expect(markdownSpans("****")).toEqual([]);
    expect(markdownSpans("snake_case_mot")).toEqual([]);
    expect(markdownSpans("")).toEqual([]);
  });
});

describe("listPrefix", () => {
  test("puce : deux caractères", () => {
    expect(listPrefix("- a")).toEqual({ list: "bullet", length: 2 });
  });

  test("numéro à deux chiffres : quatre caractères", () => {
    expect(listPrefix("12. a")).toEqual({ list: "number", length: 4 });
  });

  test("sans espace, tiret collé ou marqueur échappé : pas de liste", () => {
    for (const line of ["1.x", "-x", "\\- x", "1\\. x", ""]) {
      expect(listPrefix(line)).toEqual({ length: 0 });
    }
  });
});
```

Dans `ui/src/deck.test.ts`, dans `describe("textToParagraphs", …)` :

```ts
  test("numéro suivi d'une fin de ligne Windows : reste une liste, comme côté Rust", () => {
    expect(textToParagraphs("1. a\r")).toEqual([{ runs: [{ t: "a\r" }], list: "number" }]);
  });
```

- [ ] **Step 2 : vérifier l'échec**

Run: `cd ui && bun test src/markdown.test.ts src/deck.test.ts`
Expected: FAIL — `markdownSpans` et `listPrefix` ne sont pas exportés ; `"1. a\r"` n'est pas lu comme une liste (l'ancienne regex `(.*)$` refuse `\r`).

- [ ] **Step 3 : implémenter**

Dans `ui/src/markdown.ts` :

1. Remplacer la doc du module (bloc d'en-tête) par :
   ```ts
   /**
    * Markdown inline d'édition → runs : l'inverse de `runs_to_markdown` côté Rust.
    *
    * `**gras**`, `*italique*` ou `_italique_`, `***les deux***`, `[texte](url)`,
    * `==pastille==` ou `==pastille=={n}` (n-ième couleur de la palette du thème) ;
    * `\` échappe une ponctuation. Un délimiteur sans fermeture reste du texte.
    * Le même parcours donne les plages positionnées que l'éditeur met en forme
    * (`markdownSpans`) ; `listPrefix` est la règle unique des préfixes de liste.
    */
   ```
2. Remplacer `import type { Run } from "./api";` par `import type { Paragraph, Run } from "./api";`.
3. Après `interface Format { … }`, ajouter :
   ```ts
   /** Nature d'une plage : délimiteur, mise en forme ou adresse de lien. */
   export type SpanKind = "mark" | "bold" | "italic" | "link" | "url" | "highlight";

   /** Plage d'une ligne de markdown (indices de chaîne), mise en forme par l'éditeur. */
   export interface Span {
     from: number;
     to: number;
     kind: SpanKind;
     highlight?: number;
   }
   ```
4. Remplacer toute la fonction `parse` par (les plages ne sont relevées que si `spans` est donné) :
   ```ts
   function parse(s: string, start: number, end: number, f: Format, out: Run[], spans?: Span[]) {
     let text = "";
     const flush = () => {
       out.push(run(text, f));
       text = "";
     };
     const span = (from: number, to: number, kind: SpanKind, highlight?: number) =>
       spans?.push(highlight === undefined ? { from, to, kind } : { from, to, kind, highlight });
     let i = start;
     while (i < end) {
       const c = s[i];
       if (c === "\\" && i + 1 < end && PUNCTUATION.test(s[i + 1])) {
         span(i, i + 1, "mark");
         text += s[i + 1];
         i += 2;
         continue;
       }
       if (c === "*") {
         const n = repeated(s, i, end, "*");
         const close = n <= 3 ? closing(s, i + n, end, "*", n) : -1;
         if (close > i + n) {
           flush();
           span(i, i + n, "mark");
           if (n >= 2) span(i + n, close, "bold");
           if (n !== 2) span(i + n, close, "italic");
           span(close, close + n, "mark");
           parse(s, i + n, close, { ...f, bold: f.bold || n >= 2, italic: f.italic || n !== 2 }, out, spans);
           i = close + n;
         } else {
           text += s.slice(i, i + n);
           i += n;
         }
         continue;
       }
       if (c === "=" && repeated(s, i, end, "=") === 2) {
         const close = closing(s, i + 2, end, "=", 2);
         if (close > i + 2) {
           const color = /^\{(\d+)\}/.exec(s.slice(close + 2, end));
           const highlight = color ? Number(color[1]) : 1;
           const after = close + 2 + (color?.[0].length ?? 0);
           flush();
           span(i, i + 2, "mark");
           span(i + 2, close, "highlight", highlight);
           span(close, after, "mark");
           parse(s, i + 2, close, { ...f, highlight }, out, spans);
           i = after;
           continue;
         }
       }
       if (c === "_" && !isWord(s[i - 1])) {
         const close = closingUnderscore(s, i + 1, end);
         if (close > i + 1) {
           flush();
           span(i, i + 1, "mark");
           span(i + 1, close, "italic");
           span(close, close + 1, "mark");
           parse(s, i + 1, close, { ...f, italic: true }, out, spans);
           i = close + 1;
           continue;
         }
       }
       if (c === "[") {
         const l = link(s, i, end);
         if (l) {
           flush();
           span(i, i + 1, "mark");
           span(i + 1, l.textEnd, "link");
           span(l.textEnd, l.textEnd + 2, "mark");
           span(l.textEnd + 2, l.end - 1, "url");
           span(l.end - 1, l.end, "mark");
           parse(s, i + 1, l.textEnd, { ...f, link: l.url }, out, spans);
           i = l.end;
           continue;
         }
       }
       text += c;
       i++;
     }
     flush();
   }
   ```
5. Garder `markdownToRuns` tel quel (il appelle `parse` sans `spans`) et ajouter à sa suite :
   ```ts
   /** Plages non vides d'une ligne, l'englobante avant l'englobée. */
   export function markdownSpans(line: string): Span[] {
     const spans: Span[] = [];
     parse(line, 0, line.length, { bold: false, italic: false }, [], spans);
     return spans.filter((s) => s.to > s.from).sort((a, b) => a.from - b.from || b.to - a.to);
   }

   /** Préfixe de liste d'une ligne : `- ` (puce), `1. ` (numéro), sinon rien. */
   export function listPrefix(line: string): { list?: NonNullable<Paragraph["list"]>; length: number } {
     if (line.startsWith("- ")) return { list: "bullet", length: 2 };
     const num = /^\d+\. /.exec(line);
     return num ? { list: "number", length: num[0].length } : { length: 0 };
   }
   ```

Dans `ui/src/deck.ts`, remplacer l'import `import { markdownToRuns, sameFormatting } from "./markdown";` par `import { listPrefix, markdownToRuns, sameFormatting } from "./markdown";`, et le corps de `textToParagraphs` par :

```ts
export function textToParagraphs(text: string, previous: Paragraph[] = []): Paragraph[] {
  return text.split("\n").map((line) => {
    const { list, length } = listPrefix(line);
    const runs = markdownToRuns(line.slice(length));
    const same = previous.find((p) => p.list === list && sameFormatting(p.runs, runs));
    if (same) return same;
    return list ? { runs, list } : { runs };
  });
}
```

- [ ] **Step 4 : vérifier le succès**

Run: `cd ui && bun test && bun run typecheck`
Expected: tous les tests passent (fixture partagée comprise), typecheck sans erreur.

- [ ] **Step 5 : commit et sous-tâche**

```bash
git add ui/src/markdown.ts ui/src/markdown.test.ts ui/src/deck.ts ui/src/deck.test.ts
git commit -m "feat(ui) : plages positionnées et préfixe de liste du parseur markdown (t-126)"
oku cli task done --project slider --id t-136
```

---

### Task 2 : Éditions de texte pures (`textEdits.ts`, sous-tâche `t-137`)

Les éditions de l'éditeur sont des fonctions pures sur (texte, sélection). `toggleEmphasis` et `insertLink` quittent `markdown.ts`, qui ne garde que le parseur ; l'insertion de la dictée, aujourd'hui dans un `useEffect` de `Stage.tsx`, devient `insertDictated` ; `minimalChange` donne le plus petit remplacement, pour que CodeMirror ne réécrive jamais tout le document.

**Files:**
- Create: `ui/src/textEdits.ts`, `ui/src/textEdits.test.ts`
- Modify: `ui/src/markdown.ts` (retirer `Edit`, `toggleEmphasis`, `insertLink`), `ui/src/markdown.test.ts` (retirer leurs `describe` et leur import), `ui/src/Stage.tsx:11` (import)

**Interfaces:**
- Produces :
  ```ts
  export interface Edit { value: string; start: number; end: number }
  export interface Change { from: number; to: number; insert: string }
  export function toggleEmphasis(value: string, start: number, end: number, format: "bold" | "italic"): Edit
  export function insertLink(value: string, start: number, end: number): Edit
  export function insertDictated(value: string, start: number, end: number, text: string): Edit
  export function minimalChange(before: string, after: string): Change
  ```

- [ ] **Step 1 : écrire les tests**

Créer `ui/src/textEdits.test.ts` ci-dessous. Les blocs `describe("toggleEmphasis", …)` et `describe("insertLink", …)` de `ui/src/markdown.test.ts` remplacent le commentaire qui les annonce ; les supprimer de `markdown.test.ts` et retirer `insertLink, toggleEmphasis` de son import `./markdown`.

```ts
import { describe, expect, test } from "bun:test";
import { insertDictated, insertLink, minimalChange, toggleEmphasis } from "./textEdits";

// describe("toggleEmphasis", …) et describe("insertLink", …) de markdown.test.ts, tels quels.

describe("insertDictated", () => {
  test("après une espace et avant un mot : une espace après seulement", () => {
    expect(insertDictated("Bonjour monde", 8, 8, "cher")).toEqual({ value: "Bonjour cher monde", start: 12, end: 12 });
  });

  test("collé à la fin d'un mot : une espace avant", () => {
    expect(insertDictated("Fin", 3, 3, "suite")).toEqual({ value: "Fin suite", start: 9, end: 9 });
  });

  test("après une parenthèse et avant un point : aucune espace", () => {
    expect(insertDictated("(.", 1, 1, "note")).toEqual({ value: "(note.", start: 5, end: 5 });
  });

  test("avant un guillemet fermant : pas d'espace ajoutée (comportement actuel conservé)", () => {
    expect(insertDictated("« »", 2, 2, "mot")).toEqual({ value: "« mot»", start: 5, end: 5 });
  });

  test("document vide : le texte seul", () => {
    expect(insertDictated("", 0, 0, "Bonjour")).toEqual({ value: "Bonjour", start: 7, end: 7 });
  });

  test("sélection non vide : remplacée, curseur après le texte", () => {
    expect(insertDictated("un deux trois", 3, 7, "2")).toEqual({ value: "un 2 trois", start: 4, end: 4 });
  });
});

describe("minimalChange", () => {
  test("mot mis en gras : seul le milieu change", () => {
    expect(minimalChange("a mot b", "a **mot** b")).toEqual({ from: 2, to: 5, insert: "**mot**" });
  });

  test("gras retiré : seul le milieu change", () => {
    expect(minimalChange("a **mot** b", "a mot b")).toEqual({ from: 2, to: 9, insert: "mot" });
  });

  test("textes égaux : remplacement vide en fin de texte", () => {
    expect(minimalChange("abc", "abc")).toEqual({ from: 3, to: 3, insert: "" });
  });

  test("caractère répété ajouté ou retiré : à la fin du préfixe commun", () => {
    expect(minimalChange("aa", "aaa")).toEqual({ from: 2, to: 2, insert: "a" });
    expect(minimalChange("aaa", "aa")).toEqual({ from: 2, to: 3, insert: "" });
  });

  test("tout différent : tout remplacé", () => {
    expect(minimalChange("ab", "cd")).toEqual({ from: 0, to: 2, insert: "cd" });
  });
});
```

- [ ] **Step 2 : vérifier l'échec**

Run: `cd ui && bun test src/textEdits.test.ts`
Expected: FAIL — module `./textEdits` introuvable.

- [ ] **Step 3 : implémenter**

Créer `ui/src/textEdits.ts` ci-dessous. L'interface `Edit` et les fonctions `toggleEmphasis` et `insertLink` de `ui/src/markdown.ts` (commentaires compris, sans modification) remplacent le commentaire qui les annonce, et sont supprimées de `markdown.ts`.

```ts
/**
 * Éditions pures de l'éditeur de texte.
 *
 * Texte et sélection en entrée, nouveau texte et nouvelle sélection en sortie :
 * raccourcis de mise en forme, dictée, et plus petit changement à appliquer.
 */

// Edit, toggleEmphasis, insertLink de markdown.ts, tels quels.

/** Remplacement de `from` à `to` par `insert`. */
export interface Change {
  from: number;
  to: number;
  insert: string;
}

/**
 * Insère un texte dicté à la place de la sélection.
 *
 * Une espace de part et d'autre quand il en faut ; le curseur se pose après.
 */
export function insertDictated(value: string, start: number, end: number, text: string): Edit {
  const before = value.slice(0, start);
  const after = value.slice(end);
  const lead = before && !/[\s(«"']$/.test(before) ? " " : "";
  const trail = after && !/^[\s.,;:!?)»]/.test(after) ? " " : "";
  const caret = (before + lead + text).length;
  return { value: before + lead + text + trail + after, start: caret, end: caret };
}

/**
 * Plus petit remplacement qui change `before` en `after`.
 *
 * Il peut couper une paire de substitution UTF-16 : le texte obtenu reste juste.
 */
export function minimalChange(before: string, after: string): Change {
  const max = Math.min(before.length, after.length);
  let start = 0;
  while (start < max && before[start] === after[start]) start++;
  let end = 0;
  while (end < max - start && before[before.length - 1 - end] === after[after.length - 1 - end]) end++;
  return { from: start, to: before.length - end, insert: after.slice(start, after.length - end) };
}
```

Dans `ui/src/Stage.tsx`, remplacer `import { insertLink, toggleEmphasis } from "./markdown";` par `import { insertLink, toggleEmphasis } from "./textEdits";` (Stage change encore à la tâche 4).

- [ ] **Step 4 : vérifier le succès**

Run: `cd ui && bun test && bun run typecheck`
Expected: tout passe.

- [ ] **Step 5 : commit et sous-tâche**

```bash
git add ui/src/textEdits.ts ui/src/textEdits.test.ts ui/src/markdown.ts ui/src/markdown.test.ts ui/src/Stage.tsx
git commit -m "refactor(ui) : éditions de texte pures dans textEdits, dictée comprise (t-126)"
oku cli task done --project slider --id t-137
```

---

### Task 3 : Couleurs et décorations markdown (sous-tâche `t-138`)

**Files:**
- Modify: `ui/package.json`, `ui/bun.lock` (dépendances), `ui/src/Stage.tsx` (retire `luminance`, l'importe de `./color`)
- Create: `ui/src/color.ts`, `ui/src/color.test.ts`, `ui/src/markdownDecorations.ts`, `ui/src/markdownDecorations.test.ts`

**Interfaces:**
- Consumes : `markdownSpans`, `listPrefix` (tâche 1).
- Produces :
  ```ts
  // color.ts
  export function luminance(hex: string): number | null // luminance relative WCAG de #rgb ou #rrggbb, null sinon
  export const LIGHT_BACKGROUND: number // ((0.6 + 0.055) / 1.055) ** 2.4 ≈ 0.31855
  // markdownDecorations.ts
  export interface LineDecoration { from: number; to: number; className: string; style?: string }
  export function lineDecorations(line: string, palette: string[]): LineDecoration[]
  export function markdownDecorations(palette: string[]): Extension // Extension de @codemirror/state
  ```
  Classes : `cm-md-mark`, `cm-md-url`, `cm-md-bold`, `cm-md-italic`, `cm-md-link`, `cm-md-highlight` (CSS à la tâche 4).

- [ ] **Step 1 : dépendances**

```bash
cd ui && bun add @codemirror/state@^6.7.6 @codemirror/view@^6.43.13 @codemirror/commands@^6.11.1
```

(`@codemirror/commands` installe aussi `@codemirror/language` et `@lezer/*` ; aucune grammaire n'est chargée.)

- [ ] **Step 2 : écrire les tests qui échouent**

Créer `ui/src/color.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { LIGHT_BACKGROUND, luminance } from "./color";

describe("luminance", () => {
  test("blanc : 1, noir : 0", () => {
    expect(luminance("#ffffff")).toBeCloseTo(1, 5);
    expect(luminance("#000000")).toBeCloseTo(0, 5);
  });

  test("couleur courte #rgb : comme sa forme longue", () => {
    expect(luminance("#fff")).toBeCloseTo(1, 5);
    expect(luminance("#abc")).toBeCloseTo(luminance("#aabbcc")!, 10);
  });

  test("texte qui n'est pas une couleur : null", () => {
    expect(luminance("rouge")).toBeNull();
    expect(luminance("#abcd")).toBeNull();
  });
});

describe("LIGHT_BACKGROUND", () => {
  test("gris 60 % (luma de ink-on dans Typst) : exactement le seuil", () => {
    expect(luminance("#999999")).toBeCloseTo(LIGHT_BACKGROUND, 10);
  });
});
```

Créer `ui/src/markdownDecorations.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { lineDecorations } from "./markdownDecorations";

describe("lineDecorations", () => {
  test("préfixe de puce et marques atténués, gras décoré", () => {
    expect(lineDecorations("- **a**", [])).toEqual([
      { from: 0, to: 2, className: "cm-md-mark" },
      { from: 2, to: 4, className: "cm-md-mark" },
      { from: 4, to: 5, className: "cm-md-bold" },
      { from: 5, to: 7, className: "cm-md-mark" },
    ]);
  });

  test("préfixe numéroté : plages décalées, adresse du lien à part", () => {
    expect(lineDecorations("12. [a](u)", [])).toEqual([
      { from: 0, to: 4, className: "cm-md-mark" },
      { from: 4, to: 5, className: "cm-md-mark" },
      { from: 5, to: 6, className: "cm-md-link" },
      { from: 6, to: 8, className: "cm-md-mark" },
      { from: 8, to: 9, className: "cm-md-url" },
      { from: 9, to: 10, className: "cm-md-mark" },
    ]);
  });

  test("pastille sur fond sombre : texte couleur du fond de page", () => {
    expect(lineDecorations("==x=={2}", ["#ffffff", "#2f5d8a"])[1]).toEqual({
      from: 2,
      to: 3,
      className: "cm-md-highlight",
      style: "background-color: #2f5d8a; color: var(--t-bg)",
    });
  });

  test("pastille au-delà de la palette : on reboucle", () => {
    expect(lineDecorations("==x=={3}", ["#f5e6a0", "#2f5d8a"])[1].style).toBe(
      "background-color: #f5e6a0; color: var(--t-ink)",
    );
  });

  test("pastille {0} : la dernière couleur, comme Typst", () => {
    expect(lineDecorations("==x=={0}", ["#f5e6a0", "#2f5d8a"])[1].style).toBe(
      "background-color: #2f5d8a; color: var(--t-bg)",
    );
  });

  test("seuil de ink-on (Typst) : gris 60 % foncé, #00a7cc et #9a9a9a clairs", () => {
    const ink = (c: string) => lineDecorations("==x==", [c])[1].style;
    expect(ink("#999999")).toBe("background-color: #999999; color: var(--t-bg)");
    expect(ink("#9a9a9a")).toBe("background-color: #9a9a9a; color: var(--t-ink)");
    expect(ink("#00a7cc")).toBe("background-color: #00a7cc; color: var(--t-ink)");
  });

  test("couleur courte claire : texte foncé", () => {
    expect(lineDecorations("==x==", ["#fff"])[1].style).toBe("background-color: #fff; color: var(--t-ink)");
  });

  test("palette vide ou valeur qui n'est pas hexadécimale : pastille sans style", () => {
    expect(lineDecorations("==x==", [])[1]).toEqual({ from: 2, to: 3, className: "cm-md-highlight" });
    expect(lineDecorations("==x==", ["red;background:url(//x)"])[1]).toEqual({
      from: 2,
      to: 3,
      className: "cm-md-highlight",
    });
  });

  test("marqueurs de liste échappés : pas de préfixe, la barre oblique est une marque", () => {
    expect(lineDecorations("1\\. pas", [])).toEqual([{ from: 1, to: 2, className: "cm-md-mark" }]);
    expect(lineDecorations("\\- pas", [])).toEqual([{ from: 0, to: 1, className: "cm-md-mark" }]);
  });

  test("numéro sans espace : pas de préfixe", () => {
    expect(lineDecorations("1.x", [])).toEqual([]);
  });
});
```

- [ ] **Step 3 : vérifier l'échec**

Run: `cd ui && bun test src/color.test.ts src/markdownDecorations.test.ts`
Expected: FAIL — modules `./color` et `./markdownDecorations` introuvables.

- [ ] **Step 4 : implémenter**

Créer `ui/src/color.ts`. `luminance` y est déplacée depuis `ui/src/Stage.tsx` (lignes 93-101, à supprimer de Stage) et apprend à lire `#rgb` ; les 6 premiers chiffres d'une forme plus longue sont toujours lus, comme avant, pour `editorBackground` :

```ts
/** Couleurs : luminance, et fond clair ou foncé pour choisir une encre lisible. */

/** Luminance relative (WCAG) d'une couleur #rgb ou #rrggbb, null sinon. */
export function luminance(hex: string): number | null {
  const m = /^#?(?:([0-9a-f]{6})|([0-9a-f]{3})$)/i.exec(hex);
  if (!m) return null;
  const digits = m[1] ?? [...m[2]].map((c) => c + c).join("");
  const n = parseInt(digits, 16);
  const lin = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => lin(v / 255));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/**
 * Seuil de luminance au-dessus duquel un fond est clair (encre foncée).
 *
 * Le même que `ink-on` de Typst (`tokens.typ`) : luma > 60 %, converti en WCAG.
 */
export const LIGHT_BACKGROUND = ((0.6 + 0.055) / 1.055) ** 2.4;
```

Dans `ui/src/Stage.tsx`, ajouter `import { luminance } from "./color";` après l'import de `./appearance`.

Créer `ui/src/markdownDecorations.ts` :

```ts
/**
 * Décorations CodeMirror du markdown d'édition.
 *
 * Les marques restent visibles mais atténuées ; gras, italique, liens et
 * pastilles s'affichent. Les plages viennent de `markdownSpans` et
 * `listPrefix` : l'éditeur suit les règles de la slide.
 */
import type { Extension, Range } from "@codemirror/state";
import type { DecorationSet, EditorView, ViewUpdate } from "@codemirror/view";
import { Decoration, ViewPlugin } from "@codemirror/view";
import { LIGHT_BACKGROUND, luminance } from "./color";
import { listPrefix, markdownSpans } from "./markdown";

/** Décoration d'une plage de ligne : classe CSS, style en ligne pour une pastille. */
export interface LineDecoration {
  from: number;
  to: number;
  className: string;
  style?: string;
}

/** Seules des couleurs hexadécimales entrent dans un attribut `style`. */
const HEX = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i;

/** Fond de la n-ième couleur (modulo positif, comme Typst) et encre lisible dessus. */
function highlightStyle(palette: string[], n: number): string | undefined {
  const len = palette.length;
  if (len === 0) return undefined;
  const bg = palette[(((n - 1) % len) + len) % len];
  if (!HEX.test(bg)) return undefined;
  const l = luminance(bg);
  const ink = l !== null && l > LIGHT_BACKGROUND ? "var(--t-ink)" : "var(--t-bg)";
  return `background-color: ${bg}; color: ${ink}`;
}

/** Décorations d'une ligne : préfixe de liste, puis plages du markdown inline. */
export function lineDecorations(line: string, palette: string[]): LineDecoration[] {
  const { length } = listPrefix(line);
  const out: LineDecoration[] = length > 0 ? [{ from: 0, to: length, className: "cm-md-mark" }] : [];
  for (const s of markdownSpans(line.slice(length))) {
    const from = s.from + length;
    const to = s.to + length;
    if (s.kind !== "highlight") {
      out.push({ from, to, className: `cm-md-${s.kind}` });
      continue;
    }
    const style = highlightStyle(palette, s.highlight ?? 1);
    out.push(style ? { from, to, className: "cm-md-highlight", style } : { from, to, className: "cm-md-highlight" });
  }
  return out;
}

/** Décorations sans style en ligne, une par classe. */
const PLAIN = new Map(
  ["mark", "url", "bold", "italic", "link", "highlight"].map((kind) => {
    const className = `cm-md-${kind}`;
    return [className, Decoration.mark({ class: className })] as const;
  }),
);

function decoration(d: LineDecoration): Decoration {
  const plain = d.style ? undefined : PLAIN.get(d.className);
  return plain ?? Decoration.mark({ class: d.className, attributes: d.style ? { style: d.style } : undefined });
}

/** Décorations de tout le document (un texte de slide est court). */
function build(view: EditorView, palette: string[]): DecorationSet {
  const ranges: Range<Decoration>[] = [];
  const doc = view.state.doc;
  for (let n = 1; n <= doc.lines; n++) {
    const line = doc.line(n);
    for (const d of lineDecorations(line.text, palette)) {
      ranges.push(decoration(d).range(line.from + d.from, line.from + d.to));
    }
  }
  return Decoration.set(ranges, true);
}

/** Extension CodeMirror : markdown mis en forme, pastilles dans les couleurs de `palette`. */
export function markdownDecorations(palette: string[]): Extension {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      constructor(view: EditorView) {
        this.decorations = build(view, palette);
      }
      update(u: ViewUpdate) {
        if (u.docChanged) this.decorations = build(u.view, palette);
      }
    },
    { decorations: (v) => v.decorations },
  );
}
```

- [ ] **Step 5 : vérifier le succès**

Run: `cd ui && bun test && bun run typecheck`
Expected: tout passe.

- [ ] **Step 6 : commit et sous-tâche**

```bash
git add ui/package.json ui/bun.lock ui/src/color.ts ui/src/color.test.ts ui/src/Stage.tsx ui/src/markdownDecorations.ts ui/src/markdownDecorations.test.ts
git commit -m "feat(ui) : décorations CodeMirror du markdown d'édition (t-126)"
oku cli task done --project slider --id t-138
```

---

### Task 4 : `TextEditor` dans la scène (sous-tâche `t-139`)

**Files:**
- Create: `ui/src/TextEditor.tsx`, `ui/src/TextEditor.test.ts`, `ui/e2e/editor.cjs`
- Modify: `ui/src/Stage.tsx` (textarea → `TextEditor`, `commit(text)`, prop `palette`), `ui/src/App.tsx:155,344,543-553`, `ui/src/styles.css:218-227`, `ui/package.json` (React, script `e2e`), `ui/e2e/smoke.cjs` (adresse et sélecteur)

**Interfaces:**
- Consumes : `markdownDecorations(palette)` (tâche 3) ; `Edit`, `toggleEmphasis`, `insertLink`, `insertDictated`, `minimalChange` (tâche 2).
- Produces :
  ```ts
  export interface TextEditorHandle { insertAtCursor(text: string): void }
  export const singleLine: Extension // filtre des champs, exporté pour ses tests
  export default function TextEditor(props: TextEditorProps): React.JSX.Element
  interface TextEditorProps {
    initial: string; field: boolean; palette: string[]; label: string; style: React.CSSProperties;
    onChange: (text: string) => void; onCommit: (text: string) => void; onCancel: () => void;
    ref?: React.Ref<TextEditorHandle>;
  }
  ```
  `Stage` gagne la prop `palette: string[]` ; son `commit` devient `commit(text: string)`.

- [ ] **Step 1 : versions de React**

`useEffectEvent` demande React 19.2 au moins ; `package.json` accepte encore 19.1 :

```bash
cd ui && bun add react@^19.3.0 react-dom@^19.3.0 && bun add -d @types/react@^19.3.0 @types/react-dom@^19.3.0
```

- [ ] **Step 2 : écrire les tests qui échouent**

Créer `ui/src/TextEditor.test.ts` (le filtre des champs, sur un `EditorState`, sans DOM) :

```ts
import { describe, expect, test } from "bun:test";
import { history, undo } from "@codemirror/commands";
import type { Transaction } from "@codemirror/state";
import { EditorState } from "@codemirror/state";
import { singleLine } from "./TextEditor";

describe("singleLine", () => {
  test("collage multiligne dans un champ : les retours à la ligne deviennent des espaces", () => {
    const state = EditorState.create({ doc: "a", extensions: [singleLine] });
    expect(state.update({ changes: { from: 1, insert: "\nb\nc" } }).state.doc.toString()).toBe("a b c");
  });

  test("collage aplati : une seule annulation rend le texte d'avant", () => {
    const start = EditorState.create({ doc: "a", extensions: [history(), singleLine] });
    const pasted = start.update({ changes: { from: 1, insert: "\nb" }, userEvent: "input.paste" }).state;
    let undone: Transaction | undefined;
    undo({ state: pasted, dispatch: (tr) => { undone = tr; } });
    expect(undone?.state.doc.toString()).toBe("a");
  });
});
```

Créer `ui/e2e/editor.cjs` :

```js
// Éditeur de texte : markdown mis en forme pendant la saisie, contre `oku-slides serve fixtures/roadmap.json`.
// Lancer : just e2e (serveur neuf, port 4872 ; ce script passe en premier). À la main : bun e2e/editor.cjs dans ui/,
// E2E_URL = adresse d'un serveur neuf (le script modifie la slide 4).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const slide = (p) => p.evaluate(() =>
  fetch('/api', { method: 'POST', body: JSON.stringify({ cmd: 'slide', id: 's-constat' }) })
    .then((r) => r.json())
    .then((s) => s.slide));
const points = async (p) => (await slide(p)).blocks.find((e) => e.id === 'b-points').text;
const undoLabel = (p) => p.evaluate(() => document.querySelector('[aria-label^="Annuler"]')?.getAttribute('aria-label'));
const text = (p) => p.locator('.editor .cm-content').innerText();
const image = (p) => p.locator('.stage img').getAttribute('src');
/** Attend que `read()` vérifie `ok` (5 s au plus). */
const until = async (read, ok, what) => {
  for (let i = 0; i < 50; i++) {
    const v = await read();
    if (ok(v)) return v;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`délai dépassé : ${what}`);
};
const open = async (p, label) => {
  await p.dblclick(`.target[aria-label="${label}"] >> nth=0`);
  await p.waitForSelector('.editor .cm-editor.cm-focused');
};
const closed = (p) => until(() => p.locator('.editor').count(), (n) => n === 0, 'éditeur fermé');
/** Valide par `act`, puis attend que la slide soit redessinée : l'interface a le nouveau texte. */
const commitWith = async (p, act) => {
  const before = await image(p);
  await act();
  await closed(p);
  await until(() => image(p), (src) => src !== before, 'slide redessinée');
};
/** Collage synthétique : `plain` en text/plain, `html` en text/html. */
const paste = (p, plain, html) => p.evaluate(([plain, html]) => {
  const dt = new DataTransfer();
  if (html) dt.setData('text/html', html);
  dt.setData('text/plain', plain);
  document.querySelector('.editor .cm-content')
    .dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true }));
}, [plain, html]);
const BASE = 'Dites **montrer** et ==ici=={2}';
const RUNS = [{ t: 'Dites ' }, { t: 'montrer', bold: true }, { t: ' et ' }, { t: 'ici', highlight: 2 }];
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  // Slide 4 : le bloc de puces.
  await p.click('.thumb >> nth=3');
  await p.waitForSelector('.target[aria-label="Texte"]');

  // 1. Géométrie héritée, markdown mis en forme pendant la saisie, première validation.
  await open(p, 'Texte');
  const fit = await p.evaluate(() => {
    const host = getComputedStyle(document.querySelector('.editor'));
    const content = getComputedStyle(document.querySelector('.editor .cm-content'));
    const line = getComputedStyle(document.querySelector('.editor .cm-line'));
    return {
      font: host.fontFamily === line.fontFamily,
      lead: host.lineHeight === line.lineHeight,
      pad: line.paddingLeft,
      caret: host.caretColor === content.caretColor,
    };
  });
  assert.deepEqual(fit, { font: true, lead: true, pad: '0px', caret: true });
  await p.keyboard.press('ControlOrMeta+a');
  await p.keyboard.type(BASE);
  assert.equal(await p.locator('.editor .cm-md-bold').innerText(), 'montrer');
  const pill = p.locator('.editor .cm-md-highlight');
  assert.equal(await pill.innerText(), 'ici');
  // {2} : deuxième couleur de la palette papier, #c4622d.
  assert.equal(await pill.evaluate((el) => getComputedStyle(el).backgroundColor), 'rgb(196, 98, 45)');
  assert.equal(await p.locator('.editor .cm-md-mark').count(), 4);
  await commitWith(p, () => p.keyboard.press('ControlOrMeta+Enter'));
  assert.deepEqual(await points(p), [{ runs: RUNS }]);
  const undo1 = await undoLabel(p);
  assert.equal(undo1, 'Annuler : Modifier le texte');

  // 2. Rouvrir redonne le même markdown ; Échap ferme sans rien changer.
  await open(p, 'Texte');
  assert.equal(await text(p), BASE);
  await p.keyboard.type('x');
  await p.keyboard.press('Escape');
  await closed(p);
  await p.waitForTimeout(700); // une validation fautive aurait le temps d'arriver
  assert.deepEqual(await points(p), [{ runs: RUNS }]);

  // 3. Un clic dans la marge de l'éditeur ne le ferme pas.
  await open(p, 'Texte');
  const bb = await p.locator('.editor').boundingBox();
  const [x, y] = [bb.x + 2, bb.y + bb.height - 2];
  // Le point visé est bien hors du texte (sinon le test ne prouverait rien).
  assert.equal(await p.evaluate(([x, y]) => !document.elementFromPoint(x, y)?.closest('.cm-content'), [x, y]), true);
  await p.mouse.click(x, y);
  await p.waitForTimeout(300);
  assert.equal(await p.evaluate(() => document.activeElement?.classList.contains('cm-content')), true);
  assert.equal(await p.locator('.editor').count(), 1);

  // 4. ⌘B est un pas d'annulation à part, même suivi d'une frappe.
  await p.keyboard.press('ControlOrMeta+End');
  await p.keyboard.type(' fin');
  for (let i = 0; i < 3; i++) await p.keyboard.press('Shift+ArrowLeft');
  await p.keyboard.press('ControlOrMeta+b');
  assert.equal(await text(p), `${BASE} **fin**`);
  await p.keyboard.press('ControlOrMeta+End');
  await p.keyboard.type('X');
  await p.keyboard.press('ControlOrMeta+z');
  assert.equal(await text(p), `${BASE} **fin**`);
  await p.keyboard.press('ControlOrMeta+z');
  assert.equal(await text(p), `${BASE} fin`);
  await p.keyboard.press('ControlOrMeta+End');
  for (let i = 0; i < 4; i++) await p.keyboard.press('Backspace');
  assert.equal(await text(p), BASE);

  // 5. Collage : le texte brut seulement, mis en forme comme s'il était tapé.
  await paste(p, ' **collé**', '<i>html</i>');
  assert.equal(await text(p), `${BASE} **collé**`);
  assert.equal(await p.locator('.editor .cm-md-bold').count(), 2);
  await p.keyboard.press('ControlOrMeta+z');
  assert.equal(await text(p), BASE);

  // 6. Entrée dans un bloc : un simple retour à la ligne, même entre crochets.
  await p.keyboard.type('[]');
  await p.keyboard.press('ArrowLeft');
  await p.keyboard.press('Enter');
  assert.equal(await text(p), `${BASE}[\n]`);
  await p.keyboard.press('Backspace');
  await p.keyboard.press('Backspace');
  await p.keyboard.press('Delete');
  assert.equal(await text(p), BASE);
  await p.keyboard.press('Enter');
  await p.keyboard.type('- deux');
  // Les ⌘Z de l'éditeur n'ont pas touché le deck.
  assert.equal(await undoLabel(p), undo1);
  assert.deepEqual(await points(p), [{ runs: RUNS }]);

  // 7. La perte du focus valide, avec le texte affiché ; rouvrir le redonne.
  await commitWith(p, () => p.locator('.editor .cm-content').evaluate((el) => el.blur()));
  assert.deepEqual(await points(p), [{ runs: RUNS }, { runs: [{ t: 'deux' }], list: 'bullet' }]);
  await open(p, 'Texte');
  assert.equal(await text(p), `${BASE}\n- deux`);
  await p.keyboard.press('Escape');
  await closed(p);

  // 8. Champ : texte brut sur une ligne ; collage multiligne aplati ; Maj+Entrée et Entrée valident.
  const title = (await slide(p)).title;
  await open(p, 'Titre');
  await p.keyboard.press('ControlOrMeta+a');
  await paste(p, 'a\nb');
  assert.equal(await text(p), 'a b');
  await p.keyboard.press('ControlOrMeta+z');
  assert.equal(await text(p), title);
  await p.keyboard.press('ControlOrMeta+a');
  await p.keyboard.type('Constat');
  await commitWith(p, () => p.keyboard.press('Shift+Enter'));
  assert.equal((await slide(p)).title, 'Constat');
  await open(p, 'Titre');
  await p.keyboard.type(' **2**');
  assert.equal(await p.locator('.editor .cm-md-bold').count(), 0);
  await commitWith(p, () => p.keyboard.press('Enter'));
  assert.equal((await slide(p)).title, 'Constat **2**');

  assert.deepEqual(errors, []);
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
```

Dans `ui/package.json`, script `e2e` (l'éditeur d'abord, sur le serveur neuf) : `"bun e2e/editor.cjs && bun e2e/toolbar.cjs && bun e2e/panel.cjs"`.

- [ ] **Step 3 : vérifier l'échec**

Run: `cd ui && bun test src/TextEditor.test.ts`, puis (racine) `just e2e`
Expected: FAIL — module `./TextEditor` introuvable ; dans `editor.cjs`, `.editor .cm-editor.cm-focused` n'apparaît pas (la textarea est encore là).

- [ ] **Step 4 : créer `TextEditor`**

Créer `ui/src/TextEditor.tsx` :

```tsx
/**
 * Éditeur de texte CodeMirror posé sur la slide.
 *
 * Bloc : le markdown se met en forme pendant la saisie. Champ : texte brut
 * sur une seule ligne. Le texte validé est celui que montre CodeMirror.
 */
import { useEffectEvent, useImperativeHandle, useLayoutEffect, useRef } from "react";
import { history, historyKeymap, insertNewline, isolateHistory, standardKeymap } from "@codemirror/commands";
import type { ChangeSpec } from "@codemirror/state";
import { EditorState } from "@codemirror/state";
import type { KeyBinding } from "@codemirror/view";
import { EditorView, keymap } from "@codemirror/view";
import { markdownDecorations } from "./markdownDecorations";
import type { Edit } from "./textEdits";
import { insertDictated, insertLink, minimalChange, toggleEmphasis } from "./textEdits";

/** Ce que la scène peut demander à l'éditeur ouvert. */
export interface TextEditorHandle {
  /** Insère un texte dicté à la sélection, avec les espaces qu'il faut. */
  insertAtCursor(text: string): void;
}

interface TextEditorProps {
  /** Texte à l'ouverture (lu une seule fois). */
  initial: string;
  /** Champ (titre…) : texte brut sur une ligne, Entrée valide. Sinon bloc markdown. */
  field: boolean;
  /** Couleurs des pastilles (palette des graphiques du thème), lue à l'ouverture. */
  palette: string[];
  /** Nom accessible de la zone éditable. */
  label: string;
  /** Géométrie posée sur le texte rendu (police, taille, position). */
  style: React.CSSProperties;
  /** Texte après chaque modification. */
  onChange: (text: string) => void;
  /** Validation, avec le texte affiché. */
  onCommit: (text: string) => void;
  /** Abandon (Échap). */
  onCancel: () => void;
  /** Poignée de la scène : insertion de la dictée. */
  ref?: React.Ref<TextEditorHandle>;
}

/**
 * Réglages de CodeMirror : il hérite de la géométrie posée sur l'hôte `.editor`.
 *
 * `&.cm-editor .cm-content` est au moins aussi précis que le thème de base
 * (`&light .cm-content` fixe un curseur noir) : ce thème l'emporte.
 */
const fitHost = EditorView.theme({
  "&": { backgroundColor: "transparent", color: "inherit" },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": { fontFamily: "inherit", lineHeight: "inherit", overflow: "visible" },
  "&.cm-editor .cm-content": { padding: "0", caretColor: "var(--accent)" },
  ".cm-line": { padding: "0", textAlign: "inherit" },
});

/**
 * Champ : un retour à la ligne inséré devient une espace.
 *
 * La transaction d'origine est gardée (historique, annotations) ; le
 * remplacement s'y ajoute, de même longueur.
 */
export const singleLine = EditorState.transactionFilter.of((tr) => {
  let newline = false;
  tr.changes.iterChanges((_fromA, _toA, _fromB, _toB, inserted) => {
    if (inserted.lines > 1) newline = true;
  });
  if (!newline) return tr;
  const text = tr.newDoc.toString();
  const spaces: ChangeSpec[] = [];
  for (let i = text.indexOf("\n"); i >= 0; i = text.indexOf("\n", i + 1)) {
    spaces.push({ from: i, to: i + 1, insert: " " });
  }
  return [tr, { changes: spaces, sequential: true }];
});

/** Applique une édition pure : seul le texte changé, en un pas d'annulation à part. */
function applyEdit(
  view: EditorView,
  edit: (value: string, start: number, end: number) => Edit,
  userEvent: string,
): boolean {
  const { from, to } = view.state.selection.main;
  const before = view.state.doc.toString();
  const e = edit(before, from, to);
  view.dispatch({
    changes: minimalChange(before, e.value),
    selection: { anchor: e.start, head: e.end },
    userEvent,
    annotations: isolateHistory.of("full"),
  });
  return true;
}

/** Ne fait rien : la touche ne tombe pas dans la mise en forme native. */
const nothing = () => true;

/** Éditeur CodeMirror d'une zone de texte ; se remonte pour chaque cible. */
export default function TextEditor({ ref, ...props }: TextEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const change = useEffectEvent((text: string) => props.onChange(text));
  const commit = useEffectEvent((text: string) => props.onCommit(text));
  const cancel = useEffectEvent(() => props.onCancel());

  useLayoutEffect(() => {
    const parent = host.current;
    if (!parent) return;
    const { initial, field, palette, label } = props;
    // Une seule fin par montage ; StrictMode monte deux fois, l'état reste local à l'effet.
    let done = false;
    const finish = (view: EditorView, validate: boolean) => {
      if (done) return true;
      done = true;
      if (validate) commit(view.state.doc.toString());
      else cancel();
      return true;
    };
    const format = (edit: (value: string, start: number, end: number) => Edit) => (view: EditorView) =>
      applyEdit(view, edit, "input.format");
    // CodeMirror ignore déjà l'Entrée qui confirme une conversion IME (y compris
    // celle que WebKit envoie juste après compositionend) : pas de garde ici.
    const keys: KeyBinding[] = [
      { key: "Escape", run: (view) => finish(view, false) },
      { key: "Mod-Enter", run: (view) => finish(view, true) },
      { key: "Mod-u", run: nothing },
      ...(field
        ? [
            { key: "Enter", run: (view: EditorView) => finish(view, true), shift: (view: EditorView) => finish(view, true) },
            { key: "Mod-b", run: nothing },
            { key: "Mod-i", run: nothing },
            { key: "Mod-k", run: nothing },
          ]
        : [
            { key: "Enter", run: insertNewline, shift: insertNewline },
            { key: "Mod-b", run: format((s, a, b) => toggleEmphasis(s, a, b, "bold")) },
            { key: "Mod-i", run: format((s, a, b) => toggleEmphasis(s, a, b, "italic")) },
            { key: "Mod-k", run: format(insertLink) },
          ]),
    ];
    const doc = field ? initial.replaceAll("\n", " ") : initial;
    // Pas de limite de hauteur : l'éditeur grandit avec le texte (overflow visible).
    // Collage : CodeMirror ne lit que text/plain.
    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc,
        selection: { anchor: doc.length },
        extensions: [
          // Nos touches d'abord : ⌘Entrée valide au lieu d'insérer une ligne.
          keymap.of(keys),
          history(),
          keymap.of([...historyKeymap, ...standardKeymap]),
          fitHost,
          EditorView.lineWrapping,
          EditorView.perLineTextDirection.of(true),
          EditorView.contentAttributes.of({
            "aria-label": label,
            "aria-multiline": field ? "false" : "true",
            spellcheck: "true",
            autocorrect: "on",
          }),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) change(u.state.doc.toString());
          }),
          EditorView.domEventHandlers({
            blur: (_event, target) => {
              finish(target, true);
            },
          }),
          field ? singleLine : markdownDecorations(palette),
        ],
      }),
    });
    viewRef.current = view;
    view.focus();
    return () => {
      done = true;
      viewRef.current = null;
      view.destroy();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- monté une fois par cible (la scène donne une `key`)
  }, []);

  useImperativeHandle(ref, () => ({
    insertAtCursor(text: string) {
      const view = viewRef.current;
      if (!view) return;
      applyEdit(view, (s, a, b) => insertDictated(s, a, b, text), "input.dictation");
      view.focus();
    },
  }), []);

  return (
    <div
      ref={host}
      className="editor"
      style={props.style}
      onPointerDown={(e) => e.stopPropagation()}
      onMouseDown={(e) => {
        // Un clic dans la marge (hors du texte) garde le focus : il ne valide pas.
        if (!(e.target as HTMLElement).closest(".cm-content")) {
          e.preventDefault();
          viewRef.current?.focus();
        }
      }}
      onKeyDown={(e) => e.stopPropagation()}
    />
  );
}
```

- [ ] **Step 5 : brancher dans `Stage` et `App`**

Dans `ui/src/Stage.tsx` :

1. Imports : retirer `import { insertLink, toggleEmphasis } from "./textEdits";` ; ajouter
   ```ts
   import type { TextEditorHandle } from "./TextEditor";
   import TextEditor from "./TextEditor";
   ```
2. `Props` : ajouter, et ajouter `palette` à la déstructuration de `Stage({ … })` :
   ```ts
   /** Couleurs des pastilles dans l'éditeur (palette des graphiques du thème). */
   palette: string[];
   ```
3. Remplacer `const area = useRef<HTMLTextAreaElement>(null);` par `const editor = useRef<TextEditorHandle>(null);`.
4. Remplacer l'effet `useEffect(() => { setDraft(editTarget?.text ?? ""); }, [editing, editTarget?.text]);` par :
   ```ts
   // Brouillon à l'ouverture, avant l'affichage : il ne sert qu'à la largeur d'un champ.
   // Il ne suit pas le serveur pendant l'édition : CodeMirror est la source de vérité.
   useLayoutEffect(() => {
     setDraft(editTarget?.text ?? "");
     // eslint-disable-next-line react-hooks/exhaustive-deps
   }, [editing]);
   ```
5. Supprimer le `useLayoutEffect` « L'éditeur grandit avec le texte… ».
6. Remplacer l'effet de la dictée par :
   ```ts
   // La dictée s'insère au curseur de l'éditeur ouvert.
   useEffect(() => {
     if (dictated) editor.current?.insertAtCursor(dictated.text);
     // eslint-disable-next-line react-hooks/exhaustive-deps
   }, [dictated?.seq]);
   ```
7. Remplacer `commit` par :
   ```ts
   const commit = (text: string) => {
     if (!editTarget) return;
     if (text !== (editTarget.text ?? "")) {
       onApply(t("ui-undo-edit-text"), textOps(view, editTarget, text));
     }
     onEdit(null);
   };
   ```
8. Remplacer tout l'élément `<textarea … />` (de `<textarea` jusqu'à `onBlur={commit}` et la fermeture `/>`) par :
   ```tsx
   <TextEditor
     key={editTarget.id}
     ref={editor}
     initial={editTarget.text ?? ""}
     field={editTarget.target === "field"}
     palette={palette}
     label={t("ui-stage-edit-text")}
     style={g.style}
     onChange={setDraft}
     onCommit={commit}
     onCancel={() => onEdit(null)}
   />
   ```

Dans `ui/src/App.tsx` :
- dans `<Stage … />` (ligne ~543), ajouter `palette={snap.appearance.chart_palette}` ;
- lignes 155 et 344, remplacer `"input, textarea, select"` par `'input, textarea, select, [contenteditable="true"]'` (défense en profondeur : ces raccourcis s'arrêtent déjà pendant l'édition).

- [ ] **Step 6 : styles et scripts e2e**

Dans `ui/src/styles.css` :
- dans la règle `.editor { … }`, retirer `resize: none;` ;
- supprimer la ligne `.editor:focus { outline: none; }` ;
- remplacer la ligne `.editor::selection { … }` par :
  ```css
  .editor ::selection { background: color-mix(in oklab, var(--accent) 28%, transparent); }
  /* Markdown dans l'éditeur (plages de markdownDecorations ; la pastille porte son style en ligne). */
  .editor .cm-md-mark, .editor .cm-md-url { opacity: 0.4; }
  .editor .cm-md-bold { font-weight: bolder; }
  .editor .cm-md-italic { font-style: italic; }
  .editor .cm-md-link { color: var(--t-accent); text-decoration: underline; }
  ```

Dans `ui/e2e/smoke.cjs` :
- ligne 1, remplacer `(port 4870)` par `(E2E_URL, port 4870 par défaut)` ;
- remplacer `await p.goto('http://127.0.0.1:4870/');` par `await p.goto(process.env.E2E_URL ?? 'http://127.0.0.1:4870/');` ;
- remplacer `'textarea.editor'` par `'.editor .cm-content'`.
(`smoke.cjs` n'a pas d'assertions : script manuel, lancé à la tâche 5.)

- [ ] **Step 7 : vérifier le succès**

Run: `cd ui && bun test && bun run typecheck`, puis (racine) `just e2e`
Expected: tests et typecheck verts ; `editor.cjs`, `toolbar.cjs` et `panel.cjs` passent (code de sortie 0).

- [ ] **Step 8 : commit et sous-tâche**

```bash
git add ui/src/TextEditor.tsx ui/src/TextEditor.test.ts ui/src/Stage.tsx ui/src/App.tsx ui/src/styles.css ui/e2e/editor.cjs ui/e2e/smoke.cjs ui/package.json ui/bun.lock
git commit -m "feat(ui) : éditeur CodeMirror, markdown mis en forme pendant la saisie (t-126)"
oku cli task done --project slider --id t-139
```

---

### Task 5 : Vérification finale (sous-tâche `t-140`)

**Files:** aucun, sauf correction : tout défaut trouvé ici commence par un test qui le reproduit (e2e ou unitaire), puis le correctif.

`<scratch>` ci-dessous désigne le chemin **absolu** du scratchpad de la session.

- [ ] **Step 1 : suite complète**

Run (racine): `just test`, `cargo fmt --all --check`, `cd ui && bun run typecheck`
Expected: nextest et Bun verts, format et types sans erreur. (`just check` passe `clippy -D warnings`, qui échoue déjà sur `main` à cause de l'avertissement préexistant `large size difference between variants` d'`oku-slides-core/src/model.rs` : hors de cette tâche.)

- [ ] **Step 2 : navigateur, build de production (Chromium)**

Copier `fixtures/roadmap.json` en `<scratch>/md-demo.json` (ne jamais servir la fixture elle-même), `cd ui && bun run build`, puis ajouter au tableau `configurations` de `.claude/launch.json` (port 4873 : 4872 sert à `just e2e`) :

```json
{
  "name": "oku-slides-markdown-demo",
  "runtimeExecutable": "cargo",
  "runtimeArgs": ["run", "-q", "-p", "oku-slides-cli", "--", "--fonts", "assets/fonts", "serve", "<scratch>/md-demo.json", "--port", "4873"],
  "port": 4873
}
```

La lancer avec `preview_start`, puis `E2E_URL=http://127.0.0.1:4873/ bun ui/e2e/smoke.cjs` : la dernière ligne affichée doit être `erreurs []` (le script va sur la slide 7 et édite un titre). Sur la slide 4, ouvrir le bloc de puces et vérifier, captures à l'appui :
- l'éditeur est posé exactement sur le texte rendu (même police, même ligne de base), curseur de la couleur d'accent ;
- `Dites **montrer**.` : « montrer » en gras, `**` atténués ;
- `*mot*` en italique, `[lien](https://oku.dev)` dans l'accent du thème et souligné, `==ici=={2}` sur fond orange papier ;
- après ⌘Entrée, le rendu de la slide correspond.
Arrêter le serveur.

- [ ] **Step 3 : mode développement (StrictMode actif)**

Copier `fixtures/roadmap.json` en `<scratch>/md-dev.json` et ajouter à `.claude/launch.json` :

```json
{
  "name": "oku-slides-markdown-dev",
  "runtimeExecutable": "just",
  "runtimeArgs": ["web", "<scratch>/md-dev.json"],
  "port": 5173
}
```

La lancer avec `preview_start` (Vite sur 5173, `/api` vers 4870). Sur la slide 4 : ouvrir le bloc, Échap (l'éditeur se ferme), rouvrir, taper un mot, ⌘Entrée (l'éditeur se ferme). Vérifier : le texte est validé une seule fois, et un seul clic sur « Annuler » de la barre d'outils rend le texte d'origine. Arrêter le serveur, puis retirer les deux configurations ajoutées à `.claude/launch.json`.

- [ ] **Step 4 : app de bureau (WebKit)**

WKWebView ne se pilote pas depuis cette session : demander à l'utilisateur de lancer `just dev` et de vérifier, puis rapporter ses réponses telles quelles :
- dans un titre, taper « にほんご » en japonais, convertir avec Espace, confirmer avec Entrée : la conversion est confirmée, l'éditeur reste ouvert ; une seconde Entrée valide ;
- même chose dans un bloc de puces : Entrée confirme la conversion sans insérer de ligne ;
- dictée système macOS dans un bloc : le texte s'insère au curseur ;
- menu Édition > Annuler pendant l'édition, après une frappe puis juste après ⌘B : annule dans l'éditeur (noter si l'élément de menu est grisé), jamais dans le deck.
Tout échec devient un test (si reproductible en e2e) puis un correctif, ou une tâche de backlog si non reproductible hors WebKit.

- [ ] **Step 5 : clore**

```bash
oku cli task done --project slider --id t-140
oku cli task done --project slider --id t-126
```
