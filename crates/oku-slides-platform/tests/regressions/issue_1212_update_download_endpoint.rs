//! t-1212: behind a corporate proxy, an update download failed with "403 Forbidden" and the
//! log named no URL, so nobody could tell whether the mirror or GitHub refused it. The host now
//! logs the bundle URL with the endpoint it comes from, which `download_source` names.

use oku_slides_platform::updates::{UpdateSource, download_source};
use rstest::rstest;
use speculoos::prelude::*;

#[rstest]
#[case("https://updates.okutheory.com/v5.1.1/Oku.Slides_5.1.1_amd64.AppImage", UpdateSource::Mirror)]
#[case(
    "https://github.com/oku-theory/oku-slides/releases/download/v5.1.1/Oku.Slides_5.1.1_amd64.AppImage",
    UpdateSource::GitHub
)]
fn download_source_linux_bundle_names_its_endpoint(#[case] url: &str, #[case] expected: UpdateSource) {
    assert_that!(download_source(url)).is_equal_to(Some(expected));
}
