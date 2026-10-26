// Attente par sondage côté Node, pour les conditions asynchrones (fetch, API).
// `page.waitForFunction` n'attend pas la Promise d'un prédicat async : elle est vraie d'emblée.
// Les seuls délais fixes des specs hors attentes négatives : le pas de sondage ci-dessous (100 ms ; réglable pour `settled`),
// et la reprise de lecture d'un aperçu de widget (`widget.cjs`, 50 ms).
const assert = require('node:assert/strict');

/** Rejoue `fn` jusqu'à une valeur vraie ; échoue avec `why` après `timeout` ms. */
const until = async (p, fn, why, timeout = 8000) => {
  const t0 = Date.now();
  while (Date.now() - t0 < timeout) {
    if (await fn()) return;
    await p.waitForTimeout(100);
  }
  assert.fail(why);
};

/**
 * Attend que la boîte (`getBoundingClientRect`) de chaque sélecteur soit identique sur deux
 * `requestAnimationFrame` consécutifs : la mise en page a fini de suivre l'effet (redimensionnement…).
 * Échoue après 120 images sans stabilité.
 */
const stable = (p, selectors) => p.evaluate((sels) => new Promise((resolve, reject) => {
  const read = () => JSON.stringify(sels.map((s) => document.querySelector(s)?.getBoundingClientRect() ?? null));
  let previous = read();
  let frames = 0;
  const tick = () => requestAnimationFrame(() => {
    const now = read();
    if (now === previous) return resolve();
    if (++frames > 120) return reject(new Error(`mise en page instable : ${sels}`));
    previous = now;
    tick();
  });
  tick();
}), selectors);

/** Relit `read` jusqu'à ce que `ok(valeur)` ; rend la valeur, échoue avec `why` (et le début de la dernière lecture) après `timeout` ms. */
const untilValue = async (p, read, ok, why, timeout = 8000) => {
  const t0 = Date.now();
  let v = await read();
  while (!ok(v)) {
    if (Date.now() - t0 >= timeout) assert.fail(`${why} (dernière lecture : ${String(JSON.stringify(v)).slice(0, 200)})`);
    await p.waitForTimeout(100);
    v = await read();
  }
  return v;
};

/**
 * Relit `read` toutes les `every` ms jusqu'à deux lectures consécutives identiques (sérialisées en JSON) :
 * un chargement asynchrone a fini de changer la valeur. Rend la valeur ; échoue avec `why` après `timeout` ms.
 * `every` borne l'écart toléré entre deux changements : il doit dépasser celui du chargement attendu.
 */
const settled = async (p, read, why, timeout = 8000, every = 100) => {
  const t0 = Date.now();
  let last = JSON.stringify(await read());
  for (;;) {
    if (Date.now() - t0 >= timeout) assert.fail(why);
    await p.waitForTimeout(every);
    const v = await read();
    const now = JSON.stringify(v);
    if (now === last) return v;
    last = now;
  }
};

/** A long wait (an import and its first render, an agent turn with the witness): far more than the default 8 s. */
const SLOW = 180_000;

module.exports = { until, stable, untilValue, settled, SLOW };
