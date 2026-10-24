// Copier l'image posée par 260 (`e-image` de `s-libre`, onglet t-1) dans une page, puis la coller
// dans une seconde page ouverte ensuite, sur un nouvel onglet : l'hôte garde l'élément copié pour
// toutes les fenêtres (t-1182), la seconde le relit en s'ouvrant, et le collage apporte ses octets
// (t-1178). `serve` n'a qu'une fenêtre : deux pages de même label jouent les deux fenêtres ; ni la
// diffusion Tauri (`clip`) ni le tri par fenêtre ne sont couverts ici (voir `clipSync`, testé à part). L'hôte garde ensuite cette
// copie : une spec suivante s'ouvre avec elle. Contre `oku-slides serve` lancé par `just e2e`, après 260.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post, activeTab } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { clipboard, show } = require('./lib/clipboard.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const ID = 'sha256:c414cd0e204de974f73753c7e28d7638e7b3691bb8b1a2bab6b25bb7fed7ce77';

/** Une page de l'app, ses erreurs relevées dans `errors`, une fois sa lecture de l'élément copié faite. */
const open = async (b, errors) => {
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  const read = p.waitForResponse((r) => r.url().endsWith('/api/workspace') && r.request().postData()?.includes('"cmd":"clip"'));
  await p.goto(URL);
  await read;
  await p.waitForSelector('.stage img');
  return p;
};

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const errors = [];
  const first = await open(b, errors);
  await post(first, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  // 270 a déjà copié cette image : un leurre d'abord, pour que l'attente ci-dessous prouve notre copie.
  const decoy = { tab: 't-0', text: 'leurre', element: { id: 'e-leurre', kind: 'base/text' }, origin: 'w-0' };
  await post(first, '/api/workspace', { cmd: 'set_clip', clip: decoy });
  await first.reload();
  await first.waitForSelector('.stage img');

  // Première fenêtre : l'image de t-1, sélectionnée puis copiée ; l'hôte la garde.
  await show(first, 't-1', 's-libre');
  await first.locator('.target[aria-label="Image"]').click();
  await first.waitForSelector('.target.selected');
  const text = await clipboard(first, 'copy');
  const kept = async () => (await post(first, '/api/workspace', { cmd: 'clip' })).clip;
  await until(first, async () => (await kept())?.element.id === 'e-image', "l'hôte doit garder l'élément copié");

  // Seconde fenêtre, ouverte après la copie, sur un nouvel onglet : elle relit l'élément copié.
  const label = (await post(first, '/api/workspace', { cmd: 'view' })).windows[0].label;
  await post(first, '/api/workspace', { cmd: 'new_tab', window: label });
  const second = await open(b, errors);
  const tab = await activeTab(second);
  assert.notEqual(tab, 't-1');
  await show(second, tab, 's-1');
  await clipboard(second, 'paste', text);

  // L'image arrive sur s-1 avec le même id : ses octets sont dans le bundle du nouvel onglet.
  const slide = () => post(second, '/api', { tab, req: { cmd: 'slide', id: 's-1' } });
  await until(second, async () => JSON.stringify(await slide()).includes(ID), "l'image doit être collée sur s-1", 15000);
  assert.equal(await second.locator('.toast.error').count(), 0, 'aucun refus affiché');

  await post(second, '/api/workspace', { cmd: 'close', tab, discard: true });
  assert.deepEqual(errors, []);
  console.log('coller-autre-fenetre : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
