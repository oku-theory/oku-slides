// Reads the stage from the page: the visible image, its alt text and its render (SVG).
// An image of another slide (still there during a change) or of a hidden item view is skipped.

/** L'image visible de la scène : `alt` (« Slide n », sans les marques d'isolement de Fluent) et `svg` ;
 *  `{ alt: '', svg: '' }` s'il n'y en a pas encore. */
const stageImage = (p) => p.evaluate(async () => {
  const img = [...document.querySelectorAll('.stage img')].find((i) => i.checkVisibility());
  if (!img) return { alt: '', svg: '' };
  return { alt: img.alt.replace(/[⁨⁩]/g, ''), svg: await (await fetch(img.src)).text() };
});

/** Le SVG de l'image visible de la scène ; `''` s'il n'y en a pas encore. */
const stageSvg = async (p) => (await stageImage(p)).svg;

module.exports = { stageImage, stageSvg };
