// Automatic fallback to the local agent (t-1024): the selected agent's program is missing, so a
// prompt runs on the built-in local agent (fake engine, its own script), its receipt shows in the
// thread, the turn says why, and the thread header shows the fallback badge with the reason.
// Its own server (`lib/ownServer.cjs`, fresh data folder): the shared one of `just e2e` keeps a
// usable fake agent selected for the other specs.
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { freePort, startServer } = require('./lib/ownServer.cjs');

/** The selected agent's program: absent, so the agent is unusable before any launch. */
const MISSING = '/nonexistent/oku-slides-e2e-agent';
const TITLE = 'Fait en local';

/** `config.toml`: the missing agent selected, the local agent on the fake engine (debug builds). */
const prepare = (dir) => {
  const script = path.join(dir, 'local-script.json');
  fs.writeFileSync(script, JSON.stringify({
    turns: [{ route: 'edit', replies: [{ call: 'report', args: { title: TITLE, facts: ['Rien changé'] } }] }],
  }));
  const config = [
    '[agent]', 'selected = "absent"', '',
    '[agent.presets.absent]', 'name = "Absent"', `command = "${MISSING}"`, '',
    '[local_agent]', 'engine = "fake"', `fake_script = "${script}"`, `fake_log = "${path.join(dir, 'local-log.jsonl')}"`, '',
  ].join('\n');
  fs.writeFileSync(path.join(dir, 'data', 'config.toml'), config);
};

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  const server = await startServer(p, await freePort(), prepare);
  try {
    await p.goto(server.url);
    await p.waitForSelector('.stage img');
    await post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text: 'renomme le deck', ui: {} });

    // The local agent's receipt closes the turn.
    const turn = p.locator('.agent-turn', { hasText: 'renomme le deck' });
    await turn.locator('.a2ui-receipt .a2ui-title', { hasText: TITLE }).waitFor({ timeout: 30_000 });
    // The notice names the selected agent's reason (its missing program).
    await turn.getByText(MISSING).first().waitFor();
    // The header badge, with the reason in its title.
    const badge = p.locator('.agent-thread-head .agent-fallback');
    await badge.waitFor();
    assert.ok((await badge.getAttribute('title'))?.includes(MISSING), 'le badge du repli ne dit pas pourquoi');
    // The selected agent stays the saved one.
    const saved = fs.readFileSync(path.join(server.dir, 'data', 'config.toml'), 'utf8');
    assert.match(saved, /selected = "absent"/);
    assert.ok(fs.existsSync(path.join(server.dir, 'local-log.jsonl')), 'le moteur local n\'a rien noté');
    assert.deepEqual(errors, []);
  } finally {
    server.stop();
    await b.close();
  }
  console.log('repli local : ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
