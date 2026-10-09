// Attente par sondage côté Node, pour les conditions asynchrones (fetch, API).
// `page.waitForFunction` n'attend pas la Promise d'un prédicat async : elle est vraie d'emblée.
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

module.exports = { until, stable };
