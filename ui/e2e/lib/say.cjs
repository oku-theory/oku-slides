// Speaking to hands-free listening in e2e: a line appended to the fake utterance file.
// `just e2e` builds the server with the e2e fakes and names the file in E2E_UTTERANCES; the
// server hears each complete line appended while it listens (`fake_source.rs`).
const assert = require('node:assert/strict');
const { appendFileSync } = require('node:fs');
const { logOffset, logSince } = require('./agentLog.cjs');
const { until } = require('./until.cjs');

/** The steady indicator of the page while listening hands-free (pressed: listening). */
const LISTENING = '.device.dictation .mic.hands-free[aria-pressed="true"]';

/** The text the fake agent was prompted with in `message` (`session/prompt`), else null. */
const promptText = (message) => {
  if (message.method !== 'session/prompt') return null;
  const blocks = (message.params?.prompt ?? []).filter((b) => b.type === 'text');
  return blocks.length > 0 ? blocks[blocks.length - 1].text : null;
};

/** The texts prompted among `messages`, in order. */
const promptsIn = (messages) => messages.map(promptText).filter((t) => t !== null);

/** A wait for a prompt that is exactly `text`. */
const prompted = (text) => (messages) => promptsIn(messages).includes(text);

/**
 * Says `text`: waits until the page shows the listening state, appends the line, then waits
 * until `heard` holds over the fake agent's messages since then (by default: `text` prompted).
 * Returns those messages. Fails with `why` after `timeout` ms.
 */
const say = async (p, text, { heard = prompted(text), why = `"${text}" was not heard`, timeout = 20_000 } = {}) => {
  const file = process.env.E2E_UTTERANCES;
  const log = process.env.E2E_AGENT_LOG;
  assert.ok(file && log, 'E2E_UTTERANCES ou E2E_AGENT_LOG manquant : lancer par just e2e');
  await until(p, async () => (await p.locator(LISTENING).count()) > 0, "the page is not listening hands-free", timeout);
  const from = logOffset(log);
  appendFileSync(file, `${text}\n`);
  let since = [];
  await until(p, async () => heard((since = logSince(log, from))), why, timeout);
  return since;
};

module.exports = { LISTENING, promptText, promptsIn, prompted, say };
