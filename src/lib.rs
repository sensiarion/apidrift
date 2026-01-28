pub mod matcher;
pub mod parse_error;
pub mod render;
pub mod rules;

/// Level of specific change
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeLevel {
    Breaking,
    Warning,
    Change,
}
