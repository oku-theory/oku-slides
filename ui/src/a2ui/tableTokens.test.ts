// Les tableaux du dock ne lisent que des jetons définis : un jeton absent
// laisse le navigateur prendre un repli (ou rien), quel que soit le thème.

import { describe, expect, test } from "bun:test";

const read = (path: string) => Bun.file(new URL(path, import.meta.url)).text();

/** Noms des propriétés personnalisées déclarées (`--x:`) dans `css`. */
const declared = (css: string) => new Set([...css.matchAll(/(--[\w-]+)\s*:/g)].map((m) => m[1]));
/** Noms des propriétés personnalisées lues (`var(--x`) dans `css`. */
const used = (css: string) => [...new Set([...css.matchAll(/var\(\s*(--[\w-]+)/g)].map((m) => m[1]))];

describe("jetons des tableaux", () => {
  const sources = ["../styles.css", "./a2ui.css"];
  for (const sheet of ["./ListTable.css"]) {
    test(`${sheet} : chaque var() lit un jeton défini par l'app`, async () => {
      const defined = new Set<string>();
      for (const s of [...sources, sheet]) for (const name of declared(await read(s))) defined.add(name);
      expect(used(await read(sheet)).filter((name) => !defined.has(name))).toEqual([]);
    });
  }
});
