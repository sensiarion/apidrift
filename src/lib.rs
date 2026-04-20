pub mod diff;
pub mod matcher;
pub mod parse_error;
pub mod render;
pub mod rules;

pub use diff::{diff_openapi, diff_openapi_to_html, DiffStats, OpenApiInputFormat, ReportFormat};

/// Level of specific change
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeLevel {
    Breaking,
    Warning,
    Change,
}
