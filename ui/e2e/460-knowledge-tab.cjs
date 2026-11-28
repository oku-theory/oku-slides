// Attach a PDF in a deck's chat, let the fake agent import it into the knowledge base and open
// the knowledge tab, read the extracted file there, then close the deck tab: its knowledge tab
// goes with it, and nothing asks. Against `oku-slides serve` started by `just e2e` (fake agent
// log: E2E_AGENT_LOG). The PDF is built in memory (lib/pdf.cjs), never a committed binary.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { activeTab, post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, logSince, waitForTool } = require('./lib/agentLog.cjs');
const { assertNoQuestion } = require('./lib/question.cjs');
const { textPdf } = require('./lib/pdf.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const MARKER = 'Knowledge e2e marker';
const PDF = textPdf(MARKER);
const ID = `sha256:${createHash('sha256').update(PDF).digest('hex')}`;
// The Rust extraction test pins the same id: it writes the same bytes and reads the marker back.
const PINNED = 'sha256:a67ffa35c1576965fc27e699bfa36e41f7ec6b6fcf31c7e585ac2fe0a23105de';

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG missing: run through just e2e');
  assert.equal(ID, PINNED, 'lib/pdf.cjs and the Rust test write the same PDF');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  const asked = [];
  p.on('dialog', (d) => {
    asked.push(d.message());
    d.dismiss();
  });
  await p.goto(URL);
  await p.waitForSelector('.stage img');

  // A deck of its own: a new tab, closed at the end, so the earlier specs' tabs stay as they are.
  const before = await p.locator('.tab[role="tab"]').count();
  await p.locator('.tab-new').click();
  await until(p, async () => (await p.locator('.tab[role="tab"]').count()) === before + 1, 'the new deck tab opens');
  await p.waitForSelector('.stage img');
  const deck = await activeTab(p);
  const from = logOffset(LOG);

  // Attach the PDF through the draft's picker, then send the instruction naming it.
  await p.keyboard.press('ControlOrMeta+k');
  await p.locator('.draft-text').fill(`import knowledge ${ID}`);
  const [chooser] = await Promise.all([p.waitForEvent('filechooser'), p.locator('.draft-pick').click()]);
  await chooser.setFiles({ name: 'notes.pdf', mimeType: 'application/pdf', buffer: PDF });
  await p.locator('.draft-document').waitFor();
  await p.locator('.draft-send').click();

  // The fake agent imports the document, then opens the knowledge tab, in that order.
  const imported = await waitForTool(p, from, 'knowledge_import');
  const opened = await waitForTool(p, from, 'open_knowledge');
  const results = logSince(LOG, from).filter((m) => m.method === 'mcp/result').map((m) => m.params.tool);
  assert.ok(results.indexOf('knowledge_import') < results.indexOf('open_knowledge'), `order: ${results}`);
  assert.deepEqual(JSON.parse(imported.content[0].text).paths, ['/references/notes.md']);
  assert.equal(JSON.parse(opened.content[0].text).shown, true, 'the tab is shown');

  // The knowledge tab, right after its deck tab, is the active one.
  const view = await post(p, '/api/workspace', { cmd: 'view' });
  const tabs = view.windows[0].tabs;
  const at = tabs.findIndex((x) => x.id === deck);
  const knowledge = tabs[at + 1];
  assert.deepEqual(knowledge?.document, { kind: 'knowledge', deck_tab: deck });
  assert.equal(view.windows[0].active, knowledge.id);
  const marker = p.locator('.tab[role="tab"][data-kind="knowledge"]');
  await marker.waitFor();
  assert.equal(await marker.getAttribute('aria-selected'), 'true');

  // The tree holds notes.md under references; its row opens it.
  const tree = await post(p, '/api', { tab: deck, req: { cmd: 'knowledge_tree' } });
  const file = tree.files.find((f) => f.path === '/references/notes.md');
  assert.ok(file, `/references/notes.md in the tree: ${JSON.stringify(tree.files.map((f) => f.path))}`);
  const dir = p.locator('.knowledge-tree-dir', { has: p.locator(':scope > .knowledge-tree-name', { hasText: /^references$/ }) });
  const row = dir.locator('.knowledge-tree-row', { hasText: 'notes.pdf' });
  await row.click();
  await until(p, async () => (await row.getAttribute('aria-current')) === 'page', 'the opened file is current');

  // Its header says Reference, and its body holds the PDF's text.
  const pane = p.locator('.knowledge-pane');
  await pane.locator('.knowledge-fields dd').first().waitFor();
  assert.equal(await pane.locator('.knowledge-fields dd').first().innerText(), 'Reference');
  await until(p, async () => (await pane.innerText()).includes(MARKER), 'the body shows the extracted text');

  // Closing the deck tab closes its knowledge tab too, without a question.
  await p.locator('.tab[role="tab"]').nth(at).locator('.tab-close').click();
  await until(p, async () => (await p.locator('.tab[role="tab"]').count()) === before, 'both tabs close');
  assert.equal(await marker.count(), 0);
  const after = await post(p, '/api/workspace', { cmd: 'view' });
  const left = after.windows.flatMap((w) => w.tabs).map((x) => x.id);
  assert.ok(!left.includes(deck) && !left.includes(knowledge.id), `tabs left: ${left}`);
  assert.deepEqual(asked, []);
  await assertNoQuestion(p);

  assert.deepEqual(errors, []);
  console.log('knowledge-tab: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
