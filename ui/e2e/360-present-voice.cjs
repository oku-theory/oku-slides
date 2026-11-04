// Voice and presentation (t-998). Steps 1-2: Escape in a presentation. The listener phases of
// the window are real here (happy-dom does not reproduce them): while hands-free listening,
// Escape exits the presentation and listening goes on; with no dictation either, Escape exits.
// Push-to-talk needs a microphone, so its Escape is a render test. Steps 3-5: hands-free voice
// in a window presentation: a nav word moves at once with no agent, other speech is dropped and
// only the wake phrase reaches the agent, "quitter" leaves.
// Runs against `oku-slides serve` started by `just e2e` (E2E_UTTERANCES, fake agent). The spec
// turns hands-free off again, as it found it.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { handsFree, status } = require('./lib/voiceBox.cjs');
const { LISTENING, prompted, promptsIn, say } = require('./lib/say.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

const PRESENTER = '.presenter';

/** The presented slide's alt text: « Slide <index> sur <count> ». */
const shownSlide = (p) => p.locator(`${PRESENTER} img`).getAttribute('alt');

/** Waits for nothing: the utterance is checked later, after the sentinel. */
const nothing = () => true;

/** The turn of `text` was prompted and has ended. */
const answered = (text) => (since) => prompted(text)(since) && since.some((m) => m.method === 'turn/end');

/** Presents in the window (Cmd/Ctrl+Shift+Enter, no real full screen), shown and reported to the host. */
async function present(p) {
  const reported = p.waitForResponse(
    (r) => r.url().includes('/api/agent') && (r.request().postData() ?? '').includes('"cmd":"presenting"'),
  );
  await p.keyboard.press('ControlOrMeta+Shift+Enter');
  await p.waitForSelector(PRESENTER);
  await reported;
}

(async () => {
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
    // 1. Escape with no dictation exits the presentation.
    await present(p);
    await p.keyboard.press('Escape');
    await p.waitForSelector(PRESENTER, { state: 'detached' });

    // 2. Escape while hands-free listening exits it too; listening goes on.
    await handsFree(p, true);
    await p.waitForSelector(LISTENING);
    await present(p);
    await p.keyboard.press('Escape');
    await p.waitForSelector(PRESENTER, { state: 'detached' });
    await p.waitForSelector(LISTENING);
    assert.equal((await status(p)).state, 'listening');

    // 3. "suivant" in a window presentation shows slide 2, with no agent.
    await present(p);
    await p.keyboard.press('Home');
    await until(p, async () => /^Slide 1 sur /.test(await shownSlide(p)), 'the presentation should start on slide 1');
    const fromNav = logOffset(process.env.E2E_AGENT_LOG);
    await say(p, 'suivant', { heard: nothing });
    await until(p, async () => /^Slide 2 sur /.test(await shownSlide(p)), '« suivant » should show slide 2');

    // 4. Speech without the wake phrase is dropped; the sentinel with it is the only prompt.
    await say(p, 'bonjour à tous', { heard: nothing });
    await say(p, 'Oku, quelle heure est-il', { heard: answered('quelle heure est-il') });
    assert.deepEqual(promptsIn(logSince(process.env.E2E_AGENT_LOG, fromNav)), ['quelle heure est-il']);

    // 5. "quitter" leaves the presentation.
    await say(p, 'quitter', { heard: nothing });
    await p.waitForSelector(PRESENTER, { state: 'detached' });
  } catch (e) {
    failed = e;
  }
  try {
    if ((await status(p)).config.hands_free) await handsFree(p, false);
    await until(p, async () => (await status(p)).state === 'idle', 'listening should stop with the switch');
  } catch (e) {
    if (failed) console.error('restore:', e);
    else failed = e;
  }
  if (failed) throw failed;
  assert.deepEqual(errors, []);
  console.log('present-voice: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
