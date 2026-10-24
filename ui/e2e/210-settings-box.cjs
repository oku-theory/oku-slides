// La boîte des réglages de la dictée et de l'agent (`host-voice`), ouverte par le ⚙ de la dictée :
// changer d'agent et lire la réponse du second, changer la langue (le micro suit), « Local » sans
// modèle, puis le modèle API : l'adresse et la variable de la clé se règlent dans la boîte, et la
// valeur de la clé n'apparaît nulle part (page, champs, évènements, réponses, journal de l'agent).
// Contre `oku-slides serve` lancé par `just e2e` (deux presets : « Faux » choisi, « Faux bis » ;
// `OKU_E2E_DICTATION_KEY` exportée). L'agent, la langue, le modèle, l'adresse de l'API et la
// variable de la clé sont ceux de tout le serveur : la spec les remet comme elle les a trouvés.
const fs = require('node:fs');
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until, untilValue } = require('./lib/until.cjs');
const { waitSurfaceAction } = require('./lib/surfaceAction.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

const SECRET = 'oku-e2e-secret-1144';
const KEY_VAR = 'OKU_E2E_DICTATION_KEY';
/** Un port local fermé : aucun audio ne quitte jamais la machine. */
const CLOSED_URL = 'http://127.0.0.1:9/v1';
const BOX = '.agent-dock [data-surface-id="host-voice"]';
const part = (id) => `${BOX} [data-component-id="${id}"]`;
const chip = (p, id, name) => p.locator(`${part(id)} .chip`, { hasText: new RegExp(`^\\s*${name}\\s*$`) });
const apiChip = (p) => p.locator(`${part('d-models')} .chip`, { hasText: /^\s*API\b/ });
const isChosen = async (c) => (await c.getAttribute('aria-checked')) === 'true';
const untilChosen = (p, id, name) => until(p, () => isChosen(chip(p, id, name)), `« ${name} » devrait être choisi`);
const status = (p) => post(p, '/api/stt', { cmd: 'status' });
const prompt = (p, text, tab = 't-1') => post(p, '/api/agent', { cmd: 'prompt', tab, text, ui: {} });
/** Le ⚙ de la barre de dictée. */
const gear = (p) => p.locator('.device.dictation button[aria-label="Réglages"]');
const micTitle = (p) => p.locator('.device.dictation .mic').first().getAttribute('title');

/**
 * Clique `target` et attend que l'hôte accepte l'action `name` de la boîte. Attente et geste
 * sont attendus ensemble : un geste qui échoue ne laisse pas de rejet orphelin.
 */
const clickAction = async (p, target, name) => {
  await Promise.all([waitSurfaceAction(p, 'host-voice', name), target.click()]);
};
/** Remplit le champ `id` de la boîte avec `value` et le valide (Entrée) ; rien s'il l'a déjà. */
const fillField = async (p, id, value) => {
  const input = p.locator(`${part(id)} input`);
  await input.waitFor();
  if ((await input.inputValue()) === value) return;
  await Promise.all([
    waitSurfaceAction(p, 'host-voice', 'api'),
    (async () => {
      await input.fill(value);
      await input.press('Enter');
    })(),
  ]);
};
/** Ouvre la boîte par le ⚙ si elle n'est pas montrée. */
const openBox = async (p) => {
  if ((await p.locator(BOX).count()) > 0) return;
  await gear(p).click();
  await p.waitForSelector(BOX);
};

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  // Le serveur envoie des évènements nommés (`mcp`, `agent`) : la page en garde une copie.
  await p.addInitScript(() => {
    const Native = window.EventSource;
    window.__sse = [];
    window.EventSource = class extends Native {
      constructor(...a) {
        super(...a);
        for (const n of ['mcp', 'agent']) this.addEventListener(n, (e) => window.__sse.push(e.data));
      }
    };
  });
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');

  // 1. L'état de départ : modèle, langue, adresse et variable de l'API.
  const start = await status(p);
  let startModel = null;
  let failed = null;
  try {
    // 2. Le ⚙ ouvre la boîte dans le dock ; le modèle choisi au départ est retenu par son libellé.
    await gear(p).click();
    await p.waitForSelector(BOX);
    startModel = (await p.locator(`${part('d-models')} .chip[aria-checked="true"]`).textContent())?.trim() ?? null;
    assert.ok(startModel, 'un modèle de dictée devrait être choisi');

    // 3. « Faux » est choisi ; choisir « Faux bis ».
    assert.ok(await isChosen(chip(p, 'a-agents', 'Faux')), '« Faux » devrait être choisi');
    await clickAction(p, chip(p, 'a-agents', 'Faux bis'), 'select');
    await untilChosen(p, 'a-agents', 'Faux bis');

    // 4. Le second agent répond au prochain tour ; la boîte reste ouverte.
    await prompt(p, 'bonjour');
    await p.locator('.agent-turn', { hasText: "Réponse de l'agent bis." }).waitFor();
    assert.equal(await p.locator(BOX).count(), 1, 'la boîte devrait rester ouverte après le tour');

    // 5. La langue : le micro la montre ; puis celle du départ.
    const language = p.locator(`${part('d-language')} select`);
    await language.selectOption({ label: 'English' });
    await until(p, async () => ((await micTitle(p)) ?? '').includes('English'), 'le micro devrait dire English');
    await language.selectOption({ value: start.config.language ?? '' });
    await untilValue(p, () => status(p), (s) => (s.config.language ?? null) === (start.config.language ?? null), 'langue de départ');

    // 6. « Local » sans modèle : sur Apple silicon, la boîte propose de le télécharger (jamais cliqué :
    // 2,5 Go) ; ailleurs, l'agent local ne tourne pas et rien n'est proposé.
    await clickAction(p, chip(p, 'a-agents', 'Local'), 'select');
    await untilChosen(p, 'a-agents', 'Local');
    if (process.platform === 'darwin' && process.arch === 'arm64') {
      await p.locator(part('a-download')).waitFor();
    } else {
      assert.equal(await p.locator(part('a-download')).count(), 0, 'aucun téléchargement hors Apple silicon');
    }

    // 7. Retour à « Faux ».
    await clickAction(p, chip(p, 'a-agents', 'Faux'), 'select');
    await untilChosen(p, 'a-agents', 'Faux');

    // 8. Le modèle API : adresse et variable réglées dans la boîte ; la clé reste invisible.
    const bodies = [];
    const collect = async (r) => {
      if (!/\/api\/(agent|stt)$/.test(r.url())) return;
      try {
        bodies.push(await r.text());
      } catch {
        // Corps déjà libéré (navigation) : rien à lire.
      }
    };
    p.on('response', collect);
    await clickAction(p, apiChip(p), 'model');
    await fillField(p, 'd-api-url', CLOSED_URL);
    await fillField(p, 'd-api-key-env', KEY_VAR);
    // Témoin : la variable est bien vue par le serveur (la clé est trouvée).
    await untilValue(
      p,
      () => status(p),
      (s) => s.models.find((m) => m.id === 'api')?.installed === true,
      'lancer la spec avec just e2e : la variable du test manque',
    );
    p.off('response', collect);
    assert.ok(!(await p.content()).includes(SECRET), 'la clé dans la page');
    const values = await p.evaluate(() => [...document.querySelectorAll('input, textarea, select')].map((e) => e.value));
    assert.ok(!values.some((v) => v.includes(SECRET)), 'la clé dans un champ');
    assert.ok(!(await p.evaluate(() => window.__sse.join('\n'))).includes(SECRET), 'la clé dans un évènement');
    assert.ok(!bodies.some((t) => t.includes(SECRET)), 'la clé dans une réponse');
    // Le journal doit exister et avoir servi : un journal absent ne prouverait rien.
    const logPath = process.env.E2E_AGENT_LOG;
    assert.ok(logPath, 'lancer la spec avec just e2e : E2E_AGENT_LOG manque');
    assert.ok(fs.existsSync(logPath), `journal de l'agent introuvable : ${logPath}`);
    const log = fs.readFileSync(logPath, 'utf8');
    assert.ok(log.length > 0, "journal de l'agent vide");
    assert.ok(!log.includes(SECRET), "la clé dans le journal de l'agent");

    // 9. Fermer : la boîte quitte le dock.
    await p.locator(`${part('close')} button`).click();
    await p.waitForSelector(BOX, { state: 'detached' });
  } catch (e) {
    failed = e;
  }

  // Les specs suivantes trouvent l'agent, le modèle, la langue et l'API du départ.
  try {
    await openBox(p);
    if (!(await isChosen(chip(p, 'a-agents', 'Faux')))) await clickAction(p, chip(p, 'a-agents', 'Faux'), 'select');
    await untilChosen(p, 'a-agents', 'Faux');
    if (!(await isChosen(apiChip(p)))) await clickAction(p, apiChip(p), 'model');
    await fillField(p, 'd-api-url', start.config.api.url);
    await fillField(p, 'd-api-key-env', start.config.api.key_env);
    if (startModel) {
      const back = chip(p, 'd-models', startModel.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
      if (!(await isChosen(back))) await clickAction(p, back, 'model');
    }
    if ((await status(p)).config.language !== start.config.language) {
      await p.locator(`${part('d-language')} select`).selectOption({ value: start.config.language ?? '' });
    }
    await untilValue(
      p,
      () => status(p),
      (s) =>
        s.config.model === start.config.model &&
        s.config.api.url === start.config.api.url &&
        s.config.api.key_env === start.config.api.key_env &&
        (s.config.language ?? null) === (start.config.language ?? null),
      'réglages de départ rétablis',
    );
    await p.locator(`${part('close')} button`).click();
    await p.waitForSelector(BOX, { state: 'detached' });
  } catch (e) {
    if (failed) console.error('restauration :', e);
    else failed = e;
  }
  if (failed) throw failed;

  assert.deepEqual(errors, []);
  await b.close();
  console.log('settings-box : ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
