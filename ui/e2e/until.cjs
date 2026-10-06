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

module.exports = { until };
