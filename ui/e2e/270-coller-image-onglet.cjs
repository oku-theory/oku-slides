// Copier l'image posée par 260 (`e-image` de `s-libre`, onglet t-1), puis la coller dans un nouvel
// onglet : ses octets passent d'un bundle à l'autre (t-1178), et l'image arrive sur sa slide `s-1`.
// Contre `oku-slides serve` lancé par `just e2e`, après 260-coller-image.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post, activeTab } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const ID = 'sha256:c414cd0e204de974f73753c7e28d7638e7b3691bb8b1a2bab6b25bb7fed7ce77';

/** Montre la slide `id` de l'onglet actif (sa miniature), image chargée. */
const show = async (p, tab, id) => {
  const n = (await post(p, '/api', { tab, req: { cmd: 'snapshot' } })).slides.findIndex((s) => s.id === id) + 1;
  assert.ok(n > 0, `slide ${id} absente de ${tab}`);
  await p.click(`.thumb >> nth=${n - 1}`);
  await p.waitForFunction((n) => {
    const img = document.querySelector('.stage img');
    return img?.alt.replace(/[⁨⁩]/g, '') === `Slide ${n}` && img.complete && img.naturalWidth > 0;
  }, n);
};

/** Envoie `kind` (copy ou paste) à la page, avec `text` au presse-papiers ; le texte qu'il y laisse. */
const clipboard = (p, kind, text = '') =>
  p.evaluate(([kind, text]) => {
    const dt = new DataTransfer();
    if (text) dt.setData('text/plain', text);
    document.body.dispatchEvent(new ClipboardEvent(kind, { clipboardData: dt, bubbles: true, cancelable: true }));
    return dt.getData('text/plain');
  }, [kind, text]);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');

  // Dans t-1 : l'image collée par 260, sélectionnée puis copiée.
  await show(p, 't-1', 's-libre');
  await p.locator('.target[aria-label="Image"]').click();
  await p.waitForSelector('.target.selected');
  const text = await clipboard(p, 'copy');
  assert.ok(text, "la copie doit laisser un texte au presse-papiers");

  // Un nouvel onglet, sans recharger la page : le presse-papiers de la fenêtre reste.
  await p.getByRole('button', { name: 'Nouvel onglet' }).click();
  await until(p, async () => (await activeTab(p)) !== 't-1', 'le nouvel onglet doit devenir actif');
  const tab = await activeTab(p);
  await p.waitForSelector('.stage img');
  await show(p, tab, 's-1');
  await clipboard(p, 'paste', text);

  // L'image arrive sur s-1 avec le même id : ses octets sont dans le bundle du nouvel onglet.
  const slide = () => post(p, '/api', { tab, req: { cmd: 'slide', id: 's-1' } });
  await until(p, async () => JSON.stringify(await slide()).includes(ID), "l'image doit être collée sur s-1", 15000);
  assert.equal(await p.locator('.toast.error').count(), 0, 'aucun refus affiché');

  await post(p, '/api/workspace', { cmd: 'close', tab, discard: true });
  assert.deepEqual(errors, []);
  console.log('coller-image-onglet : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
