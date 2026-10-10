// Agent : un fil par perspective. Les slides et l'atelier gardent chacun le leur.
// Contre `oku-slides serve` lancé avec le faux agent (just e2e s'en charge).
// Précondition : t-1 sans tour en cours ni question en attente (agent.cjs répond aux siennes).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

const turn = (p, text) => p.locator('.agent-turn', { hasText: text });
/** Vide la conversation de la perspective affichée (les fils sont gardés d'un e2e à l'autre). */
const newConversation = async (p) => {
  const button = p.locator('.agent-new');
  if (await button.count()) await button.click();
  await p.waitForSelector('.agent-thread.vacant');
};
/** Le fil d'une perspective se rappelle de façon asynchrone (resync) : `action` faite, on attend que
 * l'hôte ait répondu au rappel de la portée `kind` (promesse créée avant l'action). */
const resynced = async (p, kind, action) => {
  const isResync = (rq) => {
    if (!rq.url().endsWith('/api/agent')) return false;
    const q = rq.postDataJSON();
    return q?.cmd === 'resync' && q.scope?.kind === kind;
  };
  // Seules comptent les requêtes parties après ce point : un rappel déjà en vol ne satisfait pas l'attente.
  const started = new Set();
  const onRequest = (rq) => { if (isResync(rq)) started.add(rq); };
  p.on('request', onRequest);
  const done = p.waitForResponse((r) => started.has(r.request()));
  try {
    await action();
    await done;
  } finally {
    p.off('request', onRequest);
  }
  // La réponse reçue, le fil se peint à l'image suivante.
  await p.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
};
const openAtelier = (p) => resynced(p, 'atelier', async () => {
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Atelier' }).click();
});
const closeAtelier = (p) => resynced(p, 'slides', () => p.getByRole('button', { name: 'Terminer' }).click());

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  try {
    await p.goto(URL);
    await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
    await resynced(p, 'slides', async () => {
      await p.reload();
      await p.waitForSelector('.stage img');
    });
    await newConversation(p);

    // 1. Slides : un tour.
    await post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text: 'portée slides', ui: {} });
    await turn(p, 'portée slides').waitFor();

    // 2. Atelier : son fil à lui (vidé d'un e2e précédent), puis un tour envoyé par le brouillon.
    await openAtelier(p);
    await newConversation(p);
    assert.equal(await turn(p, 'portée slides').count(), 0);
    await p.keyboard.press('ControlOrMeta+k');
    await p.locator('.draft-text').fill('portée atelier');
    await p.locator('.draft-send').click();
    await turn(p, 'portée atelier').waitFor();
    assert.equal(await p.locator('.agent-turn').count(), 1);

    // 3. Retour aux slides, sans rien vider : seul leur fil.
    await closeAtelier(p);
    await turn(p, 'portée slides').waitFor();
    assert.equal(await turn(p, 'portée atelier').count(), 0);
    assert.equal(await p.locator('.agent-turn').count(), 1);

    // 4. Rangement : les deux conversations repartent de zéro pour les e2e suivants.
    await newConversation(p);
    await openAtelier(p);
    await newConversation(p);
    await closeAtelier(p);

    assert.deepEqual(errors, []);
    console.log('agent-scopes : ok');
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
