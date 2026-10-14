//! Régression (t-755) : des catégories à plusieurs niveaux (`multiLvlStrRef`) donnaient un graphique sans catégorie.

use oku_slides_import::read_pptx;
use oku_slides_import::source::{Item, ItemKind};
use speculoos::prelude::*;

use crate::common;

fn categories(items: &[Item]) -> Option<Vec<String>> {
    items.iter().find_map(|i| match &i.kind {
        ItemKind::Chart(c) => Some(c.categories.clone()),
        ItemKind::Group { items, .. } => categories(items),
        _ => None,
    })
}

#[test]
fn multi_level_categories_keep_their_leaf_labels() {
    let chart = common::text("kitchen/ppt/charts/chart1.xml");
    let find = |tag: &str| chart.find(tag).unwrap_or_else(|| panic!("{tag} absent de la fixture chart1.xml"));
    let (start, end) = (find("<c:cat>"), find("</c:cat>") + "</c:cat>".len());
    let multi = r#"<c:cat><c:multiLvlStrRef><c:f>Feuil1!$A$2:$B$4</c:f><c:multiLvlStrCache><c:ptCount val="3"/><c:lvl><c:pt idx="0"><c:v>T1</c:v></c:pt><c:pt idx="1"><c:v>T2</c:v></c:pt><c:pt idx="2"><c:v>T3</c:v></c:pt></c:lvl><c:lvl><c:pt idx="0"><c:v>2026</c:v></c:pt></c:lvl></c:multiLvlStrCache></c:multiLvlStrRef></c:cat>"#;
    let chart = format!("{}{multi}{}", &chart[..start], &chart[end..]);
    let deck = read_pptx(&common::pptx_with("kitchen", &[("ppt/charts/chart1.xml", Some(&chart))]), &common::limits())
        .unwrap_or_else(|e| panic!("lecture : {e}"));
    let found = deck.slides.iter().find_map(|s| categories(&s.items));
    assert_that!(found).is_equal_to(Some(vec!["T1".to_owned(), "T2".to_owned(), "T3".to_owned()]));
}
