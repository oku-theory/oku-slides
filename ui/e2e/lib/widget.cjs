// The items of a pack tab in e2e: the pile's thumbnails (`PackPile`) and the selected item's preview.

/** The preview image of the selected item (its `WidgetTab`, the only one mounted). */
const PREVIEW = '.widget-tab .widget-tab-preview img';

/** The pile thumbnail named `name`, current (`aria-current="true"`) when `current` is set. */
const pileThumb = (p, name, current = false) =>
  p.locator(current ? '.pack-pile .thumb[aria-current="true"]' : '.pack-pile .thumb')
    .filter({ has: p.locator('.thumb-name', { hasText: new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`) }) });

/**
 * Adds item `name` to the pack of the active tab: "Nouveau widget" or "Nouveau layout", the name
 * typed in its field, Enter; then waits for its thumbnail, selected.
 */
async function newPackItem(p, name) {
  await p.getByRole('button', { name: /^Nouveau (widget|layout)$/ }).click();
  const field = p.getByRole('textbox', { name: /^Nom du (widget|layout)/ });
  await field.fill(name);
  await field.press('Enter');
  await pileThumb(p, name, true).waitFor();
}

/** Selects item `name` by its pile thumbnail and waits for it to be current. */
async function selectPackItem(p, name) {
  await pileThumb(p, name).click();
  await pileThumb(p, name, true).waitFor();
}

/**
 * Le SVG de l'aperçu de l'élément choisi ; `null` s'il n'y en a pas encore.
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

module.exports = { PREVIEW, pileThumb, newPackItem, selectPackItem, widgetPreviewSvg };
