// Attente de la réponse de l'hôte à une action de surface (`surface_action`) envoyée par l'interface.
const assert = require('node:assert/strict');

/** Le corps JSON d'une requête, ou `null` s'il ne se lit pas. */
const bodyOf = (request) => {
  try {
    return JSON.parse(request.postData() ?? '');
  } catch {
    return null;
  }
};

/**
 * Attend la réponse à l'action `name` de la surface `surface`, et qu'elle soit acceptée.
 * À appeler avant le geste qui l'envoie.
 */
const waitSurfaceAction = async (page, surface, name) => {
  const response = await page.waitForResponse((r) => {
    if (!r.url().endsWith('/api/agent')) return false;
    const body = bodyOf(r.request());
    return body?.cmd === 'surface_action' && body.surface === surface && body.name === name;
  });
  assert.ok(response.ok(), `action ${surface}/${name} refusée : ${response.status()}`);
  return response;
};

module.exports = { waitSurfaceAction };
