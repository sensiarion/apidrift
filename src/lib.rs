pub mod matcher;
pub mod render;

/// Level of specific change
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeLevel {
    Breaking,
    Warning,
    Change,
}

