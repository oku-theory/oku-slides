// Duplicate and reorder in a pack tab's pile (t-1249): a layout pack made from the main menu gets the
// layouts `a` and `b` ("Nouveau layout"); « Dupliquer » on `a` adds `a-2` right after it, selected and focused;
// ⌥↑ pressed twice on `b` moves it first (the focus survives each move, checked in the real browser);
// the order is stored in the pack version, so it is still `b`, `a`, `a-2` after closing and reopening
// the tab. The "Nouvelle slide" picker lists a pinned pack's layouts in that order, but a library pack
// made from the menu is not pinned by the deck and no short UI path pins it here: that order is covered
// by the session test `layouts_view_lists_a_pack_s_layouts_in_its_order`.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { closePackTabs, packTabShown, openPackTab } = require('./lib/atelier.cjs');
const { deleteLibraryPack } = require('./lib/packs.cjs');
const { pileThumb, newPackItem } = require('./lib/widget.cjs');
const { untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = 't-1';

const api = (p, tab, req) => post(p, '/api', { tab, req });
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
/** The pile's item names, in display order. */
const names = (p) => p.locator('.pack-pile .thumb .thumb-name').allTextContents().then((l) => l.map((n) => n.trim()));
const expectOrder = (p, order, what) => untilValue(p, () => names(p), (l) => l.join() === order.join(), `${what} : pile ${order.join(', ')} attendue`);
/** The name of the pile thumbnail that has the keyboard focus; `''` when none. */
const focused = (p) => p.evaluate(() => document.activeElement?.closest('.thumb')?.querySelector('.thumb-name')?.textContent?.trim() ?? '');

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');

  // 1. "Nouveau pack de layouts" opens an empty layout pack; add `a` and `b`.
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Nouveau pack de layouts' }).click();
  await packTabShown(p);
  const tab = (await view(p)).windows[0].active;
  assert.notEqual(tab, TAB, "« Nouveau pack de layouts » n'ouvre pas d'onglet");
  const editing = (await api(p, tab, { cmd: 'snapshot' })).editing;
  assert.equal(editing?.kind, 'layouts', `l'onglet n'édite pas un pack de layouts : ${JSON.stringify(editing)}`);
  const packId = editing.pack;
  try {
    await newPackItem(p, 'a');
    await newPackItem(p, 'b');
    await expectOrder(p, ['a', 'b'], 'ajout');

    // 2. « Dupliquer » on `a`: the copy `a-2` comes right after it, selected and focused.
    await pileThumb(p, 'a').click();
    await pileThumb(p, 'a', true).waitFor();
    // The context menu entry: ⌘D stays the window's dictation shortcut, the pile has none of its own.
    await pileThumb(p, 'a').click({ button: 'right' });
    await p.getByRole('menuitem', { name: 'Dupliquer' }).click();
    await expectOrder(p, ['a', 'a-2', 'b'], 'duplication');
    await pileThumb(p, 'a-2', true).waitFor();
    await untilValue(p, () => focused(p), (n) => n === 'a-2', 'la copie a-2 ne prend pas le focus');

    // 3. ⌥↑ twice on `b`: first, with the focus kept between the two presses.
    await pileThumb(p, 'b').click();
    await pileThumb(p, 'b', true).waitFor();
    await p.keyboard.press('Alt+ArrowUp');
    await expectOrder(p, ['a', 'b', 'a-2'], 'premier ⌥↑');
    await untilValue(p, () => focused(p), (n) => n === 'b', 'le focus quitte b après le premier ⌥↑');
    await p.keyboard.press('Alt+ArrowUp');
    await expectOrder(p, ['b', 'a', 'a-2'], 'second ⌥↑');
    await untilValue(p, () => focused(p), (n) => n === 'b', 'le focus quitte b après le second ⌥↑');

    // 4. Stored in the pack version: save it in place (no copy to clean up), close, reopen the saved pack, same order.
    const saved = await api(p, tab, { cmd: 'save_pack', as_new: false });
    assert.equal(saved.error, undefined, `save_pack refusé : ${saved.error}`);
    const closed = await post(p, '/api/workspace', { cmd: 'close', tab, discard: true });
    assert.equal(closed.error, undefined, `fermeture refusée : ${closed.error}`);
    const library = (await api(p, TAB, { cmd: 'packs', kind: 'layouts' })).find((pack) => pack.id === packId);
    assert.ok(library, "le pack de layouts n'est pas dans la bibliothèque");
    await post(p, '/api/workspace', { cmd: 'activate', tab: TAB });
    await openPackTab(p, TAB, packId);
    await p.reload();
    await packTabShown(p);
    await expectOrder(p, ['b', 'a', 'a-2'], 'réouverture');
  } finally {
    await closePackTabs(p);
    await deleteLibraryPack(p, TAB, packId);
  }
  assert.deepEqual(errors, []);
  console.log('pile : dupliquer et réordonner ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
