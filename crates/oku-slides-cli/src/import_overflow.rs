//! Débordements relevés sur le rendu d'un import : dans les deux modes, un
//! champ de texte qui sort de sa page devient une perte `Overflow` de sa slide
//! (t-816 : un contenu plus haut que sa place). En hi-fi, seuls les champs
//! comptent : une forme ou une image qui déborde volontairement (fond perdu)
//! n'en est pas une. En retrofit, tout est placé dans la zone du thème : une
//! boîte rendue sous la marge basse de la page déborde aussi, même quand son
//! texte reste dans la page (t-895, comme le test de rendu `import_retrofit`).

use oku_slides_import::ImportReport;
use oku_slides_import::report::{Loss, LossKind, Mode};
use oku_slides_import::retrofit::PageTokens;
use oku_slides_render::Layout;

/// Marge tolérée au bord de la page, en pt (arrondis du rendu).
const TOLERANCE: f64 = 1.0;

/// Ajoute à `report` une perte `Overflow` par champ non vide de `layout` qui
/// sort de sa page et, en retrofit, par boîte d'élément qui passe sous la
/// marge basse, sur la slide qui le porte (la première fois seulement).
pub(crate) fn note_overflows(layout: &Layout, report: &mut ImportReport) {
    let page = layout.tokens.as_ref().and_then(|t| t.as_object()).map(PageTokens::from_tokens).unwrap_or_default();
    for field in layout.fields.iter().filter(|f| !f.empty) {
        let inside = field.x >= -TOLERANCE
            && field.y >= -TOLERANCE
            && field.x + field.w <= page.width + TOLERANCE
            && field.y + field.h <= page.height + TOLERANCE;
        if !inside {
            note(layout, report, field.page, format!("{}/{}", field.owner, field.field));
        }
    }
    if report.mode != Mode::Retrofit {
        return;
    }
    let bottom = page.height - page.margin_bottom + TOLERANCE;
    for b in layout.boxes.iter().filter(|b| b.y + b.h > bottom) {
        note(layout, report, b.page, b.id.clone());
    }
}

/// Ajoute la perte `Overflow` de `element` à la slide de la page `page`, une fois.
fn note(layout: &Layout, report: &mut ImportReport, page: usize, element: String) {
    let Some(slide) = layout.slides.iter().find(|s| s.page == page) else { return };
    let Some(r) = report.slides.iter_mut().find(|r| r.slide == slide.slide) else { return };
    let loss = Loss::slide(LossKind::Overflow { element });
    if !r.losses.contains(&loss) {
        r.losses.push(loss);
    }
}

#[cfg(test)]
#[path = "import_overflow_tests.rs"]
mod import_overflow_tests;
