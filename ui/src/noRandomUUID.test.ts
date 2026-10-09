// Garde (t-695) : `crypto.randomUUID` lève hors contexte sûr ; l'UI tire ses identifiants par `randomId`.

import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const path = join(dir, e.name);
    if (e.isDirectory()) return sources(path);
    return /\.tsx?$/.test(e.name) && !/\.test\.tsx?$/.test(e.name) ? [path] : [];
  });
}

test("ui/src n'appelle randomUUID nulle part hors des tests", () => {
  const offenders = sources(import.meta.dir).filter((f) => readFileSync(f, "utf8").includes("randomUUID"));
  expect(offenders).toEqual([]);
});
