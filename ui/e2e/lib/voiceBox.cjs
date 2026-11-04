// The dictation settings box (host-voice) in e2e specs: its switches and the dictation status.
const { post } = require('./http.cjs');
const { until } = require('./until.cjs');
const { waitSurfaceAction } = require('./surfaceAction.cjs');

/** The settings box, shown as a popover. */
const BOX = '.agent-popover [data-surface-id="host-voice"]';
/** The switch of box row `id`. */
const switchOf = (id) => `${BOX} [data-component-id="${id}"] input[role="switch"]`;
/** The gear of the dictation bar, which opens the box. */
const gear = (p) => p.locator('.device.dictation button[aria-label="Réglages"]');
/** The dictation status, as the host reports it. */
const status = (p) => post(p, '/api/stt', { cmd: 'status' });

/** Opens the box, sets switch `id` (action `action`) to `on`, waits for `done`, closes the box. */
async function setSwitch(p, id, action, on, done) {
  await gear(p).click();
  await p.waitForSelector(BOX);
  if ((await p.locator(switchOf(id)).isChecked()) !== on) {
    await Promise.all([waitSurfaceAction(p, 'host-voice', action), p.locator(switchOf(id)).click()]);
  }
  await until(p, async () => done(await status(p)), `${action} should be ${on ? 'on' : 'off'}`);
  await p.locator(`${BOX} [data-component-id="close"] button`).click();
  await p.waitForSelector(BOX, { state: 'detached' });
}

/** Sets the hands-free switch to `on` (the user's gesture) through the box. */
const handsFree = (p, on) => setSwitch(p, 'd-hands-free', 'hands_free', on, (s) => s.config.hands_free === on);

/** Sets the read-aloud switch to `on` through the box. */
const readAloud = (p, on) => setSwitch(p, 's-read-aloud', 'read_aloud', on, (s) => s.config.read_aloud === on);

module.exports = { BOX, gear, handsFree, readAloud, setSwitch, status, switchOf };
