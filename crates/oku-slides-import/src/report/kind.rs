//! Le nom de chaque sorte de perte, tel que l'écrit le rapport JSON.

use super::LossKind;

impl LossKind {
    /// Le nom de la sorte, l'étiquette `type` du JSON (`list_levels`, `overflow`…).
    pub fn name(&self) -> &'static str {
        match self {
            Self::ListLevels => "list_levels",
            Self::MergedCells => "merged_cells",
            Self::PercentStacked => "percent_stacked",
            Self::ChartFormat(_) => "chart_format",
            Self::RunStyle(_) => "run_style",
            Self::Transform => "transform",
            Self::ColorDropped(_) => "color_dropped",
            Self::TextTruncated => "text_truncated",
            Self::TableTruncated => "table_truncated",
            Self::ChartTruncated => "chart_truncated",
            Self::Opaque(_) => "opaque",
            Self::SmartArtLayout => "smart_art_layout",
            Self::UnsupportedImage(_) => "unsupported_image",
            Self::HiddenSlide => "hidden_slide",
            Self::Effects => "effects",
            Self::SeriesDropped { .. } => "series_dropped",
            Self::RepeatedDecor { .. } => "repeated_decor",
            Self::NoSlot => "no_slot",
            Self::MovedToNotes => "moved_to_notes",
            Self::ItemsDropped { .. } => "items_dropped",
            Self::PageFitted { .. } => "page_fitted",
            Self::ParagraphStyle => "paragraph_style",
            Self::ShapeStyle(_) => "shape_style",
            Self::LayoutDropped { .. } => "layout_dropped",
            Self::DecorTruncated { .. } => "decor_truncated",
            Self::TableStyle => "table_style",
            Self::EmptyData => "empty_data",
            Self::Overflow { .. } => "overflow",
        }
    }
}

#[cfg(test)]
#[path = "kind_tests.rs"]
mod kind_tests;
