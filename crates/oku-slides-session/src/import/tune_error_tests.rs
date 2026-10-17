use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(NoImportSnafu.build(), TuneErrorKind::NoImport, "session-import-tune-none", "Aucun import")]
#[case(HifiOnlySnafu.build(), TuneErrorKind::HifiOnly, "session-import-tune-hifi-only", "retrofit")]
#[case(
    NoReferenceSnafu { cause: ReferenceCause::Timeout }.build(),
    TuneErrorKind::NoReference,
    "session-import-tune-no-reference",
    "trop de temps"
)]
#[case(
    ReferencePendingSnafu.build(),
    TuneErrorKind::ReferencePending,
    "session-import-tune-reference-pending",
    "en cours"
)]
#[case(UnknownSlideSnafu { slide: "s9" }.build(), TuneErrorKind::UnknownSlide, "session-import-tune-unknown-slide", "s9")]
#[case(
    UnknownRegionSnafu { slide: "s1", region: "s1/42" }.build(),
    TuneErrorKind::UnknownRegion,
    "session-import-tune-unknown-region",
    "s1/42"
)]
#[case(TriesUsedSnafu { slide: "s2", tries: 3u32 }.build(), TuneErrorKind::TriesUsed, "session-import-tune-tries-used", "s2")]
#[case(
    UntunableSnafu { slide: "s3", state: SlideState::Witness }.build(),
    TuneErrorKind::Untunable,
    "session-import-tune-untunable",
    "témoin"
)]
#[case(CompareSnafu { detail: "x" }.build(), TuneErrorKind::Compare, "session-import-tune-compare", "")]
fn tune_error_has_its_kind_and_a_french_message(
    #[case] inner: TuneInner,
    #[case] kind: TuneErrorKind,
    #[case] key: &str,
    #[case] fragment: &str,
) {
    let e = TuneError::from(inner);
    assert_that!(e.kind()).is_equal_to(kind);
    let fr = Localizer::new(Some("fr"));
    let localized = e.localize(&fr);
    assert_that!(fr.has(key)).is_true();
    assert_that!(localized.as_str()).contains(fragment);
    assert_that!(localized.as_str()).does_not_contain("session-import");
}
