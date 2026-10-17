use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

#[rstest]
#[case(ProbeMismatchSnafu { widget: "l1" }.build(), ImportErrorKind::Probe, "le pack généré ne se sonde pas comme il est déclaré (widget l1)")]
#[case(ProbeLimitSnafu { widget: "l2" }.build(), ImportErrorKind::Probe, "le pack généré dépasse les bornes de la sonde (widget l2)")]
#[case(UnknownPackSnafu { pack: "Nope" }.build(), ImportErrorKind::UnknownPack, "pack inconnu : Nope")]
#[case(BadPackFilesSnafu { pack: "x" }.build(), ImportErrorKind::BadPack, "pack illisible x")]
#[case(PackTooLargeSnafu { path: "/p", max: 3u64 << 20 }.build(), ImportErrorKind::TooLarge, "3")]
#[case(ReaderPanicSnafu { path: "/x.pptx" }.build(), ImportErrorKind::Presentation, "/x.pptx")]
#[case(ThresholdSnafu { value: -1.0 }.build(), ImportErrorKind::Threshold, "pas -1")]
#[case(TooLargeSnafu.build(), ImportErrorKind::TooLarge, "")]
#[case(PackMissingSnafu.build(), ImportErrorKind::Generated, "aucun pack")]
#[case(ThemeMissingSnafu.build(), ImportErrorKind::Generated, "aucun thème")]
#[case(ProbeFailedSnafu { error: "lien rompu" }.build(), ImportErrorKind::Probe, "processus de rendu : lien rompu")]
fn import_error_has_its_kind_and_a_french_message(
    #[case] inner: ImportInner,
    #[case] kind: ImportErrorKind,
    #[case] message: &str,
) {
    let e = ImportError::from(inner);
    assert_that!(e.kind()).is_equal_to(kind);
    let localized = e.localize(&fr());
    assert_that!(localized.as_str()).contains(message);
    assert_that!(localized.as_str()).does_not_contain("session-import");
}
