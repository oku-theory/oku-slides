// Requêtes JSON vers `oku-slides serve` depuis la page (garde anti-CSRF : JSON déclaré).
const post = (p, endpoint, req) =>
  p.evaluate(
    ([e, r]) => fetch(e, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(r) }).then((x) => x.json()),
    [endpoint, req],
  );
/** L'onglet actif de la (seule) fenêtre de `serve`. */
const activeTab = async (p) => (await post(p, '/api/workspace', { cmd: 'view' })).windows[0].active;
/** L'instantané de l'onglet actif. */
const activeSnapshot = async (p) => post(p, '/api', { tab: await activeTab(p), req: { cmd: 'snapshot' } });
module.exports = { post, activeTab, activeSnapshot };
