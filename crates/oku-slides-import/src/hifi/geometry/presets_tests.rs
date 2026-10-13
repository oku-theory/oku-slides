use rstest::rstest;
use speculoos::prelude::*;

use super::*;

/// Un preset `p` d'un seul sous-chemin aux attributs `attrs`.
fn one_path(attrs: &str) -> PresetPath {
    let xml = format!(
        r#"<presetShapeDefinitons><p><avLst/><gdLst/><pathLst><path {attrs}><moveTo><pt x="0" y="0"/></moveTo><close/></path></pathLst></p></presetShapeDefinitons>"#
    );
    let mut presets = parse(&xml);
    let Some(mut p) = presets.remove("p") else { panic!("preset illisible") };
    p.paths.remove(0)
}

/// `stroke` se lit comme au lecteur (`attr_bool`, vrai par défaut).
#[rstest]
#[case::absent("", true)]
#[case::true_("stroke=\"true\"", true)]
#[case::on("stroke=\"on\"", true)]
#[case::false_("stroke=\"false\"", false)]
#[case::zero("stroke=\"0\"", false)]
#[case::off("stroke=\"off\"", false)]
fn path_stroke_reads_like_the_reader(#[case] attrs: &str, #[case] stroke: bool) {
    assert_that!(one_path(attrs).stroke).is_equal_to(stroke);
}

#[test]
fn path_fill_none_is_unfilled() {
    assert_that!(one_path("fill=\"none\"").fill).is_false();
    assert_that!(one_path("fill=\"darken\"").fill).is_true();
}
