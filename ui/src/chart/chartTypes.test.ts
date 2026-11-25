// The data cell is defined once: the catalog's zod schemas and the op types read chartTypes.ts.

import { expect, test } from "bun:test";

/** A definition of the cell: a zod union or a TS union of the four cell types. */
const DEFINITION = /z\.union\(\[\s*z\.number\(\),\s*z\.string\(\),\s*z\.boolean\(\),\s*z\.null\(\)\s*\]\)|=\s*number \| string \| boolean \| null\b/g;

test("catalog_and_ops_schemas_define_cell_once", async () => {
  const root = `${import.meta.dir}/..`;
  const found: string[] = [];
  for await (const file of new Bun.Glob("**/*.{ts,tsx}").scan(root)) {
    if (/\.test\.tsx?$/.test(file)) continue;
    const text = await Bun.file(`${root}/${file}`).text();
    for (const _ of text.matchAll(DEFINITION)) found.push(file);
  }
  expect(found).toEqual(["chart/chartTypes.ts"]);
});
