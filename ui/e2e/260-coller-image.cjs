// Coller une image dans le brouillon de l'agent : la puce apparaît, la consigne part avec la
// liste des images et un bloc image, et le faux agent la pose sur `s-libre` (élément `e-image`).
// Contre `oku-slides serve` lancé par `just e2e` (journal du faux agent : E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post, activeTab } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const PNG = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
const ID = 'sha256:c414cd0e204de974f73753c7e28d7638e7b3691bb8b1a2bab6b25bb7fed7ce77';

/** Colle le PNG dans le brouillon ouvert, comme le presse-papier d'une capture d'écran. */
const pastePng = (p) =>
  p.locator('.draft').evaluate((el, b64) => {
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const dt = new DataTransfer();
    dt.items.add(new File([bytes], 'capture.png', { type: 'image/png' }));
    el.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true }));
  }, PNG);

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');
  const from = logOffset(LOG);

  await p.keyboard.press('ControlOrMeta+k');
  await p.locator('.draft-text').fill("pose l'image collée");
  await pastePng(p);
  await p.locator('.draft-image').waitFor();
  assert.match(await p.locator('.draft-image').innerText(), /PNG 1×1/);
  assert.equal(await p.locator('.draft-image img').count(), 1, 'miniature locale');

  await p.locator('.draft-send').click();
  await p.waitForSelector('.draft-text', { state: 'detached' });
  // Le journal du faux agent : une ligne `{ method, params }` par message reçu.
  const prompts = () => logSince(LOG, from).filter((m) => m.method === 'session/prompt');
  await until(p, async () => prompts().length > 0, 'la consigne doit atteindre le faux agent', 15000);
  const blocks = prompts()[0].params.prompt;
  assert.ok(blocks.some((x) => x.type === 'text' && x.text.includes(ID)), 'bloc « Pasted images »');
  assert.ok(blocks.some((x) => x.type === 'image' && x.mimeType === 'image/png' && x.data === PNG), 'bloc image');
  assert.equal(blocks.at(-1).text, "pose l'image collée");

  // Le faux agent pose l'image : elle est dans le deck, et la bulle du fil garde sa puce.
  const tab = await activeTab(p);
  const slide = () => post(p, '/api', { tab, req: { cmd: 'slide', id: 's-libre' } });
  await until(p, async () => JSON.stringify(await slide()).includes(ID), "l'image doit être posée sur s-libre", 15000);
  await p.locator('.a2ui-prompt-image').first().waitFor();

  assert.deepEqual(errors, []);
  console.log('coller-image : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
