// Lecture de l'aperçu d'un widget à l'atelier : le SVG de l'image du sous-onglet visible.

/** L'image de l'aperçu du widget du sous-onglet visible. */
const PREVIEW = '.widget-tab:not([hidden]) .widget-tab-preview img';

/**
 * Le SVG de l'aperçu du widget visible ; `null` s'il n'y en a pas encore.
 * Chaque nouvel aperçu révoque l'URL blob du précédent (WidgetTab) : un `fetch` qui tombe entre la
 * révocation et la nouvelle image échoue, on relit alors l'image courante, toutes les 50 ms (t-1087).
 */
const widgetPreviewSvg = (p) => p.evaluate(async (sel) => {
  let last;
  for (let tries = 0; tries < 50; tries++) {
    const img = document.querySelector(sel);
    if (!img) return null;
    try {
      return await (await fetch(img.src)).text();
    } catch (e) {
      last = e;
      await new Promise((r) => setTimeout(r, 50));
    }
  }
  throw new Error(`aperçu du widget illisible (${document.querySelector(sel)?.src}) : ${last}`);
}, PREVIEW);

module.exports = { PREVIEW, widgetPreviewSvg };
