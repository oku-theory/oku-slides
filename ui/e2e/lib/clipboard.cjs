// Copier-coller d'un élément de la scène dans les specs : montrer une slide, envoyer copy ou paste.
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');

/** Montre la slide `id` de l'onglet actif (sa miniature), image chargée. */
const show = async (p, tab, id) => {
  const n = (await post(p, '/api', { tab, req: { cmd: 'snapshot' } })).slides.findIndex((s) => s.id === id) + 1;
  assert.ok(n > 0, `slide ${id} absente de ${tab}`);
  await p.click(`.thumb >> nth=${n - 1}`);
  await p.waitForFunction((n) => {
    const img = document.querySelector('.stage img');
    return img?.alt.replace(/[⁨⁩]/g, '') === `Slide ${n}` && img.complete && img.naturalWidth > 0;
  }, n);
};

/** Envoie `kind` (copy ou paste) à la page, avec `text` au presse-papiers ; le texte qu'il y laisse. */
const clipboard = (p, kind, text = '') =>
  p.evaluate(([kind, text]) => {
    const dt = new DataTransfer();
    if (text) dt.setData('text/plain', text);
    document.body.dispatchEvent(new ClipboardEvent(kind, { clipboardData: dt, bubbles: true, cancelable: true }));
    return dt.getData('text/plain');
  }, [kind, text]);

module.exports = { clipboard, show };
