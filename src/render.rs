use crate::render::report_model::DiffReport;
use std::error::Error;

pub mod html;
pub mod report_model;
pub mod yaml_agent;

/// Trait for rendering full diff reports in different formats
pub trait DiffReportRenderer {
    /// Render a diff report and return output as a string
    fn render_report(&self, report: &DiffReport) -> Result<String, Box<dyn Error>>;

    /// Get the file extension for this renderer output
    fn file_extension(&self) -> &'static str;
}
