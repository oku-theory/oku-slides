// Read-aloud (t-1002): with the fake voice, each speech is a line of E2E_SPEECH. The user turns the
// read-aloud on in the box; a hands-free prompt whose turn shows a receipt (`report`) has its title
// and facts read. Presenting, a wake-phrase turn's receipt stays silent (in_presentation off): once
// the presentation is left, the sentinel receipt is the only speech since it began. Last, the
// agent's set_speech changes the rate (a scripted turn), then sets it back.
// Runs against `oku-slides serve` started by `just e2e` (fake agent, fake voice, E2E_UTTERANCES,
// E2E_SPEECH). It waits on events, never timers: a speech suspends listening, so each next line is
// said once the dictation reports it is no longer suspended. It turns the read-aloud and
// hands-free off again and the rate back to 1, as it found them.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { existsSync, readFileSync } = require('node:fs');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { handsFree, readAloud, status } = require('./lib/voiceBox.cjs');
const { LISTENING, prompted, say } = require('./lib/say.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const SPEECH = process.env.E2E_SPEECH;

const PRESENTER = '.presenter';
const nothing = () => true;

/** The lines the fake voice has said so far. */
const spoken = () => (existsSync(SPEECH) ? readFileSync(SPEECH, 'utf8').split('\n').filter((l) => l !== '') : []);

/** The turn of `text` was prompted and has ended. */
const answered = (text) => (since) => prompted(text)(since) && since.some((m) => m.method === 'turn/end');

/** Says `text`, whose scripted turn calls set_speech, and waits for the saved `rate`. */
async function rateBy(p, text, rate) {
  await say(p, text, { heard: answered(text) });
  await until(p, async () => (await status(p)).config.speech.rate === rate, `the rate should be ${rate}`);
}


/** Waits until the read-aloud no longer holds the microphone: the next line is heard. */
const unsuspended = (p) => until(p, async () => !(await status(p)).suspended, 'listening should resume after the speech');

/** Waits until the fake voice has said `line`. */
const said = (p, line) => until(p, async () => spoken().includes(line), `« ${line} » should be read aloud`);

/** Presents in the window (Cmd/Ctrl+Shift+Enter), shown and reported to the host. */
async function present(p) {
  const reported = p.waitForResponse(
    (r) => r.url().includes('/api/agent') && (r.request().postData() ?? '').includes('"cmd":"presenting"'),
  );
  await p.keyboard.press('ControlOrMeta+Shift+Enter');
  await p.waitForSelector(PRESENTER);
  await reported;
}

(async () => {
  assert.ok(SPEECH, 'E2E_SPEECH missing: run through just e2e');
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
    // 1. Read-aloud on, then hands-free: a scripted receipt's title and fact are read.
    await readAloud(p, true);
    await handsFree(p, true);
    await p.waitForSelector(LISTENING);
    await say(p, 'lis le reçu', { heard: answered('lis le reçu') });
    await said(p, 'Reçu lu à voix haute. Diapo deux changée');
    await unsuspended(p);

    // 2. Presenting, the wake-phrase turn's receipt is not read; the sentinel after it is the only speech.
    const before = spoken().length;
    await present(p);
    await say(p, 'Oku, lis le reçu présenté', { heard: answered('lis le reçu présenté') });
    await say(p, 'quitter', { heard: nothing });
    await p.waitForSelector(PRESENTER, { state: 'detached' });
    await unsuspended(p);
    await say(p, 'lis le reçu témoin', { heard: answered('lis le reçu témoin') });
    await said(p, 'Reçu témoin. Après la présentation');
    assert.deepEqual(spoken().slice(before), ['Reçu témoin. Après la présentation']);
    await unsuspended(p);

    // 3. The agent's set_speech, through the MCP server: the rate is saved, then set back.
    await rateBy(p, 'lis plus vite', 1.5);
    await rateBy(p, 'lis normalement', 1);
  } catch (e) {
    failed = e;
  }
  // The server is shared: read-aloud and hands-free off again, listening stopped.
  try {
    if ((await status(p)).config.speech.rate !== 1) {
      if (!(await status(p)).config.hands_free) await handsFree(p, true);
      await rateBy(p, 'lis normalement', 1);
    }
    if ((await status(p)).config.read_aloud) await readAloud(p, false);
    if ((await status(p)).config.hands_free) await handsFree(p, false);
    await until(p, async () => (await status(p)).state === 'idle', 'listening should stop with the switch');
  } catch (e) {
    if (failed) console.error('restore:', e);
    else failed = e;
  }
  if (failed) throw failed;
  assert.deepEqual(errors, []);
  console.log('read-aloud: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
