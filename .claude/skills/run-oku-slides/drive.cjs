// Pilote d'Oku Slides lancé par `serve.sh` : ouvre l'app dans Chromium (Playwright de `ui/`),
// envoie éventuellement une consigne à l'agent, déplie le fil, fait une capture et imprime
// le texte du fil et les erreurs de la console.
//
// node .claude/skills/run-oku-slides/drive.cjs [--url U] [--prompt TEXTE] [--expand] [--shot FICHIER]
//   --url     l'app (défaut http://127.0.0.1:4991/)
//   --prompt  consigne envoyée à l'agent de l'onglet t-1 ; attend la fin de son tour
//   --expand  déplie la timeline du dernier tour, chaque étape et chaque appel d'outil
//   --shot    capture de la page (défaut /tmp/oku-slides-shots/page.png)

const path = require('node:path');
const fs = require('node:fs');
const { createRequire } = require('node:module');

// Playwright est une dépendance de `ui/` (e2e) : résolu depuis là, pas depuis ce dossier.
const root = path.resolve(__dirname, '../../..');
const { chromium } = createRequire(path.join(root, 'ui/package.json'))('playwright');

const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const i = args.indexOf(name);
  return i < 0 ? fallback : args[i + 1];
};
const url = opt('--url', 'http://127.0.0.1:4991/');
const prompt = opt('--prompt', null);
const expand = args.includes('--expand');
const shot = opt('--shot', '/tmp/oku-slides-shots/page.png');

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(url);
  await page.locator('.agent-thread').waitFor({ state: 'attached' });
  if (prompt) {
    // Même chemin que l'interface : POST /api/agent (en-tête JSON exigé par la garde anti-CSRF).
    await page.evaluate(
      (text) =>
        fetch('/api/agent', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ cmd: 'prompt', tab: 't-1', text, ui: {} }),
        }),
      prompt,
    );
    // Fin du tour : la durée (« · N s ») n'apparaît dans le résumé qu'une fois le tour fini.
    await page.locator('.agent-turn').last().locator('.a2ui-timeline-summary').filter({ hasText: / s$/ }).waitFor();
  }
  if (expand) {
    const turn = page.locator('.agent-turn').last();
    const summary = turn.locator('.a2ui-timeline-summary');
    if ((await summary.getAttribute('aria-expanded')) !== 'true') await summary.click();
    // Les étapes puis les appels : déplier une étape fait apparaître ses appels.
    // La première ligne repliée tant qu'il en reste : `.all()` rendrait des `nth(i)` réévalués,
    // que chaque dépliage décale.
    for (const selector of ['.a2ui-step-toggle', '.a2ui-call-toggle']) {
      const folded = turn.locator(`${selector}[aria-expanded="false"]`);
      while ((await folded.count()) > 0) await folded.first().click();
    }
  }
  fs.mkdirSync(path.dirname(shot), { recursive: true });
  await page.screenshot({ path: shot, fullPage: true });
  console.log(`capture : ${shot}`);
  console.log('--- fil de l’agent ---');
  console.log(await page.locator('.agent-thread').innerText());
  console.log(`--- erreurs de la console : ${errors.length} ---`);
  for (const e of errors) console.log(e);
  await browser.close();
  process.exit(errors.length ? 1 : 0);
})().catch((e) => {
  console.error(e);
  process.exit(2);
});
