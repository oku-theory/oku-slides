// A deck file written by another installation (330-guides): a second `oku-slides serve` on its own
// data folder pins a pack it made, with a use guide, and saves a copy. Opened here, the pack is received.
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const { copyFileSync, mkdtempSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const net = require('node:net');
const path = require('node:path');
const { until, untilValue } = require('./until.cjs');

const ROOT = path.resolve(__dirname, '..', '..', '..');
/** The `oku-slides` binary `just e2e` built (its target dir, as the recipe reads it). */
const BIN = path.resolve(ROOT, process.env.CARGO_TARGET_DIR ?? 'target', 'debug', 'oku-slides');

/** A port nobody listens on. */
const freePort = () => new Promise((resolve, reject) => {
  const probe = net.createServer();
  probe.once('error', reject);
  probe.listen(0, '127.0.0.1', () => {
    const { port } = probe.address();
    probe.close(() => resolve(port));
  });
});

/**
 * Starts a server on `port` with fresh data and deck folders, then returns `{ url, dir, stop }`.
 * `dir` holds the folders; `stop` kills the server and removes them.
 */
async function startForeign(p, port) {
  const dir = mkdtempSync(path.join(tmpdir(), 'oku-slides-e2e-foreign-'));
  const env = { ...process.env, OKU_SLIDES_PATHS__DATA: path.join(dir, 'data'), OKU_SLIDES_PATHS__DECKS: path.join(dir, 'decks') };
  delete env.OKU_SLIDES_AGENT__SELECTED;
  const server = spawn(BIN, ['--fonts', path.join(ROOT, 'assets/fonts'), 'serve', path.join(ROOT, 'fixtures/roadmap.json'), '--ui', path.join(ROOT, 'ui/dist'), '--port', String(port)], { env, stdio: 'ignore' });
  const url = `http://127.0.0.1:${port}/`;
  const stop = () => {
    server.kill();
    rmSync(dir, { recursive: true, force: true });
  };
  try {
    await until(p, () => fetch(url).then((r) => r.ok, () => false), `${url} : le second serveur ne répond pas`, 30_000);
  } catch (e) {
    stop();
    throw e;
  }
  return { url, dir, stop };
}

/**
 * Writes `<foreign dir>/recu.oku-slides.pdf`: a deck pinning a pack named `name` whose use guide is
 * `guide`, made on a data folder that this app has never seen. Returns the file's path.
 */
async function writeDeckBringingGuide(browser, name, guide) {
  const port = await freePort();
  const page = await browser.newPage();
  const foreign = await startForeign(page, port);
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
      { cmd: 'set_guide', which: 'use', text: guide },
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

module.exports = { writeDeckBringingGuide };
