// Lecture de la scène depuis la page : l'image visible, son texte alternatif et son rendu (SVG).
// Une image d'une autre slide (encore présente pendant un changement) ou d'un sous-onglet caché est écartée.

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
