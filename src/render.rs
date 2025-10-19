use crate::rules::MatchResult;
use std::error::Error;

pub mod html;

/// Trait for rendering match results in different formats
pub trait Renderer {
    /// Render the match results and return the output as a string
    fn render(&self, results: &[MatchResult]) -> Result<String, Box<dyn Error>>;

    /// Get the file extension for this renderer
    fn file_extension(&self) -> &str;
}
