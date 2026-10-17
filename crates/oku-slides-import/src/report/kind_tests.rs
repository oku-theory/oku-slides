use rstest::rstest;
use speculoos::prelude::*;

use super::super::{RunEffect, ShapeEffect};
use super::*;
use crate::source::OpaqueReason;

#[rstest]
#[case(LossKind::ListLevels)]
#[case(LossKind::MergedCells)]
#[case(LossKind::PercentStacked)]
#[case(LossKind::ChartFormat("0%".into()))]
#[case(LossKind::RunStyle(RunEffect::Underline))]
#[case(LossKind::Transform)]
#[case(LossKind::ColorDropped("#ff0000".into()))]
#[case(LossKind::TextTruncated)]
#[case(LossKind::TableTruncated)]
#[case(LossKind::ChartTruncated)]
#[case(LossKind::Opaque(OpaqueReason::Media))]
#[case(LossKind::SmartArtLayout)]
#[case(LossKind::UnsupportedImage("image/x-emf".into()))]
#[case(LossKind::HiddenSlide)]
#[case(LossKind::Effects)]
#[case(LossKind::SeriesDropped { kept: 1, dropped: 2 })]
#[case(LossKind::RepeatedDecor { text: "x".into(), slides: 2 })]
#[case(LossKind::NoSlot)]
#[case(LossKind::MovedToNotes)]
#[case(LossKind::ItemsDropped { dropped: 3 })]
#[case(LossKind::PageFitted { width: 720, height: 540 })]
#[case(LossKind::ParagraphStyle)]
#[case(LossKind::ShapeStyle(ShapeEffect::Dash))]
#[case(LossKind::LayoutDropped { layout: "Titre".into() })]
#[case(LossKind::DecorTruncated { omitted: 1 })]
#[case(LossKind::TableStyle)]
#[case(LossKind::EmptyData)]
#[case(LossKind::Overflow { element: "s3-2".into() })]
fn loss_kind_name_is_the_type_tag_written_in_json(#[case] kind: LossKind) {
    let json = serde_json::to_value(&kind).unwrap();
    assert_that!(json["type"].as_str()).is_equal_to(Some(kind.name()));
}
