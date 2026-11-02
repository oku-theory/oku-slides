// A second `oku-slides serve` of the spec's own, on fresh data and deck folders: for what the
// shared server of `just e2e` cannot be set up for without breaking later specs.
const { spawn } = require('node:child_process');
const { mkdirSync, mkdtempSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const net = require('node:net');
const path = require('node:path');
const { until } = require('./until.cjs');

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
 * `prepare(dir)` runs first: it may write `<dir>/data/config.toml`. `dir` holds the folders;
 * `stop` kills the server and removes them.
 */
async function startServer(p, port, prepare = () => {}) {
  const dir = mkdtempSync(path.join(tmpdir(), 'oku-slides-e2e-own-'));
  mkdirSync(path.join(dir, 'data'));
  prepare(dir);
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

module.exports = { ROOT, freePort, startServer };
