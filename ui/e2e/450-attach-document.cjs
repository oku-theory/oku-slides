// Attach a document in the deck chat with the draft's file picker: its chip shows, and the
// instruction reaches the fake agent with an "Attached documents" block naming it.
// Against `oku-slides serve` started by `just e2e` (fake agent log: E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const NOTES = Buffer.from('# Notes\n\nLe chiffre clé : 42.\n');
const ID = `sha256:${createHash('sha256').update(NOTES).digest('hex')}`;

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG missing: run through just e2e');
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
  await p.locator('.draft-text').fill('lis le document joint');
  // The picker button opens the file chooser of the draft's hidden input.
  const [chooser] = await Promise.all([p.waitForEvent('filechooser'), p.locator('.draft-pick').click()]);
  assert.ok(chooser.isMultiple(), 'the picker takes several files');
  await chooser.setFiles({ name: 'notes.md', mimeType: 'text/markdown', buffer: NOTES });
  const chip = p.locator('.draft-document');
  await chip.waitFor();
  assert.equal((await chip.locator('.draft-name').innerText()).replace(/[⁨⁩]/g, ''), 'notes.md');
  assert.match(await chip.locator('.draft-size').innerText(), /^1 (KB|Ko)$/);

  await p.locator('.draft-send').click();
  await p.waitForSelector('.draft-text', { state: 'detached' });
  const prompts = () => logSince(LOG, from).filter((m) => m.method === 'session/prompt');
  await until(p, async () => prompts().length > 0, 'the instruction must reach the fake agent', 15000);
  const blocks = prompts()[0].params.prompt;
  const listed = blocks.find((x) => x.type === 'text' && x.text.startsWith('Attached documents'));
  assert.ok(listed, '"Attached documents" block');
  assert.ok(listed.text.includes(ID) && listed.text.includes('notes.md'), 'the block names the document');
  assert.equal(blocks.at(-1).text, 'lis le document joint');
  // The document is not shown in the sent turn until lot 5's receipts: the bubble has no chip.
  const turn = p.locator('.a2ui-prompt-turn', { hasText: 'lis le document joint' }).first();
  await turn.waitFor();
  assert.equal(await turn.locator('.a2ui-prompt-image').count(), 0);

  assert.deepEqual(errors, []);
  console.log('attach-document: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
