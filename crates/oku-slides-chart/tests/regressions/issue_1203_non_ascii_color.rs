//! A theme color of three non-ASCII bytes after `#` panicked the hex parser (not a char boundary).

use oku_slides_chart::{ChartRenderer, ChartTheme, ChartsRsRenderer, Size, parse_option};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

const SIZE: Size = Size { width: 320.0, height: 200.0 };

fn render_with_color(color: &str) -> bool {
    let theme = ChartTheme {
        background: Some(color.into()),
        text: color.into(),
        text_muted: color.into(),
        axis: color.into(),
        grid: color.into(),
        palette: vec![color.into()],
        ..ChartTheme::default()
    };
    let Ok(option) = parse_option(&json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 10], ["Fév", 12]] },
        "xAxis": { "type": "category" }, "yAxis": { "type": "value" },
        "series": [{ "type": "bar" }]
    })) else {
        return false;
    };
    ChartsRsRenderer::new(theme).render(&option, SIZE).is_ok_and(|svg| svg.starts_with("<svg"))
}

#[rstest]
#[case::two_byte_then_ascii("#é1")]
#[case::ascii_then_two_byte("#1é")]
#[case::three_byte("#€")]
#[case::three_byte_no_hash("€")]
#[case::six_bytes_three_byte("#€€")]
#[case::six_bytes_two_byte("#ééé")]
#[case::eight_bytes("#€€é")]
#[case::four_byte_emoji("#🎨")]
fn theme_color_non_ascii_renders_without_panic(#[case] color: &str) {
    assert_that!(render_with_color(color)).is_true();
}

fn render_bar_svg(palette_color: &str) -> Option<String> {
    let theme = ChartTheme { palette: vec![palette_color.into()], ..ChartTheme::default() };
    let option = parse_option(&json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 10]] },
        "xAxis": { "type": "category" }, "yAxis": { "type": "value" },
        "series": [{ "type": "bar" }]
    }))
    .ok()?;
    ChartsRsRenderer::new(theme).render(&option, SIZE).ok()
}

#[test]
fn theme_color_palette_change_changes_the_svg() {
    let black = render_bar_svg("#000");
    asserting!("the black render succeeds").that(&black).is_some();
    asserting!("the palette color reaches the SVG").that(&render_bar_svg("#fff")).is_not_equal_to(&black);
}

#[rstest]
#[case::non_ascii("#€")]
#[case::invalid_ascii("#zzz")]
fn theme_color_invalid_renders_like_black(#[case] color: &str) {
    let black = render_bar_svg("#000");
    asserting!("the black render succeeds").that(&black).is_some();
    assert_that!(render_bar_svg(color)).is_equal_to(black);
}
