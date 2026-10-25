use rstest::rstest;
use snafu::IntoError;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(CheckTimeoutSnafu.build(), UpdateStage::Check)]
#[case(TooLargeSnafu.build(), UpdateStage::Download)]
#[case(UntrustedSnafu { url: "x".to_owned() }.build(), UpdateStage::Download)]
#[case(UnavailableSnafu.build(), UpdateStage::Unavailable)]
#[case(RefusedSnafu { why: "agent window" }.build(), UpdateStage::Refused)]
#[case(
    DownloadSnafu { url: "x", endpoint: UpdateSource::GitHub }
        .into_error(tauri_plugin_updater::Error::Network("403".to_owned())),
    UpdateStage::Download
)]
fn stage_of_error_names_its_message(#[case] inner: Inner, #[case] expected: UpdateStage) {
    assert_that!(stage_of(&inner)).is_equal_to(expected);
}

#[test]
fn download_error_display_names_url_endpoint_and_cause() {
    let url = "https://updates.okutheory.com/v5.1.1/Oku.Slides_5.1.1_amd64.AppImage";
    let error = DownloadSnafu { url, endpoint: UpdateSource::Mirror }
        .into_error(tauri_plugin_updater::Error::Network("403 Forbidden".to_owned()));
    let line = one_line(&error);
    assert_that!(line.contains(url)).is_true();
    assert_that!(line.contains("(mirror)")).is_true();
    assert_that!(line.contains("403 Forbidden")).is_true();
}
