// Hands-free listening, routing only (the real voice detection is tested in Rust): the user turns
// it on in the box, then each line said (`lib/say.cjs`, the e2e fake utterance source) reaches the
// fake agent as a prompt; "stop" cancels a running turn and prompts nothing; after a reload,
// listening is still on (host state) and the next utterance is prompted.
// Runs against `oku-slides serve` started by `just e2e` (fake agent "Faux", E2E_AGENT_LOG,
// E2E_UTTERANCES). The paused-at-launch rule is a Rust test: this server is never relaunched.
// Turning hands-free on the first time turns the read-aloud on too: the spec checks it, then turns
// the read-aloud off at once (410 reads aloud on its own), and hands-free off again, as it found it.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { handsFree, readAloud, status } = require('./lib/voiceBox.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const { LISTENING, prompted, promptsIn, say } = require('./lib/say.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;

/** The turn of `text` was prompted and has ended. */
const answered = (text) => (since) => prompted(text)(since) && since.some((m) => m.method === 'turn/end');

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
  let failed = null;
  try {
    // 1. Off at first: push-to-talk; the user turns hands-free on in the box, listening starts.
    assert.equal((await status(p)).state, 'idle');
    await handsFree(p, true);
    await p.waitForSelector(LISTENING);
    assert.equal(await p.locator('.device.dictation [role="status"]').textContent(), 'À l\'écoute, mains libres');
    // The first hands-free turned the read-aloud on; off again, for this spec and the next ones.
    assert.equal((await status(p)).config.read_aloud, true);
    await readAloud(p, false);

    // 2. An utterance is a prompt of the active tab.
    await say(p, 'ajoute une diapositive titre', { heard: answered('ajoute une diapositive titre') });

    // 3. "stop" during a turn the fake agent holds until cancelled: cancelled, nothing prompted;
    // the sentinel that follows is the only prompt after it.
    await say(p, 'attends mon signal');
    const fromStop = logOffset(LOG);
    await say(p, 'stop', { heard: (since) => since.some((m) => m.method === 'session/cancel'), why: 'stop should cancel the turn' });
    await say(p, 'bonjour tout le monde', { heard: answered('bonjour tout le monde') });
    assert.deepEqual(promptsIn(logSince(LOG, fromStop)), ['bonjour tout le monde']);

    // 4. A reload: listening is host state, the indicator shows it and the next line is prompted.
    await p.reload();
    await p.waitForSelector('.stage img');
    await p.waitForSelector(LISTENING);
    await say(p, 'après le rechargement', { heard: answered('après le rechargement') });
  } catch (e) {
    failed = e;
  }
  // The server is shared: hands-free off again, listening stopped.
  try {
    if ((await status(p)).config.read_aloud) await readAloud(p, false);
    if ((await status(p)).config.hands_free) await handsFree(p, false);
    await until(p, async () => (await status(p)).state === 'idle', 'listening should stop with the switch');
  } catch (e) {
    if (failed) console.error('restore:', e);
    else failed = e;
  }
  if (failed) throw failed;
  assert.deepEqual(errors, []);
  console.log('hands-free: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
