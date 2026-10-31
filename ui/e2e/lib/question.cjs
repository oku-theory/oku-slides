// The in-page questions of the close flows (useConfirm's ConfirmDialog), answered or ruled out.
const assert = require('node:assert/strict');

/**
 * Waits for the question shown, checks its message against `pattern`, then clicks `button` once
 * the dialog is armed; resolves to the match, for its groups.
 */
async function answerQuestion(p, pattern, button) {
  const dialog = p.getByRole('alertdialog');
  await dialog.waitFor();
  const message = await dialog.locator('.confirm-message').innerText();
  const match = pattern.exec(message);
  assert.ok(match, `question inattendue : ${message}`);
  await p.waitForSelector('.confirm-dialog[data-armed]');
  await dialog.getByRole('button', { name: button, exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  return match;
}

/** Fails if a question is shown now. */
async function assertNoQuestion(p) {
  const shown = p.locator('.confirm-dialog .confirm-message');
  assert.equal(await shown.count(), 0, `question inattendue : ${await shown.allInnerTexts()}`);
}

module.exports = { answerQuestion, assertNoQuestion };
