// Agent détaché : la fenêtre principale perd sa colonne, la fenêtre de l'agent (ici une page
// `?window=agent` ; le canal passe par BroadcastChannel) a le fil, le brouillon et la dictée.
// Contre `oku-slides serve` lancé avec le faux agent (just e2e s'en charge).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const ctx = await b.newContext({ viewport: { width: 1440, height: 900 } });
  const main = await ctx.newPage();
  const errors = [];
  main.on('pageerror', (e) => errors.push(e.message));
  try {
    await main.goto(URL);
    await post(main, '/api/workspace', { cmd: 'activate', tab: 't-1' });
    await post(main, '/api/workspace', { cmd: 'detach_agent' });
    await main.reload();
    await main.waitForSelector('.stage img');

    // 1. La fenêtre principale n'a plus le fil.
    assert.equal(await main.locator('.agent-thread').count(), 0);

    // 2. La fenêtre de l'agent l'a, avec sa dictée ; on part d'une conversation vide.
    await post(main, '/api/agent', { cmd: 'new_conversation', tab: 't-1' });
    const agent = await ctx.newPage();
    agent.on('pageerror', (e) => errors.push(e.message));
    await agent.goto(`${URL}?window=agent`);
    await agent.waitForSelector('.agent-window .agent-thread');
    // La fenêtre de l'agent montre toujours « Rattacher » : son fil n'est jamais `.vacant`, on attend l'absence de tours.
    await agent.waitForFunction(() => !document.querySelector('.agent-turn'));

    // 3. Une consigne écrite dans le brouillon de la fenêtre de l'agent ouvre un tour.
    await agent.keyboard.press('ControlOrMeta+k');
    await agent.locator('.draft-text').fill('depuis la fenêtre agent');
    await agent.locator('.draft-send').click();
    await agent.locator('.agent-turn', { hasText: 'depuis la fenêtre agent' }).waitFor();
    await agent.waitForSelector('.draft-text', { state: 'detached' });

    // 4. ⌘K dans la fenêtre principale ouvre le brouillon de la fenêtre de l'agent.
    await main.bringToFront();
    await main.keyboard.press('ControlOrMeta+k');
    await agent.locator('.draft-text').waitFor();

    // 5. Le brouillon survit au rattachement : « Rattacher » envoie la frappe en attente, puis rattache.
    await agent.keyboard.press('ControlOrMeta+k');
    await agent.locator('.draft-text').fill('brouillon gardé');
    await agent.locator('.agent-attach').click();
    await main.reload();
    await main.waitForSelector('.notes .agent-thread');
    await main.locator('.draft-note', { hasText: 'brouillon gardé' }).waitFor();

    // Rangement : brouillon fermé, conversation vidée.
    await main.locator('.draft-discard').click();

    // 6. Sans fenêtre de l'agent (ouverture échouée), la fenêtre principale garde de quoi rattacher.
    await post(main, '/api/workspace', { cmd: 'detach_agent' });
    await main.reload();
    // Le séparateur du fil part avec lui (régression : il restait visible, sous son enveloppe `display: contents`).
    await main.waitForSelector('.table.agent-detached');
    assert.equal(await main.locator('.col-divider[data-side="notes"]').isVisible(), false);
    await main.locator('.table.agent-detached .agent-reattach').click();
    await main.reload();
    await main.waitForSelector('.notes .agent-thread');
    assert.deepEqual(errors, []);
  } finally {
    await post(main, '/api/agent', { cmd: 'new_conversation', tab: 't-1' }).catch(() => {});
    await post(main, '/api/agent', { cmd: 'set_draft', tab: 't-1', text: null, origin: 'e2e' }).catch(() => {});
    await post(main, '/api/workspace', { cmd: 'attach_agent' }).catch(() => {});
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
