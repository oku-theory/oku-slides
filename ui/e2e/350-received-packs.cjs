// Received packs wait for the user (t-1414): a deck written by another installation brings a new pack.
// Opening it shows the received dialog; « Plus tard » leaves the library without the pack while the deck
// still renders; « Ajouter à ma bibliothèque » in "Packs du deck" then adds it. A screenshot goes to
// E2E_SHOTS (default .superpowers/sdd/agent-sandbox).
// Runs against `oku-slides serve` started by `just e2e`.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const path = require('node:path');
const { post } = require('./lib/http.cjs');
const { closePackTabs } = require('./lib/atelier.cjs');
const { deleteLibraryPack } = require('./lib/packs.cjs');
const { writeDeckBringingPack } = require('./lib/foreignDeck.cjs');
const { until } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = 't-1';
const NAME = 'Apport';
const PANEL = '.agent-dock [data-surface-id="host-packs"]';
const SHOTS = process.env.E2E_SHOTS ?? path.resolve(__dirname, '..', '..', '.superpowers', 'sdd', 'agent-sandbox');

const api = (p, tab, req) => post(p, '/api', { tab, req });
const snapshot = (p, tab) => api(p, tab, { cmd: 'snapshot' });
const inLibrary = async (p) => (await api(p, TAB, { cmd: 'packs', kind: 'widgets' })).find((x) => x.name === NAME);

/** Opens `file` in a new tab; returns its id with the page reloaded on it. */
async function openFile(p, file) {
  const v = await post(p, '/api/workspace', { cmd: 'open_tab', window: 'w-1', path: file });
  assert.equal(v.error, undefined, `open_tab refusé : ${v.error}`);
  const win = (await post(p, '/api/workspace', { cmd: 'view' })).windows[0];
  await p.reload();
  return win.active;
}

(async () => {
  mkdirSync(SHOTS, { recursive: true });
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');

  const file = await writeDeckBringingPack(b, NAME);
  let received = null;
  try {
    // 1. Opened, the deck lists the new pack and the dialog shows its manifest and the offline guarantee.
    received = await openFile(p, file);
    assert.notEqual(received, TAB, 'le deck reçu n\'ouvre pas d\'onglet');
    const [review, ...more] = (await snapshot(p, received)).received_to_review;
    assert.deepEqual(more, [], 'le deck reçu liste plus d\'une version');
    assert.equal(review?.capabilities.name, NAME, `le deck reçu ne liste pas ${NAME} : ${JSON.stringify(review)}`);
    assert.deepEqual([review.relation, review.install, review.blocker], ['new_pack', true, null], 'le pack reçu n\'est pas un nouveau pack à installer');
    assert.equal(await inLibrary(p), undefined, `${NAME} est entré dans la bibliothèque sans validation`);
    const ask = p.getByRole('alertdialog', { name: 'Ce deck apporte des packs ou des thèmes' });
    await ask.waitFor();
    await p.waitForSelector('.received-dialog[data-armed]');
    await ask.getByRole('heading', { name: NAME, exact: true }).waitFor();
    await ask.getByText('\u2068Widgets\u2069 · \u2068Nouveau pack\u2069', { exact: true }).waitFor();
    await ask.getByText('Les packs et les thèmes ne peuvent lire ni vos fichiers ni le réseau.', { exact: true }).waitFor();
    await ask.getByRole('button', { name: 'Ajouter à ma bibliothèque' }).waitFor();
    await p.screenshot({ path: path.join(SHOTS, 'e2e-received-dialog.png') });

    // 2. « Plus tard »: no dialog, the library still lacks the pack, the deck renders from its own copy.
    await ask.getByRole('button', { name: 'Plus tard' }).click();
    await ask.waitFor({ state: 'detached' });
    assert.deepEqual((await snapshot(p, received)).received_to_review, [], 'le deck demande encore après « Plus tard »');
    assert.equal(await inLibrary(p), undefined, `${NAME} est entré dans la bibliothèque après « Plus tard »`);
    await p.waitForSelector('.stage img');

    // 3. "Packs du deck" offers to add it; the button adds it to the library.
    await p.getByRole('button', { name: /^Packs du deck/ }).click();
    const add = p.locator(`${PANEL} [data-component-id="widgets/${review.capabilities.namespace}/add-to-library"]`);
    await add.getByRole('button', { name: 'Ajouter à ma bibliothèque' }).click();
    await until(p, async () => (await inLibrary(p)) !== undefined, `${NAME} absent de la bibliothèque après « Ajouter à ma bibliothèque »`);
    await add.waitFor({ state: 'detached' });
  } finally {
    if (received !== null) await post(p, '/api/workspace', { cmd: 'close', tab: received, discard: true });
    const left = await inLibrary(p);
    if (left) await deleteLibraryPack(p, TAB, left.id);
  }
  const v = await post(p, '/api/workspace', { cmd: 'activate', tab: TAB });
  assert.equal(v.error, undefined, `activation de ${TAB} refusée : ${v.error}`);
  await p.reload();
  assert.deepEqual(errors, []);
  console.log('received packs ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
