// A deck file written by another installation (330-guides, 350-received-packs): a second `oku-slides serve`
// on its own data folder pins a pack it made, with an optional use guide, and saves a copy. Opened here,
// the pack is received.
const assert = require('node:assert/strict');
const { copyFileSync, mkdtempSync } = require('node:fs');
const { tmpdir } = require('node:os');
const path = require('node:path');
const { untilValue } = require('./until.cjs');
const { freePort, startServer } = require('./ownServer.cjs');

/**
 * Writes `<foreign dir>/recu.oku-slides.pdf`: a deck pinning a pack named `name` whose use guide is
 * `guide` (none when empty), made on a data folder that this app has never seen. Returns the file's path.
 */
async function writeDeckBringingPack(browser, name, guide = '') {
  const port = await freePort();
  const page = await browser.newPage();
  const foreign = await startServer(page, port);
  try {
    const call = (endpoint, body) =>
      fetch(foreign.url + endpoint, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }).then((r) => r.json());
    const api = (tab, req) => call('api', { tab, req });
    const base = (await api('t-1', { cmd: 'packs', kind: 'widgets' })).find((x) => x.namespace === 'base');
    assert.ok(base, 'le second serveur n\'a pas le pack base');
    const opened = await call('api/workspace', { cmd: 'open_pack', tab: 't-1', pack: base.id, widget: null });
    const tab = opened.windows?.flatMap((w) => w.tabs).find((t) => t.document.kind === 'pack' && t.document.pack === base.id)?.id;
    assert.ok(tab, `open_pack refusé : ${JSON.stringify(opened).slice(0, 300)}`);
    for (const req of [
      { cmd: 'apply', label: 'Nom', ops: [{ op: 'set_deck_param', key: 'title', value: name }], merge: null },
      ...(guide ? [{ cmd: 'set_guide', which: 'use', text: guide }] : []),
      { cmd: 'save_pack', as_new: true },
    ]) {
      const reply = await api(tab, req);
      assert.equal(reply.error, undefined, `${req.cmd} refusé : ${reply.error}`);
    }
    await call('api/workspace', { cmd: 'close', tab, discard: true });
    // Pin the new pack from "Packs du deck", as a user would.
    await page.goto(foreign.url);
    await page.getByRole('button', { name: /^Packs du deck/ }).click();
    const add = page.locator('[data-surface-id="host-packs"] [data-component-id="widgets-add"] select');
    await untilValue(page, () => add.locator('option').allTextContents(), (l) => l.includes(name), `« ${name} » absent de la bibliothèque du second serveur`);
    await add.selectOption({ label: name });
    await page.locator('[data-surface-id="host-packs"]').getByText(name, { exact: true }).first().waitFor();
    const file = path.join(foreign.dir, 'recu.oku-slides.pdf');
    const saved = await api('t-1', { cmd: 'save_copy', path: file });
    assert.equal(saved.error, undefined, `save_copy refusé : ${saved.error}`);
    // Copy it out of the folder `stop` removes.
    const kept = path.join(mkdtempSync(path.join(tmpdir(), 'oku-slides-e2e-recu-')), 'recu.oku-slides.pdf');
    copyFileSync(file, kept);
    return kept;
  } finally {
    foreign.stop();
    await page.close();
  }
}

module.exports = { writeDeckBringingPack };
