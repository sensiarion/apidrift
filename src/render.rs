use crate::matcher::SchemaMatchResult;
use std::error::Error;

pub mod html;

/// Trait for rendering schema match results in different formats
pub trait Renderer {
    /// Render the schema match results and return the output as a string
    fn render(&self, results: &[SchemaMatchResult]) -> Result<String, Box<dyn Error>>;
    
    /// Get the file extension for this renderer
    fn file_extension(&self) -> &str;
}

