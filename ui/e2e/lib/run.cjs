// Lance les specs e2e : chaque `e2e/NNN-nom.cjs`, un par un, dans l'ordre de leur numéro.
// Les specs partagent un serveur (`just e2e`) : le numéro fixe l'ordre dont elles dépendent.
// Les aides (`http.cjs`, `stage.cjs`, `until.cjs`) et ce lanceur vivent dans `e2e/lib/` ;
// tout autre fichier de `e2e/` est une spec et doit suivre le motif.
const { spawnSync } = require('node:child_process');
const { readdirSync } = require('node:fs');
const path = require('node:path');

const SPEC = /^(\d{3})-[a-z0-9-]+\.cjs$/;

/** Ordonne les fichiers de `e2e/` par numéro ; refuse un nom hors motif, un doublon, aucune spec. */
const orderSpecs = (files) => {
  const bad = files.filter((n) => !SPEC.test(n));
  if (bad.length > 0) {
    throw new Error(`e2e : attendu NNN-nom.cjs (minuscules, chiffres, tirets) : ${bad.join(', ')}`);
  }
  if (files.length === 0) throw new Error('e2e : aucune spec dans e2e/');
  const sorted = [...files].sort();
  const seen = new Map();
  for (const n of sorted) {
    const num = n.slice(0, 3);
    if (seen.has(num)) throw new Error(`e2e : numéro ${num} en double : ${seen.get(num)}, ${n}`);
    seen.set(num, n);
  }
  return sorted;
};

if (require.main === module) {
  const dir = path.join(__dirname, '..');
  const files = readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isFile() && e.name !== '.DS_Store')
    .map((e) => e.name);
  for (const spec of orderSpecs(files)) {
    console.log(`▶ e2e/${spec}`);
    const run = spawnSync(process.execPath, [path.join(dir, spec)], { stdio: 'inherit' });
    if (run.status !== 0) {
      const why = run.error?.message ?? (run.signal ? `signal ${run.signal}` : `code ${run.status}`);
      console.error(`✗ e2e/${spec} (${why})`);
      process.exit(run.status || 1);
    }
  }
}

module.exports = { orderSpecs };
